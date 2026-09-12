//! 会话快照模型（v2.0）：
//! - `SessionSummary`：侧栏列表用的元数据切片（R2 前置）
//! - `SnapshotMessage` / `ChunkParse` / `SessionSource`：全文索引的解析协议（R2）
//! - `SearchQuery` / `SearchHit` / `IndexStatus`：索引查询面

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 一条历史会话的摘要（从 Agent 官方落盘文件只读提取）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionSummary {
    /// Agent 全小写 id（"claude" / "codex" / …）
    pub agent: String,
    /// Agent 侧会话标识（resume 用，如 Claude 的 uuid）
    pub session_key: String,
    /// 会话所属项目目录（Agent 落盘的 cwd）
    pub project_path: String,
    /// 标题（首条用户消息截断；提取不到时为空，由前端兜底）
    pub title: String,
    /// 最近活跃时刻（Unix 毫秒，取文件 mtime）
    #[ts(type = "number")]
    pub last_active_at: i64,
    /// 源文件路径（只读回指，调试与排重用）
    pub source_path: String,
    /// 是否已声明官方 resume 参数（false = 仅浏览，UI 不提供续聊）
    pub resumable: bool,
    /// 续聊须走 TUI（PTY）通道：GUI 流式协议 resume 该会话必失败
    /// （如 codex rollout 引用当前配置无法解析的 model provider，实测 0.152.0）
    #[serde(default)]
    pub resume_via_tui: bool,
}

/// 消息角色（索引投影用，保持最小集合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SnapshotRole {
    User,
    Assistant,
    /// 思考/推理内容（默认折叠展示）
    Thinking,
    Tool,
    System,
}

/// 一条可检索的消息投影（原文仍在 Agent 源文件，索引只存文本投影）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SnapshotMessage {
    /// 所在源文件的行号（0 基），作为会话内稳定序号
    #[ts(type = "number")]
    pub seq: usize,
    pub role: SnapshotRole,
    /// 提取的纯文本（搜索面）
    pub text: String,
    pub tool_name: Option<String>,
    #[ts(type = "number")]
    pub at: Option<i64>,
}

/// Adapter 对一段原始行的解析结果（纯函数输出；字段可为 None = 本段无法判定，
/// 由索引层沿用既有值）。`messages` 的 seq 为切片内行号，索引层加上行基址。
#[derive(Debug, Clone, Default)]
pub struct ChunkParse {
    pub session_key: Option<String>,
    pub project_path: Option<String>,
    /// 标题（首条用户消息，仅首页块提供；索引做占位回填）
    pub title: Option<String>,
    pub messages: Vec<(usize, SnapshotMessage)>,
}

/// 一个会话落盘扫描源（目录下递归收集 *.jsonl，最近优先，数量有上限）。
#[derive(Debug, Clone)]
pub struct SessionSource {
    pub dir: PathBuf,
    pub max_files: usize,
}

/// 全文检索查询（agents 为空 = 不过滤）。
#[derive(Debug, Clone, Deserialize, Serialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchQuery {
    pub text: String,
    pub agents: Vec<String>,
    pub project: Option<String>,
    pub limit: Option<usize>,
}

/// 一条全文命中（消息级定位）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchHit {
    pub agent: String,
    pub session_key: String,
    #[ts(type = "number")]
    pub seq: usize,
    pub session_title: String,
    pub project_path: Option<String>,
    #[ts(type = "number")]
    pub at: Option<i64>,
    /// 命中上下文片段（带标记）
    pub snippet: String,
}

/// 原生视图分页：会话消息总数 + 从 from_seq 起的升序消息。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionMessagesPage {
    #[ts(type = "number")]
    pub total: i64,
    pub messages: Vec<SnapshotMessage>,
}

/// 会话元数据（agent-cli 只读输出；索引 sessions 表的投影，跨 Agent 共享面）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionMeta {
    /// Agent 全小写 id（"claude" / "codex" / …）
    pub agent: String,
    /// Agent 侧会话标识（同 SessionSummary.session_key）
    pub session_key: String,
    pub title: String,
    pub project_path: String,
    #[ts(type = "number")]
    pub message_count: i64,
    /// 源文件最近修改时刻（Unix 毫秒，≈ 会话最近活跃）
    #[ts(type = "number")]
    pub last_active_at: i64,
    /// 源文件路径（只读回指，调试用）
    pub source_path: String,
}

/// 索引库状态（Settings / Doctor 展示）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexStatus {
    #[ts(type = "number")]
    pub sessions: i64,
    #[ts(type = "number")]
    pub messages: i64,
    /// 库文件体积（字节）
    #[ts(type = "number")]
    pub bytes: i64,
}
