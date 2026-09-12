//! 系统命令：健康检查、窗口置顶

use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AppHealth {
    pub name: &'static str,
    pub version: &'static str,
}

#[tauri::command]
#[specta::specta]
pub fn app_health() -> AppHealth {
    AppHealth {
        name: "hamster-hub",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
#[specta::specta]
pub fn window_set_pinned(window: tauri::Window, pinned: bool) -> Result<(), AppError> {
    window
        .set_always_on_top(pinned)
        .map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn open_url(url: String) -> Result<(), AppError> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(AppError::validate("仅允许 http(s) URL"));
    }
    if !hamster_platform::shell::shell_open(std::path::Path::new(&url)) {
        return Err(AppError::io(format!("打开 URL 失败: {url}")));
    }
    Ok(())
}
