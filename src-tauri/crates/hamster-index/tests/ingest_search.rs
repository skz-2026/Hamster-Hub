//! 索引端到端测试：摄取（全量/增量/重建）→ 中文与英文检索。
//! FakeAdapter 只服务本测试（解析自定义的最小 JSON 行格式），不触碰真实 ~/ 配置。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use hamster_core::adapter::{AgentAdapter, Caps, CommandSpec};
use hamster_core::error::Result;
use hamster_core::model::agent::InstallInfo;
use hamster_core::model::mcp::{McpServer, Scope};
use hamster_core::model::session::{ChunkParse, SessionSource, SnapshotMessage, SnapshotRole};
use hamster_core::{Registry, SearchQuery};

use hamster_index::{ingest_all, IndexStore};

fn temp_root(tag: u32) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hamster-index-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sessions")).unwrap();
    dir
}

/// 行格式：{"id":"s1","cwd":"D:/proj-a","role":"user","text":"..."}
fn line(id: &str, cwd: &str, role: &str, text: &str) -> String {
    format!(r#"{{"id":"{id}","cwd":"{cwd}","role":"{role}","text":"{text}"}}"#)
}

struct FakeAdapter {
    root: PathBuf,
    counter: AtomicU32,
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
        let mut title = None;
        let mut messages = Vec::new();
        for (i, raw) in lines.iter().enumerate() {
            let v: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
            if key.is_none() {
                key = v.get("id").and_then(|x| x.as_str()).map(String::from);
            }
            if project.is_none() {
                project = v.get("cwd").and_then(|x| x.as_str()).map(String::from);
            }
            if let (Some(role), Some(text)) = (
                v.get("role").and_then(|x| x.as_str()),
                v.get("text").and_then(|x| x.as_str()),
            ) {
                let role = match role {
                    "user" => SnapshotRole::User,
                    _ => SnapshotRole::Assistant,
                };
                if title.is_none() && role == SnapshotRole::User {
                    title = Some(text.chars().take(80).collect());
                }
                let _ = self.counter.fetch_add(1, Ordering::Relaxed);
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
        }
        Some(ChunkParse {
            session_key: key,
            project_path: project,
            title,
            messages,
        })
    }
}

fn write_session(path: &PathBuf, lines: &[String]) {
    std::fs::write(path, lines.join("\n") + "\n").unwrap();
}

fn query(text: &str) -> SearchQuery {
    SearchQuery {
        text: text.into(),
        agents: vec![],
        project: None,
        limit: Some(10),
    }
}

#[test]
fn ingest_increment_and_search() {
    let root = temp_root(1);
    let sessions = root.join("sessions");
    let adapter = FakeAdapter {
        root: root.clone(),
        counter: AtomicU32::new(0),
    };
    let registry = Registry::new(vec![Box::new(adapter)]);
    let store = IndexStore::open(&root.join("index.db")).unwrap();

    // 会话 A：中文长文本（FTS trigram 路径）；会话 B：短词（LIKE 路径）
    write_session(
        &sessions.join("a.jsonl"),
        &[
            line(
                "s-a",
                "D:/proj-a",
                "user",
                "帮我分析模型加载失败重试三次的原因",
            ),
            line("s-a", "D:/proj-a", "assistant", "已定位到权重文件缺失"),
        ],
    );
    write_session(
        &sessions.join("b.jsonl"),
        &[
            line("s-b", "D:/proj-b", "user", "登录 bug"),
            line("s-b", "D:/proj-b", "assistant", "复现步骤已记录"),
        ],
    );

    let report = ingest_all(&registry, &store).unwrap();
    assert_eq!(report.updated, 2, "首轮两个文件都应摄取");

    // FTS trigram 路径（≥3 字符）
    let hits = store.search(&query("模型加载失败")).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].session_key, "s-a");
    assert!(
        hits[0].snippet.contains("模型"),
        "snippet 应含命中上下文：{}",
        hits[0].snippet
    );

    // LIKE 路径（2 字中文）
    let hits = store.search(&query("登录")).unwrap();
    assert!(hits.iter().any(|h| h.session_key == "s-b"));

    // 标题命中排最前
    let hits = store.search(&query("登录 bug")).unwrap();
    assert_eq!(hits[0].session_key, "s-b", "标题命中应排最前");

    // 增量追加：新消息可被检索
    let mut appended = std::fs::read_to_string(sessions.join("a.jsonl")).unwrap();
    appended += &(line("s-a", "D:/proj-a", "user", "CORS 跨域预检请求补充说明") + "\n");
    std::fs::write(sessions.join("a.jsonl"), &appended).unwrap();
    let report = ingest_all(&registry, &store).unwrap();
    assert_eq!(report.updated, 1, "只有 a.jsonl 需要增量摄取");
    let hits = store.search(&query("CORS 跨域")).unwrap();
    assert_eq!(hits.len(), 1);

    // agent 过滤
    let mut q = query("登录");
    q.agents = vec!["nobody".into()];
    assert!(store.search(&q).unwrap().is_empty());

    // 项目过滤
    let mut q = query("登录");
    q.project = Some("D:/proj-b".into());
    let hits = store.search(&q).unwrap();
    assert!(hits
        .iter()
        .all(|h| h.project_path.as_deref() == Some("D:/proj-b")));

    // 分页：从尾部取最近 2 条 / 从 seq=0 取升序 / total 正确
    let page = store.messages_page("fake", "s-a", -1, 2).unwrap();
    assert_eq!(page.total, 3, "s-a 应有 3 条消息（含增量追加）");
    assert_eq!(page.messages.len(), 2);
    assert_eq!(page.messages[1].text, "CORS 跨域预检请求补充说明");
    let page = store.messages_page("fake", "s-a", 0, 2).unwrap();
    assert_eq!(page.messages.len(), 2);
    assert_eq!(page.messages[0].seq, 0);
    assert_eq!(page.messages[0].role, hamster_core::SnapshotRole::User);

    // 重建等价性：清空 → 重摄取 → 搜索结果一致
    store.rebuild().unwrap();
    let st = store.stats().unwrap();
    assert_eq!((st.sessions, st.messages), (0, 0));
    ingest_all(&registry, &store).unwrap();
    assert_eq!(store.search(&query("模型加载失败")).unwrap().len(), 1);
}
