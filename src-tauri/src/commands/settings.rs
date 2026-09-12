//! 设置读写命令（IPC 薄层：校验 → 调 store → 返回）

use tauri::State;

use crate::error::AppError;
use crate::mcp_server::McpHub;
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
pub fn settings_save(
    state: State<'_, AppState>,
    hub: State<'_, McpHub>,
    settings: Settings,
) -> Result<Settings, AppError> {
    if !(0.5..=2.0).contains(&settings.appearance.font_scale) {
        return Err(AppError::validate("font_scale 需在 0.5-2.0 之间"));
    }
    let old_token = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        repo::load(&conn)?.agent.mcp_user_token
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let saved = repo::save(&conn, &settings)?;
    // computer use 档位即时生效（内嵌 server 红利：无需重启助手会话）
    hub.set_cu_allowed(saved.agent.computer_use_enabled);
    // 用户令牌轮换：旧令牌立即失效，新令牌即刻可用（外部配置需同步更新）
    match (old_token.as_deref(), saved.agent.mcp_user_token.as_deref()) {
        (Some(old), Some(new)) if old != new && !new.trim().is_empty() => {
            hub.rotate_user_token(old, new);
        }
        (None, Some(new)) if !new.trim().is_empty() => hub.register_user_token(new),
        _ => {}
    }
    Ok(saved)
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
