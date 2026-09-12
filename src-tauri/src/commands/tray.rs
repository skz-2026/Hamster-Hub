//! 「托盘」命令：打开 Windows 原生托盘溢出窗口（dock 右端托盘按钮）。

use tauri::{AppHandle, Manager};

use crate::error::AppError;

/// 打开原生托盘溢出弹层（系统自带 UI，含全部后台托盘 app，可直接交互）。
/// 期间系统任务栏会短暂闪烁一次；弹层出现后把自家 dock 压回最上（z 序恢复）。
#[tauri::command]
#[specta::specta]
pub fn tray_open_overflow(app: AppHandle) -> Result<(), AppError> {
    // dock_hwnd = 自家 taskbar 常驻条窗口（open_overflow 弹层出现后要把它压回最上）
    let dock_hwnd = app
        .get_webview_window("taskbar")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as isize)
        .ok_or_else(|| AppError::new("TRAY_NO_DOCK", "taskbar 窗口不存在（未处于桌面模式）"))?;
    hamster_platform::tray::open_overflow(dock_hwnd)
        .map_err(|e| AppError::new("TRAY_OPEN", e.to_string()))
}
