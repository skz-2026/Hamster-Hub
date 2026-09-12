//! 指示.md 规则文件与 Skills 清单模型（v1.5）。

use serde::Serialize;
use ts_rs::TS;

/// 规则文件（指示.md）：如 CLAUDE.md / AGENTS.md。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RuleFile {
    /// 展示名（CLAUDE.md / AGENTS.md）
    pub name: String,
    pub path: String,
}

/// Skill 条目：skills 目录下的一个子目录。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SkillInfo {
    pub name: String,
    pub path: String,
}
