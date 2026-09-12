//! `list_sessions` / `session_meta`（molto-cli 只读共享面）的索引单测。
//! FakeAdapter 模式同 ingest_search.rs：临时目录 + 自定义 JSONL，不触碰真实 ~/ 配置。

use std::path::PathBuf;

use molto_core::adapter::{AgentAdapter, Caps, CommandSpec};
use molto_core::error::Result;
use molto_core::model::agent::InstallInfo;
use molto_core::model::mcp::{McpServer, Scope};
use molto_core::model::session::{ChunkParse, SessionSource, SnapshotMessage, SnapshotRole};

use molto_index::{ingest_all, IndexStore};

struct FakeAdapter {
    root: PathBuf,
}

impl AgentAdapter for FakeAdapter {
    fn id(&self) -> &'static str {
        "fake"
    }
    fn name(&self) -> &'static str {
        "Fake"
    }
    fn detect(&self) -> Option<InstallInfo> {
        None
    }
    fn capabilities(&self) -> Caps {
        Caps::default()
    }
    fn mcp_path(&self, _scope: &Scope) -> Option<PathBuf> {
        None
    }
    fn read_mcp(&self, _raw: &str) -> Result<Vec<McpServer>> {
        Ok(vec![])
    }
    fn render_mcp(&self, _current: Option<&str>, _servers: &[McpServer]) -> Result<String> {
        Ok(String::new())
    }
    fn launch_cmd(&self, project_dir: &std::path::Path) -> CommandSpec {
        CommandSpec {
            program: "fake".into(),
            args: vec![],
            working_dir: project_dir.to_path_buf(),
        }
    }
    fn session_sources(&self) -> Vec<SessionSource> {
        vec![SessionSource {
            dir: self.root.join("sessions"),
            max_files: 100,
        }]
    }
    fn parse_chunk(&self, lines: &[String]) -> Option<ChunkParse> {
        let mut key = None;
        let mut project = None;
        let mut messages = Vec::new();
        for (i, raw) in lines.iter().enumerate() {
            let v: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
            if key.is_none() {
                key = v.get("id").and_then(|x| x.as_str()).map(String::from);
            }
            if project.is_none() {
                project = v.get("cwd").and_then(|x| x.as_str()).map(String::from);
            }
            let text = v.get("text").and_then(|x| x.as_str()).unwrap_or_default();
            let role = match v.get("role").and_then(|x| x.as_str()) {
                Some("assistant") => SnapshotRole::Assistant,
                _ => SnapshotRole::User,
            };
            messages.push((
                i,
                SnapshotMessage {
                    seq: i,
                    role,
                    text: text.to_string(),
                    tool_name: None,
                    at: None,
                },
            ));
        }
        Some(ChunkParse {
            session_key: key,
            project_path: project,
            title: None,
            messages,
        })
    }
}

fn setup() -> (tempfile::TempDir, IndexStore) {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();

    let line = |role: &str, text: &str| {
        format!(r#"{{"id":"s-a","cwd":"D:/p","role":"{role}","text":"{text}"}}"#)
    };
    std::fs::write(
        sessions.join("a.jsonl"),
        [line("user", "问题 A"), line("assistant", "答案 A")].join("\n"),
    )
    .unwrap();
    let line_b = |role: &str, text: &str| {
        format!(r#"{{"id":"s-b","cwd":"D:/p","role":"{role}","text":"{text}"}}"#)
    };
    std::fs::write(
        sessions.join("b.jsonl"),
        [line_b("user", "问题 B")].join("\n"),
    )
    .unwrap();
    // 让 b 比 a 新（mtime 决定排序）
    let new_time = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
    let f = std::fs::File::options()
        .append(true)
        .open(sessions.join("b.jsonl"))
        .unwrap();
    f.set_times(std::fs::FileTimes::new().set_modified(new_time))
        .unwrap();

    let store = IndexStore::open(&dir.path().join("index/sessions.db")).unwrap();
    let registry = molto_core::Registry::new(vec![Box::new(FakeAdapter {
        root: dir.path().to_path_buf(),
    })]);
    ingest_all(&registry, &store).unwrap();
    (dir, store)
}

#[test]
fn list_sessions_orders_by_recency_and_filters_by_agent() {
    let (_d, store) = setup();
    let all = store.list_sessions(None, 10).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].session_key, "s-b", "mtime 新的在前");
    assert!(all.iter().all(|s| s.message_count > 0));
    assert!(all.iter().all(|s| !s.source_path.is_empty()));

    let filtered = store.list_sessions(Some("fake"), 10).unwrap();
    assert_eq!(filtered.len(), 2);
    let none = store.list_sessions(Some("nope"), 10).unwrap();
    assert!(none.is_empty());

    // limit 生效
    let limited = store.list_sessions(None, 1).unwrap();
    assert_eq!(limited.len(), 1);
}

#[test]
fn session_meta_returns_exact_row_and_none_for_missing() {
    let (_d, store) = setup();
    let meta = store.session_meta("fake", "s-a").unwrap().unwrap();
    assert_eq!(meta.agent, "fake");
    assert_eq!(meta.session_key, "s-a");
    assert_eq!(meta.message_count, 2);
    assert_eq!(meta.project_path, "D:/p");

    assert!(store.session_meta("fake", "missing").unwrap().is_none());
    assert!(store.session_meta("other", "s-a").unwrap().is_none());
}
