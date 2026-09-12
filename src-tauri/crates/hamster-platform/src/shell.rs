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
