//! 番茄钟命令（薄层）：状态/落库都在 core::focus 与 store::focus

use tauri::{AppHandle, State};

use crate::core::focus::{self, FocusState, FocusStatus};
use crate::error::AppError;
use crate::store::focus::FocusDayStat;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn focus_start(
    app: AppHandle,
    focus: State<'_, FocusState>,
    state: State<'_, AppState>,
    minutes: u32,
    todo_id: Option<u32>,
) -> Result<FocusStatus, AppError> {
    let db_path = state.db_path.clone();
    Ok(focus::start(
        &focus,
        app,
        &db_path,
        focus::KIND_FOCUS,
        minutes,
        todo_id,
    ))
}

#[tauri::command]
#[specta::specta]
pub fn focus_break(
    app: AppHandle,
    focus: State<'_, FocusState>,
    state: State<'_, AppState>,
    minutes: u32,
) -> Result<FocusStatus, AppError> {
    let db_path = state.db_path.clone();
    Ok(focus::start(
        &focus,
        app,
        &db_path,
        focus::KIND_BREAK,
        minutes,
        None,
    ))
}

#[tauri::command]
#[specta::specta]
pub fn focus_pause(focus: State<'_, FocusState>) -> Result<(), AppError> {
    focus::set_paused(&focus, true);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn focus_resume(focus: State<'_, FocusState>) -> Result<(), AppError> {
    focus::set_paused(&focus, false);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn focus_stop(
    focus: State<'_, FocusState>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    focus::stop(&focus, &state.db_path);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn focus_status(focus: State<'_, FocusState>) -> Option<FocusStatus> {
    focus.status()
}

#[tauri::command]
#[specta::specta]
pub fn focus_history(
    state: State<'_, AppState>,
    days: Option<u32>,
) -> Result<Vec<FocusDayStat>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    crate::store::focus::daily_stats(&conn, days.unwrap_or(7).clamp(1, 30))
}
