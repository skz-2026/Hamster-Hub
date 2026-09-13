//! 托盘后台应用：枚举有窗口的应用（list）+ 恢复前置（activate），
//! 以及打开 Windows 原生托盘溢出弹层（open_overflow）。
//!
//! open_overflow 原理：系统任务栏（Shell_TrayWnd）被桌面接管隐藏后，托盘
//! 弹层不可达。这里短暂显示任务栏 → UIA Invoke「显示隐藏的图标」→ 系统原生
//! 的 TopLevelWindowForOverflowXamlIsland 弹层出现（独立窗口，含全部后台
//! 托盘 app）→ 再把任务栏藏回去，弹层留在屏幕上供用户直接交互。
//! 注意：不可对任务栏用 DWM cloak 之类手段「可见但不上屏」——实测会把
//! explorer/DWM 挂死（整机假死）。

use base64::Engine as _;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{w, Interface, Result, BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationInvokePattern, UIA_InvokePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GetClassLongPtrW, GetClassNameW, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SendMessageTimeoutW, SetForegroundWindow, ShowWindow, GCLP_HICON, GCLP_HICONSM, GWL_EXSTYLE,
    GWL_STYLE, GW_OWNER, HICON, ICON_BIG, ICON_SMALL, SMTO_ABORTIFHUNG, SW_HIDE, SW_RESTORE,
    SW_SHOW, SW_SHOWNA, WM_GETICON, WS_CAPTION, WS_EX_TOOLWINDOW,
};

/// 一枚后台应用窗口（托盘面板条目）
pub struct TrayApp {
    /// 窗口句柄数值（activate 目标）
    pub id: u64,
    /// 窗口标题
    pub title: String,
    /// 进程名（不含路径）
    pub process: String,
    /// 窗口图标 PNG 的 data URL（提取失败为空串，前端回退首字占位）
    pub png: String,
    pub minimized: bool,
}

struct EnumCtx {
    self_pid: u32,
    out: *mut Vec<TrayApp>,
}

/// 枚举「有窗口的后台应用」：可见、有标题、带边框的顶级窗口，
/// 排除工具窗/属主窗/幻影 cloak 窗/自身进程/系统壳窗口。
pub fn list(self_pid: u32) -> Vec<TrayApp> {
    let mut out: Vec<TrayApp> = Vec::new();
    let mut ctx = EnumCtx {
        self_pid,
        out: &mut out,
    };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut EnumCtx as isize));
    }
    out
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &*(lparam.0 as *const EnumCtx);
    if let Some(app) = inspect(hwnd, ctx.self_pid) {
        (*ctx.out).push(app);
    }
    BOOL(1)
}

/// 用户可感知窗口的基础过滤：可见、有标题、非工具窗、未被 cloak、
/// 非自身进程、非系统壳窗口；通过则返回窗口进程 pid。
/// 托盘列表（inspect）在此之上还会限制带边框、无属主。
unsafe fn user_visible_pid(hwnd: HWND, self_pid: u32) -> Option<u32> {
    // 不可见窗口直接排除；最小化窗口仍算可见（Win32 语义下保持 WS_VISIBLE）
    if !IsWindowVisible(hwnd).as_bool() {
        return None;
    }
    if GetWindowTextLengthW(hwnd) == 0 {
        return None;
    }
    // 工具窗（提示条/浮窗）不算应用窗口
    let exstyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if exstyle & WS_EX_TOOLWINDOW.0 as isize != 0 {
        return None;
    }
    // UWP 挂起/幽灵窗口被 cloak，用户看不见，跳过
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
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 || pid == self_pid {
        return None;
    }
    // 系统壳窗口（桌面/任务栏载体）不算应用窗口
    let cls = class_name(hwnd);
    if matches!(cls.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
        return None;
    }
    Some(pid)
}

unsafe fn inspect(hwnd: HWND, self_pid: u32) -> Option<TrayApp> {
    user_visible_pid(hwnd, self_pid)?;
    // 无边框窗（暂存通知/隐藏面板）不算应用窗口
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    if style & WS_CAPTION.0 as isize != WS_CAPTION.0 as isize {
        return None;
    }
    // 有属主的窗口（对话框等）跟随主窗口，不单列
    if GetWindow(hwnd, GW_OWNER)
        .map(|owner| !owner.0.is_null())
        .unwrap_or(false)
    {
        return None;
    }

    Some(TrayApp {
        id: hwnd.0 as u64,
        title: window_title(hwnd),
        process: process_name(hwnd),
        png: window_icon_png(hwnd),
        minimized: IsIconic(hwnd).as_bool(),
    })
}

