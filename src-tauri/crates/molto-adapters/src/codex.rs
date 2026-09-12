//! Codex CLI 适配器。
//!
//! `~/.codex/config.toml`：MCP 在 `[mcp_servers.<name>]` 表下。
//! 必须用 `toml_edit` 做保真编辑——保留注释、排版与未知键；禁止 serde 全量序列化回写。

use std::path::{Path, PathBuf};

use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value};

use molto_core::adapter::{AgentAdapter, CommandSpec};
use molto_core::error::{MoltoError, Result};
use molto_core::model::agent::{Caps, InstallInfo};
use molto_core::model::mcp::{McpServer, Scope, Transport};
use molto_core::model::rules::RuleFile;
use molto_core::model::runtime::{LaunchOption, PromptInject, RuntimeSpec};
use molto_core::model::session::{
    ChunkParse, SessionSource, SessionSummary, SnapshotMessage, SnapshotRole,
};
use molto_core::model::streaming::{ProtocolDialect, StructuredChannel};

use crate::jsonutil;
use molto_core::model::workspace::WorkspaceRecord;

pub const ID: &str = "codex";
pub const NAME: &str = "Codex CLI";
const CONFIG_DIR: &str = ".codex";
const CONFIG_FILE: &str = "config.toml";
const SERVERS_KEY: &str = "mcp_servers";
const SKILLS_DIR: &str = "skills";

pub struct CodexAdapter {
    home: PathBuf,
}

impl CodexAdapter {
    fn sessions_root(&self) -> PathBuf {
        self.home.join(CONFIG_DIR).join("sessions")
    }

