//! 设置读写命令（IPC 薄层：校验 → 调 store → 返回）

use tauri::State;

use crate::error::AppError;
use crate::store::config::Settings;
use crate::store::settings as repo;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn settings_load(state: State<'_, AppState>) -> Result<Settings, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    repo::load(&conn)
}

#[tauri::command]
#[specta::specta]
pub fn settings_save(state: State<'_, AppState>, settings: Settings) -> Result<Settings, AppError> {
    if !(0.5..=2.0).contains(&settings.appearance.font_scale) {
        return Err(AppError::validate("font_scale 需在 0.5-2.0 之间"));
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    repo::save(&conn, &settings)
}

// ===== 通用 KV（白名单键）=====

#[tauri::command]
#[specta::specta]
pub fn kv_get(state: State<'_, AppState>, key: String) -> Result<Option<String>, AppError> {
    if !repo::KV_ALLOWED_KEYS.contains(&key.as_str()) {
        return Err(AppError::validate(format!("key 不在白名单: {key}")));
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    repo::kv_load(&conn, &key)
}

#[tauri::command]
#[specta::specta]
pub fn kv_set(state: State<'_, AppState>, key: String, value: String) -> Result<(), AppError> {
    if !repo::KV_ALLOWED_KEYS.contains(&key.as_str()) {
        return Err(AppError::validate(format!("key 不在白名单: {key}")));
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    repo::kv_save(&conn, &key, &value)
}
