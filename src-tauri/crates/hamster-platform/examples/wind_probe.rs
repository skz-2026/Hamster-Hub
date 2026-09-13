//! Win+D 拦截机制探针：装 WH_KEYBOARD_LL 20 秒，吞掉纯 Win+D，
//! 打印所有 Win/D 相关事件（vk/消息类型/Win 状态/注入标志），验证：
//! 1) LL 钩子能否看到 Win+D；2) 吞掉后系统「显示桌面」是否还被触发。
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LWIN, VK_RWIN};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PeekMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION,
    KBDLLHOOKSTRUCT, MSG, PM_REMOVE, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP,
};

fn win_down() -> bool {
    unsafe {
        let l = GetAsyncKeyState(VK_LWIN.0 as i32);
        let r = GetAsyncKeyState(VK_RWIN.0 as i32);
        (l as u16 & 0x8000) != 0 || (r as u16 & 0x8000) != 0
    }
}

unsafe extern "system" fn proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code as u32 == HC_ACTION {
        let info = &*(l.0 as *const KBDLLHOOKSTRUCT);
        let msg = w.0 as u32;
        let name = match msg {
            m if m == WM_KEYDOWN => "KEYDOWN ",
            m if m == WM_KEYUP => "KEYUP   ",
            m if m == WM_SYSKEYDOWN => "SYSDOWN ",
            m if m == WM_SYSKEYUP => "SYSUP   ",
            _ => "OTHER   ",
        };
        // 0x44='D' 0x5B=LWIN 0x5C=RWIN
        if info.vkCode == 0x44 || info.vkCode == 0x5B || info.vkCode == 0x5C {
            eprintln!(
                "[probe] {name} vk={:#04x} win_down={} flags={:#010b}",
                info.vkCode,
                win_down(),
                info.flags.0
            );
        }
        let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        if info.vkCode == 0x44 && is_down && win_down() {
            eprintln!("[probe] >>> 吞掉 Win+D（返回 1 不传给系统）");
            return LRESULT(1);
        }
    }
    CallNextHookEx(None, code, w, l)
}

fn main() {
    unsafe {
        let tid = GetCurrentThreadId();
        let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[probe] SetWindowsHookExW 失败: {e}");
                return;
            }
        };
        eprintln!("[probe] hook 已装 (tid={tid})，20 秒内按 Win+D，之后自动退出");
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut msg = MSG::default();
        while Instant::now() < deadline {
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE);
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = UnhookWindowsHookEx(hook);
        eprintln!("[probe] done");
    }
}