fn window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}

fn process_name(hwnd: HWND) -> String {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return String::new();
    }
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok =
            QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
                .is_ok();
        let _ = CloseHandle(h);
        if ok {
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            return full.rsplit(['\\', '/']).next().unwrap_or(&full).to_string();
        }
    }
    String::new()
}

fn window_icon_png(hwnd: HWND) -> String {
    let hicon = msg_icon(hwnd, ICON_BIG as usize)
        .or_else(|| msg_icon(hwnd, ICON_SMALL as usize))
        .or_else(|| class_icon(hwnd, GCLP_HICON))
        .or_else(|| class_icon(hwnd, GCLP_HICONSM));
    hicon
        .and_then(|h| unsafe { crate::icons::hicon_png_bytes(h) })
        .map(|bytes| {
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        })
        .unwrap_or_default()
}

/// WM_GETICON 取窗口自报图标（带超时，防个别窗口挂死拖住枚举）
fn msg_icon(hwnd: HWND, wparam: usize) -> Option<HICON> {
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            WPARAM(wparam),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            300,
            Some(&mut result),
        );
    }
    to_hicon(result as isize)
}

/// 类图标兜底（窗口没自报图标时）
fn class_icon(
    hwnd: HWND,
    index: windows::Win32::UI::WindowsAndMessaging::GET_CLASS_LONG_INDEX,
) -> Option<HICON> {
    let ptr = unsafe { GetClassLongPtrW(hwnd, index) };
    to_hicon(ptr as isize)
}

fn to_hicon(v: isize) -> Option<HICON> {
    if v == 0 {
        None
    } else {
        Some(HICON(v as *mut _))
    }
}

/// 恢复并前置后台应用窗口
pub fn activate(id: u64) -> Result<()> {
    let hwnd = HWND(id as *mut core::ffi::c_void);
    if hwnd.is_invalid() {
        return Err(windows::core::Error::from_win32());
    }
    unsafe {
        let _ = ShowWindow(
            hwnd,
            if IsIconic(hwnd).as_bool() {
                SW_RESTORE
            } else {
                SW_SHOW
            },
        );
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

struct FindCtx {
    self_pid: u32,
    /// 目标 exe 文件名（小写）
    exe: String,
    out: *mut Option<u64>,
}

/// 按 exe 文件名（大小写不敏感）找一个已运行的可见顶层窗口。
/// EnumWindows 按 z 序枚举，首个命中即当前最上层的那扇。
pub fn find_by_process(self_pid: u32, exe: &str) -> Option<u64> {
    let mut found: Option<u64> = None;
    let mut ctx = FindCtx {
        self_pid,
        exe: exe.to_ascii_lowercase(),
        out: &mut found,
    };
    unsafe {
        let _ = EnumWindows(Some(find_proc), LPARAM(&mut ctx as *mut FindCtx as isize));
    }
    found
}

unsafe extern "system" fn find_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &*(lparam.0 as *const FindCtx);
    if user_visible_pid(hwnd, ctx.self_pid).is_some()
        && process_name(hwnd).to_ascii_lowercase() == ctx.exe
    {
        unsafe { *ctx.out = Some(hwnd.0 as u64) };
        return BOOL(0); // 停止枚举
    }
    BOOL(1)
}

/// 一扇命中窗口（dock 悬停卡片行）
pub struct WindowHit {
    /// 窗口句柄数值（activate 目标）
    pub id: u64,
    pub title: String,
    pub minimized: bool,
    /// 窗口图标 PNG 的 data URL（提取失败为空串）
    pub png: String,
}

/// 按 exe 文件名枚举其全部可见顶层窗口（z 序，最上层在前）。
/// dock 悬停卡片用：多开的应用列出所有窗口供用户挑一扇前置。
pub fn list_by_process(self_pid: u32, exe: &str) -> Vec<WindowHit> {
    let mut ctx = ListCtx {
        self_pid,
        exe: exe.to_ascii_lowercase(),
        out: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(Some(list_proc), LPARAM(&mut ctx as *mut ListCtx as isize));
    }
    ctx.out
}

struct ListCtx {
    self_pid: u32,
    exe: String,
    out: Vec<WindowHit>,
}

unsafe extern "system" fn list_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut ListCtx);
    if user_visible_pid(hwnd, ctx.self_pid).is_some()
        && process_name(hwnd).to_ascii_lowercase() == ctx.exe
    {
        // 与托盘列表同规格：无边框、无属主的独立用户窗口
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        if style & WS_CAPTION.0 as isize == WS_CAPTION.0 as isize {
            let owned = GetWindow(hwnd, GW_OWNER)
                .map(|owner| !owner.0.is_null())
                .unwrap_or(false);
            if !owned {
                ctx.out.push(WindowHit {
                    id: hwnd.0 as u64,
                    title: window_title(hwnd),
                    minimized: IsIconic(hwnd).as_bool(),
                    png: window_icon_png(hwnd),
                });
            }
        }
    }
    BOOL(1)
}

