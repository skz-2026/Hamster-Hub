//! settings 表读写（key='app' 存整份 Settings JSON）

use rusqlite::{params, Connection, OptionalExtension};

use super::config::Settings;
use crate::error::AppError;

const KEY: &str = "app";

pub fn load(conn: &Connection) -> Result<Settings, AppError> {
    let json: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [KEY], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default())
}

pub fn save(conn: &Connection, settings: &Settings) -> Result<Settings, AppError> {
    let json = serde_json::to_string(settings)?;
    conn.execute(
        "INSERT INTO settings(key, value, updated_at) VALUES(?1, ?2, unixepoch())
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![KEY, json],
    )?;
    Ok(settings.clone())
}

// ===== 通用 KV（主屏布局等前端拥有的 JSON 文档）=====

pub const KV_ALLOWED_KEYS: &[&str] = &["home.layout", "bench.mySessions", "plugins.disabled", "assistant.session.v1"];

pub fn kv_load(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    let json: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(json)
}

pub fn kv_save(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO settings(key, value, updated_at) VALUES(?1, ?2, unixepoch())
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .unwrap();
        c
    }

    #[test]
    fn save_then_load_roundtrip() {
        let c = conn();
        let mut s = Settings::default();
        s.appearance.theme = "light".into();
        let saved = save(&c, &s).unwrap();
        assert_eq!(saved.appearance.theme, "light");
        let loaded = load(&c).unwrap();
        assert_eq!(loaded.appearance.theme, "light");
        assert_eq!(loaded.appearance.accent, "#FF8A3D");
    }

    #[test]
    fn load_missing_returns_default() {
        let c = conn();
        assert_eq!(load(&c).unwrap().behavior.language, "zh-CN");
    }
}
