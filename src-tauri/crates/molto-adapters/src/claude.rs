//! Claude Code 适配器。
//!
//! 关键约束：`~/.claude.json` 是 Claude Code 的**运行时状态文件**
//! （含会话历史、项目状态等 19+ 个顶层键，Agent 自身高频写入）。
//! Molto 只允许改动顶层 `mcpServers` 键，其余字段一律原样保留；写入由引擎原子化执行。

use std::path::{Path, PathBuf};

use molto_core::adapter::{AgentAdapter, CommandSpec};
use molto_core::error::Result;
use molto_core::model::agent::{Caps, InstallInfo};
use molto_core::model::mcp::{McpServer, Scope};

use molto_core::model::rules::RuleFile;
use molto_core::model::runtime::{LaunchOption, PromptInject, RuntimeSpec};
use molto_core::model::session::{
    ChunkParse, SessionSource, SessionSummary, SnapshotMessage, SnapshotRole,
};
use molto_core::model::streaming::StructuredChannel;
use molto_core::model::workspace::WorkspaceRecord;

use crate::jsonutil;

pub const ID: &str = "claude";
pub const NAME: &str = "Claude Code";
const STATE_FILE: &str = ".claude.json";
const CONFIG_DIR: &str = ".claude";
const SKILLS_DIR: &str = "skills";
const SERVERS_KEY: &[&str] = &["mcpServers"];
const SETTINGS_FILE: &str = ".claude/settings.json";
const TOOL_RULE_PREFIX: &str = "mcp__";

pub struct ClaudeAdapter {
    home: PathBuf,
}

