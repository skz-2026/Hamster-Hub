//! 桌面模式（iOS 主屏接管）命令

use crate::desktop_mode;
use crate::error::AppError;

#[tauri::command]
#[specta::specta]
pub fn desktop_mode_enter(app: tauri::AppHandle) -> Result<(), AppError> {
    desktop_mode::enter(&app)
}

#[tauri::command]
#[specta::specta]
pub fn desktop_mode_exit(app: tauri::AppHandle) -> Result<(), AppError> {
    desktop_mode::exit(&app)
}

#[tauri::command]
#[specta::specta]
pub fn desktop_mode_is_active() -> bool {
    desktop_mode::is_active()
}

/// 唤出系统真实「开始」菜单（注入 Ctrl+Esc，见 hamster_platform::shell）
#[tauri::command]
#[specta::specta]
pub fn start_menu_open() -> Result<(), AppError> {
    hamster_platform::shell::open_start_menu();
    Ok(())
}
