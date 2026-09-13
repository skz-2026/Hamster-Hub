//! Win+D 守卫：桌面接管期间拦截系统「显示桌面」热键。
//!
//! Win+D 由 explorer 在内部处理（不走 RegisterHotKey，全局热键注册抢不到，
//! 也无法通过隐藏任务栏解除），桌面模式下按下会把全屏主窗最小化，露出一个
//! 无图标无任务栏的黑屏。这里用 WH_KEYBOARD_LL 低级钩子在输入到达 shell
//! 之前吞掉 Win+D，改为回调上层（仓鼠版「显示桌面」），接管保持不动。
//!
//! 修饰键判定用 GetAsyncKeyState（系统原始输入层维护的物理键状态）——
//! 不能用钩子事件流自己记账：shell 任务视图/开始菜单等会吃掉部分按键
//! 事件（实测日志出现大量有来无回的 ESC↑/TAB↑/Alt↑），自记账的修饰键位
//! 会卡在「按下」，把 Win+D 误判成 Ctrl+Win+D 放行 → 触发系统「新建虚拟
//! 桌面」，用户被切到只剩裸壁纸的空虚拟桌面。GetAsyncKeyState 不经钩子
//! 链，吞键不影响其准确性，无此漂移。

use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL, VK_RMENU, VK_RSHIFT,
    VK_RWIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EnumWindows, GetClassNameW, GetMessageW, GetWindowLongW,
    GetWindowThreadProcessId, IsWindowVisible, PostThreadMessageW, SetWindowsHookExW, ShowWindow,
    UnhookWindowsHookEx, GWL_EXSTYLE, HC_ACTION, KBDLLHOOKSTRUCT, MSG, SW_MINIMIZE, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP, WS_EX_TOOLWINDOW,
};

/// 'D' 的虚拟键码（字母键与 ASCII 同值，SDK 未提供常量）
const VK_D: u16 = 0x44;

/// 守卫开关：false 时钩子（若残留）直通一切按键
static ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// 同一次 Win 按压只触发一次回调（按住 D 的 key repeat 不会重复退出）
static TRIGGERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static HOOK_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static CALLBACK: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = Mutex::new(None);