impl ClaudeAdapter {
    pub fn new(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
        }
    }

    fn state_file(&self) -> PathBuf {
        self.home.join(STATE_FILE)
    }

    fn config_root(&self) -> PathBuf {
        self.home.join(CONFIG_DIR)
    }

    fn settings_path(&self) -> PathBuf {
        self.home.join(SETTINGS_FILE)
    }
    /// 读取 settings.json 中用户当前配置的模型，作为下拉默认值并入选项
    /// （"真实情况"来自用户自己的配置，只读）。
    fn model_launch_option(&self) -> LaunchOption {
        let configured = self.configured_model();
        let mut choices: Vec<String> = vec!["opus".into(), "sonnet".into(), "haiku".into()];
        if let Some(m) = &configured {
            if !choices.iter().any(|c| c == m) {
                choices.insert(0, m.clone());
            }
        }
        LaunchOption {
            choices,
            arg_template: "--model {v}".into(),
            default: configured,
            // 流式（headless print）同样吃 --model 启动 flag（实测 2.1.251）
            stream_override: true,
        }
    }

    fn configured_model(&self) -> Option<String> {
        let raw = std::fs::read_to_string(self.settings_path()).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        v.get("model")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string())
    }

    /// Claude 会话落盘：~/.claude/projects/<编码路径>/<uuid>.jsonl（append-only）。
    /// 只读扫描最近 SESSIONS_CAP 个，提取 resume 所需元数据（uuid/cwd/首条用户消息）。
    fn recent_sessions_impl(&self) -> Vec<SessionSummary> {
        let root = self.config_root().join("projects");
        let mut files: Vec<PathBuf> = Vec::new();
        let Ok(project_dirs) = std::fs::read_dir(&root) else {
            return Vec::new();
        };
        for dir in project_dirs.filter_map(|e| e.ok()) {
            let p = dir.path();
            if !p.is_dir() {
                continue;
            }
            if let Ok(list) = std::fs::read_dir(&p) {
                for f in list.filter_map(|e| e.ok()) {
                    let fp = f.path();
                    if fp.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        files.push(fp);
                    }
                }
            }
        }
        files.sort_by_key(|p| std::cmp::Reverse(mtime_ms(p)));
        files.truncate(SESSIONS_CAP);

        let mut out = Vec::new();
        for f in files {
            let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.is_empty() {
                continue;
            }
            let head = jsonutil::read_jsonl_head(&f, 60);
            let cwd = head
                .iter()
                .find_map(|v| v.get("cwd").and_then(|c| c.as_str()));
            let Some(cwd) = cwd else { continue };
            let title = head.iter().find_map(claude_user_text);
            out.push(SessionSummary {
                agent: ID.to_string(),
                session_key: stem.to_string(),
                project_path: cwd.to_string(),
                title: title.unwrap_or_default(),
                last_active_at: mtime_ms(&f),
                source_path: f.display().to_string(),
                resumable: true,
                resume_via_tui: false,
            });
        }
        out
    }

    /// 工具级权限渲染：把 disabledTools 变成 settings.json permissions.deny 中的
    /// `mcp__<server>__<tool>` 规则。只增删属于已分发 Server 的精确条目，
    /// 其余规则（含用户自己的 allow/deny）原样保留。
    fn render_deny_rules(
        &self,
        current: Option<&str>,
        servers: &[McpServer],
    ) -> Result<Option<String>> {
        use std::collections::BTreeSet;

        let mut desired: BTreeSet<String> = BTreeSet::new();
        let mut managed: BTreeSet<String> = BTreeSet::new();
        for s in servers.iter().filter(|s| s.enabled) {
            managed.insert(s.name.clone());
            for t in &s.disabled_tools {
                desired.insert(format!("{TOOL_RULE_PREFIX}{}__{t}", s.name));
            }
        }
        if desired.is_empty() && current.is_none() {
            return Ok(None); // 无工具级配置且文件不存在：不凭空创建
        }
        if current.is_some() {
            // 文件已存在且没有需要增删的条目：不产出计划（避免无谓的格式重排）
            let has_ours = current
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                .and_then(|v| {
                    v.get("permissions")
                        .and_then(|p| p.get("deny"))
                        .and_then(|d| d.as_array())
                        .cloned()
                })
                .is_some_and(|deny| {
                    deny.iter().any(|e| {
                        e.as_str().is_some_and(|rule| {
                            rule.starts_with(TOOL_RULE_PREFIX)
                                && managed.iter().any(|name| {
                                    rule.starts_with(&format!("{TOOL_RULE_PREFIX}{name}__"))
                                })
                        })
                    })
                });
            if desired.is_empty() && !has_ours {
                return Ok(None);
            }
        }

        let path = self.settings_path();
        let mut root: serde_json::Value = match current {
            Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
                .map_err(|e| molto_core::MoltoError::json_parse(&path, e.to_string()))?,
            _ => serde_json::json!({}),
        };

        let permissions = root
            .as_object_mut()
            .ok_or_else(|| {
                molto_core::MoltoError::config_invalid(format!(
                    "「{}」顶层不是对象，无法写入工具权限",
                    path.display()
                ))
            })?
            .entry("permissions".to_string())
            .or_insert_with(|| serde_json::json!({}));
        let deny = permissions
            .as_object_mut()
            .ok_or_else(|| {
                molto_core::MoltoError::config_invalid(format!(
                    "「{}」中 permissions 不是对象",
                    path.display()
                ))
            })?
            .entry("deny".to_string())
            .or_insert_with(|| serde_json::json!([]));
        let deny_arr = deny.as_array_mut().ok_or_else(|| {
            molto_core::MoltoError::config_invalid(format!(
                "「{}」中 permissions.deny 不是数组，无法安全写入；请人工检查后重试",
                path.display()
            ))
        })?;

        let is_ours = |entry: &serde_json::Value| -> bool {
            entry.as_str().is_some_and(|rule| {
                rule.starts_with(TOOL_RULE_PREFIX)
                    && managed
                        .iter()
                        .any(|name| rule.starts_with(&format!("{TOOL_RULE_PREFIX}{name}__")))
            })
        };
        let mut new_deny: Vec<serde_json::Value> =
            deny_arr.iter().filter(|e| !is_ours(e)).cloned().collect();
        for rule in &desired {
            new_deny.push(serde_json::Value::String(rule.clone()));
        }
        deny_arr.clear();
        deny_arr.extend(new_deny);

        let mut out = serde_json::to_string_pretty(&root)
            .map_err(|e| molto_core::MoltoError::Other(format!("序列化 JSON 失败：{e}")))?;
        out.push('\n');
        Ok(Some(out))
    }
}

