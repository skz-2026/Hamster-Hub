//! 待办命令

use tauri::State;

use crate::error::AppError;
use crate::store::todo::{self, Todo};
use crate::AppState;

/// 截止时间合法范围（2001-09 ~ 2100-01），挡住误传的毫秒时间戳/垃圾值
const DUE_MIN: i64 = 1_000_000_000;
const DUE_MAX: i64 = 4_102_444_800;

fn validate_due(due_at: Option<i64>) -> Result<(), AppError> {
    match due_at {
        None => Ok(()),
        Some(ts) if (DUE_MIN..=DUE_MAX).contains(&ts) => Ok(()),
        Some(ts) => Err(AppError::validate(format!("截止时间超出范围: {ts}"))),
    }
}

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
pub fn todo_create(
    state: State<'_, AppState>,
    content: String,
    due_at: Option<i64>,
    remind: Option<bool>,
) -> Result<Todo, AppError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(AppError::validate("内容不能为空"));
    }
    if content.chars().count() > 200 {
        return Err(AppError::validate("待办最长 200 字"));
    }
    validate_due(due_at)?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    todo::create(&conn, content, due_at, remind.unwrap_or(false))
}

#[tauri::command]
#[specta::specta]
pub fn todo_set_due(
    state: State<'_, AppState>,
    id: u32,
    due_at: Option<i64>,
    remind: bool,
) -> Result<Todo, AppError> {
    validate_due(due_at)?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    todo::set_due(&conn, id, due_at, remind)
}

#[tauri::command]
#[specta::specta]
pub fn todo_set_recur(
    state: State<'_, AppState>,
    id: u32,
    recur: Option<String>,
) -> Result<Todo, AppError> {
    if let Some(rule) = recur.as_deref() {
        if !crate::core::recur::is_valid(rule) {
            return Err(AppError::validate(format!("未知循环规则: {rule}")));
        }
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    todo::set_recur(&conn, id, recur.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn todo_toggle(state: State<'_, AppState>, id: u32, done: bool) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    todo::set_done(&conn, id, done, chrono::Utc::now().timestamp())
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
