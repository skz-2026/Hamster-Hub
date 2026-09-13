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

/// app_key → exec_target（app_meta 单行查询）
fn exec_target_of(conn: &rusqlite::Connection, app_key: &str) -> Option<String> {
    conn.query_row(
        "SELECT exec_target FROM app_meta WHERE app_key = ?1",
        [app_key],
        |r| r.get(0),
    )
    .ok()
}

/// 启动实现：new_instance = true 时跳过「已运行则激活」，强制再开（dock 右键「多开」）
fn launch(state: State<'_, AppState>, app_key: String, new_instance: bool) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let Some(target) = exec_target_of(&conn, &app_key) else {
        return Err(AppError::validate(format!("应用不在索引中: {app_key}")));
    };
    let target_path = Path::new(&target);
    // 已运行则激活现有窗口（浏览器等单进程多窗应用，重复 shell_open 会再开新窗）
    if !new_instance && hamster_platform::shell::activate_running(target_path) {
        appindex::log_usage(&conn, &app_key);
        return Ok(());
    }
    if !hamster_platform::shell::shell_open(target_path) {
        return Err(AppError::io(format!("启动失败: {target}")));
    }
    appindex::log_usage(&conn, &app_key);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn app_launch(state: State<'_, AppState>, app_key: String) -> Result<(), AppError> {
    launch(state, app_key, false)
}

/// 多开：绕过「已运行激活现有窗口」，直接 shell_open 再开一个。
/// 文件夹/记事本/浏览器得到新窗口；单实例应用由其自身去重（等效激活）。
#[tauri::command]
#[specta::specta]
pub fn app_launch_new(state: State<'_, AppState>, app_key: String) -> Result<(), AppError> {
    launch(state, app_key, true)
}

/// 关闭应用：对其 exe 名下全部可见窗口投递 WM_CLOSE（温和关闭，应用可弹
/// 保存确认）。解析不出 exe（UWP/文件夹）或没有可见窗口时报错。
#[tauri::command]
#[specta::specta]
pub fn app_close(state: State<'_, AppState>, app_key: String) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let Some(target) = exec_target_of(&conn, &app_key) else {
        return Err(AppError::validate(format!("应用不在索引中: {app_key}")));
    };
    let Some(exe) = hamster_platform::shell::cached_exe_file_name(Path::new(&target)) else {
        return Err(AppError::validate(format!("无法解析应用进程: {target}")));
    };
    drop(conn);
    let sent = hamster_platform::tray::close_all_by_process(std::process::id(), &exe);
    if sent == 0 {
        return Err(AppError::validate(format!(
            "应用当前没有可见窗口: {app_key}"
        )));
    }
    Ok(())
}

/// dock 运行态轮询：返回 keys 中当前有可见窗口的应用 app_key。
/// DB 查询留在调用线程（毫秒级）；lnk 解析 + EnumWindows 放线程池，不占主线程。
#[tauri::command]
#[specta::specta]
pub async fn apps_running(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<Vec<String>, AppError> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    let mut targets: Vec<(String, String)> = Vec::new();
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        let placeholders = vec!["?"; keys.len()].join(",");
        let sql =
            format!("SELECT app_key, exec_target FROM app_meta WHERE app_key IN ({placeholders})");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::io(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(keys.iter()), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::io(e.to_string()))?;
        for row in rows.flatten() {
            targets.push(row);
        }
    }
    tauri::async_runtime::spawn_blocking(move || {
        let procs = hamster_platform::tray::visible_process_set(std::process::id());
        targets
            .into_iter()
            .filter(|(_, target)| {
                hamster_platform::shell::cached_exe_file_name(Path::new(target))
                    .is_some_and(|exe| procs.contains(&exe))
            })
            .map(|(key, _)| key)
            .collect::<Vec<String>>()
    })
    .await
    .map_err(|e| AppError::io(format!("运行态枚举失败: {e}")))
}
