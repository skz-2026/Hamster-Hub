//! 索引库：SQLite（WAL）+ FTS5 trigram。派生数据——损坏即重建，绝不影响源文件。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension};

use molto_core::error::{MoltoError, Result};
use molto_core::{IndexStatus, SearchHit, SearchQuery};

/// 解析协议版本：升级时自动清空派生数据重建。
/// v3：codex 注入上下文（AGENTS.md 等）降级 system、标题取真实首条用户消息。
const SCHEMA_REV: &str = "3";

/// 短查询（< TRIGRAM_MIN 字符）退化为 LIKE 扫描：trigram 最小匹配长度为 3，
/// 1~2 字（中文常见）走全表 LIKE，量级内可接受（G5 预算内）。
const TRIGRAM_MIN: usize = 3;

pub struct IndexStore {
    conn: Mutex<Connection>,
    path: PathBuf,
}

/// 一次摄取块的落库输入（ingest 层组装；偏移/行基址由其精确计算）。
pub struct ApplyChunk<'a> {
    pub agent: &'a str,
    pub source_path: &'a str,
    pub source_size: i64,
    pub source_mtime: i64,
    /// 摄取后的新字节游标
    pub new_byte_offset: i64,
    /// 摄取后的新行数游标
    pub new_line_base: i64,
    /// 本块首行之前的行数（消息 seq 基址）
    pub seq_line_base: i64,
    pub parse: &'a molto_core::ChunkParse,
}