/// 当前存在用户可见窗口的进程 exe 名集合（小写，不含路径）。
/// dock 运行态判定用：一次枚举得到全集，再对每个应用做集合匹配，
/// 避免 N 个应用各自 EnumWindows + OpenProcess。
pub fn visible_process_set(self_pid: u32) -> std::collections::HashSet<String> {
    let mut ctx = CollectCtx {
        self_pid,
        out: std::collections::HashSet::new(),
    };
    unsafe {
        let _ = EnumWindows(
            Some(collect_proc),
            LPARAM(&mut ctx as *mut CollectCtx as isize),
        );
    }
    ctx.out
}

struct CollectCtx {
    self_pid: u32,
    out: std::collections::HashSet<String>,
}

unsafe extern "system" fn collect_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut CollectCtx);
    if user_visible_pid(hwnd, ctx.self_pid).is_some() {
        let name = process_name(hwnd).to_ascii_lowercase();
        if !name.is_empty() {
            ctx.out.insert(name);
        }
    }
    BOOL(1)
}

/// 按 exe 文件名（大小写不敏感）关闭其全部可见顶层窗口。
/// WM_CLOSE 温和关闭（应用可弹保存确认/拦截），不用 TerminateProcess；
/// PostMessage 异步投递，回调内不会边枚举边销毁窗口。
/// 提权进程的窗口会被 UIPI 静默拦截，属预期。返回已送达 WM_CLOSE 的窗口数。
pub fn close_all_by_process(self_pid: u32, exe: &str) -> usize {
    let mut ctx = CloseCtx {
        self_pid,
        exe: exe.to_ascii_lowercase(),
        sent: 0,
    };
    unsafe {
        let _ = EnumWindows(Some(close_proc), LPARAM(&mut ctx as *mut CloseCtx as isize));
    }
    ctx.sent
}

struct CloseCtx {
    self_pid: u32,
    exe: String,
    sent: usize,
}

unsafe extern "system" fn close_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
    let ctx = &mut *(lparam.0 as *mut CloseCtx);
    if user_visible_pid(hwnd, ctx.self_pid).is_some()
        && process_name(hwnd).to_ascii_lowercase() == ctx.exe
    {
        unsafe {
            if PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).is_ok() {
                ctx.sent += 1;
            }
        }
    }
    BOOL(1)
}

const CHEVRON_NAME: &str = "显示隐藏的图标";

/// 按 exe 文件名（大小写不敏感）数可见顶层窗口数。
/// 「多开」能力自学习用：多开前后各数一次，没涨 = 单实例应用。
pub fn count_visible_by_process(self_pid: u32, exe: &str) -> usize {
    let mut ctx = CountCtx {
        self_pid,
        exe: exe.to_ascii_lowercase(),
        count: 0,
    };
    unsafe {
        let _ = EnumWindows(Some(count_proc), LPARAM(&mut ctx as *mut CountCtx as isize));
    }
    ctx.count
}

struct CountCtx {
    self_pid: u32,
    exe: String,
    count: usize,
}

unsafe extern "system" fn count_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut CountCtx);
    if user_visible_pid(hwnd, ctx.self_pid).is_some()
        && process_name(hwnd).to_ascii_lowercase() == ctx.exe
    {
        ctx.count += 1;
    }
    BOOL(1)
}