    /// 读取 config.toml 中用户当前配置的 model / model_reasoning_effort，
    /// 作为下拉默认值并入选项（只读，toml_edit 保真解析）。
    fn configured(&self, key: &str) -> Option<String> {
        let raw = std::fs::read_to_string(self.home.join(CONFIG_DIR).join(CONFIG_FILE)).ok()?;
        let doc = raw.parse::<DocumentMut>().ok()?;
        doc.get(key)
            .and_then(|i| i.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
    }

    fn model_launch_option(&self) -> LaunchOption {
        let configured = self.configured("model");
        let mut choices: Vec<String> = vec!["gpt-5-codex".into(), "gpt-5".into()];
        if let Some(m) = &configured {
            if !choices.iter().any(|c| c == m) {
                choices.insert(0, m.clone());
            }
        }
        LaunchOption {
            choices,
            arg_template: "-m {v}".into(),
            default: configured,
            // 流式通道无启动 flag，但模型随 turn/start 参数同样生效
            stream_override: true,
        }
    }

    fn effort_launch_option(&self) -> LaunchOption {
        let configured = self.configured("model_reasoning_effort");
        LaunchOption {
            choices: vec!["low".into(), "medium".into(), "high".into()],
            arg_template: "-c model_reasoning_effort={v}".into(),
            default: configured,
            stream_override: true, // 同模型：随 turn 参数生效
        }
    }

    /// Codex 会话落盘：~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl。
    /// 首行 session_meta.payload 含 id/cwd；标题取首个真实 user 消息（跳过注入上下文）。
    fn recent_sessions_impl(&self) -> Vec<SessionSummary> {
        let mut files: Vec<PathBuf> = Vec::new();
        collect_jsonl_recent_first(&self.sessions_root(), &mut files, 0);
        files.sort_by_key(|p| std::cmp::Reverse(mtime_ms(p)));
        files.truncate(SESSIONS_CAP);

        let mut out = Vec::new();
        for f in files {
            let head = jsonutil::read_jsonl_head(&f, 40);
            let meta = head
                .iter()
                .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("session_meta"));
            let payload = meta.and_then(|v| v.get("payload"));
            let Some(key) = payload
                .and_then(|p| p.get("id"))
                .and_then(|i| i.as_str())
                .map(|s| s.to_string())
            else {
                continue;
            };
            let Some(cwd) = payload
                .and_then(|p| p.get("cwd"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
            else {
                continue;
            };
            // 与 parse_chunk 一致：标题取 response_item 首条真实 user 消息
            // （跳过 codex 注入的 AGENTS.md / 环境上下文，rollout 头 40 行内找不到则为空）
            let title = head
                .iter()
                .filter(|v| {
                    let p = v.get("payload");
                    p.and_then(|p| p.get("type")).and_then(|t| t.as_str()) == Some("message")
                        && p.and_then(|p| p.get("role")).and_then(|r| r.as_str()) == Some("user")
                })
                .filter_map(|v| {
                    v.get("payload")
                        .and_then(|p| p.get("content"))
                        .and_then(|c| c.as_array())
                })
                .map(|items| {
                    let mut t = String::new();
                    for it in items {
                        if it.get("type").and_then(|x| x.as_str()) == Some("input_text") {
                            if let Some(s) = it.get("text").and_then(|x| x.as_str()) {
                                t.push_str(s);
                            }
                        }
                    }
                    t
                })
                .find(|t| !is_injected_context(t))
                .map(|t| {
                    t.lines()
                        .map(str::trim)
                        .find(|l| !l.is_empty())
                        .unwrap_or("")
                        .chars()
                        .take(80)
                        .collect::<String>()
                });
            // 续聊通道声明（实测 0.152.0）：GUI 流式 thread/resume 要求 rollout
            // 记录的 model_provider 能被当前配置解析（Codex Desktop 等外部形态
            // 创建的会话常引用其自身配置里的 provider，解析不了即失败，且协议
            // 无参数级覆盖）；CLI 家族（codex resume / exec resume）则会回退到
            // 配置默认 provider 正常续聊。故 provider 解析不了 → 仍可续，但标记
            // 走 TUI。meta 缺 provider 字段时交由 codex 默认行为（GUI 可续）。
            let provider = payload
                .and_then(|p| p.get("model_provider"))
                .and_then(|s| s.as_str());
            let provider_known = match provider {
                Some(id) => self.config_provider_ids().contains(id),
                None => true,
            };
            out.push(SessionSummary {
                agent: ID.to_string(),
                session_key: key,
                project_path: cwd,
                title: title.unwrap_or_default(),
                last_active_at: mtime_ms(&f),
                source_path: f.display().to_string(),
                resumable: true,
                resume_via_tui: !provider_known,
            });
        }
        out
    }

    pub fn new(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
        }
    }

    fn config_path(&self) -> PathBuf {
        self.home.join(CONFIG_DIR).join(CONFIG_FILE)
    }

    /// 当前 CLI 配置可解析的 model provider id：内置（openai/oss）+
    /// config.toml `[model_providers.<id>]` 用户自定义。只读，resume 前置校验用。
    fn config_provider_ids(&self) -> std::collections::BTreeSet<String> {
        let mut ids: std::collections::BTreeSet<String> =
            ["openai", "oss"].into_iter().map(String::from).collect();
        if let Ok(raw) = std::fs::read_to_string(self.config_path()) {
            if let Ok(doc) = raw.parse::<DocumentMut>() {
                if let Some(Item::Table(t)) = doc.get("model_providers") {
                    for (k, _) in t.iter() {
                        ids.insert(k.to_string());
                    }
                }
            }
        }
        ids
    }

    fn config_root(&self) -> PathBuf {
        self.home.join(CONFIG_DIR)
    }
}

impl AgentAdapter for CodexAdapter {
    fn id(&self) -> &'static str {
        ID
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn detect(&self) -> Option<InstallInfo> {
        self.config_root().exists().then(|| InstallInfo {
            id: ID.into(),
            name: NAME.into(),
            version: None,
            config_root: self.config_root().to_string_lossy().to_string(),
        })
    }

