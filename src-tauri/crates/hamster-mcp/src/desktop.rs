//! desktop 工具组：仓鼠Hub 本地数据与系统能力 → MCP 工具（零 Tauri）。
//!
//! 数据源：HAMSTER_DB_PATH 指向主库 hamsterhub.db（SQLite WAL，busy_timeout
//! 与主进程并发共存）。系统能力：hamster-platform（ShellExecute 启动 / 音量 COM）。
//! 安全边界：file_open 仅放行设置里配置的索引根目录（settings JSON
//! `file_index.roots`，~ 前缀按 USERPROFILE 展开）；computer_* 工具的开关在
//! mcp.rs 侧（HAMSTER_CU_MODE，默认关）。

use std::path::{Path, PathBuf};

use hamster_core::HamsterError;
use rusqlite::{params, Connection, OpenFlags};

/// 一枚应用命中（app_fts 检索）
pub struct AppHit {
    pub app_key: String,
    pub display_name: String,
}

/// 一枚文件命中（file_fts 检索）
pub struct FileHit {
    pub path: String,
    pub name: String,
    pub dir: String,
}

/// 一条待办
pub struct TodoItem {
    pub id: i64,
    pub content: String,
    pub done: bool,
    pub due_date: Option<String>,
}

/// 打开主库连接（读写；WAL 下与主进程并发，busy_timeout 容忍写锁）
pub fn open_db(path: &Path) -> Result<Connection, HamsterError> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| HamsterError::Other(format!("打开主库失败（{}）：{e}", path.display())))?;
    conn.busy_timeout(std::time::Duration::from_secs(3))
        .map_err(|e| HamsterError::Other(format!("设置 busy_timeout 失败：{e}")))?;
    Ok(conn)
}

fn err_db(path: impl std::fmt::Display, e: rusqlite::Error) -> HamsterError {
    HamsterError::Other(format!("主库查询失败（{path}）：{e}"))
}

/// FTS5 短语前缀 MATCH 串：`"微信"*` 命中「微信 / 微信支付」及拼音前缀
fn phrase_prefix(q: &str) -> String {
    let q = q.replace('"', " ");
    format!("\"{}\"*", q.trim())
}

// ===== 应用 =====

pub fn app_search(conn: &Connection, query: &str, limit: i64) -> Result<Vec<AppHit>, HamsterError> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let limit = limit.clamp(1, 25);
    let mut stmt = conn
        .prepare(
            "SELECT m.app_key, m.display_name FROM app_fts f
             JOIN app_meta m ON m.rowid = f.rowid
             WHERE app_fts MATCH ?1 ORDER BY bm25(app_fts) LIMIT ?2",
        )
        .map_err(|e| err_db("app_fts", e))?;
    let mut hits: Vec<AppHit> = stmt
        .query_map(params![phrase_prefix(q), limit], |r| {
            Ok(AppHit {
                app_key: r.get(0)?,
                display_name: r.get(1)?,
            })
        })
        .map_err(|e| err_db("app_fts", e))?
        .collect::<Result<_, _>>()
        .map_err(|e| err_db("app_fts", e))?;
    // FTS 是整 token 索引（unicode61 下 CJK 连续串为单 token），中文子串回退 LIKE
    if hits.is_empty() {
        let mut stmt = conn
            .prepare(
                "SELECT app_key, display_name FROM app_meta
                 WHERE display_name LIKE '%' || ?1 || '%' LIMIT ?2",
            )
            .map_err(|e| err_db("app_meta", e))?;
        hits = stmt
            .query_map(params![q, limit], |r| {
                Ok(AppHit {
                    app_key: r.get(0)?,
                    display_name: r.get(1)?,
                })
            })
            .map_err(|e| err_db("app_meta", e))?
            .collect::<Result<_, _>>()
            .map_err(|e| err_db("app_meta", e))?;
    }
    Ok(hits)
}

/// 按 app_key 启动应用：查 exec_target → 已运行则激活现有窗口，否则 ShellExecute；
/// 写入 usage_log。保持主界面「常用组」与 agent 启动同源。
pub fn app_launch(conn: &Connection, app_key: &str) -> Result<String, HamsterError> {
    let target: Option<String> = conn
        .query_row(
            "SELECT exec_target FROM app_meta WHERE app_key = ?1",
            [app_key],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(err_db("app_meta", other)),
        })?;
    let Some(target) = target else {
        return Err(HamsterError::not_found(format!("应用 {app_key}（未索引）")));
    };
    // 已运行则激活现有窗口（浏览器等单进程多窗应用，重复 shell_open 会再开新窗）
    if hamster_platform::shell::activate_running(Path::new(&target)) {
        let _ = conn.execute(
            "INSERT INTO usage_log(target_key, target_kind, at) VALUES (?1, 'app', unixepoch())",
            params![app_key],
        );
        return Ok(target);
    }
    if !hamster_platform::shell::shell_open(Path::new(&target)) {
        return Err(HamsterError::Other(format!("启动失败：{target}")));
    }
    let _ = conn.execute(
        "INSERT INTO usage_log(target_key, target_kind, at) VALUES (?1, 'app', unixepoch())",
        params![app_key],
    );
    Ok(target)
}

