//! 待办命令

use tauri::State;

use crate::error::AppError;
use crate::store::todo::{self, Todo};
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn todo_list(state: State<'_, AppState>) -> Result<Vec<Todo>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    todo::list(&conn)
}

#[tauri::command]
#[specta::specta]
pub fn todo_create(state: State<'_, AppState>, content: String) -> Result<Todo, AppError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(AppError::validate("内容不能为空"));
    }
    if content.chars().count() > 200 {
        return Err(AppError::validate("待办最长 200 字"));
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    todo::create(&conn, content)
}

#[tauri::command]
#[specta::specta]
pub fn todo_toggle(state: State<'_, AppState>, id: u32, done: bool) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    todo::toggle(&conn, id, done)
}

#[tauri::command]
#[specta::specta]
pub fn todo_delete(state: State<'_, AppState>, id: u32) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    todo::delete(&conn, id)
}
