//! 设置读写命令（IPC 薄层：校验 → 调 store → 返回）

use std::collections::BTreeMap;
use std::path::Path;

use tauri::State;

use crate::bench::BenchContext;
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
    app: tauri::AppHandle,
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
    // 语言可能变化：托盘菜单按新语言重建
    crate::refresh_tray_menu(&app);
    Ok(saved)
}

// ===== Agent CLI 手动指定路径 =====

/// 手动指定 Agent CLI 程序路径（设置 → Agent；绿色版/自拷贝 CLI 未上 PATH 时用）。
/// 先校验再落库：agent id 必须在注册表内、路径必须是存在的文件；`path = None` 清除。
/// 生效点：扫描（installed/program）与启动（覆盖自动探测的最高版本结果）。
#[tauri::command]
#[specta::specta]
pub fn agent_cli_path_set(
    state: State<'_, AppState>,
    bench: State<'_, BenchContext>,
    agent_id: String,
    path: Option<String>,
) -> Result<BTreeMap<String, String>, AppError> {
    if bench.registry.get(&agent_id).is_err() {
        return Err(AppError::validate(format!("未知的 Agent: {agent_id}")));
    }
    let trimmed = path.as_deref().map(str::trim).filter(|p| !p.is_empty());
    if let Some(p) = trimmed {
        if !Path::new(p).is_file() {
            return Err(AppError::validate(format!("路径不存在或不是文件: {p}")));
        }
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let mut settings = repo::load(&conn)?;
    match trimmed {
        Some(p) => {
            settings.agent.cli_paths.insert(agent_id, p.to_string());
        }
        None => {
            settings.agent.cli_paths.remove(agent_id.as_str());
        }
    }
    let saved = repo::save(&conn, &settings)?;
    Ok(saved.agent.cli_paths)
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
