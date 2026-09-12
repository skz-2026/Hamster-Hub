//! 便签与倒数日命令

use tauri::State;

use crate::error::AppError;
use crate::store::note::{self, CountdownCustom, Note};
use crate::AppState;

fn validate_len(content: &str, max: usize) -> Result<(), AppError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(AppError::validate("内容不能为空"));
    }
    if content.chars().count() > max {
        return Err(AppError::validate(format!("内容最长 {max} 字")));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn note_list(state: State<'_, AppState>) -> Result<Vec<Note>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    note::note_list(&conn)
}

#[tauri::command]
#[specta::specta]
pub fn note_create(state: State<'_, AppState>, content: String) -> Result<Note, AppError> {
    validate_len(&content, 2000)?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    note::note_create(&conn, content.trim())
}

#[tauri::command]
#[specta::specta]
pub fn note_update(state: State<'_, AppState>, id: u32, content: String) -> Result<(), AppError> {
    validate_len(&content, 2000)?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    note::note_update(&conn, id, content.trim())
}

#[tauri::command]
#[specta::specta]
pub fn note_toggle_pin(state: State<'_, AppState>, id: u32, pinned: bool) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    note::note_toggle_pin(&conn, id, pinned)
}

#[tauri::command]
#[specta::specta]
pub fn note_delete(state: State<'_, AppState>, id: u32) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    note::note_delete(&conn, id)
}

#[tauri::command]
#[specta::specta]
pub fn countdown_custom_list(state: State<'_, AppState>) -> Result<Vec<CountdownCustom>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    note::countdown_list_custom(&conn)
}

#[tauri::command]
#[specta::specta]
pub fn countdown_custom_create(
    state: State<'_, AppState>,
    title: String,
    target_date: String,
    emoji: Option<String>,
) -> Result<CountdownCustom, AppError> {
    validate_len(&title, 60)?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    note::countdown_create(&conn, title.trim(), &target_date, emoji.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn countdown_custom_delete(state: State<'_, AppState>, id: u32) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    note::countdown_delete(&conn, id)
}
