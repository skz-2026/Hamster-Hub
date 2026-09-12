//! 文件检索与打开命令

use std::path::PathBuf;

use tauri::State;

use crate::core::fileindex::{self, FileHit};
use crate::error::AppError;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn file_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<FileHit>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    fileindex::search(&conn, &query, limit.unwrap_or(8).clamp(1, 50))
}

/// 手动触发全量重扫（后台线程，完成发 FileIndexUpdated 事件）
#[tauri::command]
#[specta::specta]
pub fn file_index_refresh(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let roots = state.settings_snapshot_roots();
    let max = state.settings_snapshot_max_files();
    fileindex::spawn_scan(app, state.db_path.clone(), roots, max);
    Ok(())
}

/// 打开文件（须位于索引根目录内，SDD §8 白名单）
#[tauri::command]
#[specta::specta]
pub fn open_path(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    ensure_within_roots(&state, &path)?;
    if !hamster_platform::shell::shell_open(&PathBuf::from(&path)) {
        return Err(AppError::io(format!("打开失败: {path}")));
    }
    Ok(())
}

/// 在资源管理器中定位
#[tauri::command]
#[specta::specta]
pub fn reveal_in_explorer(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    ensure_within_roots(&state, &path)?;
    if !hamster_platform::shell::reveal_in_explorer(&PathBuf::from(&path)) {
        return Err(AppError::io(format!("定位失败: {path}")));
    }
    Ok(())
}

fn ensure_within_roots(state: &AppState, path: &str) -> Result<(), AppError> {
    let canonical =
        std::fs::canonicalize(path).map_err(|e| AppError::validate(format!("路径无效: {e}")))?;
    for root in state.settings_snapshot_roots() {
        if let Ok(root_canon) = std::fs::canonicalize(&root) {
            if canonical.starts_with(&root_canon) {
                return Ok(());
            }
        }
    }
    Err(AppError::validate("路径不在索引范围内"))
}
