//! 任务栏窗口查找与显示控制（Shell_TrayWnd / Shell_SecondaryTrayWnd）

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, IsWindowVisible, ShowWindow, SW_HIDE, SW_SHOW,
};

pub const TRAY_CLASS: &str = "Shell_TrayWnd";
pub const TRAY_CLASS_SECONDARY: &str = "Shell_SecondaryTrayWnd";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskbarWindow {
    pub class: String,
    pub hwnd: isize,
    pub visible: bool,
}

fn to_hwnd(v: isize) -> HWND {
    HWND(v as *mut core::ffi::c_void)
}

/// 枚举主任务栏 + 所有副屏任务栏，附当前可见性
pub fn find_all() -> Vec<TaskbarWindow> {
    let mut out = Vec::new();
    unsafe {
        let main = &windows::core::HSTRING::from(TRAY_CLASS);
        if let Ok(h) = FindWindowW(main, None) {
            if !h.is_invalid() {
                out.push(TaskbarWindow {
                    class: TRAY_CLASS.into(),
                    hwnd: h.0 as isize,
                    visible: IsWindowVisible(h).as_bool(),
                });
            }
        }
        let sec = &windows::core::HSTRING::from(TRAY_CLASS_SECONDARY);
        let mut cur = FindWindowExW(None, None, sec, None);
        while let Ok(h) = cur {
            if h.is_invalid() {
                break;
            }
            out.push(TaskbarWindow {
                class: TRAY_CLASS_SECONDARY.into(),
                hwnd: h.0 as isize,
                visible: IsWindowVisible(h).as_bool(),
            });
            cur = FindWindowExW(None, Some(h), sec, None);
        }
    }
    out
}

pub fn any_visible() -> bool {
    find_all().iter().any(|t| t.visible)
}

/// 隐藏全部任务栏（按类名现查，容忍 explorer 重启后的新 hwnd）
pub fn hide_all() {
    for t in find_all() {
        if t.visible {
            unsafe {
                let _ = ShowWindow(to_hwnd(t.hwnd), SW_HIDE);
            };
        }
    }
}

/// 按快照还原可见性；快照里本就隐藏的保持隐藏
pub fn restore(list: &[TaskbarWindow]) {
    for t in list {
        if !t.visible {
            continue;
        }
        let hwnd = to_hwnd(t.hwnd);
        if !hwnd.is_invalid() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
            };
        }
    }
    // 兜底：按类名确保主任务栏可见（hwnd 可能因 explorer 重启失效）
    show_main_by_class();
}

/// 无视快照，直接恢复主/副任务栏可见（看门狗崩溃恢复用）
pub fn restore_all_by_class() {
    show_main_by_class();
    unsafe {
        let sec = &windows::core::HSTRING::from(TRAY_CLASS_SECONDARY);
        let mut cur = FindWindowExW(None, None, sec, None);
        while let Ok(h) = cur {
            if h.is_invalid() {
                break;
            }
            let _ = ShowWindow(h, SW_SHOW);
            cur = FindWindowExW(None, Some(h), sec, None);
        }
    }
}

fn show_main_by_class() {
    unsafe {
        let main = &windows::core::HSTRING::from(TRAY_CLASS);
        if let Ok(h) = FindWindowW(main, None) {
            if !h.is_invalid() {
                let _ = ShowWindow(h, SW_SHOW);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_all_returns_main_tray() {
        // 真机测试环境必有 Shell_TrayWnd
        let list = find_all();
        assert!(list.iter().any(|t| t.class == TRAY_CLASS), "未找到主任务栏");
    }
}
