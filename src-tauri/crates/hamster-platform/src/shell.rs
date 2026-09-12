//! Shell 启动：打开 .lnk/exe/文件、资源管理器定位

use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 用系统关联方式打开（lnk/exe/文档/URL）
pub fn shell_open(path: &Path) -> bool {
    let p = to_wide(&path.to_string_lossy());
    let verb = to_wide("open");
    unsafe {
        let r = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(p.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );
        // ShellExecuteW 返回 HINSTANCE，>32 视为成功
        r.0 as usize > 32
    }
}

/// 在资源管理器中定位文件
pub fn reveal_in_explorer(path: &Path) -> bool {
    let arg = format!("/select,{}", path.to_string_lossy());
    let sel = to_wide(&arg);
    let exe = to_wide("explorer.exe");
    unsafe {
        let r = ShellExecuteW(
            None,
            None,
            PCWSTR(exe.as_ptr()),
            PCWSTR(sel.as_ptr()),
            None,
            SW_SHOWNORMAL,
        );
        r.0 as usize > 32
    }
}

/// 唤出系统真实「开始」菜单：注入 Ctrl+Esc 系统热键。
/// 接管模式隐藏了真任务栏，但开始菜单（StartMenuExperienceHost）是独立
/// 系统界面，仍可召出；Win 键注入会被 UIPI 过滤，Ctrl+Esc 不受限。
pub fn open_start_menu() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_CONTROL, VK_ESCAPE,
    };
    unsafe {
        keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_ESCAPE.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_ESCAPE.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}
