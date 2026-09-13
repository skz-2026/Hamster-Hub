//! 密码箱命令（薄层）：会话密钥在 AppState.vault，加解密全部在 Rust 侧完成，
//! 密钥与明文密码永不出后端（reveal 是唯一取密口，copy 走系统剪贴板）。

use std::time::Duration;

use tauri::State;
use tauri_plugin_clipboard_manager::ClipboardExt;
use zeroize::Zeroizing;

use crate::core::vault::{self as crypto, GenOptions, KEY_LEN};
use crate::error::AppError;
use crate::store::vault::{self as store, VaultItem, VaultItemInput, VaultSecret, VaultStatus};
use crate::AppState;

/// 主密码长度限制
const MASTER_MIN_CHARS: usize = 8;
const MASTER_MAX_CHARS: usize = 128;

fn status_of(state: &AppState) -> Result<VaultStatus, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let meta = store::meta_get(&conn)?;
    let session = state
        .vault
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    Ok(match meta {
        None => VaultStatus {
            initialized: false,
            unlocked: false,
            hint: None,
            auto_lock_secs: crypto::DEFAULT_AUTO_LOCK_SECS,
        },
        Some(m) => VaultStatus {
            initialized: true,
            unlocked: session.is_unlocked(),
            hint: m.hint,
            auto_lock_secs: m.auto_lock_secs,
        },
    })
}

fn validate_master(password: &str) -> Result<(), AppError> {
    let n = password.chars().count();
    if n < MASTER_MIN_CHARS {
        return Err(AppError::validate("主密码至少 8 个字符"));
    }
    if n > MASTER_MAX_CHARS {
        return Err(AppError::validate("主密码最长 128 个字符"));
    }
    Ok(())
}

fn clamp_auto_lock(secs: Option<u32>) -> u32 {
    secs.unwrap_or(crypto::DEFAULT_AUTO_LOCK_SECS)
        .clamp(30, 3600)
}

/// 解锁校验：派生 + verifier 比对；失败计数递增延迟（叠加 Argon2id 本身的成本）
fn check_master(
    conn: &rusqlite::Connection,
    password: &str,
) -> Result<(Zeroizing<[u8; KEY_LEN]>, store::VaultMeta), AppError> {
    let meta = store::meta_get(conn)?.ok_or_else(|| AppError::validate("密码箱尚未初始化"))?;
    let params: crypto::KdfParams = serde_json::from_str(&meta.kdf_params)
        .map_err(|e| AppError::db(format!("KDF 参数损坏: {e}")))?;
    let key = crypto::derive_key(password, &params)?;
    if !crypto::verify(&key, &meta.verifier) {
        return Err(AppError::new("VAULT_AUTH", "主密码不正确"));
    }
    Ok((key, meta))
}

#[tauri::command]
#[specta::specta]
pub fn vault_status(state: State<'_, AppState>) -> Result<VaultStatus, AppError> {
    status_of(&state)
}

/// 首次创建密码库：生成 KDF 参数与 verifier，解锁会话
#[tauri::command]
#[specta::specta]
pub fn vault_setup(
    state: State<'_, AppState>,
    password: String,
    hint: Option<String>,
    auto_lock_secs: Option<u32>,
) -> Result<VaultStatus, AppError> {
    validate_master(&password)?;
    let hint = hint
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .map(|h| h.chars().take(100).collect::<String>());
    let auto_lock_secs = clamp_auto_lock(auto_lock_secs);

    let params = crypto::fresh_params();
    let key = crypto::derive_key(&password, &params)?;
    let verifier = crypto::make_verifier(&key)?;

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    if store::meta_get(&conn)?.is_some() {
        return Err(AppError::validate("密码箱已初始化，请直接解锁"));
    }
    store::meta_insert(
        &conn,
        "argon2id",
        &serde_json::to_string(&params)?,
        &verifier,
        hint.as_deref(),
        auto_lock_secs,
    )?;
    drop(conn);

    let mut session = state
        .vault
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    session.unlock(key, auto_lock_secs);
    session.note_successful_unlock();
    Ok(VaultStatus {
        initialized: true,
        unlocked: true,
        hint,
        auto_lock_secs,
    })
}

