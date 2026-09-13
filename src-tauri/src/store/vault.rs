//! 密码箱 repo（表在 0006 迁移）：vault_meta 单行 + vault_item 条目。
//! 字段级加密：敏感字段（密码/备注）在 store 层装信封，密钥由命令层传入；
//! title/username/url 明文入库供 LIKE 搜索。

use rusqlite::{params, Connection};
use zeroize::Zeroizing;

use crate::core::vault as crypto;
use crate::core::vault::KEY_LEN;
use crate::error::AppError;

/// 条目元数据（列表用，不含敏感字段）
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct VaultItem {
    pub id: u32,
    pub title: String,
    pub username: String,
    pub url: String,
    pub favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub password_updated_at: i64,
}

/// 敏感字段（信封内的 JSON 明文结构）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct VaultSecret {
    pub password: String,
    pub notes: String,
}

/// 密码箱总览状态（页面三态：未初始化 / 已锁 / 已解锁）
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct VaultStatus {
    pub initialized: bool,
    pub unlocked: bool,
    pub hint: Option<String>,
    pub auto_lock_secs: u32,
}

/// 新建/编辑入参（明文经 IPC 传入，store 层立即加密落库）
#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
pub struct VaultItemInput {
    pub title: String,
    pub username: String,
    pub url: String,
    pub password: String,
    pub notes: String,
    pub favorite: bool,
}

pub fn validate_input(input: &VaultItemInput) -> Result<(), AppError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::validate("标题不能为空"));
    }
    if title.chars().count() > 100 {
        return Err(AppError::validate("标题最长 100 字"));
    }
    if input.username.chars().count() > 200 || input.url.chars().count() > 200 {
        return Err(AppError::validate("用户名/网址最长 200 字"));
    }
    if input.password.len() > 256 {
        return Err(AppError::validate("密码最长 256 字节"));
    }
    if input.notes.chars().count() > 2000 {
        return Err(AppError::validate("备注最长 2000 字"));
    }
    Ok(())
}

fn row_to_item(r: &rusqlite::Row) -> rusqlite::Result<VaultItem> {
    Ok(VaultItem {
        id: r.get::<_, i64>(0)? as u32,
        title: r.get(1)?,
        username: r.get(2)?,
        url: r.get(3)?,
        favorite: r.get::<_, i64>(4)? != 0,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
        password_updated_at: r.get(7)?,
    })
}

/// LIKE 转义（% _ \），保证用户输入按字面匹配
fn escape_like(q: &str) -> String {
    q.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn seal_secret(key: &Zeroizing<[u8; KEY_LEN]>, secret: &VaultSecret) -> Result<Vec<u8>, AppError> {
    let json = serde_json::to_vec(secret)?;
    crypto::seal(key, &json)
}

fn open_secret(key: &Zeroizing<[u8; KEY_LEN]>, envelope: &[u8]) -> Result<VaultSecret, AppError> {
    let json = crypto::open(key, envelope)?;
    Ok(serde_json::from_slice(&json)?)
}

// ===== vault_meta =====

#[derive(Debug, Clone)]
pub struct VaultMeta {
    pub kdf_params: String,
    pub verifier: Vec<u8>,
    pub hint: Option<String>,
    pub auto_lock_secs: u32,
}

pub fn meta_get(conn: &Connection) -> Result<Option<VaultMeta>, AppError> {
    let Ok(row) = conn.query_row(
        // kdf_algo 列（'argon2id'）为将来算法升级预留，当前恒为 argon2id 不读
        "SELECT kdf_params, verifier, hint, auto_lock_secs FROM vault_meta WHERE id = 1",
        [],
        |r| {
            Ok(VaultMeta {
                kdf_params: r.get(0)?,
                verifier: r.get(1)?,
                hint: r.get(2)?,
                auto_lock_secs: r.get::<_, i64>(3)? as u32,
            })
        },
    ) else {
        return Ok(None);
    };
    Ok(Some(row))
}

pub fn meta_insert(
    conn: &Connection,
    kdf_algo: &str,
    kdf_params: &str,
    verifier: &[u8],
    hint: Option<&str>,
    auto_lock_secs: u32,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO vault_meta(id, kdf_algo, kdf_params, verifier, hint, auto_lock_secs)
         VALUES(1, ?1, ?2, ?3, ?4, ?5)",
        params![kdf_algo, kdf_params, verifier, hint, auto_lock_secs as i64],
    )?;
    Ok(())
}