/// Mutex 中毒恢复（持锁线程 panic 的极端场景下继续服务，与会话宿主同一约定）。
fn lock_ok(mutex: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl IndexStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| MoltoError::io(dir, e))?;
        }
        let conn = Connection::open(path)
            .map_err(|e| MoltoError::Other(format!("打开会话索引失败：{e}")))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(sqlite_err)?;
        let store = Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = lock_ok(&self.conn);
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta(
              key TEXT PRIMARY KEY, value TEXT
            );
            CREATE TABLE IF NOT EXISTS sessions(
              agent TEXT NOT NULL,
              session_key TEXT NOT NULL,
              project_path TEXT,
              title TEXT NOT NULL DEFAULT '',
              message_count INTEGER NOT NULL DEFAULT 0,
              source_path TEXT NOT NULL,
              source_size INTEGER NOT NULL DEFAULT 0,
              source_mtime INTEGER NOT NULL DEFAULT 0,
              parse_offset INTEGER NOT NULL DEFAULT 0,
              parse_lines INTEGER NOT NULL DEFAULT 0,
              indexed_at INTEGER NOT NULL DEFAULT 0,
              PRIMARY KEY(agent, session_key)
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_source
              ON sessions(agent, source_path);
            CREATE TABLE IF NOT EXISTS messages(
              agent TEXT NOT NULL,
              session_key TEXT NOT NULL,
              seq INTEGER NOT NULL,
              role TEXT NOT NULL,
              text TEXT NOT NULL,
              tool_name TEXT,
              at INTEGER,
              PRIMARY KEY(agent, session_key, seq)
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
              text, agent UNINDEXED, session_key UNINDEXED, seq UNINDEXED,
              tokenize='trigram'
            );
            "#,
        )
        .map_err(sqlite_err)?;
        // 解析协议升级时自动清空派生数据（随后由摄取流程重建）
        let rev: Option<String> = conn
            .query_row("SELECT value FROM meta WHERE key = 'schema_rev'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(sqlite_err)?;
        if rev.as_deref() != Some(SCHEMA_REV) {
            conn.execute_batch(
                "DELETE FROM messages_fts; DELETE FROM messages; DELETE FROM sessions;",
            )
            .map_err(sqlite_err)?;
            conn.execute(
                "INSERT OR REPLACE INTO meta(key, value) VALUES ('schema_rev', ?1)",
                [SCHEMA_REV],
            )
            .map_err(sqlite_err)?;
        }
        Ok(())
    }

    /// 源文件游标：(source_size, source_mtime, parse_offset(bytes), parse_lines)。
    pub fn source_state(
        &self,
        agent: &str,
        source_path: &str,
    ) -> Result<Option<(i64, i64, i64, i64)>> {
        let conn = lock_ok(&self.conn);
        conn.query_row(
            "SELECT source_size, source_mtime, parse_offset, parse_lines
             FROM sessions WHERE agent = ?1 AND source_path = ?2",
            [agent, source_path],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(sqlite_err)
    }

    /// 落一个解析块（消息追加 + FTS 写入 + 标题回填 + 游标推进）。单事务。
    pub fn apply_chunk(&self, chunk: &ApplyChunk<'_>) -> Result<()> {
        let mut conn = lock_ok(&self.conn);
        let tx = conn.transaction().map_err(sqlite_err)?;

        let existing_key: Option<String> = tx
            .query_row(
                "SELECT session_key FROM sessions WHERE agent = ?1 AND source_path = ?2",
                [chunk.agent, chunk.source_path],
                |r| r.get(0),
            )
            .optional()
            .map_err(sqlite_err)?;
        // 增量块可能不含 session_key：沿用既有归属；首块无法归属则跳过（不推游标）
        let Some(session_key) = chunk.parse.session_key.clone().or(existing_key) else {
            return Ok(());
        };

        let existing_title: String = tx
            .query_row(
                "SELECT title FROM sessions WHERE agent = ?1 AND session_key = ?2",
                [&chunk.agent.to_string(), &session_key],
                |r| r.get(0),
            )
            .optional()
            .map_err(sqlite_err)?
            .unwrap_or_default();
        let title = match &chunk.parse.title {
            Some(t) if existing_title.is_empty() => t.clone(),
            _ => existing_title,
        };
        let added = chunk.parse.messages.len() as i64;
        let now = now_ms();

        tx.execute(
            "INSERT INTO sessions(
               agent, session_key, project_path, title, message_count,
               source_path, source_size, source_mtime, parse_offset, parse_lines, indexed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(agent, session_key) DO UPDATE SET
               project_path = COALESCE(excluded.project_path, sessions.project_path),
               title = excluded.title,
               message_count = sessions.message_count + ?5,
               source_size = excluded.source_size,
               source_mtime = excluded.source_mtime,
               parse_offset = excluded.parse_offset,
               parse_lines = excluded.parse_lines,
               indexed_at = excluded.indexed_at",
            rusqlite::params![
                chunk.agent,
                session_key,
                chunk.parse.project_path,
                title,
                added,
                chunk.source_path,
                chunk.source_size,
                chunk.source_mtime,
                chunk.new_byte_offset,
                chunk.new_line_base,
                now,
            ],
        )
        .map_err(sqlite_err)?;

        for (rel_seq, m) in &chunk.parse.messages {
            let seq = chunk.seq_line_base + *rel_seq as i64;
            tx.execute(
                "INSERT OR REPLACE INTO messages(agent, session_key, seq, role, text, tool_name, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    chunk.agent,
                    session_key,
                    seq,
                    serde_role(m.role),
                    m.text,
                    m.tool_name,
                    m.at,
                ],
            )
            .map_err(sqlite_err)?;
            tx.execute(
                "INSERT INTO messages_fts(text, agent, session_key, seq) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![m.text, chunk.agent, session_key, seq],
            )
            .map_err(sqlite_err)?;
        }

        tx.commit().map_err(sqlite_err)
    }

    /// 会话的源文件路径（删除会话前定位落盘文件）。
    pub fn source_path_for(&self, agent: &str, session_key: &str) -> Result<Option<String>> {
        let conn = lock_ok(&self.conn);
        conn.query_row(
            "SELECT source_path FROM sessions WHERE agent = ?1 AND session_key = ?2",
            [agent, session_key],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(sqlite_err)
    }

    /// 源文件收缩/重写：清掉该文件的会话与消息（下次全量重解析）。
    pub fn reset_file(&self, agent: &str, source_path: &str) -> Result<()> {
        let mut conn = lock_ok(&self.conn);
        let tx = conn.transaction().map_err(sqlite_err)?;
        let key: Option<String> = tx
            .query_row(
                "SELECT session_key FROM sessions WHERE agent = ?1 AND source_path = ?2",
                [agent, source_path],
                |r| r.get(0),
            )
            .optional()
            .map_err(sqlite_err)?;
        let Some(key) = key else {
            return Ok(());
        };
        for sql in [
            "DELETE FROM messages_fts WHERE agent = ?1 AND session_key = ?2",
            "DELETE FROM messages WHERE agent = ?1 AND session_key = ?2",
            "DELETE FROM sessions WHERE agent = ?1 AND session_key = ?2",
        ] {
            tx.execute(sql, rusqlite::params![agent, key])
                .map_err(sqlite_err)?;
        }
        tx.commit().map_err(sqlite_err)
    }

    /// 全文检索：≥3 字符走 FTS5 trigram；更短（中文常见 1~2 字）退化为 LIKE。
    /// 标题命中一并返回（排最前）。
    pub fn search(&self, q: &SearchQuery) -> Result<Vec<SearchHit>> {
        let text = q.text.trim().to_string();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let limit = q.limit.unwrap_or(50).min(200);
        let conn = lock_ok(&self.conn);

        // (agent, session_key, seq, title_override, snippet)
        let mut raw: Vec<(String, String, i64, String, String)> = Vec::new();

        if text.chars().count() >= TRIGRAM_MIN {
            let phrase = format!("\"{}\"", text.replace('"', "\"\""));
            let mut stmt = conn
                .prepare(
                    "SELECT agent, session_key, seq,
                            snippet(messages_fts, 0, '[', ']', '…', 12)
                     FROM messages_fts WHERE messages_fts MATCH ?1
                     ORDER BY rank LIMIT ?2",
                )
                .map_err(sqlite_err)?;
            let rows = stmt
                .query_map(rusqlite::params![phrase, (limit as i64) * 2], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })
                .map_err(sqlite_err)?;
            for (a, k, s, snip) in rows.flatten() {
                raw.push((a, k, s, String::new(), snip));
            }
        } else {
            let pattern = like_pattern(&text);
            let mut stmt = conn
                .prepare(
                    "SELECT agent, session_key, seq, text FROM messages
                     WHERE text LIKE ?1 LIMIT ?2",
                )
                .map_err(sqlite_err)?;
            let rows = stmt
                .query_map(rusqlite::params![pattern, (limit as i64) * 2], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })
                .map_err(sqlite_err)?;
            for (a, k, s, text) in rows.flatten() {
                let snippet = like_snippet(&text, q.text.trim());
                raw.push((a, k, s, String::new(), snippet));
            }
        }

        // 标题命中（排最前；同会话已有消息命中则不再重复）
        {
            let pattern = like_pattern(&text);
            let already: std::collections::BTreeSet<(String, String)> = raw
                .iter()
                .map(|(a, k, ..)| (a.clone(), k.clone()))
                .collect();
            let mut stmt = conn
                .prepare(
                    "SELECT agent, session_key, title FROM sessions
                     WHERE title LIKE ?1 LIMIT ?2",
                )
                .map_err(sqlite_err)?;
            let rows = stmt
                .query_map(rusqlite::params![pattern, limit as i64], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(sqlite_err)?;
            for (a, k, title) in rows.flatten() {
                if !already.contains(&(a.clone(), k.clone())) {
                    raw.insert(0, (a, k, 0, title, String::new()));
                }
            }
        }

        // 补元数据 + 过滤 + 去重截断
        let mut out: Vec<SearchHit> = Vec::with_capacity(raw.len());
        let mut seen = std::collections::BTreeSet::new();
        for (agent, key, seq, title_override, snippet) in raw {
            if !q.agents.is_empty() && !q.agents.iter().any(|a| a == &agent) {
                continue;
            }
            if !seen.insert((agent.clone(), key.clone(), seq)) {
                continue;
            }
            let meta: Option<(String, Option<String>)> = conn
                .query_row(
                    "SELECT title, project_path FROM sessions
                     WHERE agent = ?1 AND session_key = ?2",
                    [&agent, &key],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(sqlite_err)?;
            let Some((session_title, project_path)) = meta else {
                continue;
            };
            if let Some(proj) = &q.project {
                match &project_path {
                    Some(p) if norm_path(p) == norm_path(proj) => {}
                    _ => continue,
                }
            }
            let session_title = if title_override.is_empty() {
                session_title
            } else {
                title_override
            };
            out.push(SearchHit {
                agent,
                session_key: key,
                seq: seq.max(0) as usize,
                session_title,
                project_path,
                at: None,
                snippet,
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    pub fn stats(&self) -> Result<IndexStatus> {
        let conn = lock_ok(&self.conn);
        let sessions: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
            .map_err(sqlite_err)?;
        let messages: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
            .map_err(sqlite_err)?;
        let bytes = std::fs::metadata(&self.path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        Ok(IndexStatus {
            sessions,
            messages,
            bytes,
        })
    }

    /// 某 Agent 在指定项目下最新入索引的会话（活会话 → 索引键桥接；
    /// 路径归一比较，取 indexed_at 最新）。
    pub fn latest_for_project(
        &self,
        agent: &str,
        project_dir: &str,
    ) -> Result<Option<(String, String)>> {
        let conn = lock_ok(&self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT session_key, COALESCE(NULLIF(title, ''), session_key), project_path
                 FROM sessions WHERE agent = ?1 ORDER BY indexed_at DESC LIMIT 5",
            )
            .map_err(sqlite_err)?;
        let want = norm_path(project_dir);
        let rows = stmt
            .query_map([agent], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, {
                    let p: Option<String> = r.get(2)?;
                    p
                }))
            })
            .map_err(sqlite_err)?;
        for row in rows.flatten() {
            let (key, title, project) = row;
            if let Some(p) = project {
                if norm_path(&p) == want {
                    return Ok(Some((key, title)));
                }
            }
        }
        Ok(None)
    }

    /// 取一条消息的前缀文本（冒烟/诊断用：以其为检索词验证摄取-检索回环）。
    pub fn first_message_sample(&self, chars: usize) -> Result<Option<String>> {
        let conn = lock_ok(&self.conn);
        conn.query_row("SELECT text FROM messages LIMIT 1", [], |r| {
            r.get::<_, String>(0)
        })
        .optional()
        .map(|opt| {
            opt.map(|t| {
                t.chars()
                    .skip(4)
                    .take(chars.max(TRIGRAM_MIN))
                    .collect::<String>()
            })
        })
        .map_err(sqlite_err)
    }

    /// 会话列表（molto-cli 只读共享面）：按源文件 mtime 倒序（≈最近活跃优先），
    /// `agent` 过滤可选。
    pub fn list_sessions(
        &self,
        agent: Option<&str>,
        limit: usize,
    ) -> Result<Vec<molto_core::SessionMeta>> {
        let conn = lock_ok(&self.conn);
        let sql = if agent.is_some() {
            "SELECT agent, session_key, title, project_path, message_count, source_mtime, source_path
             FROM sessions WHERE agent = ?1 ORDER BY source_mtime DESC LIMIT ?2"
        } else {
            "SELECT agent, session_key, title, project_path, message_count, source_mtime, source_path
             FROM sessions ORDER BY source_mtime DESC LIMIT ?1"
        };
        let mut stmt = conn.prepare(sql).map_err(sqlite_err)?;
        let map_row = |r: &rusqlite::Row| -> rusqlite::Result<molto_core::SessionMeta> {
            Ok(molto_core::SessionMeta {
                agent: r.get(0)?,
                session_key: r.get(1)?,
                title: r.get(2)?,
                project_path: r.get(3)?,
                message_count: r.get(4)?,
                last_active_at: r.get(5)?,
                source_path: r.get(6)?,
            })
        };
        let rows = if let Some(a) = agent {
            stmt.query_map(rusqlite::params![a, limit as i64], map_row)
        } else {
            stmt.query_map(rusqlite::params![limit as i64], map_row)
        }
        .map_err(sqlite_err)?;
        Ok(rows.flatten().collect())
    }

    /// 单个会话元数据（molto-cli session get 的头部；不存在返回 None）。
    pub fn session_meta(
        &self,
        agent: &str,
        session_key: &str,
    ) -> Result<Option<molto_core::SessionMeta>> {
        let conn = lock_ok(&self.conn);
        conn.query_row(
            "SELECT agent, session_key, title, project_path, message_count, source_mtime, source_path
             FROM sessions WHERE agent = ?1 AND session_key = ?2",
            [agent, session_key],
            |r| {
                Ok(molto_core::SessionMeta {
                    agent: r.get(0)?,
                    session_key: r.get(1)?,
                    title: r.get(2)?,
                    project_path: r.get(3)?,
                    message_count: r.get(4)?,
                    last_active_at: r.get(5)?,
                    source_path: r.get(6)?,
                })
            },
        )
        .optional()
        .map_err(sqlite_err)
    }

    /// 从 from_seq 起的升序消息分页（原生视图加载历史；from_seq=-1 表示从尾部）。
    pub fn messages_page(
        &self,
        agent: &str,
        session_key: &str,
        from_seq: i64,
        limit: i64,
    ) -> Result<molto_core::SessionMessagesPage> {
        let conn = lock_ok(&self.conn);
        let total: i64 = conn
            .query_row(
                "SELECT message_count FROM sessions WHERE agent = ?1 AND session_key = ?2",
                [agent, session_key],
                |r| r.get(0),
            )
            .optional()
            .map_err(sqlite_err)?
            .unwrap_or(0);
        // from_seq < 0：从尾部取最近 limit 条（升序返回）
        let ascending = from_seq >= 0;
        // 方向是关键字而非值：按语义拼入（from_seq/limit 为绑定参数）
        let sql = format!(
            "SELECT seq, role, text, tool_name, at FROM messages
             WHERE agent = ?1 AND session_key = ?2 AND seq >= ?3
             ORDER BY seq {} LIMIT ?4",
            if ascending { "ASC" } else { "DESC" }
        );
        let mut stmt = conn.prepare(&sql).map_err(sqlite_err)?;
        let rows = stmt
            .query_map(
                rusqlite::params![
                    agent,
                    session_key,
                    if ascending { from_seq.max(0) } else { -1 },
                    limit
                ],
                |r| {
                    let seq: i64 = r.get(0)?;
                    let role: String = r.get(1)?;
                    let text: String = r.get(2)?;
                    let tool_name: Option<String> = r.get(3)?;
                    let at: Option<i64> = r.get(4)?;
                    Ok((seq, role, text, tool_name, at))
                },
            )
            .map_err(sqlite_err)?;
        type MessageRow = (i64, String, String, Option<String>, Option<i64>);
        let mut rows: Vec<MessageRow> = rows.flatten().collect();
        if !ascending {
            rows.reverse();
        }
        let messages = rows
            .into_iter()
            .map(
                |(seq, role, text, tool_name, at)| molto_core::SnapshotMessage {
                    seq: seq.max(0) as usize,
                    role: parse_role(&role),
                    text,
                    tool_name,
                    at,
                },
            )
            .collect();
        Ok(molto_core::SessionMessagesPage { total, messages })
    }

    /// 命中消息的上下文窗口（seq ± window，按序返回）。
    pub fn messages_around(
        &self,
        agent: &str,
        session_key: &str,
        seq: i64,
        window: i64,
    ) -> Result<Vec<molto_core::SnapshotMessage>> {
        let conn = lock_ok(&self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT seq, role, text, tool_name, at FROM messages
                 WHERE agent = ?1 AND session_key = ?2 AND seq BETWEEN ?3 AND ?4
                 ORDER BY seq",
            )
            .map_err(sqlite_err)?;
        let rows = stmt
            .query_map(
                rusqlite::params![agent, session_key, (seq - window).max(0), seq + window],
                |r| {
                    let seq: i64 = r.get(0)?;
                    let role: String = r.get(1)?;
                    let text: String = r.get(2)?;
                    let tool_name: Option<String> = r.get(3)?;
                    let at: Option<i64> = r.get(4)?;
                    Ok((seq, role, text, tool_name, at))
                },
            )
            .map_err(sqlite_err)?;
        let mut out = Vec::new();
        for (seq, role, text, tool_name, at) in rows.flatten() {
            let role = parse_role(&role);
            out.push(molto_core::SnapshotMessage {
                seq: seq.max(0) as usize,
                role,
                text,
                tool_name,
                at,
            });
        }
        Ok(out)
    }

    /// 一键重建：清空全部派生数据（源文件不受任何影响，红线①）。
    pub fn rebuild(&self) -> Result<()> {
        let conn = lock_ok(&self.conn);
        conn.execute_batch("DELETE FROM messages_fts; DELETE FROM messages; DELETE FROM sessions;")
            .map_err(sqlite_err)
    }
}

fn like_pattern(text: &str) -> String {
    format!(
        "%{}%",
        text.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}

fn norm_path(p: &str) -> String {
    p.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

fn like_snippet(text: &str, needle: &str) -> String {
    let Some(byte_pos) = text.to_lowercase().find(&needle.to_lowercase()) else {
        return text.chars().take(100).collect();
    };
    let chars_before = text[..byte_pos].chars().count();
    text.chars()
        .skip(chars_before.saturating_sub(20))
        .take(100)
        .collect()
}

fn parse_role(role: &str) -> molto_core::SnapshotRole {
    match role {
        "assistant" => molto_core::SnapshotRole::Assistant,
        "thinking" => molto_core::SnapshotRole::Thinking,
        "tool" => molto_core::SnapshotRole::Tool,
        "system" => molto_core::SnapshotRole::System,
        _ => molto_core::SnapshotRole::User,
    }
}

fn serde_role(role: molto_core::SnapshotRole) -> &'static str {
    match role {
        molto_core::SnapshotRole::User => "user",
        molto_core::SnapshotRole::Assistant => "assistant",
        molto_core::SnapshotRole::Thinking => "thinking",
        molto_core::SnapshotRole::Tool => "tool",
        molto_core::SnapshotRole::System => "system",
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn sqlite_err(e: rusqlite::Error) -> MoltoError {
    MoltoError::Other(format!("会话索引错误：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use molto_core::model::session::{ChunkParse, SnapshotMessage, SnapshotRole};

    /// 造一个摄取块并落库：会话键 / 项目 / 标题 + n 条消息。
    fn apply(
        s: &IndexStore,
        agent: &str,
        key: &str,
        project: Option<&str>,
        msgs: &[&str],
        indexed_at: i64,
    ) {
        let parse = ChunkParse {
            session_key: Some(key.to_string()),
            project_path: project.map(String::from),
            title: msgs.first().map(|s| s.chars().take(80).collect()),
            messages: msgs
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    (
                        i,
                        SnapshotMessage {
                            seq: i,
                            role: if i % 2 == 0 {
                                SnapshotRole::User
                            } else {
                                SnapshotRole::Assistant
                            },
                            text: text.to_string(),
                            tool_name: None,
                            at: Some(indexed_at),
                        },
                    )
                })
                .collect(),
        };
        let path = format!("/src/{key}.jsonl");
        s.apply_chunk(&ApplyChunk {
            agent,
            source_path: &path,
            source_size: 100,
            source_mtime: indexed_at,
            new_byte_offset: 100,
            new_line_base: msgs.len() as i64,
            seq_line_base: 0,
            parse: &parse,
        })
        .unwrap();
    }

    fn store() -> IndexStore {
        IndexStore::open(std::path::Path::new(":memory:")).unwrap()
    }

    /// 回归：latest_for_project 曾引用不存在的 last_active_at 列且缺第 3 列，
    /// 查询报错被前端吞掉 → 原生视图永远显示「未入索引」。必须正常返回。
    #[test]
    fn latest_for_project_matches_by_normalized_path() {
        let s = store();
        apply(&s, "codex", "old", Some("D:/ksa/Molto"), &["旧会话"], 1000);
        apply(
            &s,
            "codex",
            "new",
            Some("D:\\ksa\\Molto"),
            &["新会话"],
            2000,
        );
        apply(&s, "codex", "other", Some("D:/other"), &["别处"], 3000);

        let found = s.latest_for_project("codex", "d:/ksa/molto/").unwrap();
        let (key, title) = found.expect("同项目会话必须可找到（反斜杠/正斜杠/大小写归一）");
        assert_eq!(key, "new", "indexed_at 最新者优先");
        assert_eq!(title, "新会话");

        assert!(s
            .latest_for_project("codex", "D:/missing")
            .unwrap()
            .is_none());
        assert!(s
            .latest_for_project("claude", "D:/ksa/Molto")
            .unwrap()
            .is_none());
    }

    /// 分页：from_seq=-1 取尾部升序；指定 from_seq 从该行起升序；total 正确。
    #[test]
    fn messages_page_tail_and_ascending() {
        let s = store();
        let texts: Vec<&str> = vec!["m0", "m1", "m2", "m3", "m4"];
        apply(&s, "codex", "k", Some("D:/p"), &texts, 1000);

        let tail = s.messages_page("codex", "k", -1, 2).unwrap();
        assert_eq!(tail.total, 5);
        assert_eq!(
            tail.messages
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>(),
            vec!["m3", "m4"],
            "尾部模式取最近 limit 条且升序返回"
        );

        let from2 = s.messages_page("codex", "k", 2, 10).unwrap();
        assert_eq!(
            from2
                .messages
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>(),
            vec!["m2", "m3", "m4"]
        );
    }
}
