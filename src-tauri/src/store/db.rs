//! SQLite 打开与迁移（WAL；user_version 顺序迁移，只增不改）

use rusqlite::Connection;

use crate::error::AppError;

/// 迁移列表：(名称, SQL)。索引即版本号，新增只允许追加。
const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../../migrations/0001_init.sql")),
    (
        "0002_fileindex",
        include_str!("../../migrations/0002_fileindex.sql"),
    ),
    (
        "0003_apps_pinyin",
        include_str!("../../migrations/0003_apps_pinyin.sql"),
    ),
    (
        "0004_todo_due_remind",
        include_str!("../../migrations/0004_todo_due_remind.sql"),
    ),
    (
        "0005_focus_session",
        include_str!("../../migrations/0005_focus_session.sql"),
    ),
    (
        "0006_vault",
        include_str!("../../migrations/0006_vault.sql"),
    ),
];

/// 打开指定路径的库并迁移（后台线程各自开连接时复用）
pub fn open_at(path: &std::path::Path) -> Result<Connection, AppError> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&conn)?;
    Ok(conn)
}

/// 迁移到最新版本（open_at 与测试复用）
pub(crate) fn migrate(conn: &Connection) -> Result<(), AppError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let target = (i + 1) as i64;
        if version < target {
            conn.execute_batch(sql)
                .map_err(|e| AppError::db(format!("迁移 {name} 失败: {e}")))?;
            conn.pragma_update(None, "user_version", target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_and_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap(); // 幂等
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
        // 核心表存在
        conn.execute("INSERT INTO todo(content) VALUES('t')", [])
            .unwrap();
    }
}