impl AgentAdapter for ClaudeAdapter {
    fn id(&self) -> &'static str {
        ID
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn detect(&self) -> Option<InstallInfo> {
        if self.state_file().exists() || self.config_root().exists() {
            Some(InstallInfo {
                id: ID.into(),
                name: NAME.into(),
                version: None,
                config_root: self.config_root().to_string_lossy().to_string(),
            })
        } else {
            None
        }
    }

    fn capabilities(&self) -> Caps {
        Caps {
            mcp: true,
            project_mcp: true,
            rules: false,
            skills: true,
            tool_granularity: true,
        }
    }

    fn mcp_path(&self, scope: &Scope) -> Option<PathBuf> {
        match scope {
            Scope::User => Some(self.state_file()),
            Scope::Project { dir } => Some(Path::new(dir).join(".mcp.json")),
        }
    }

    fn read_mcp(&self, raw: &str) -> Result<Vec<McpServer>> {
        jsonutil::read_servers(raw, SERVERS_KEY, &self.state_file())
    }

    fn render_mcp(&self, current: Option<&str>, servers: &[McpServer]) -> Result<String> {
        jsonutil::render_servers(current, servers, SERVERS_KEY, &self.state_file())
    }

    fn tool_config_path(&self, _scope: &Scope) -> Option<PathBuf> {
        Some(self.settings_path())
    }

    fn render_tool_config(
        &self,
        current: Option<&str>,
        servers: &[McpServer],
    ) -> Result<Option<String>> {
        self.render_deny_rules(current, servers)
    }

    /// Claude Code：~/.claude.json 顶层 projects 键（路径 → 项目配置）。
    fn recent_workspaces(&self) -> Vec<WorkspaceRecord> {
        read_claude_workspaces(&self.state_file())
    }

    /// 指示.md：用户级 CLAUDE.md；项目级同名（项目根）。
    fn rule_files(&self) -> Vec<RuleFile> {
        vec![RuleFile {
            name: "CLAUDE.md".into(),
            path: self
                .config_root()
                .join("CLAUDE.md")
                .to_string_lossy()
                .to_string(),
        }]
    }

    fn project_rule_name(&self) -> Option<String> {
        Some("CLAUDE.md".into())
    }

    fn skills_dir(&self) -> Option<PathBuf> {
        Some(self.config_root().join(SKILLS_DIR))
    }

    /// 跨 Agent 会话共享 skill（v2.4）：~/.claude/skills/molto-session-resolver/SKILL.md
    fn render_session_skill(&self, current: Option<&str>) -> Result<Option<String>> {
        Ok(Some(crate::session_skill::render(current)?))
    }

    fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
        CommandSpec {
            program: crate::resolve_program("claude"),
            args: vec![],
            working_dir: project_dir.to_path_buf(),
        }
    }

    /// v2.0：claude CLI 接受位置参数作为首条 prompt（`claude "消息"`）。
    /// Windows 下 npm 安装形态是 .cmd shim，需经 cmd /c 解析。
    fn runtime(&self) -> Option<RuntimeSpec> {
        Some(RuntimeSpec {
            program: crate::resolve_program("claude"),
            args: vec![],
            windows_shim: true,
            prompt_inject: PromptInject::Argv,
            // 结构化流式（v2.0 R3）：claude headless print 模式（实测 2.1.251）。
            // stream-json 输出要求 --verbose；--include-partial-messages 提供逐 token 增量
            structured: Some(StructuredChannel {
                dialect: molto_core::ProtocolDialect::ClaudeStream,
                model_flag: Some("--model".into()),
                args: vec![
                    "-p".into(),
                    "--output-format".into(),
                    "stream-json".into(),
                    "--input-format".into(),
                    "stream-json".into(),
                    "--include-partial-messages".into(),
                    "--verbose".into(),
                ],
            }),
            model: Some(self.model_launch_option()),
            effort: None, // 官方无独立推理强度启动参数
            resume_args: Some("--resume {key}".into()),
            // 无人值守任务模式（channel 跟单）：headless print，进程退出即完成，
            // stdout 末段为含 result 字段的 JSON 对象（实测 2.1.251）
            task_mode: Some(molto_core::TaskModeSpec {
                args: vec!["-p".into(), "--output-format".into(), "json".into()],
                json_result: true,
                prompt_stdin: true,
                autonomy_args: vec!["--dangerously-skip-permissions".into()],
                headless_mcp: true,
            }),
        })
    }

    /// trait 接线：历史会话只读扫描（R2 换 SQLite 索引）。
    fn recent_sessions(&self) -> Vec<SessionSummary> {
        self.recent_sessions_impl()
    }

    /// 索引摄取源：~/.claude/projects 下递归 *.jsonl。
    fn session_sources(&self) -> Vec<SessionSource> {
        vec![SessionSource {
            dir: self.config_root().join("projects"),
            max_files: 200,
        }]
    }

    /// 纯函数：原始行 → 消息投影（用户/助手文本、工具调用）。
    /// 每行都携带 sessionId 与 cwd，因此任意增量块都能自证归属。
    fn parse_chunk(&self, lines: &[String]) -> Option<ChunkParse> {
        let mut key = None;
        let mut project = None;
        let mut title = None;
        let mut messages: Vec<(usize, SnapshotMessage)> = Vec::new();

        for (i, raw) in lines.iter().enumerate() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(raw.trim()) else {
                continue;
            };
            if key.is_none() {
                key = v
                    .get("sessionId")
                    .and_then(|x| x.as_str())
                    .map(String::from);
            }
            if project.is_none() {
                project = v.get("cwd").and_then(|x| x.as_str()).map(String::from);
            }
            let at = v
                .get("timestamp")
                .and_then(|x| x.as_str())
                .and_then(jsonutil::iso_to_ms);
            match v.get("type").and_then(|x| x.as_str()) {
                Some("user") => {
                    if v.get("isMeta").and_then(|m| m.as_bool()).unwrap_or(false) {
                        continue;
                    }
                    let Some(text) = claude_message_text(&v) else {
                        continue;
                    };
                    if title.is_none() {
                        title = Some(first_line_brief(&text));
                    }
                    messages.push((
                        i,
                        SnapshotMessage {
                            seq: i,
                            role: SnapshotRole::User,
                            text,
                            tool_name: None,
                            at,
                        },
                    ));
                }
                Some("assistant") => {
                    let Some(blocks) = v
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_array())
                    else {
                        continue;
                    };
                    for b in blocks {
                        match b.get("type").and_then(|t| t.as_str()) {
                            Some("text") => {
                                let Some(text) = b.get("text").and_then(|t| t.as_str()) else {
                                    continue;
                                };
                                if text.trim().is_empty() {
                                    continue;
                                }
                                messages.push((
                                    i,
                                    SnapshotMessage {
                                        seq: i,
                                        role: SnapshotRole::Assistant,
                                        text: text.chars().take(2000).collect(),
                                        tool_name: None,
                                        at,
                                    },
                                ));
                            }
                            Some("thinking") => {
                                // 思考内容：前端默认折叠展示
                                let Some(text) = b.get("thinking").and_then(|t| t.as_str()) else {
                                    continue;
                                };
                                if text.trim().is_empty() {
                                    continue;
                                }
                                messages.push((
                                    i,
                                    SnapshotMessage {
                                        seq: i,
                                        role: SnapshotRole::Thinking,
                                        text: text.chars().take(6000).collect(),
                                        tool_name: None,
                                        at,
                                    },
                                ));
                            }
                            Some("tool_use") => {
                                let name = b.get("name").and_then(|n| n.as_str()).map(String::from);
                                let input = serde_json::to_string(
                                    b.get("input").unwrap_or(&serde_json::Value::Null),
                                )
                                .unwrap_or_default();
                                messages.push((
                                    i,
                                    SnapshotMessage {
                                        seq: i,
                                        role: SnapshotRole::Tool,
                                        text: input.chars().take(4000).collect(),
                                        tool_name: name,
                                        at,
                                    },
                                ));
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
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
/// 每适配器历史会话扫描上限（R2 换 SQLite 索引后取消）。
const SESSIONS_CAP: usize = 120;

fn mtime_ms(p: &Path) -> i64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 提取用户消息全文投影（首行用于标题；总长 cap 2000 字符）。
fn claude_message_text(v: &serde_json::Value) -> Option<String> {
    let content = v.get("message")?.get("content")?;
    let text = match content {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(items) => {
            let mut t = String::new();
            for it in items {
                if it.get("type").and_then(|x| x.as_str()) == Some("text") {
                    if let Some(s) = it.get("text").and_then(|x| x.as_str()) {
                        t.push_str(s);
                    }
                }
            }
            (!t.trim().is_empty()).then_some(t)
        }
        _ => None,
    }?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(2000).collect())
}

fn first_line_brief(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .chars()
        .take(80)
        .collect()
}

/// 从一行 Claude 会话记录提取首条用户消息文本（跳过 isMeta）。
fn claude_user_text(v: &serde_json::Value) -> Option<String> {
    if v.get("type")?.as_str()? != "user" {
        return None;
    }
    if v.get("isMeta").and_then(|m| m.as_bool()).unwrap_or(false) {
        return None;
    }
    let content = v.get("message")?.get("content")?;
    let text = match content {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(items) => {
            let mut t = String::new();
            for it in items {
                if it.get("type").and_then(|x| x.as_str()) == Some("text") {
                    if let Some(s) = it.get("text").and_then(|x| x.as_str()) {
                        t.push_str(s);
                    }
                }
            }
            (!t.is_empty()).then_some(t)
        }
        _ => None,
    }?;
    let first_line = text.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return None;
    }
    Some(first_line.chars().take(80).collect())
}

/// 从 ~/.claude.json 原文解析历史 workspace（projects 键）。
pub(crate) fn read_claude_workspaces(state_file: &Path) -> Vec<WorkspaceRecord> {
    let raw = match std::fs::read_to_string(state_file) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    v.get("projects")
        .and_then(|p| p.as_object())
        .map(|obj| {
            obj.keys()
                .filter(|k| !k.trim().is_empty())
                .map(|k| WorkspaceRecord {
                    dir: k.clone(),
                    source: "claude projects".into(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {

    #[test]
    fn session_scan_and_chunk_parse() {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join(".claude/projects/D--ksa-proj");
        std::fs::create_dir_all(&dir).unwrap();
        let lines = [
            r#"{"type":"user","sessionId":"u1","cwd":"D:/ksa/proj","timestamp":"2026-08-30T10:00:00Z","message":{"content":"修复登录超时"}}"#.to_string(),
            r#"{"type":"assistant","timestamp":"2026-08-30T10:00:05Z","message":{"content":[{"type":"thinking","thinking":"先查 token 逻辑"},{"type":"text","text":"查到是 token 过期"},{"type":"tool_use","name":"Bash","input":{"command":"grep token" }}]}}"#.to_string(),
            r#"{"type":"user","isMeta":true,"message":{"content":"meta 行应被跳过"}}"#.to_string(),
        ];
        std::fs::write(
            dir.join("u1.jsonl"),
            lines.join(
                "
",
            ),
        )
        .unwrap();

        let adapter = ClaudeAdapter::new(home.path());
        let sessions = adapter.recent_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_key, "u1");
        assert_eq!(sessions[0].project_path, "D:/ksa/proj");
        assert!(
            sessions[0].title.contains("登录超时"),
            "title={}",
            sessions[0].title
        );

        let parse = adapter.parse_chunk(&lines).unwrap();
        assert_eq!(parse.session_key.as_deref(), Some("u1"));
        assert_eq!(parse.messages.len(), 4, "user + thinking + text + tool_use");
        assert_eq!(parse.messages[0].1.role, molto_core::SnapshotRole::User);
        assert!(parse.messages[0].1.at.is_some(), "时间戳应被解析");
        assert_eq!(parse.messages[1].1.role, molto_core::SnapshotRole::Thinking);
        assert!(parse.messages[1].1.text.contains("token 逻辑"));
        assert_eq!(
            parse.messages[2].1.role,
            molto_core::SnapshotRole::Assistant
        );
        assert_eq!(parse.messages[3].1.role, molto_core::SnapshotRole::Tool);
        assert_eq!(parse.messages[3].1.tool_name.as_deref(), Some("Bash"));
        assert!(
            parse.messages[3].1.text.contains("grep token"),
            "工具输入应被捕获"
        );
    }
    use super::*;

    const FIXTURE_STATE: &str = r#"{
  "numStartups": 42,
  "installMethod": "npm",
  "tipsHistory": { "a": 1 },
  "projects": { "D:/work": { "allowedTools": [] } },
  "mcpServers": {
    "fs": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-fs", "D:/pub"],
      "env": { "DEBUG": "1" }
    },
    "search": {
      "type": "http",
      "url": "https://mcp.example.com/search",
      "headers": { "Authorization": "Bearer x" },
      "unknownField": "keep-me"
    }
  }
}"#;

    fn servers() -> Vec<McpServer> {
        let adapter = ClaudeAdapter::new(Path::new("/h"));
        adapter.read_mcp(FIXTURE_STATE).unwrap()
    }

    /// 往返：read → render → read 结果不变。
    #[test]
    fn roundtrip_read_render_read() {
        let original = servers();
        let out = ClaudeAdapter::new(Path::new("/h"))
            .render_mcp(Some(FIXTURE_STATE), &original)
            .unwrap();
        let reparsed = ClaudeAdapter::new(Path::new("/h")).read_mcp(&out).unwrap();
        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].name, original[0].name);
        match (&reparsed[0].transport, &original[0].transport) {
            (
                molto_core::model::mcp::Transport::Stdio {
                    command: c1,
                    args: a1,
                    env: e1,
                },
                molto_core::model::mcp::Transport::Stdio {
                    command: c2,
                    args: a2,
                    env: e2,
                },
            ) => {
                assert_eq!(c1, c2);
                assert_eq!(a1, a2);
                assert_eq!(e1, e2);
            }
            _ => panic!("transport type changed"),
        }
    }

    /// 保真：与 Molto 无关的顶层字段、条目内未知字段、键序原样保留。
    #[test]
    fn preserves_unknown_fields_and_key_order() {
        let out = ClaudeAdapter::new(Path::new("/h"))
            .render_mcp(Some(FIXTURE_STATE), &servers())
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        // 顶层运行时字段一个都不能少
        assert_eq!(v["numStartups"], 42);
        assert_eq!(v["installMethod"], "npm");
        assert!(v.get("projects").is_some());
        assert!(v.get("tipsHistory").is_some());
        // 条目内未知字段保留
        assert_eq!(v["mcpServers"]["search"]["unknownField"], "keep-me");
        // 键序：mcpServers 之后追加的键不破坏已有顺序（numStartups 仍在最前）
        let out_str = out.as_str();
        let num_pos = out_str.find("\"numStartups\"").unwrap();
        let mcp_pos = out_str.find("\"mcpServers\"").unwrap();
        assert!(num_pos < mcp_pos);
    }

    /// 严格单源：源里删掉的 server，渲染后从目标消失。
    #[test]
    fn removed_server_disappears_on_render() {
        let mut srv = servers();
        srv.retain(|s| s.name == "fs");
        let out = ClaudeAdapter::new(Path::new("/h"))
            .render_mcp(Some(FIXTURE_STATE), &srv)
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["mcpServers"].get("search").is_none());
        assert_eq!(v["numStartups"], 42);
    }

    /// 空 current：生成只含 mcpServers 的最小合法结构。
    #[test]
    fn render_from_empty_creates_minimal() {
        let srv = vec![McpServer {
            name: "fs".into(),
            transport: molto_core::model::mcp::Transport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into()],
                env: Default::default(),
            },
            enabled: true,
            disabled_tools: Vec::new(),
        }];
        let out = ClaudeAdapter::new(Path::new("/h"))
            .render_mcp(None, &srv)
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["mcpServers"]["fs"]["command"], "npx");
        assert_eq!(v["mcpServers"]["fs"]["args"], serde_json::json!(["-y"]));
        assert!(v["mcpServers"]["fs"].get("env").is_none());
    }

    #[test]
    fn detect_requires_state_file_or_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ClaudeAdapter::new(dir.path()).detect().is_none());
        std::fs::write(dir.path().join(STATE_FILE), "{}").unwrap();
        let info = ClaudeAdapter::new(dir.path()).detect().unwrap();
        assert_eq!(info.id, "claude");
        assert!(info.config_root.contains(".claude"));
    }

    /// 工具颗粒度：deny 规则写入 + 用户已有规则/未知键保真。
    #[test]
    fn tool_config_merges_with_fidelity() {
        let fixture = r#"{
  "env": { "MY_VAR": "1" },
  "hooks": {},
  "permissions": {
    "allow": ["Bash(npm run *)"],
    "deny": ["Read(.env)", "mcp__other__tool_x"]
  },
  "unknownTop": true
}"#;
        let adapter = ClaudeAdapter::new(Path::new("/h"));
        let mut srv = servers();
        srv[0].disabled_tools = vec!["write_file".into(), "delete".into()];
        let out = adapter
            .render_tool_config(Some(fixture), &srv)
            .unwrap()
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        // 我们的条目写入
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.contains(&serde_json::json!("mcp__fs__write_file")));
        assert!(deny.contains(&serde_json::json!("mcp__fs__delete")));
        // 用户已有规则原样保留（顺序与其他条目）
        assert_eq!(deny[0], serde_json::json!("Read(.env)"));
        assert!(deny.contains(&serde_json::json!("mcp__other__tool_x")));
        assert!(v["permissions"]["allow"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("Bash(npm run *)")));
        // 未知键保真
        assert_eq!(v["unknownTop"], true);
        assert_eq!(v["env"]["MY_VAR"], "1");
    }

    /// 工具重新启用：旧条目精确移除，用户规则不动。
    #[test]
    fn tool_config_removes_stale_entries() {
        let fixture = r#"{
  "permissions": {
    "deny": ["Read(.env)", "mcp__fs__write_file", "mcp__fs__delete"]
  }
}"#;
        let adapter = ClaudeAdapter::new(Path::new("/h"));
        let mut srv = servers();
        srv[0].disabled_tools = vec!["delete".into()]; // write_file 重新启用
        let out = adapter
            .render_tool_config(Some(fixture), &srv)
            .unwrap()
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.contains(&serde_json::json!("mcp__fs__delete")));
        assert!(!deny.contains(&serde_json::json!("mcp__fs__write_file")));
        assert!(deny.contains(&serde_json::json!("Read(.env)")));
    }

    /// 幂等 + 无配置时不创建文件。
    #[test]
    fn tool_config_idempotent_and_absent() {
        let adapter = ClaudeAdapter::new(Path::new("/h"));
        let mut srv = servers();
        srv[0].disabled_tools = vec!["write_file".into()];
        let out1 = adapter.render_tool_config(None, &srv).unwrap().unwrap();
        let out2 = adapter
            .render_tool_config(Some(&out1), &srv)
            .unwrap()
            .unwrap();
        assert_eq!(out1, out2, "工具权限渲染必须幂等");
        // 没有任何 disabledTools 且文件不存在：None
        assert!(adapter
            .render_tool_config(None, &servers())
            .unwrap()
            .is_none());
    }

    #[test]
    fn workspaces_parsed_from_projects_key() {
        let fixture =
            r#"{"numStartups":1,"projects":{"D:/work/a":{},"D:/work/b":{"mcpServers":{}}}}"#;
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join(".claude.json");
        std::fs::write(&f, fixture).unwrap();
        let ws = read_claude_workspaces(&f);
        let dirs: Vec<&str> = ws.iter().map(|w| w.dir.as_str()).collect();
        assert!(dirs.contains(&"D:/work/a"));
        assert!(dirs.contains(&"D:/work/b"));
        assert_eq!(ws[0].source, "claude projects");
    }

    #[test]
    fn mcp_paths() {
        let adapter = ClaudeAdapter::new(Path::new("/h"));
        assert_eq!(
            adapter.mcp_path(&Scope::User).unwrap(),
            PathBuf::from("/h/.claude.json")
        );
        assert_eq!(
            adapter
                .mcp_path(&Scope::Project {
                    dir: "D:/work".into()
                })
                .unwrap(),
            PathBuf::from("D:/work/.mcp.json")
        );
    }
}