    fn capabilities(&self) -> Caps {
        Caps {
            mcp: true,
            project_mcp: false,
            rules: false,
            skills: true,
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
        let path = self.config_path();
        let doc = raw
            .parse::<DocumentMut>()
            .map_err(|e| MoltoError::toml_parse(path, e.to_string()))?;
        let Some(item) = doc.get(SERVERS_KEY) else {
            return Ok(vec![]);
        };
        let Some(table) = item.as_table() else {
            return Ok(vec![]);
        };
        let mut out = vec![];
        for (name, entry) in table.iter() {
            let Some(t) = entry.as_table() else { continue };
            let transport = if let Some(command) = t.get("command").and_then(|v| v.as_str()) {
                let args = t
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let env = string_map_from_item(t.get("env"));
                Some(Transport::Stdio {
                    command: command.to_string(),
                    args,
                    env,
                })
            } else if let Some(url) = t.get("url").and_then(|v| v.as_str()) {
                let headers = string_map_from_item(t.get("headers"));
                Some(Transport::Http {
                    url: url.to_string(),
                    headers,
                })
            } else {
                None
            };
            if let Some(transport) = transport {
                out.push(McpServer {
                    name: name.to_string(),
                    transport,
                    enabled: true,
                    disabled_tools: Vec::new(),
                });
            }
        }
        Ok(out)
    }

    fn render_mcp(&self, current: Option<&str>, servers: &[McpServer]) -> Result<String> {
        let path = self.config_path();
        let mut doc: DocumentMut = match current {
            Some(raw) if !raw.trim().is_empty() => raw
                .parse::<DocumentMut>()
                .map_err(|e| MoltoError::toml_parse(&path, e.to_string()))?,
            _ => DocumentMut::new(),
        };
        let enabled: Vec<&McpServer> = servers.iter().filter(|s| s.enabled).collect();

        // 严格单源：不在源里的 server 条目移除（外部增删由漂移流程保护）
        if let Some(item) = doc.get_mut(SERVERS_KEY) {
            if let Some(table) = item.as_table_mut() {
                let names: Vec<String> = enabled.iter().map(|s| s.name.clone()).collect();
                let stale: Vec<String> = table
                    .iter()
                    .map(|(k, _)| k.to_string())
                    .filter(|k| !names.contains(k))
                    .collect();
                for key in stale {
                    table.remove(&key);
                }
            }
        }
        if enabled.is_empty() {
            return Ok(doc.to_string());
        }

        // 确保 [mcp_servers] 父表存在；implicit 保证不渲染空的父表头
        if doc.get(SERVERS_KEY).is_none() {
            let mut parent = Table::new();
            parent.set_implicit(true);
            doc.insert(SERVERS_KEY, Item::Table(parent));
        }
        let parent = doc[SERVERS_KEY].as_table_mut().ok_or_else(|| {
            MoltoError::config_invalid(format!(
                "「{}」中 {} 不是表，无法写入 MCP 配置",
                path.display(),
                SERVERS_KEY
            ))
        })?;

        for server in enabled {
            let item = parent
                .entry(&server.name)
                .or_insert(Item::Table(Table::new()));
            if item.as_table_mut().is_none() {
                // 已有条目不是表（如内联值）：整体替换为表
                *item = Item::Table(Table::new());
            }
            let t = item.as_table_mut().ok_or_else(|| {
                MoltoError::config_invalid(format!(
                    "「{}」中 mcp_servers.{} 无法作为表写入",
                    path.display(),
                    server.name
                ))
            })?;
            apply_transport(t, &server.transport);
        }
        Ok(doc.to_string())
    }

    /// 指示.md：用户级 ~/.codex/AGENTS.md；项目级 AGENTS.md（项目根）。
    fn rule_files(&self) -> Vec<RuleFile> {
        vec![RuleFile {
            name: "AGENTS.md".into(),
            path: self
                .config_root()
                .join("AGENTS.md")
                .to_string_lossy()
                .to_string(),
        }]
    }

    /// Codex 官方 skills 目录（实测 0.144.5）。
    fn skills_dir(&self) -> Option<PathBuf> {
        Some(self.config_root().join(SKILLS_DIR))
    }

    /// 跨 Agent 会话共享 skill（v2.4）：~/.codex/skills/molto-session-resolver/SKILL.md
    fn render_session_skill(&self, current: Option<&str>) -> Result<Option<String>> {
        Ok(Some(crate::session_skill::render(current)?))
    }

    fn project_rule_name(&self) -> Option<String> {
        Some("AGENTS.md".into())
    }

    /// Codex：扫 ~/.codex/sessions/**/rollout-*.jsonl 首行 session_meta.payload.cwd。
    /// 只扫最近的文件（时间序目录遍历，上限 50 个），避免全量解析。
    fn recent_workspaces(&self) -> Vec<WorkspaceRecord> {
        read_codex_workspaces(&self.sessions_root())
    }

    fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
        CommandSpec {
            program: crate::resolve_program("codex"),
            args: vec![],
            working_dir: project_dir.to_path_buf(),
        }
    }

