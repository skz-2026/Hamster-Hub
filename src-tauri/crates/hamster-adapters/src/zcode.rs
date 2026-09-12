//! ZCode 适配器。
//!
//! `~/.zcode/cli/config.json`：MCP 在 `mcp.servers` 键下（本机实测 2026-08-30）。
//! 形态与 JSON 家族一致：name → { url } 或 { command, args?, env? }。

use std::path::{Path, PathBuf};

use hamster_core::adapter::{AgentAdapter, CommandSpec};
use hamster_core::error::Result;
use hamster_core::model::agent::{Caps, InstallInfo};
use hamster_core::model::mcp::{McpServer, Scope};

use hamster_core::model::rules::RuleFile;
use hamster_core::model::runtime::{PromptInject, RuntimeSpec};
use hamster_core::model::session::SessionSummary;
use hamster_core::model::workspace::WorkspaceRecord;

use crate::jsonutil;

pub const ID: &str = "zcode";
pub const NAME: &str = "ZCode";
const CONFIG_DIR: &str = ".zcode";
const CONFIG_FILE: &str = "cli/config.json";
const SERVERS_KEY: &[&str] = &["mcp", "servers"];

pub struct ZcodeAdapter {
    home: PathBuf,
}

impl ZcodeAdapter {
    pub fn new(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
        }
    }

    fn config_path(&self) -> PathBuf {
        self.home.join(CONFIG_DIR).join(CONFIG_FILE)
    }

    fn config_root(&self) -> PathBuf {
        self.home.join(CONFIG_DIR)
    }

    /// ZCode 会话库：~/.zcode/cli/db/db.sqlite 的 session 表（本机实测 2026-08-30）。
    /// 只读打开（WAL 并发安全）；仅列主会话（parent_id IS NULL），最近优先。
    /// 注意：rollout/model-io-*.jsonl 是模型 I/O 诊断日志，非会话库，不索引。
    fn recent_sessions_impl(&self) -> Vec<SessionSummary> {
        const DB_PATH: &str = "cli/db/db.sqlite";
        let db_path = self.home.join(DB_PATH);
        if !db_path.is_file() {
            return Vec::new();
        }
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
        let Ok(conn) = rusqlite::Connection::open_with_flags(&db_path, flags) else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, directory, title, time_updated FROM session
             WHERE parent_id IS NULL ORDER BY time_updated DESC LIMIT 100",
        ) else {
            return Vec::new();
        };
        let Ok(rows) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?,
            ))
        }) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for row in rows.into_iter().flatten() {
            let (id, directory, title, updated) = row;
            out.push(SessionSummary {
                agent: ID.to_string(),
                session_key: id,
                project_path: directory.unwrap_or_default(),
                title: title.unwrap_or_default(),
                last_active_at: updated.unwrap_or(0),
                source_path: format!("{}#{DB_PATH}", db_path.display()),
                resumable: false, // resume 参数未实测，仅浏览（product-plan §8.2）
                resume_via_tui: false,
            });
        }
        out
    }
}

impl AgentAdapter for ZcodeAdapter {
    fn id(&self) -> &'static str {
        ID
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn detect(&self) -> Option<InstallInfo> {
        self.config_path().exists().then(|| InstallInfo {
            id: ID.into(),
            name: NAME.into(),
            version: None,
            config_root: self.config_root().to_string_lossy().to_string(),
        })
    }

    fn capabilities(&self) -> Caps {
        // skills 目录结构未验证（⚠️），rules 体系待确认：M0 如实声明仅支持用户级 MCP
        Caps {
            mcp: true,
            project_mcp: false,
            rules: false,
            skills: false,
            tool_granularity: false,
        }
    }

    fn mcp_path(&self, scope: &Scope) -> Option<PathBuf> {
        match scope {
            Scope::User => Some(self.config_path()),
            Scope::Project { .. } => None,
        }
    }

    fn read_mcp(&self, raw: &str) -> Result<Vec<McpServer>> {
        jsonutil::read_servers(raw, SERVERS_KEY, &self.config_path())
    }

    fn render_mcp(&self, current: Option<&str>, servers: &[McpServer]) -> Result<String> {
        jsonutil::render_servers(current, servers, SERVERS_KEY, &self.config_path())
    }

    /// 指示.md：用户级 ~/.zcode/AGENTS.md（⚠️ 路径按官方约定实测后再确认）；项目级 AGENTS.md。
    fn rule_files(&self) -> Vec<RuleFile> {
        vec![RuleFile {
            name: "AGENTS.md".into(),
            path: self
                .home
                .join(".zcode")
                .join("AGENTS.md")
                .to_string_lossy()
                .to_string(),
        }]
    }

    fn project_rule_name(&self) -> Option<String> {
        Some("AGENTS.md".into())
    }

    /// ZCode：~/.zcode/v2/setting.json 的 recentProjects 数组。
    fn recent_workspaces(&self) -> Vec<WorkspaceRecord> {
        read_zcode_workspaces(&self.home.join(".zcode/v2/setting.json"))
    }

    fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
        CommandSpec {
            program: "zcode".into(),
            args: vec![],
            working_dir: project_dir.to_path_buf(),
        }
    }

    /// v2.0：位置参数注入 prompt 的行为未实测（product-plan §8.2 ⚠️），
    /// 先按 PromptInject::None 如实声明（开终端手动聊），实机验证后改 Argv。
    fn runtime(&self) -> Option<RuntimeSpec> {
        Some(RuntimeSpec {
            program: "zcode".into(),
            args: vec![],
            windows_shim: true,
            prompt_inject: PromptInject::None,
            structured: None,
            model: None, // 官方无启动级模型参数
            effort: None,
            resume_args: None, // 会话机制未实测，不声明 resume
            task_mode: None,   // headless 一次性模式未实测，不声明（不可承担跟单任务）
        })
    }

    /// trait 接线：历史会话只读扫描（resume 参数未实测，列表仅可浏览）。
    fn recent_sessions(&self) -> Vec<SessionSummary> {
        self.recent_sessions_impl()
    }
}

/// 从 v2/setting.json 解析 recentProjects。
pub(crate) fn read_zcode_workspaces(setting_file: &Path) -> Vec<WorkspaceRecord> {
    let raw = match std::fs::read_to_string(setting_file) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    v.get("recentProjects")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| d.as_str())
                .filter(|d| !d.trim().is_empty())
                .map(|d| WorkspaceRecord {
                    dir: d.to_string(),
                    source: "zcode recentProjects".into(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {

    #[test]
    fn session_scan_from_sqlite() {
        let home = tempfile::tempdir().unwrap();
        let db = home.path().join("cli/db/db.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE session(id TEXT PRIMARY KEY, parent_id TEXT, directory TEXT, title TEXT, time_updated INTEGER);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO session VALUES('sess_a', NULL, 'D:/proj-a', '修复登录', 100)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO session VALUES('sess_sub', 'sess_a', 'D:/proj-a', '子任务', 99)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO session VALUES('sess_b', NULL, 'D:/proj-b', NULL, 200)",
            [],
        )
        .unwrap();
        drop(conn);

        let adapter = ZcodeAdapter::new(home.path());
        let sessions = adapter.recent_sessions();
        assert_eq!(sessions.len(), 2, "子会话应被过滤");
        assert_eq!(sessions[0].session_key, "sess_b", "最新优先");
        assert!(!sessions[0].resumable, "resume 未实测必须如实标记");
        assert_eq!(sessions[1].title, "修复登录");
    }
    use super::*;
    use hamster_core::model::mcp::Transport;
    use std::collections::BTreeMap;

    /// 与本机实测结构一致的样例。
    const FIXTURE: &str = r#"{
  "mcp": {
    "servers": {
      "cocos-creator": {
        "url": "http://localhost:3000/mcp"
      }
    }
  }
}"#;

    #[test]
    fn roundtrip_read_render_read() {
        let adapter = ZcodeAdapter::new(Path::new("/h"));
        let original = adapter.read_mcp(FIXTURE).unwrap();
        assert_eq!(original.len(), 1);
        assert_eq!(original[0].name, "cocos-creator");
        match &original[0].transport {
            Transport::Http { url, headers } => {
                assert_eq!(url, "http://localhost:3000/mcp");
                assert!(headers.is_empty());
            }
            _ => panic!("expected http"),
        }
        let out = adapter.render_mcp(Some(FIXTURE), &original).unwrap();
        let reparsed = adapter.read_mcp(&out).unwrap();
        assert_eq!(reparsed[0].name, original[0].name);
    }

    #[test]
    fn preserves_structure_and_adds_stdio() {
        let adapter = ZcodeAdapter::new(Path::new("/h"));
        let mut servers = adapter.read_mcp(FIXTURE).unwrap();
        servers.push(McpServer {
            name: "fs".into(),
            transport: Transport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into()],
                env: BTreeMap::new(),
            },
            enabled: true,
            disabled_tools: Vec::new(),
        });
        let out = adapter.render_mcp(Some(FIXTURE), &servers).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["mcp"]["servers"]["cocos-creator"]["url"],
            "http://localhost:3000/mcp"
        );
        assert_eq!(v["mcp"]["servers"]["fs"]["command"], "npx");
    }

    #[test]
    fn render_from_empty() {
        let adapter = ZcodeAdapter::new(Path::new("/h"));
        let servers = vec![McpServer {
            name: "x".into(),
            transport: Transport::Http {
                url: "http://a.b".into(),
                headers: BTreeMap::new(),
            },
            enabled: true,
            disabled_tools: Vec::new(),
        }];
        let out = adapter.render_mcp(None, &servers).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["mcp"]["servers"]["x"]["url"], "http://a.b");
    }

    #[test]
    fn workspaces_from_recent_projects() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("setting.json");
        // JSON 转义：\\ → 单个反斜杠
        std::fs::write(
            &f,
            r#"{"recentProjects":["D:\\ksa\\上游","D:\\ksa\\game"],"other":1}"#,
        )
        .unwrap();
        let ws = read_zcode_workspaces(&f);
        assert_eq!(ws.len(), 2);
        assert_eq!(ws[0].dir, "D:\\ksa\\上游");
    }

    #[test]
    fn detect_and_paths() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ZcodeAdapter::new(dir.path()).detect().is_none());
        std::fs::create_dir_all(dir.path().join(".zcode/cli")).unwrap();
        std::fs::write(dir.path().join(".zcode/cli/config.json"), "{}").unwrap();
        let adapter = ZcodeAdapter::new(dir.path());
        let info = adapter.detect().unwrap();
        assert_eq!(info.id, "zcode");
        assert_eq!(
            adapter.mcp_path(&Scope::User).unwrap(),
            dir.path().join(".zcode/cli/config.json")
        );
    }
}