/// 换主密码：更新 KDF 参数与 verifier（reencrypt_all 已在先/后单独调用）
pub fn meta_rotate(
    conn: &Connection,
    kdf_params: &str,
    verifier: &[u8],
    hint: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE vault_meta SET kdf_params = ?1, verifier = ?2, hint = ?3 WHERE id = 1",
        params![kdf_params, verifier, hint],
    )?;
    Ok(())
}

pub fn meta_set_auto_lock(conn: &Connection, secs: u32) -> Result<(), AppError> {
    conn.execute(
        "UPDATE vault_meta SET auto_lock_secs = ?1 WHERE id = 1",
        params![secs as i64],
    )?;
    Ok(())
}

// ===== vault_item =====

/// 列表/搜索（搜索命中 title/username/url；LIKE 字面匹配，收藏优先 + 最近更新在前）
pub fn item_list(conn: &Connection, query: Option<&str>) -> Result<Vec<VaultItem>, AppError> {
    let q = query.map(str::trim).filter(|q| !q.is_empty());
    let rows: Vec<VaultItem> = match q {
        Some(q) => {
            let like = format!("%{}%", escape_like(q));
            let mut stmt = conn.prepare(
                "SELECT id, title, username, url, favorite, created_at, updated_at, password_updated_at
                 FROM vault_item
                 WHERE deleted_at IS NULL
                   AND (title LIKE ?1 ESCAPE '\\' OR username LIKE ?1 ESCAPE '\\' OR url LIKE ?1 ESCAPE '\\')
                 ORDER BY favorite DESC, updated_at DESC",
            )?;
            let rows: Vec<VaultItem> = stmt
                .query_map(params![like], row_to_item)?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, title, username, url, favorite, created_at, updated_at, password_updated_at
                 FROM vault_item
                 WHERE deleted_at IS NULL
                 ORDER BY favorite DESC, updated_at DESC",
            )?;
            let rows: Vec<VaultItem> = stmt
                .query_map([], row_to_item)?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        }
    };
    Ok(rows)
}

/// 取单条（含信封），reveal/update 内部用
fn item_get_raw(conn: &Connection, id: u32) -> Result<(VaultItem, Vec<u8>), AppError> {
    conn.query_row(
        "SELECT id, title, username, url, favorite, created_at, updated_at, password_updated_at, secret
         FROM vault_item WHERE id = ?1 AND deleted_at IS NULL",
        [id as i64],
        |r| {
            Ok((
                VaultItem {
                    id: r.get::<_, i64>(0)? as u32,
                    title: r.get(1)?,
                    username: r.get(2)?,
                    url: r.get(3)?,
                    favorite: r.get::<_, i64>(4)? != 0,
                    created_at: r.get(5)?,
                    updated_at: r.get(6)?,
                    password_updated_at: r.get(7)?,
                },
                r.get(8)?,
            ))
        },
    )
    .map_err(|_| AppError::validate(format!("密码条目不存在: {id}")))
}

pub fn item_reveal(
    conn: &Connection,
    key: &Zeroizing<[u8; KEY_LEN]>,
    id: u32,
) -> Result<VaultSecret, AppError> {
    let (_, envelope) = item_get_raw(conn, id)?;
    open_secret(key, &envelope)
}

pub fn item_create(
    conn: &Connection,
    key: &Zeroizing<[u8; KEY_LEN]>,
    input: &VaultItemInput,
) -> Result<VaultItem, AppError> {
    validate_input(input)?;
    let secret = VaultSecret {
        password: input.password.clone(),
        notes: input.notes.trim().to_string(),
    };
    let envelope = seal_secret(key, &secret)?;
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO vault_item(title, username, url, secret, favorite, created_at, updated_at, password_updated_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?6, ?6)",
        params![
            input.title.trim(),
            input.username.trim(),
            input.url.trim(),
            envelope,
            input.favorite as i64,
            now
        ],
    )?;
    let id = conn.last_insert_rowid() as u32;
    item_get_meta(conn, id)
}

/// 单条元数据（复制用户名等不需要解密的场景）
pub fn item_get_meta(conn: &Connection, id: u32) -> Result<VaultItem, AppError> {
    conn.query_row(
        "SELECT id, title, username, url, favorite, created_at, updated_at, password_updated_at
         FROM vault_item WHERE id = ?1",
        [id as i64],
        row_to_item,
    )
    .map_err(|_| AppError::validate(format!("密码条目不存在: {id}")))
}