#[tauri::command]
#[specta::specta]
pub fn vault_unlock(state: State<'_, AppState>, password: String) -> Result<VaultStatus, AppError> {
    // 连续失败递增延迟（封顶 8×300ms），叠加 Argon2id 本身的计算成本
    {
        let session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        let delay = session.failed_attempts().min(8) as u64 * 300;
        if delay > 0 {
            std::thread::sleep(Duration::from_millis(delay));
        }
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let result = check_master(&conn, &password);
    drop(conn);

    match result {
        Ok((key, meta)) => {
            let mut session = state
                .vault
                .lock()
                .map_err(|e| AppError::poison(e.to_string()))?;
            session.unlock(key, meta.auto_lock_secs);
            session.note_successful_unlock();
            Ok(VaultStatus {
                initialized: true,
                unlocked: true,
                hint: meta.hint,
                auto_lock_secs: meta.auto_lock_secs,
            })
        }
        Err(e) => {
            let mut session = state
                .vault
                .lock()
                .map_err(|e| AppError::poison(e.to_string()))?;
            session.note_failed_unlock();
            Err(e)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn vault_lock(state: State<'_, AppState>) -> Result<VaultStatus, AppError> {
    {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.lock();
    }
    status_of(&state)
}

#[tauri::command]
#[specta::specta]
pub fn vault_set_auto_lock(state: State<'_, AppState>, secs: u32) -> Result<VaultStatus, AppError> {
    let secs = clamp_auto_lock(Some(secs));
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::meta_set_auto_lock(&conn, secs)?;
    drop(conn);
    {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.set_auto_lock(secs);
    }
    status_of(&state)
}

#[tauri::command]
#[specta::specta]
pub fn vault_item_list(state: State<'_, AppState>) -> Result<Vec<VaultItem>, AppError> {
    // 列表不含敏感字段，但仍需解锁态并刷新活跃时间（自动锁判定）
    let _key = {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.key_touch()?
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::item_list(&conn, None)
}

#[tauri::command]
#[specta::specta]
pub fn vault_item_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<VaultItem>, AppError> {
    let _key = {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.key_touch()?
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::item_list(&conn, Some(&query))
}

#[tauri::command]
#[specta::specta]
pub fn vault_item_create(
    state: State<'_, AppState>,
    input: VaultItemInput,
) -> Result<VaultItem, AppError> {
    let key = {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.key_touch()?
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::item_create(&conn, &key, &input)
}

#[tauri::command]
#[specta::specta]
pub fn vault_item_update(
    state: State<'_, AppState>,
    id: u32,
    input: VaultItemInput,
) -> Result<VaultItem, AppError> {
    let key = {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.key_touch()?
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::item_update(&conn, &key, id, &input)
}

#[tauri::command]
#[specta::specta]
pub fn vault_item_toggle_favorite(
    state: State<'_, AppState>,
    id: u32,
    favorite: bool,
) -> Result<VaultItem, AppError> {
    let _key = {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.key_touch()?
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::item_toggle_favorite(&conn, id, favorite)
}

/// 软删除（回收站 M2 展示）
#[tauri::command]
#[specta::specta]
pub fn vault_item_delete(state: State<'_, AppState>, id: u32) -> Result<(), AppError> {
    let _key = {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.key_touch()?
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::item_soft_delete(&conn, id)
}

/// 取某条敏感字段（唯一取密口；UI 显式点击才调用）
#[tauri::command]
#[specta::specta]
pub fn vault_item_reveal(state: State<'_, AppState>, id: u32) -> Result<VaultSecret, AppError> {
    let key = {
        let mut session = state
            .vault
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        session.key_touch()?
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::item_reveal(&conn, &key, id)
}

#[tauri::command]
#[specta::specta]
pub fn vault_password_generate(options: GenOptions) -> Result<String, AppError> {
    crypto::generate_password(&options)
}

#[tauri::command]
#[specta::specta]
pub fn vault_password_strength(password: String) -> u8 {
    crypto::password_strength(&password)
}

/// 换主密码：校验旧密码 → 全量重加密 → 轮换 KDF 参数与 verifier
#[tauri::command]
#[specta::specta]
pub fn vault_change_password(
    state: State<'_, AppState>,
    old_password: String,
    new_password: String,
    hint: Option<String>,
) -> Result<VaultStatus, AppError> {
    validate_master(&new_password)?;
    let hint = hint
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .map(|h| h.chars().take(100).collect::<String>());

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let (old_key, meta) = check_master(&conn, &old_password)?;
    let new_params = crypto::fresh_params();
    let new_key = crypto::derive_key(&new_password, &new_params)?;
    store::reencrypt_all(&conn, &old_key, &new_key)?;
    store::meta_rotate(
        &conn,
        &serde_json::to_string(&new_params)?,
        &crypto::make_verifier(&new_key)?,
        hint.as_deref(),
    )?;
    let auto_lock_secs = meta.auto_lock_secs;
    drop(conn);

    let mut session = state
        .vault
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    session.unlock(new_key, auto_lock_secs);
    session.note_successful_unlock();
    Ok(VaultStatus {
        initialized: true,
        unlocked: true,
        hint,
        auto_lock_secs,
    })
}

/// 复制字段到剪贴板（Rust 侧写入；密码在 CLIPBOARD_CLEAR_SECS 后自动清除，
/// 仅当剪贴板内容仍是本次写入时才清，避免误伤用户后续复制）
#[tauri::command]
#[specta::specta]
pub fn vault_copy_field(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: u32,
    field: String,
) -> Result<u32, AppError> {
    const FIELD_PASSWORD: &str = "password";
    const FIELD_USERNAME: &str = "username";
    if field != FIELD_PASSWORD && field != FIELD_USERNAME {
        return Err(AppError::validate("字段需为 password 或 username"));
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let text = if field == FIELD_USERNAME {
        store::item_get_meta(&conn, id)?.username
    } else {
        let key = {
            let mut session = state
                .vault
                .lock()
                .map_err(|e| AppError::poison(e.to_string()))?;
            session.key_touch()?
        };
        store::item_reveal(&conn, &key, id)?.password
    };
    drop(conn);

    app.clipboard()
        .write_text(text.clone())
        .map_err(|e| AppError::io(format!("写剪贴板失败: {e}")))?;

    if field == FIELD_PASSWORD {
        let clear_secs = crypto::CLIPBOARD_CLEAR_SECS;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(clear_secs as u64));
            let clip = app.clipboard();
            if let Ok(current) = clip.read_text() {
                if current == text {
                    let _ = clip.clear();
                }
            }
        });
        Ok(clear_secs)
    } else {
        Ok(0)
    }
}