    /// v2.0：codex CLI 接受位置参数作为首条 prompt（`codex "消息"`）；
    /// 模型经官方 `-m`、推理强度经 `-c model_reasoning_effort=<v>` 指定 ⚠️ 实测待复核。
    /// 结构化通道（R3 流式）：`codex app-server` v2 JSON-RPC（实测 0.144.5+）。
    /// resume 兼容性随二进制版本：旧版 CLI 读不了新版 rollout（0.144.5 resume
    /// 0.152.0 的 paginated 历史线程报 "paginated_threads is not supported yet"），
    /// program 一律走 resolve_program 固定到 npm 全局（受管理的最新副本）。
    fn runtime(&self) -> Option<RuntimeSpec> {
        Some(RuntimeSpec {
            program: crate::resolve_program("codex"),
            args: vec![],
            windows_shim: true,
            prompt_inject: PromptInject::Argv,
            structured: Some(StructuredChannel {
                dialect: ProtocolDialect::CodexAppServer,
                // 无启动级模型 flag：模型/推理强度随 turn/start 参数下发（send_turn）
                model_flag: None,
                args: vec!["app-server".into()],
            }),
            model: Some(self.model_launch_option()),
            effort: Some(self.effort_launch_option()),
            resume_args: Some("resume {key}".into()),
            // 无人值守任务模式（channel 跟单）：官方 `codex exec` 非交互执行，
            // 进程退出即完成，stdout 为人类可读输出（实测 0.144.5）
            task_mode: Some(molto_core::TaskModeSpec {
                args: vec!["exec".into()],
                json_result: false,
                prompt_stdin: true,
                // 实测 0.144.5：--full-auto 已从 exec 移除；exec 默认 read-only
                // 沙箱 + 审批 never（写不了任何文件），须显式 workspace-write
                autonomy_args: vec!["-s".into(), "workspace-write".into()],
                headless_mcp: false,
            }),
        })
    }

    /// trait 接线：历史会话只读扫描（R2 换 SQLite 索引）。
    fn recent_sessions(&self) -> Vec<SessionSummary> {
        self.recent_sessions_impl()
    }

    /// 索引摄取源：~/.codex/sessions 下递归 rollout-*.jsonl。
    fn session_sources(&self) -> Vec<SessionSource> {
        vec![SessionSource {
            dir: self.sessions_root(),
            max_files: 150,
        }]
    }

