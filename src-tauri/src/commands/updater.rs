//! 应用自更新：tauri-plugin-updater + GitHub Releases（latest.json 端点）。
//! 检查/安装全走 Rust 命令（不引入 JS 插件，capabilities 保持最小权限面）；
//! 端点与 minisign 公钥在 tauri.conf.json，签名私钥经 CI secrets 注入。

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::Event;

use crate::error::AppError;
use crate::events;

fn updater_err(e: tauri_plugin_updater::Error) -> AppError {
    AppError::new("UPDATER", e.to_string())
}

/// 可用更新摘要（version 为目标版本；notes 为 release 正文，仅展示用）
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

/// 检查更新：None = 已是最新；Some = 可升级到对应版本
#[tauri::command]
#[specta::specta]
pub async fn update_check(app: AppHandle) -> Result<Option<UpdateInfo>, AppError> {
    let Some(update) = app
        .updater()
        .map_err(updater_err)?
        .check()
        .await
        .map_err(updater_err)?
    else {
        return Ok(None);
    };
    Ok(Some(UpdateInfo {
        version: update.version.clone(),
        notes: update.body.clone(),
    }))
}

/// 下载并安装更新：进度经 UpdateProgress 事件推送（约 100ms 一帧节流）。
/// 完成后应用退出、由 NSIS 静默安装器接管（installMode=quiet）；若进程
/// 未被接管（非 Windows 行为兜底）则主动重启。
#[tauri::command]
#[specta::specta]
pub async fn update_install(app: AppHandle) -> Result<(), AppError> {
    let update = app
        .updater()
        .map_err(updater_err)?
        .check()
        .await
        .map_err(updater_err)?
        .ok_or_else(|| AppError::new("UPDATER", "已是最新版本，无需安装"))?;

    let handle = app.clone();
    let mut downloaded: u64 = 0;
    let mut last_emit = std::time::Instant::now();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                if last_emit.elapsed().as_millis() >= 100 {
                    last_emit = std::time::Instant::now();
                    let _ = events::UpdateProgress {
                        downloaded,
                        total,
                        done: false,
                    }
                    .emit(&handle);
                }
            },
            || {},
        )
        .await
        .map_err(updater_err)?;

    let _ = events::UpdateProgress {
        downloaded: 0,
        total: None,
        done: true,
    }
    .emit(&app);
    app.restart()
}