/// 全量替换编辑：密码变化时刷新 password_updated_at（旧值解密比对）
pub fn item_update(
    conn: &Connection,
    key: &Zeroizing<[u8; KEY_LEN]>,
    id: u32,
    input: &VaultItemInput,
) -> Result<VaultItem, AppError> {
    validate_input(input)?;
    let (_, old_envelope) = item_get_raw(conn, id)?;
    let old = open_secret(key, &old_envelope)?;
    let secret = VaultSecret {
        password: input.password.clone(),
        notes: input.notes.trim().to_string(),
    };
    let envelope = seal_secret(key, &secret)?;
    let now = chrono::Utc::now().timestamp();
    let pw_at = if old.password != input.password {
        now
    } else {
        -1
    };
    let n = conn.execute(
        "UPDATE vault_item
         SET title = ?1, username = ?2, url = ?3, secret = ?4, favorite = ?5, updated_at = ?6,
             password_updated_at = CASE WHEN ?7 THEN ?6 ELSE password_updated_at END
         WHERE id = ?8 AND deleted_at IS NULL",
        params![
            input.title.trim(),
            input.username.trim(),
            input.url.trim(),
            envelope,
            input.favorite as i64,
            now,
            pw_at != -1,
            id as i64
        ],
    )?;
    if n == 0 {
        return Err(AppError::validate(format!("密码条目不存在: {id}")));
    }
    item_get_meta(conn, id)
}

pub fn item_toggle_favorite(
    conn: &Connection,
    id: u32,
    favorite: bool,
) -> Result<VaultItem, AppError> {
    let n = conn.execute(
        "UPDATE vault_item SET favorite = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        params![favorite as i64, id as i64],
    )?;
    if n == 0 {
        return Err(AppError::validate(format!("密码条目不存在: {id}")));
    }
    item_get_meta(conn, id)
}

/// 软删除（回收站 M2 展示；数据保留可恢复）
pub fn item_soft_delete(conn: &Connection, id: u32) -> Result<(), AppError> {
    let n = conn.execute(
        "UPDATE vault_item SET deleted_at = unixepoch() WHERE id = ?1 AND deleted_at IS NULL",
        [id as i64],
    )?;
    if n == 0 {
        return Err(AppError::validate(format!("密码条目不存在: {id}")));
    }
    Ok(())
}