    /// 纯函数：原始行 → 消息投影。meta（id/cwd）只在首块出现，增补块由索引
    /// 沿用既有归属；用户输入取 response_item（event_msg 的 user_message 会重复）。
    fn parse_chunk(&self, lines: &[String]) -> Option<ChunkParse> {
        let mut key = None;
        let mut project = None;
        let mut title = None;
        let mut messages: Vec<(usize, SnapshotMessage)> = Vec::new();

        for (i, raw) in lines.iter().enumerate() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(raw.trim()) else {
                continue;
            };
            let at = v
                .get("timestamp")
                .and_then(|x| x.as_str())
                .and_then(jsonutil::iso_to_ms);
            if v.get("type").and_then(|x| x.as_str()) == Some("session_meta") {
                if key.is_none() {
                    key = v
                        .get("payload")
                        .and_then(|p| p.get("id"))
                        .and_then(|x| x.as_str())
                        .map(String::from);
                }
                if project.is_none() {
                    project = v
                        .get("payload")
                        .and_then(|p| p.get("cwd"))
                        .and_then(|x| x.as_str())
                        .map(String::from);
                }
                continue;
            }
            let Some(payload) = v.get("payload") else {
                continue;
            };
            match payload.get("type").and_then(|x| x.as_str()) {
                Some("message") => {
                    let Some(role) = payload.get("role").and_then(|r| r.as_str()) else {
                        continue;
                    };
                    let role = match role {
                        "user" => SnapshotRole::User,
                        "assistant" => SnapshotRole::Assistant,
                        _ => continue,
                    };
                    let mut text = String::new();
                    if let Some(items) = payload.get("content").and_then(|c| c.as_array()) {
                        for it in items {
                            let kind = it.get("type").and_then(|t| t.as_str());
                            if kind == Some("input_text") || kind == Some("output_text") {
                                if let Some(t) = it.get("text").and_then(|t| t.as_str()) {
                                    text.push_str(t);
                                }
                            }
                        }
                    }
                    if text.trim().is_empty() {
                        continue;
                    }
                    let text = text.trim().chars().take(2000).collect::<String>();
                    // codex 注入的项目指令/环境上下文不是用户说的话：降级 system
                    // （可全文搜索、以弱化样式展示，不占用户气泡），标题也跳过
                    let role = if role == SnapshotRole::User && is_injected_context(&text) {
                        SnapshotRole::System
                    } else {
                        role
                    };
                    if title.is_none() && role == SnapshotRole::User {
                        title = Some(
                            text.lines()
                                .map(str::trim)
                                .find(|l| !l.is_empty())
                                .unwrap_or("")
                                .chars()
                                .take(80)
                                .collect(),
                        );
                    }
                    messages.push((
                        i,
                        SnapshotMessage {
                            seq: i,
                            role,
                            text,
                            tool_name: None,
                            at,
                        },
                    ));
                }
                Some("reasoning") => {
                    // 模型思考摘要（默认折叠展示）
                    let mut text = String::new();
                    if let Some(items) = payload.get("summary").and_then(|c| c.as_array()) {
                        for it in items {
                            if let Some(t) = it.get("text").and_then(|t| t.as_str()) {
                                text.push_str(t);
                            }
                        }
                    }
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
                Some("function_call") => {
                    let name = payload
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(String::from);
                    let args = payload
                        .get("arguments")
                        .and_then(|a| a.as_str())
                        .unwrap_or("");
                    if let Some(name) = name {
                        messages.push((
                            i,
                            SnapshotMessage {
                                seq: i,
                                role: SnapshotRole::Tool,
                                text: args.chars().take(4000).collect::<String>(),
                                tool_name: Some(name),
                                at,
                            },
                        ));
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

/// codex 注入的环境上下文前缀（rollout 里先于用户真实输入的 user 消息）。
/// 这类内容不是用户说的话：标题提取跳过；消息投影降级为 system（可搜、不占用户气泡）。
const INJECTED_CONTEXT_PREFIXES: &[&str] = &[
    "# AGENTS.md instructions",
    "<user_instructions>",
    "<environment_context>",
    "<ENVIRONMENT_CONTEXT>",
    "<turn_context>",
];

fn is_injected_context(text: &str) -> bool {
    let t = text.trim_start();
    INJECTED_CONTEXT_PREFIXES.iter().any(|p| t.starts_with(p))
}

/// 只改已知键，保留条目内未知键与格式。
fn apply_transport(t: &mut Table, transport: &Transport) {
    match transport {
        Transport::Stdio { command, args, env } => {
            t.remove("url");
            t.remove("headers");
            t.insert("command", toml_edit::value(command.clone()));
            if args.is_empty() {
                t.remove("args");
            } else {
                let mut arr = Array::new();
                for a in args {
                    arr.push(Value::from(a.clone()));
                }
                t.insert("args", toml_edit::value(arr));
            }
            if env.is_empty() {
                t.remove("env");
            } else {
                t.insert("env", toml_edit::value(string_map_to_inline(env)));
            }
        }
        Transport::Http { url, headers } => {
            t.remove("command");
            t.remove("args");
            t.remove("env");
            t.insert("url", toml_edit::value(url.clone()));
            if headers.is_empty() {
                t.remove("headers");
            } else {
                t.insert("headers", toml_edit::value(string_map_to_inline(headers)));
            }
        }
    }
}

fn string_map_from_item(item: Option<&Item>) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let Some(item) = item else { return out };
    if let Some(t) = item.as_table() {
        for (k, v) in t.iter() {
            if let Some(s) = v.as_str() {
                out.insert(k.to_string(), s.to_string());
            }
        }
    } else if let Some(it) = item.as_inline_table() {
        for (k, v) in it.iter() {
            if let Some(s) = v.as_str() {
                out.insert(k.to_string(), s.to_string());
            }
        }
    }
    out
}

fn string_map_to_inline(map: &std::collections::BTreeMap<String, String>) -> InlineTable {
    let mut inline = InlineTable::new();
    for (k, v) in map {
        inline.insert(k, Value::from(v.clone()));
    }
    inline
}

/// 从 sessions 目录解析历史 workspace（rollout 首行 cwd），最近优先，去重。
pub(crate) fn read_codex_workspaces(sessions_root: &Path) -> Vec<WorkspaceRecord> {
    let mut files: Vec<PathBuf> = vec![];
    collect_jsonl_recent_first(sessions_root, &mut files, 0);
    files.truncate(50);

    let mut seen = std::collections::BTreeSet::new();
    let mut out = vec![];
    for f in files {
        let Ok(head) = read_first_line(&f) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&head) else {
            continue;
        };
        let cwd = v
            .get("payload")
            .and_then(|p| p.get("cwd"))
            .and_then(|c| c.as_str())
            .unwrap_or_default();
        if cwd.is_empty() {
            continue;
        }
        let key = cwd.replace('/', "\\").to_lowercase();
        if seen.insert(key) {
            out.push(WorkspaceRecord {
                dir: cwd.to_string(),
                source: "codex sessions".into(),
            });
        }
    }
    out
}

fn read_first_line(path: &Path) -> std::io::Result<String> {
    use std::io::BufRead;
    let file = std::fs::File::open(path)?;
    let mut line = String::new();
    std::io::BufReader::new(file).read_line(&mut line)?;
    Ok(line)
}

/// 深度优先收集 *.jsonl；目录先按名倒序（sessions 按日期组织，名近 = 时间近）。
fn collect_jsonl_recent_first(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    items.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    for entry in items {
        let p = entry.path();
        if p.is_dir() {
            collect_jsonl_recent_first(&p, out, depth + 1);
        } else if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            out.push(p);
        }
    }
}

/// 每适配器历史会话扫描上限（R2 换 SQLite 索引后取消）。
const SESSIONS_CAP: usize = 100;

fn mtime_ms(p: &Path) -> i64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {

    #[test]
    fn session_scan_parse_and_config_defaults() {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join(".codex/sessions/2026/08/30");
        std::fs::create_dir_all(&dir).unwrap();
        let lines = [
            r#"{"type":"session_meta","payload":{"id":"u2","cwd":"D:/ksa/shop","model_provider":"ZAI"}}"#.to_string(),
            r#"{"type":"response_item","timestamp":"2026-08-30T11:00:00Z","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"排查 CORS 504"}]}}"#.to_string(),
            r#"{"type":"response_item","timestamp":"2026-08-30T11:00:09Z","payload":{"type":"reasoning","summary":[{"type":"summary_text","text":"先看网关日志"}]}}"#.to_string(),
            r#"{"type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"cmd\":\"tail -f gw.log\"}"}}"#.to_string(),
        ];
        std::fs::write(
            dir.join("rollout-1-u2.jsonl"),
            lines.join(
                "
",
            ),
        )
        .unwrap();
        // 外部形态（Codex Desktop）创建的会话：provider 只在其自身配置定义，
        // CLI 配置解析不了 → 不得声明 resumable
        let desktop = [
            r#"{"type":"session_meta","payload":{"id":"u3","cwd":"D:/ksa/shop","model_provider":"custom"}}"#.to_string(),
        ];
        std::fs::write(
            dir.join("rollout-2-u3.jsonl"),
            desktop.join(
                "
",
            ),
        )
        .unwrap();
        std::fs::write(
            home.path().join(".codex/config.toml"),
            "model = \"gpt-5.2\"
model_reasoning_effort = \"high\"

[model_providers.ZAI]
name = \"ZAI\"
",
        )
        .unwrap();

        let adapter = CodexAdapter::new(home.path());
        let sessions = adapter.recent_sessions();
        assert_eq!(sessions.len(), 2);
        let by_key = |k: &str| sessions.iter().find(|s| s.session_key == k).unwrap();
        assert_eq!(by_key("u2").project_path, "D:/ksa/shop");
        assert!(by_key("u2").resumable, "配置已定义 ZAI：可续");
        assert!(!by_key("u2").resume_via_tui, "provider 可解析：GUI 可续");
        assert!(
            by_key("u3").resumable && by_key("u3").resume_via_tui,
            "provider custom 不在配置中：仍可续，但须走 TUI"
        );
        assert!(
            by_key("u2").title.contains("CORS"),
            "title={}",
            by_key("u2").title
        );

        let parse = adapter.parse_chunk(&lines).unwrap();
        assert_eq!(parse.messages.len(), 3, "user + reasoning + function_call");
        assert_eq!(parse.messages[0].1.role, molto_core::SnapshotRole::User);
        assert!(parse.messages[0].1.at.is_some(), "时间戳应被解析");
        assert_eq!(parse.messages[1].1.role, molto_core::SnapshotRole::Thinking);
        assert!(parse.messages[1].1.text.contains("网关日志"));
        assert_eq!(parse.messages[2].1.tool_name.as_deref(), Some("shell"));
        assert!(
            parse.messages[2].1.text.contains("tail -f"),
            "arguments 应被捕获"
        );

        let spec = adapter.runtime().unwrap();
        assert_eq!(
            spec.model.as_ref().unwrap().default.as_deref(),
            Some("gpt-5.2")
        );
        assert_eq!(spec.model.unwrap().choices[0], "gpt-5.2", "配置值置顶");
        assert_eq!(spec.effort.unwrap().default.as_deref(), Some("high"));
    }

    /// 回归：codex 把 AGENTS.md / 环境上下文作为首条 user 消息写入 rollout——
    /// 标题必须跳过它取真实首条消息；投影里降级为 system（不占用户气泡）。
    #[test]
    fn injected_context_skipped_for_title_and_role() {
        let lines = [
            r#"{"type":"session_meta","payload":{"id":"u9","cwd":"D:/p"}}"#.to_string(),
            r##"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"# AGENTS.md instructions for D:\\p\n\n很长的项目说明…"}]}}"##.to_string(),
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"修一下登录超时"}]}}"#.to_string(),
        ];
        let adapter = CodexAdapter::new(Path::new("/h"));
        let parse = adapter.parse_chunk(&lines).unwrap();
        assert_eq!(parse.title.as_deref(), Some("修一下登录超时"));
        assert_eq!(
            parse.messages[0].1.role,
            molto_core::SnapshotRole::System,
            "注入上下文降级 system"
        );
        assert_eq!(parse.messages[1].1.role, molto_core::SnapshotRole::User);

