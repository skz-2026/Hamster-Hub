//! Workspace 历史发现（v1.4）：各 Agent 用自己的机制发现历史工作目录。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 一条 workspace 记录。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceRecord {
    pub dir: String,
    /// 发现来源描述（展示用），如 "claude projects" / "codex sessions" / "zcode recentProjects"
    pub source: String,
}
