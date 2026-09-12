//! Runtime 域模型（v2.0）：PTY 会话宿主消费的启动画像（runtime-design.md §3.5）。
//!
//! Adapter 以纯数据形式提供 per-Agent 知识，molto-runtime 只面向这里编程，
//! 不感知具体 Agent（AGENTS.md §2 依赖铁律）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::streaming::StructuredChannel;

/// PTY 启动画像：AgentAdapter::runtime() 的产物。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RuntimeSpec {
    /// 可执行程序名（"claude" / "codex" / …，由 PATH 解析）
    pub program: String,
    /// 基础参数（TUI 模式，通常为空）
    pub args: Vec<String>,
    /// Windows 下该程序可能是 npm 的 .cmd shim：spawn 时需经 `cmd /c` 解析
    pub windows_shim: bool,
    /// 首条 prompt 的送达方式
    pub prompt_inject: PromptInject,
    /// 官方结构化通道（v2.0 R3 流式）：None = 仅 PTY TUI
    pub structured: Option<StructuredChannel>,
    /// 模型选择（官方 CLI 参数）；None = 该 Agent 不支持启动时指定
    pub model: Option<LaunchOption>,
    /// 推理强度选择（官方 CLI 参数）；None = 不支持
    pub effort: Option<LaunchOption>,
    /// 续聊参数模板（含 {key} 占位符，如 "--resume {key}" / "resume {key}"）；
    /// None = 不支持 resume
    pub resume_args: Option<String>,
    /// 无人值守任务模式（channel 跟单）：官方 headless 一次性运行参数；
    /// None = 该 Agent 未声明，不可承担 channel 开发任务
    pub task_mode: Option<TaskModeSpec>,
}

/// headless 一次性任务画像：args 拼在 prompt 之前，进程退出即任务完成。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TaskModeSpec {
    /// headless 参数（如 claude 的 ["-p", "--output-format", "json"]）
    pub args: Vec<String>,
    /// stdout 末段是否为 JSON 结果对象（result 字段为最终回复）
    pub json_result: bool,
    /// prompt 经 stdin 送达（多行 prompt 走 cmd /c argv 会被换行截断；
    /// claude -p / codex exec 均支持 stdin 读 prompt）
    pub prompt_stdin: bool,
    /// 无人值守法权限参数（headless 下免交互批准/沙箱放行；worktree 为一次性沙箱）。
    /// claude: --dangerously-skip-permissions；codex: -s workspace-write
    /// （0.144.5 实测：--full-auto 已移除，exec 默认 read-only 沙箱写不了文件）
    pub autonomy_args: Vec<String>,
    /// headless 下支持官方 `--mcp-config` 注入会话级 MCP server（channel 浏览器
    /// 验证角色的接线方式，见 docs/verify-agent.md）。当前仅 claude 实测支持；
    /// codex 的 MCP 走 config.toml，无同名启动参数，不可担任验证角色。
    pub headless_mcp: bool,
}

/// 一个启动选项的声明（choices 供前端下拉，arg_template 是含 {v} 占位符的
/// 官方参数模板，可含多个 token，spawn 前按空格拆分）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LaunchOption {
    pub choices: Vec<String>,
    pub arg_template: String,
    pub default: Option<String>,
    /// 选择的值在 GUI 流式会话同样生效（claude 启动 flag / codex turn 参数）；
    /// false = 仅 TUI 会话生效（如 opencode：TUI `--model`，其 ACP 通道
    /// 无启动级模型参数、`session/set_config` 上游未实现）
    pub stream_override: bool,
}

/// 前端可选的启动参数组合（供 AgentInfo 能力暴露）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LaunchOptions {
    pub model: Option<LaunchOption>,
    pub effort: Option<LaunchOption>,
}

/// 用户在欢迎屏选择的启动覆盖项（None = 用 Agent 自己的默认）。
#[derive(Debug, Clone, Default)]
pub struct LaunchOverrides {
    pub model: Option<String>,
    pub effort: Option<String>,
    /// 要续聊的历史会话标识（SessionSummary.session_key）
    pub resume_key: Option<String>,
}

/// 首条 prompt 送达方式（runtime-design.md §3.4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PromptInject {
    /// 作为位置参数拼进启动 argv（`claude "消息"`）
    #[default]
    Argv,
    /// 先开终端，用户自己在 TUI 内输入（Molto 不注入）
    None,
}

/// 会话形态：Agent TUI 对话（Chat 域主体）或工作台通用终端（底部 dock，
/// git/构建等日常 shell，不属于任何 Agent）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SessionKind {
    #[default]
    Agent,
    Terminal,
}

/// 会话通道形态：PTY 内嵌 TUI，或官方结构化协议流式（v2.0 R3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SessionChannel {
    #[default]
    Pty,
    Stream,
}

/// 活会话信息（前端 Chat 左栏 / 上下文条的数据形态）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LiveSessionInfo {
    /// Molto 侧会话 id（uuid）
    pub session_id: String,
    pub agent_id: String,
    /// 会话形态（Agent 对话 / 通用终端）；终端会话不进 Chat 侧栏
    pub kind: SessionKind,
    /// 通道：PTY TUI 或结构化流式（原生视图据此分流数据源）
    pub channel: SessionChannel,
    pub project_dir: String,
    /// 会话是否仍在运行（false 时 exit_code 有效）
    pub running: bool,
    pub exit_code: Option<i32>,
    /// Unix 毫秒时间戳
    #[ts(type = "number")]
    pub started_at: i64,
    /// 最近一次输入/输出的时刻（用于排序）
    #[ts(type = "number")]
    pub last_active_at: i64,
    /// rollout 会话键（resume 启动时记录）：侧栏按会话聚合 + spawn 幂等裁决
    #[serde(default)]
    pub resume_key: Option<String>,
}