        // 摘要面同样跳过（写盘走 recent_sessions 的标题逻辑）
        let home = tempfile::tempdir().unwrap();
        let day = home.path().join(".codex/sessions/2026/08/31");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::write(day.join("rollout-u9.jsonl"), lines.join("\n")).unwrap();
        let sessions = CodexAdapter::new(home.path()).recent_sessions();
        assert_eq!(sessions[0].title, "修一下登录超时");
    }
    use super::*;

    const FIXTURE: &str = r#"# cc-switch 管理的模型配置
model_provider = "custom"
model = "gpt-5.2"
[model_providers.custom]
name = "openai"
base_url = "https://api.example.com/v1"

# MCP servers（Molto 管理区起点）
[mcp_servers.fs]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-fs"]
env = { DEBUG = "1" }

[mcp_servers.search]
url = "https://mcp.example.com/search"
custom_note = "keep-me" # 未知键
"#;

    fn servers() -> Vec<McpServer> {
        CodexAdapter::new(Path::new("/h"))
            .read_mcp(FIXTURE)
            .unwrap()
    }

    #[test]
    fn roundtrip_read_render_read() {
        let original = servers();
        assert_eq!(original.len(), 2);
        let out = CodexAdapter::new(Path::new("/h"))
            .render_mcp(Some(FIXTURE), &original)
            .unwrap();
        let reparsed = CodexAdapter::new(Path::new("/h")).read_mcp(&out).unwrap();
        assert_eq!(reparsed.len(), 2);
        match (&reparsed[0].transport, &original[0].transport) {
            (
                Transport::Stdio {
                    command: c1,
                    args: a1,
                    env: e1,
                },
                Transport::Stdio {
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

    #[test]
    fn preserves_comments_and_unknown_keys() {
        let out = CodexAdapter::new(Path::new("/h"))
            .render_mcp(Some(FIXTURE), &servers())
            .unwrap();
        assert!(out.contains("# cc-switch 管理的模型配置"));
        assert!(out.contains("# MCP servers（Molto 管理区起点）"));
        assert!(out.contains("custom_note"));
        assert!(out.contains("keep-me"));
        assert!(out.contains("model_provider = \"custom\""));
        assert!(out.contains("base_url = \"https://api.example.com/v1\""));
    }

    #[test]
    fn strict_single_source_removes_stale() {
        let mut srv = servers();
        srv.retain(|s| s.name == "fs");
        let out = CodexAdapter::new(Path::new("/h"))
            .render_mcp(Some(FIXTURE), &srv)
            .unwrap();
        assert!(out.contains("mcp_servers.fs"));
        assert!(!out.contains("mcp_servers.search"));
        assert!(out.contains("model_provider"));
    }

    #[test]
    fn render_from_empty_creates_valid_toml() {
        let srv = vec![McpServer {
            name: "db".into(),
            transport: Transport::Stdio {
                command: "mcp-postgres".into(),
                args: vec!["--port".into(), "5432".into()],
                env: [("PGHOST".to_string(), "localhost".to_string())]
                    .into_iter()
                    .collect(),
            },
            enabled: true,
            disabled_tools: Vec::new(),
        }];
        let out = CodexAdapter::new(Path::new("/h"))
            .render_mcp(None, &srv)
            .unwrap();
        let parsed: Result<Vec<McpServer>> = CodexAdapter::new(Path::new("/h")).read_mcp(&out);
        let read = parsed.unwrap();
        assert_eq!(read.len(), 1);
        match &read[0].transport {
            Transport::Stdio { command, args, env } => {
                assert_eq!(command, "mcp-postgres");
                assert_eq!(args, &["--port".to_string(), "5432".to_string()]);
                assert_eq!(env.get("PGHOST").map(String::as_str), Some("localhost"));
            }
            _ => panic!("expected stdio"),
        }
    }

    #[test]
    fn workspaces_from_rollout_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let day = dir.path().join("2026/08/30");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::write(
            day.join("rollout-a.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"D:\\\\work\\\\game\"}}\n",
        )
        .unwrap();
        std::fs::write(
            day.join("rollout-b.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"D:\\\\work\\\\game\"}}\n", // 重复目录去重
        )
        .unwrap();
        let ws = read_codex_workspaces(dir.path());
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].dir, "D:\\work\\game");
        assert_eq!(ws[0].source, "codex sessions");
    }

    #[test]
    fn detect_and_paths() {
        let dir = tempfile::tempdir().unwrap();
        assert!(CodexAdapter::new(dir.path()).detect().is_none());
        std::fs::create_dir_all(dir.path().join(CONFIG_DIR)).unwrap();
        assert!(CodexAdapter::new(dir.path()).detect().is_some());
        let adapter = CodexAdapter::new(dir.path());
        assert_eq!(
            adapter.mcp_path(&Scope::User).unwrap(),
            dir.path().join(CONFIG_DIR).join(CONFIG_FILE)
        );
        assert!(adapter
            .mcp_path(&Scope::Project { dir: "D:/w".into() })
            .is_none());
    }

    /// 幂等性：render(render(x)) == render(x)（e2e NoChange 断言的前提）。
    #[test]
    fn render_is_idempotent() {
        let srv = servers();
        let adapter = CodexAdapter::new(Path::new("/h"));
        let out1 = adapter.render_mcp(Some(FIXTURE), &srv).unwrap();
        let out2 = adapter.render_mcp(Some(&out1), &srv).unwrap();
        assert_eq!(out1, out2, "codex render 必须幂等");
    }

    /// e2e 场景复现：空 parent 表（删除唯一子表后）必须保持幂等。
    #[test]
    fn render_idempotent_with_empty_parent() {
        let fixture = "# 用户手写注释
model = \"gpt-5\"

[mcp_servers.local]
command = \"local-cmd\" # 手写注释
";
        let adapter = CodexAdapter::new(Path::new("/h"));
        let original = adapter.read_mcp(fixture).unwrap();
        let out1 = adapter.render_mcp(Some(fixture), &original).unwrap();
        eprintln!(
            "OUT1:
{}",
            out1
        );
        let out2 = adapter.render_mcp(Some(&out1), &original).unwrap();
        eprintln!(
            "OUT2:
{}",
            out2
        );
        assert_eq!(out1, out2, "codex render 必须幂等（e2e fixture）");
    }
}