// ===== 文件 =====

pub fn file_search(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> Result<Vec<FileHit>, HamsterError> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let limit = limit.clamp(1, 25);
    let mut stmt = conn
        .prepare(
            "SELECT m.path, m.name, m.dir FROM file_fts f
             JOIN file_meta m ON m.id = f.rowid
             WHERE file_fts MATCH ?1 ORDER BY bm25(file_fts) LIMIT ?2",
        )
        .map_err(|e| err_db("file_fts", e))?;
    let hits = stmt
        .query_map(params![phrase_prefix(q), limit], |r| {
            Ok(FileHit {
                path: r.get(0)?,
                name: r.get(1)?,
                dir: r.get(2)?,
            })
        })
        .map_err(|e| err_db("file_fts", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| err_db("file_fts", e))?;
    if !hits.is_empty() {
        return Ok(hits);
    }
    let mut stmt = conn
        .prepare(
            "SELECT path, name, dir FROM file_meta
             WHERE name LIKE '%' || ?1 || '%' LIMIT ?2",
        )
        .map_err(|e| err_db("file_meta", e))?;
    let rows = stmt
        .query_map(params![q, limit], |r| {
            Ok(FileHit {
                path: r.get(0)?,
                name: r.get(1)?,
                dir: r.get(2)?,
            })
        })
        .map_err(|e| err_db("file_meta", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| err_db("file_meta", e))?;
    Ok(rows)
}

/// ~ 前缀按 USERPROFILE 展开（settings.roots 的存储形态与 fileindex 一致）
fn expand_root(root: &str) -> PathBuf {
    if let Some(rest) = root.strip_prefix('~') {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        return PathBuf::from(home).join(rest.trim_start_matches(['\\', '/']));
    }
    PathBuf::from(root)
}

/// file_open 白名单：路径必须落在设置配置的索引根目录内（缺配置 = 全部拒绝）
pub fn file_allowed(conn: &Connection, path: &Path) -> Result<bool, HamsterError> {
    let json: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = 'app'", [], |r| {
            r.get(0)
        })
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(err_db("settings", other)),
        })?;
    let Some(json) = json else {
        return Ok(false);
    };
    let roots = serde_json::from_str::<serde_json::Value>(&json)
        .ok()
        .and_then(|v| {
            v.pointer("/file_index/roots")
                .and_then(|r| r.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(expand_root))
                        .collect::<Vec<_>>()
                })
        })
        .unwrap_or_default();
    if roots.is_empty() {
        return Ok(false);
    }
    let Some(target) = path.canonicalize().ok() else {
        return Ok(false);
    };
    Ok(roots
        .iter()
        .filter_map(|r| r.canonicalize().ok())
        .any(|r| target.starts_with(&r)))
}

/// 打开文件（白名单校验后 ShellExecute）
pub fn file_open(conn: &Connection, path: &str) -> Result<(), HamsterError> {
    let p = Path::new(path);
    if !p.is_file() {
        return Err(HamsterError::not_found(format!("文件不存在：{path}")));
    }
    if !file_allowed(conn, p)? {
        return Err(HamsterError::Other(format!(
            "拒绝打开：{path} 不在索引根目录白名单内（设置 → 文件索引）"
        )));
    }
    if !hamster_platform::shell::shell_open(p) {
        return Err(HamsterError::Other(format!("打开失败：{path}")));
    }
    Ok(())
}

// ===== 待办 =====

pub fn todo_list(conn: &Connection, limit: i64) -> Result<Vec<TodoItem>, HamsterError> {
    let limit = limit.clamp(1, 50);
    let mut stmt = conn
        .prepare(
            "SELECT id, content, done, due_date FROM todo
             ORDER BY done ASC, sort_order ASC, id DESC LIMIT ?1",
        )
        .map_err(|e| err_db("todo", e))?;
    let rows = stmt
        .query_map(params![limit], |r| {
            Ok(TodoItem {
                id: r.get(0)?,
                content: r.get(1)?,
                done: r.get::<_, i64>(2)? != 0,
                due_date: r.get(3)?,
            })
        })
        .map_err(|e| err_db("todo", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| err_db("todo", e))?;
    Ok(rows)
}

pub fn todo_create(
    conn: &Connection,
    content: &str,
    due_date: Option<&str>,
) -> Result<i64, HamsterError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(HamsterError::config_invalid("待办内容不能为空"));
    }
    conn.execute(
        "INSERT INTO todo(content, due_date) VALUES (?1, ?2)",
        params![content, due_date],
    )
    .map_err(|e| err_db("todo", e))?;
    Ok(conn.last_insert_rowid())
}

