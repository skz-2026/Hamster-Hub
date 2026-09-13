//! 应用索引查询与启动

use std::path::Path;

use tauri::State;

use crate::core::appindex::{self, AppEntry};
use crate::error::AppError;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn app_list(state: State<'_, AppState>) -> Result<Vec<AppEntry>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    appindex::list(&conn)
}

/// 拼音/首字母/名称检索（Spotlight 应用路）
#[tauri::command]
#[specta::specta]
pub fn app_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<AppEntry>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    appindex::search(&conn, &query, limit.unwrap_or(8))
}

#[tauri::command]
#[specta::specta]
pub fn app_launch(state: State<'_, AppState>, app_key: String) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let target: Option<String> = conn
        .query_row(
            "SELECT exec_target FROM app_meta WHERE app_key = ?1",
            [&app_key],
            |r| r.get(0),
        )
        .ok();
    let Some(target) = target else {
        return Err(AppError::validate(format!("应用不在索引中: {app_key}")));
    };
    let target_path = Path::new(&target);
    // 已运行则激活现有窗口（浏览器等单进程多窗应用，重复 shell_open 会再开新窗）
    if hamster_platform::shell::activate_running(target_path) {
        appindex::log_usage(&conn, &app_key);
        return Ok(());
    }
    if !hamster_platform::shell::shell_open(target_path) {
        return Err(AppError::io(format!("启动失败: {target}")));
    }
    appindex::log_usage(&conn, &app_key);
    Ok(())
}