fn key_down(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

/// 纯 Win 按下（排除 Ctrl/Alt/Shift：Ctrl+Win+D 是新建虚拟桌面等系统功能）。
/// 以系统物理键状态为准，见模块注释。
fn win_only_down() -> bool {
    let win = key_down(VK_LWIN.0 as i32) || key_down(VK_RWIN.0 as i32);
    let other = key_down(VK_LCONTROL.0 as i32)
        || key_down(VK_RCONTROL.0 as i32)
        || key_down(VK_LMENU.0 as i32)
        || key_down(VK_RMENU.0 as i32)
        || key_down(VK_LSHIFT.0 as i32)
        || key_down(VK_RSHIFT.0 as i32);
    win && !other
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code as u32 != HC_ACTION {
        return CallNextHookEx(None, code, wparam, lparam);
    }
    let msg = wparam.0 as u32;
    let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    if !is_down && !is_up {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let is_win = info.vkCode == VK_LWIN.0 as u32 || info.vkCode == VK_RWIN.0 as u32;
    // 每次物理按下 Win 重新武装触发锁（在 D 到达之前必然先看到 Win↓）
    if is_win && is_down {
        TRIGGERED.store(false, SeqCst);
    }

    let is_d = info.vkCode == VK_D as u32;
    if is_d && is_down {
        eprintln!(
            "[wind_guard] D↓ enabled={} win_only={}",
            ENABLED.load(SeqCst),
            win_only_down()
        );
    }
    // 纯 Win+D（物理状态判定）：吞掉 down/up，down 时触发一次回调
    if ENABLED.load(SeqCst) && is_d && (is_down || is_up) && win_only_down() {
        if is_down && !TRIGGERED.swap(true, SeqCst) {
            eprintln!("[wind_guard] 拦截 Win+D，触发回调");
            if let Some(cb) = CALLBACK.lock().ok().and_then(|c| c.clone()) {
                // 钩子过程必须秒回（超时会被系统摘钩），重活甩给独立线程
                std::thread::spawn(move || cb());
            }
        }
        return LRESULT(1);
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// 安装守卫（幂等；重复调用只替换回调并重新武装）。callback 在独立线程上
/// 触发，可安全执行耗时操作。
pub fn install(callback: Box<dyn Fn() + Send + Sync>) {
    if let Ok(mut slot) = CALLBACK.lock() {
        *slot = Some(Arc::from(callback));
    }
    TRIGGERED.store(false, SeqCst);
    ENABLED.store(true, SeqCst);
    if HOOK_THREAD_ID.load(SeqCst) != 0 {
        return;
    }
    if let Err(e) = std::thread::Builder::new()
        .name("wind-guard".into())
        .spawn(run_hook_thread)
    {
        eprintln!("[wind_guard] 守卫线程拉起失败，Win+D 交还系统: {e}");
        ENABLED.store(false, SeqCst);
    }
}

fn run_hook_thread() {
    unsafe {
        let my_tid = GetCurrentThreadId();
        HOOK_THREAD_ID.store(my_tid, SeqCst);
        let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[wind_guard] WH_KEYBOARD_LL 安装失败，Win+D 交还系统: {e}");
                HOOK_THREAD_ID.store(0, SeqCst);
                ENABLED.store(false, SeqCst);
                return;
            }
        };
        eprintln!("[wind_guard] 钩子已装 (tid={my_tid})");
        // LL 钩子要求装钩线程持续泵消息；WM_QUIT 到达即撤钩收摊
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
        let _ = UnhookWindowsHookEx(hook);
        // 退出竞态：stop() 可能已把 HOOK_THREAD_ID 换成 0 或继任线程的 tid
        //（快速 exit→enter 时），仅当仍归本线程所有才允许熄灯，避免误杀继任者
        if HOOK_THREAD_ID
            .compare_exchange(my_tid, 0, SeqCst, SeqCst)
            .is_ok()
        {
            ENABLED.store(false, SeqCst);
        }
        TRIGGERED.store(false, SeqCst);
        eprintln!("[wind_guard] 钩子已撤");
    }
}

/// 撤除守卫（幂等）：先关直通开关再通知钩子线程退出，恢复期间的 Win+D
/// 立即交还系统原生行为。
pub fn stop() {
    ENABLED.store(false, SeqCst);
    let tid = HOOK_THREAD_ID.swap(0, SeqCst);
    if tid != 0 {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

/// 「显示桌面」的仓鼠语义：最小化其他进程的可见顶层窗口（自家 main/
/// taskbar/spotlight 同进程全部跳过），让全屏仓鼠桌面重新露出——与系统
/// Win+D 同构，只是目标桌面是仓鼠桌面而非 Windows 裸桌面。
/// ShowWindow(SW_MINIMIZE) 不需要前台权限，被其他应用抢占焦点时同样有效。
pub fn minimize_all_except(our_pid: u32) {
    unsafe {
        let _ = EnumWindows(Some(minimize_proc), LPARAM(our_pid as isize));
    }
}

unsafe extern "system" fn minimize_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let our_pid = lparam.0 as u32;
    let mut pid = 0u32;
    let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 || pid == our_pid || !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    // 工具窗（托盘浮层、提示条等）不算「挡在桌面前」的窗口
    if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
        return BOOL(1);
    }
    let mut class_buf = [0u16; 32];
    let n = GetClassNameW(hwnd, &mut class_buf).max(0) as usize;
    // 桌面本体/外壳层就是「桌面」，不动
    if n > 0 {
        let class = String::from_utf16_lossy(&class_buf[..n]);
        if class == "Progman" || class == "WorkerW" {
            return BOOL(1);
        }
    }
    // UWP 挂起（cloaked）窗口不占屏，动了反而干扰
    let mut cloaked = 0u32;
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut u32 as *mut _,
        std::mem::size_of::<u32>() as u32,
    )
    .is_ok()
        && cloaked != 0
    {
        return BOOL(1);
    }
    let _ = ShowWindow(hwnd, SW_MINIMIZE);
    BOOL(1)
}
