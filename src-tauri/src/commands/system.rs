//! 系统命令：健康检查、窗口置顶、自定义壁纸导入

use std::path::Path;

use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::AppState;

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

/// 导入自定义主屏壁纸：校验图片扩展名 → 拷贝进 app_data_dir/wallpapers →
/// 返回新文件的绝对路径（前端存入 home.layout.wallpaperImage，经 asset
/// protocol 显示）。previous 传上一张自定义壁纸路径，仅在 wallpapers
/// 目录内才会被删除，避免误删用户目录里的原图。
#[tauri::command]
#[specta::specta]
pub fn wallpaper_image_import(
    state: State<'_, AppState>,
    path: String,
    previous: Option<String>,
) -> Result<String, AppError> {
    const ALLOWED: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];
    let src = Path::new(&path);
    if !src.is_file() {
        return Err(AppError::validate(format!("文件不存在: {path}")));
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !ALLOWED.contains(&ext.as_str()) {
        return Err(AppError::validate(format!("不支持的图片格式: {ext}")));
    }
    let dir = state
        .db_path
        .parent()
        .ok_or_else(|| AppError::io("无法定位应用数据目录"))?
        .join("wallpapers");
    std::fs::create_dir_all(&dir)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    let dest = dir.join(format!("wallpaper-{ts}.{ext}"));
    std::fs::copy(src, &dest)?;
    if let Some(prev) = previous.as_deref() {
        let prev = Path::new(prev);
        if prev.starts_with(&dir) && prev.is_file() {
            let _ = std::fs::remove_file(prev);
        }
    }
    Ok(dest.to_string_lossy().into_owned())
}
