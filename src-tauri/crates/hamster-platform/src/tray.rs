//! 托盘后台应用：枚举有窗口的应用（list）+ 恢复前置（activate），
//! 以及打开 Windows 原生托盘溢出弹层（open_overflow）。
//!
//! open_overflow 原理：系统任务栏（Shell_TrayWnd）被桌面接管隐藏后，托盘
//! 弹层不可达。这里短暂显示任务栏 → UIA Invoke「显示隐藏的图标」→ 系统原生
//! 的 TopLevelWindowForOverflowXamlIsland 弹层出现（独立窗口，含全部后台
//! 托盘 app）→ 再把任务栏藏回去，弹层留在屏幕上供用户直接交互。

use base64::Engine as _;
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

unsafe fn inspect(hwnd: HWND, self_pid: u32) -> Option<TrayApp> {
    // 不可见窗口直接排除；最小化窗口保留（面板里标「已最小化」）
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
    // 系统壳窗口（桌面/任务栏载体）不进托盘列表
    let cls = class_name(hwnd);
    if matches!(cls.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
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

const CHEVRON_NAME: &str = "显示隐藏的图标";
/// chevron Invoke 后等弹层弹出
const FLYOUT_WAIT_MS: u64 = 600;
/// 任务栏显示后等 XAML 合成填充
const SHELL_WAIT_MS: u64 = 250;

fn find_window(cls: &str) -> Option<HWND> {
    let wide: Vec<u16> = cls.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { FindWindowW(windows::core::PCWSTR::from_raw(wide.as_ptr()), None).ok() }
}

/// 打开原生托盘溢出弹层。返回前任务栏已重新隐藏，弹层独立保留。
///
/// `dock_hwnd`：我们的 dock 窗口句柄。系统任务栏闪现的 ~850ms 里会盖在
/// dock 图标行上（观感 =「dock 往上浮动」）——先显示任务栏，再把 dock
/// 重新置顶压在它上面，闪现部分被不透明的 dock 完全遮住。
pub fn open_overflow(dock_hwnd: isize) -> Result<()> {
    let Some(shell) = find_window("Shell_TrayWnd") else {
        eprintln!("[tray] Shell_TrayWnd 未找到");
        return Err(windows::core::Error::from_win32());
    };
    eprintln!("[tray] 任务栏已显示");
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        let _ = ShowWindow(shell, SW_SHOWNA);
        // 任务栏显示后再把 dock 压回最上（z 序：dock > 系统任务栏 > 弹层出现后 > 弹层）
        std::thread::sleep(std::time::Duration::from_millis(60));
        let _ = SetWindowPos(
            HWND(dock_hwnd as *mut _),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
    std::thread::sleep(std::time::Duration::from_millis(SHELL_WAIT_MS));

    // ② UIA：找到「显示隐藏的图标」并 Invoke
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
                    eprintln!("[tray] chevron 已 Invoke，弹层应已出现");
                    return Ok(());
                }
            }
        }
        Err(windows::core::Error::from_win32())
    })();

    // ③ 把任务栏藏回去（溢出弹层是独立窗口，不受影响）
    unsafe {
        let _ = ShowWindow(shell, SW_HIDE);
    }
    result
}

// w! 宏在动态类名场景无用，避免未使用告警的占位引用
#[allow(dead_code)]
fn _w_ref() {
    let _ = w!("Shell_TrayWnd");
}