pub fn todo_set_done(conn: &Connection, id: i64, done: bool) -> Result<(), HamsterError> {
    let n = conn
        .execute(
            "UPDATE todo SET done = ?1, completed_at = CASE WHEN ?1 != 0 THEN unixepoch() ELSE NULL END
             WHERE id = ?2",
            params![done as i64, id],
        )
        .map_err(|e| err_db("todo", e))?;
    if n == 0 {
        return Err(HamsterError::not_found(format!("待办 #{id}")));
    }
    Ok(())
}

// ===== 音量 =====

pub fn volume_get() -> Result<(f32, bool), HamsterError> {
    hamster_platform::volume::get()
        .map(|s| (s.level, s.muted))
        .map_err(HamsterError::Other)
}

pub fn volume_set(level: f32, mute: Option<bool>) -> Result<(), HamsterError> {
    let level = level.clamp(0.0, 1.0);
    let muted = match mute {
        Some(m) => m,
        None => hamster_platform::volume::get()
            .map(|s| s.muted)
            .unwrap_or(false),
    };
    hamster_platform::volume::set(level, muted).map_err(HamsterError::Other)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        // 与主库同迁移链（相对本文件 = src-tauri/migrations）
        c.execute_batch(include_str!("../../../migrations/0001_init.sql"))
            .unwrap();
        c.execute_batch(include_str!("../../../migrations/0002_fileindex.sql"))
            .unwrap();
        c.execute_batch(include_str!("../../../migrations/0003_apps_pinyin.sql"))
            .unwrap();
        c
    }

    fn seed_app(c: &Connection, key: &str, name: &str, target: &str) {
        c.execute(
            "INSERT INTO app_meta(app_key, display_name, exec_target, kind, indexed_at, pinyin_full, pinyin_initials)
             VALUES (?1, ?2, ?3, 'lnk', unixepoch(), '', '')",
            params![key, name, target],
        )
        .unwrap();
    }

    #[test]
    fn app_search_finds_by_name_and_pinyin_prefix() {
        let c = conn();
        seed_app(&c, "C:\\lnk\\wechat.lnk", "微信", "C:\\WeChat\\WeChat.exe");
        seed_app(
            &c,
            "C:\\lnk\\code.lnk",
            "Visual Studio Code",
            "C:\\code\\Code.exe",
        );
        // 名称整 token 前缀
        let hits = app_search(&c, "微信", 8).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].app_key, "C:\\lnk\\wechat.lnk");
        // ASCII 词首（pinyin_initials 空时回退 LIKE display_name）
        let hits = app_search(&c, "Visual", 8).unwrap();
        assert_eq!(hits.len(), 1);
        // 无命中
        assert!(app_search(&c, "不存在的东西", 8).unwrap().is_empty());
    }

    #[test]
    fn app_launch_requires_indexed_target() {
        let c = conn();
        assert!(app_launch(&c, "nope").is_err());
        seed_app(&c, "k", "名", "C:\\Windows\\System32\\notepad.exe");
        // 真启动不在单测里执行（会弹窗）：这里只验证查表路径错误分支
        assert!(app_launch(&c, "nope").is_err());
    }

    fn seed_file(c: &Connection, id: i64, path: &str, name: &str) {
        c.execute(
            "INSERT INTO file_meta(id, path, name, dir, size, mtime, kind, pinyin_full, pinyin_initials)
             VALUES (?1, ?2, ?3, 'D:\\\\docs', 1, 0, 'doc', '', '')",
            params![id, path, name],
        )
        .unwrap();
    }

    #[test]
    fn file_search_falls_back_to_like() {
        let c = conn();
        seed_file(&c, 1, "D:\\docs\\报销单.xlsx", "报销单.xlsx");
        let hits = file_search(&c, "报销", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].path.ends_with("报销单.xlsx"));
    }

    #[test]
    fn file_open_whitelist_denies_unconfigured_and_outside_roots() {
        let c = conn();
        // 未配置 settings → 全部拒绝
        assert!(file_open(&c, "D:\\docs\\a.txt").is_err());
        // 配置根目录后：外部路径拒绝
        c.execute(
            "INSERT INTO settings(key, value, updated_at) VALUES ('app', ?1, 0)",
            [r#"{"file_index":{"roots":["D:\\docs"]}}"#],
        )
        .unwrap();
        assert!(file_open(&c, "C:\\Windows\\notepad.exe").is_err());
    }

    #[test]
    fn todo_crud_roundtrip() {
        let c = conn();
        let id = todo_create(&c, "给仓鼠添粮", None).unwrap();
        todo_create(&c, "回消息", Some("2026-09-20")).unwrap();
        todo_set_done(&c, id, true).unwrap();
        let list = todo_list(&c, 10).unwrap();
        assert_eq!(list.len(), 2);
        // 已完成的排后面
        assert_eq!(list[0].content, "回消息");
        assert!(list[1].done);
        assert!(todo_set_done(&c, 999, true).is_err());
        assert!(todo_create(&c, "  ", None).is_err());
    }

    #[test]
    fn phrase_prefix_escapes_quotes() {
        assert_eq!(phrase_prefix("a\"b"), "\"a b\"*");
    }
}