/// 换主密码：全量重加密（含软删除条目，回收站数据不丢）
pub fn reencrypt_all(
    conn: &Connection,
    old_key: &Zeroizing<[u8; KEY_LEN]>,
    new_key: &Zeroizing<[u8; KEY_LEN]>,
) -> Result<usize, AppError> {
    let items: Vec<(i64, Vec<u8>)> = conn
        .prepare("SELECT id, secret FROM vault_item")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut count = 0;
    for (id, envelope) in items {
        let secret = open_secret(old_key, &envelope)?;
        let new_env = seal_secret(new_key, &secret)?;
        conn.execute(
            "UPDATE vault_item SET secret = ?1 WHERE id = ?2",
            params![new_env, id],
        )?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::vault::{derive_key, fresh_params, make_verifier, verify};

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::store::db::migrate(&c).unwrap();
        c
    }

    fn setup_vault(c: &Connection, password: &str) -> Zeroizing<[u8; KEY_LEN]> {
        let params = fresh_params();
        let key = derive_key(password, &params).unwrap();
        let verifier = make_verifier(&key).unwrap();
        meta_insert(
            c,
            "argon2id",
            &serde_json::to_string(&params).unwrap(),
            &verifier,
            Some("提示"),
            300,
        )
        .unwrap();
        key
    }

    fn input(title: &str, password: &str) -> VaultItemInput {
        VaultItemInput {
            title: title.into(),
            username: "hamster@example.com".into(),
            url: "https://example.com".into(),
            password: password.into(),
            notes: "备注".into(),
            favorite: false,
        }
    }

    #[test]
    fn meta_lifecycle() {
        let c = conn();
        assert!(meta_get(&c).unwrap().is_none());
        let key = setup_vault(&c, "master-pw");
        let m = meta_get(&c).unwrap().unwrap();
        assert_eq!(m.auto_lock_secs, 300);
        assert_eq!(m.hint.as_deref(), Some("提示"));
        assert!(verify(&key, &m.verifier));
        meta_set_auto_lock(&c, 120).unwrap();
        assert_eq!(meta_get(&c).unwrap().unwrap().auto_lock_secs, 120);
    }

    #[test]
    fn item_crud_and_reveal() {
        let c = conn();
        let key = setup_vault(&c, "master-pw");
        let item = item_create(&c, &key, &input("GitHub", "s3cret!")).unwrap();
        assert_eq!(item.title, "GitHub");
        assert!(!item.favorite);

        // 敏感字段不出现在明文列；信封可解
        let raw: String = c
            .query_row(
                "SELECT secret FROM vault_item WHERE id = ?1",
                [item.id as i64],
                |r| {
                    let blob: Vec<u8> = r.get(0)?;
                    Ok(String::from_utf8_lossy(&blob).into_owned())
                },
            )
            .unwrap();
        assert!(!raw.contains("s3cret"));
        let secret = item_reveal(&c, &key, item.id).unwrap();
        assert_eq!(secret.password, "s3cret!");
        assert_eq!(secret.notes, "备注");

        // 改密码 → password_updated_at 变（同秒精度下取 >=）；只改备注 → 不变
        let updated = item_update(&c, &key, item.id, &input("GitHub", "n3w-pw")).unwrap();
        assert!(updated.password_updated_at >= item.password_updated_at);
        let again = item_update(&c, &key, item.id, &input("GitHub", "n3w-pw")).unwrap();
        assert_eq!(again.password_updated_at, updated.password_updated_at);
        assert_eq!(item_reveal(&c, &key, item.id).unwrap().password, "n3w-pw");

        item_toggle_favorite(&c, item.id, true).unwrap();
        assert!(item_list(&c, None).unwrap()[0].favorite);

        item_soft_delete(&c, item.id).unwrap();
        assert!(item_list(&c, None).unwrap().is_empty());
        assert!(item_reveal(&c, &key, item.id).is_err());
        assert!(item_soft_delete(&c, item.id).is_err());
    }

    #[test]
    fn search_literal_like() {
        let c = conn();
        let key = setup_vault(&c, "master-pw");
        item_create(&c, &key, &input("GitHub", "a")).unwrap();
        item_create(&c, &key, &input("GitLab", "b")).unwrap();
        let zhihu = VaultItemInput {
            title: "知乎".into(),
            ..input("知乎", "c")
        };
        item_create(&c, &key, &zhihu).unwrap();

        assert_eq!(item_list(&c, Some("git")).unwrap().len(), 2);
        assert_eq!(item_list(&c, Some("hub")).unwrap().len(), 1);
        assert_eq!(item_list(&c, Some("知乎")).unwrap().len(), 1);
        // LIKE 通配符按字面处理
        assert!(item_list(&c, Some("%")).unwrap().is_empty());
        assert!(item_list(&c, Some("  ")).unwrap().len() == 3); // 空白视作列出全部
    }

    #[test]
    fn change_password_reencrypts_everything() {
        let c = conn();
        let old_key = setup_vault(&c, "old-master");
        let a = item_create(&c, &old_key, &input("GitHub", "pw-a")).unwrap();
        let b = item_create(&c, &old_key, &input("GitLab", "pw-b")).unwrap();
        item_soft_delete(&c, b.id).unwrap(); // 回收站条目也要换钥匙

        let new_params = fresh_params();
        let new_key = derive_key("new-master", &new_params).unwrap();
        assert_eq!(reencrypt_all(&c, &old_key, &new_key).unwrap(), 2);
        meta_rotate(
            &c,
            &serde_json::to_string(&new_params).unwrap(),
            &make_verifier(&new_key).unwrap(),
            None,
        )
        .unwrap();

        // 旧钥匙解不开，新钥匙全通（含软删除的 b）
        assert!(item_reveal(&c, &old_key, a.id).is_err());
        assert_eq!(item_reveal(&c, &new_key, a.id).unwrap().password, "pw-a");
        let b_raw: Vec<u8> = c
            .query_row(
                "SELECT secret FROM vault_item WHERE id = ?1",
                [b.id as i64],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<VaultSecret>(&crypto::open(&new_key, &b_raw).unwrap())
                .unwrap()
                .password,
            "pw-b"
        );

        let m = meta_get(&c).unwrap().unwrap();
        assert!(verify(&new_key, &m.verifier));
        assert!(!verify(&old_key, &m.verifier));
    }

    #[test]
    fn input_validation() {
        let c = conn();
        let key = setup_vault(&c, "m");
        assert!(item_create(&c, &key, &input("  ", "x")).is_err());
        let long_title = "x".repeat(101);
        assert!(item_create(&c, &key, &input(&long_title, "x")).is_err());
        let ok = input("正常", "x");
        assert!(item_create(&c, &key, &ok).is_ok());
    }
}