/// chevron Invoke 后等弹层弹出
const FLYOUT_WAIT_MS: u64 = 600;
/// 任务栏显示后等 XAML 合成填充
const SHELL_WAIT_MS: u64 = 250;

fn find_window(cls: &str) -> Option<HWND> {
    let wide: Vec<u16> = cls.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { FindWindowW(windows::core::PCWSTR::from_raw(wide.as_ptr()), None).ok() }
}

/// 调试日志（追加到临时目录，排查托盘闪现问题用）
fn tray_log(msg: &str) {
    let path = std::env::temp_dir().join("hamster-tray-debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = writeln!(f, "{}", msg);
    }
}

/// open_overflow 借壳显示任务栏期间为 true。desktop_mode 的 explorer 巡检
/// 据此跳过「任务栏复活 → 重新隐藏」，否则巡检会把借壳中的任务栏提前藏掉，
/// chevron Invoke 失败、托盘弹层打不开（~1s 借壳撞上 5s 巡检并不罕见）。
static SHELL_FLASH: AtomicBool = AtomicBool::new(false);

/// 是否正处于借壳显示任务栏的闪现流程中
pub fn is_shell_flash_in_progress() -> bool {
    SHELL_FLASH.load(Ordering::SeqCst)
}

/// 打开原生托盘溢出弹层。返回前任务栏已重新隐藏，弹层独立保留。
///
/// `dock_hwnd`：我们的 dock 窗口句柄（借壳显示期间做置顶压制，尽量缩短
/// 系统任务栏盖在 dock 上的时间）。
pub fn open_overflow(dock_hwnd: isize) -> Result<()> {
    let Some(shell) = find_window("Shell_TrayWnd") else {
        tray_log("[tray] Shell_TrayWnd 未找到");
        return Err(windows::core::Error::from_win32());
    };
    tray_log("[tray] 任务栏已显示");
    // 巡检豁免窗口开始（此行之后到流程结束之间不再有提前返回路径）
    SHELL_FLASH.store(true, Ordering::SeqCst);
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        let _ = ShowWindow(shell, SW_SHOWNA);
        // 跨线程 ShowWindow 由 explorer 线程异步生效，任务栏插入 z 序的时机
        // 不定——在 ~200ms 内连续把 dock 压回最上，尽量缩短任务栏盖在 dock
        // 图标行上的时间（Win11 任务栏 z-band 特权，无法完全压住）
        for _ in 0..10 {
            let _ = SetWindowPos(
                HWND(dock_hwnd as *mut _),
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(SHELL_WAIT_MS));

    // ② UIA：找到「显示隐藏的图标」并 Invoke
    tray_log("开始 UIA 枚举");
    let result = (|| -> Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
            let elem = uia.ElementFromHandle(shell)?;
            let cond = uia.CreateTrueCondition()?;
            let all = elem.FindAll(
                windows::Win32::UI::Accessibility::TreeScope_Descendants,
                &cond,
            )?;
            eprintln!("[tray] UIA 按钮数: {}", all.Length()?);
            for i in 0..all.Length()? {
                let e = all.GetElement(i)?;
                if e.CurrentControlType()?.0 != 50000 {
                    continue;
                }
                let name = e.CurrentName()?.to_string();
                eprintln!("[tray] 按钮: {:?}", name);
                if name.contains(CHEVRON_NAME) {
                    let invoke: IUIAutomationInvokePattern =
                        e.GetCurrentPattern(UIA_InvokePatternId)?.cast().unwrap();
                    invoke.Invoke()?;
                    std::thread::sleep(std::time::Duration::from_millis(FLYOUT_WAIT_MS));
                    tray_log("[tray] chevron 已 Invoke，弹层应已出现");
                    return Ok(());
                }
            }
        }
        Err(windows::core::Error::from_win32())
    })();

    // ③ 把任务栏藏回去（溢出弹层是独立窗口，不受影响），巡检豁免结束
    unsafe {
        let _ = ShowWindow(shell, SW_HIDE);
    }
    SHELL_FLASH.store(false, Ordering::SeqCst);
    result
}

// w! 宏在动态类名场景无用，避免未使用告警的占位引用
#[allow(dead_code)]
fn _w_ref() {
    let _ = w!("Shell_TrayWnd");
}
