//! 结构化流式通道模型（v2.0 R3，t3code 对齐）：
//! Agent 经官方 headless 协议（如 codex app-server JSON-RPC）对话，
//! 原生视图实时渲染增量。Adapter 只声明「有此通道 + 协议方言」（纯数据），
//! 方言的具体实现归 molto-runtime，与 PTY 宿主同一分层。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 官方结构化协议方言（runtime 按方言实现；新增方言 = 扩枚举 + runtime 实现）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum ProtocolDialect {
    /// OpenAI codex `app-server` v2 JSON-RPC（newline-delimited，实测 0.144.5）
    CodexAppServer,
    /// Claude Code headless：`-p --output-format stream-json --input-format
    /// stream-json --include-partial-messages`（JSONL 事件，实测 2.1.251）
    ClaudeStream,
    /// ACP（Agent Client Protocol，Zed 主导）：JSON-RPC 2.0 over stdio。
    /// 通用方言——Gemini CLI 原生支持（`--experimental-acp`），OpenCode 等复用
    /// 同一实现（runtime-design §11.5）
    Acp,
}

/// RuntimeSpec 的结构化通道声明（None = 该 Agent 只支持 PTY TUI）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StructuredChannel {
    pub dialect: ProtocolDialect,
    /// 启动参数（如 ["app-server"]，拼在 program 后）
    pub args: Vec<String>,
    /// 模型覆盖的启动 flag（如 "--model" / "-m"，与模型值成对拼在 args 后）；
    /// None = 该通道无启动级模型参数——codex 的模型走 turn/start 参数，
    /// opencode 等通道型 Agent 的 TUI 模型 flag 由 RuntimeSpec.model 单独声明
    pub model_flag: Option<String>,
}

/// 工具条目的结构化 diff（ACP `tool_call` 的 `content[].type=diff`；其余方言/情形为 None）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamEventDiff {
    pub path: String,
    pub old_text: Option<String>,
    pub new_text: String,
}

/// 归一化流式事件（方言无关；前端据此增量渲染原生视图）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamEvent {
    pub session_id: String,
    /// 方言侧条目 id（增量拼接的去重键；无条目语义的事件为空）
    pub item_id: String,
    #[ts(type = "number")]
    pub at: i64,
    pub kind: StreamEventKind,
    /// 文本增量 / 条目全文（按 kind 语义）
    pub text: String,
    /// 工具类条目的名称（command 首词 / server.tool）
    pub tool_name: Option<String>,
    /// 工具条目状态（completed/failed/inProgress…，方言原样透传）
    pub status: Option<String>,
    /// 结构化文件修改（仅 ToolItem 且方言提供时；前端渲染 diff 卡片）
    #[serde(default)]
    pub diff: Option<StreamEventDiff>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum StreamEventKind {
    /// 用户消息回显（权威全文）
    UserEcho,
    /// 助手消息增量（append）
    AgentDelta,
    /// 助手消息完成（权威全文，替换该条目）
    AgentDone,
    /// 思考增量（append）
    ReasoningDelta,
    /// 条目完成：工具调用（command/fileChange/mcpToolCall…）
    ToolItem,
    /// 条目完成：思考全文
    ReasoningDone,
    /// 轮次开始（输入已被接受）
    TurnStarted,
    /// 轮次完成（running=false）
    TurnCompleted,
    /// 协议错误 / 会话错误
    Error,
    /// 通道进程退出
    Exit,
}

/// ACP 会话级配置项（session/new|load 应答 `configOptions` 的归一化，§11.5）。
/// ACP 官方机制：mode 走 `session/set_mode`；其余（model…）走
/// `session/set_config_option`——均实测 opencode 1.18.3 生效（此前误调
/// `session/set_config` 被误记为上游未实现）。settable 项在会话工具栏直接出下拉，
/// 选项清单即 agent 声明的原始值。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamConfigOption {
    pub id: String,
    pub name: String,
    /// 归类（model / thought_level / mode…，方言原样）
    pub category: Option<String>,
    pub current_value: String,
    pub choices: Vec<StreamConfigChoice>,
    /// 当前 mode 与 model 类可写
    pub settable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamConfigChoice {
    pub value: String,
    pub name: String,
}

/// 流式活会话信息（侧栏/工具栏数据形态；与 PTY 的 LiveSessionInfo 平行）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LiveStreamInfo {
    pub session_id: String,
    pub agent_id: String,
    pub project_dir: String,
    /// 方言侧会话标识（codex threadId；resume/诊断用）
    pub thread_id: String,
    pub running: bool,
    /// 已完成轮次数
    #[ts(type = "number")]
    pub turns: i64,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number")]
    pub last_active_at: i64,
    /// rollout 会话键（resume 启动时记录）：侧栏按会话聚合 + spawn 幂等裁决
    #[serde(default)]
    pub resume_key: Option<String>,
    /// ACP 会话级配置项（codex/claude 方言为空）
    #[serde(default)]
    pub config_options: Vec<StreamConfigOption>,
}
