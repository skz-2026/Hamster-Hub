//! 托管式分屏命令（桌面接管态的 dock 右键「分屏添加」/任务栏胶囊）。
//!
//! 会话与摆窗在 `crate::split_mode`（会话状态）与 `hamster_platform::tile`
//! （几何 + 窗口状态读写，零 Tauri 依赖带单测）；本层只做 app_key → 进程名
//! 解析（要查 DB）与错误映射。

use std::path::Path;

use tauri::{AppHandle, State};

use crate::commands::apps::exec_target_of;
use crate::error::AppError;
use crate::split_mode::{self, SplitState};
use crate::AppState;

/// 分屏会话快照。读的时候会自愈：已被关掉的窗口剔除、剩下的按新成员数补位。
#[tauri::command]
#[specta::specta]
pub fn split_state(app: AppHandle) -> Result<SplitState, AppError> {
    Ok(split_mode::state(&app))
}

/// 把应用的一扇窗口加入分屏（同一应用多次调用依次加入它的多扇窗口）。
/// 首次调用即建立会话；已满 4 扇或该应用没有可分屏的窗口时报错（前端据此提示）。
#[tauri::command]
#[specta::specta]
pub fn split_add(
    app: AppHandle,
    state: State<'_, AppState>,
    app_key: String,
) -> Result<SplitState, AppError> {
    let exe = exe_of(&state, &app_key)?;
    split_mode::add(&app, &exe, &app_key).map_err(AppError::validate)
}

/// 把某扇窗口移出分屏：它回到加入前的位置/状态，剩下的补位。
/// （「关闭窗口」是另一回事，前端调 app_window_close + split_state 补位。）
#[tauri::command]
#[specta::specta]
pub fn split_remove(app: AppHandle, window_id: u64) -> Result<SplitState, AppError> {
    Ok(split_mode::remove(&app, window_id))
}

/// 退出分屏：全部窗口回到加入前的位置与显示状态。
#[tauri::command]
#[specta::specta]
pub fn split_exit() -> Result<SplitState, AppError> {
    Ok(split_mode::exit())
}

/// app_key → 进程名（分屏按进程名找该应用的窗口，与 dock 悬停卡片同口径）
fn exe_of(state: &State<'_, AppState>, app_key: &str) -> Result<String, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let target = exec_target_of(&conn, app_key)
        .ok_or_else(|| AppError::validate(format!("应用不在索引中: {app_key}")))?;
    hamster_platform::shell::cached_exe_file_name(Path::new(&target))
        .ok_or_else(|| AppError::validate(format!("无法解析应用进程: {target}")))
}
