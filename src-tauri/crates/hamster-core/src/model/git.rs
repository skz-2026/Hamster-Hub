//! 工作台 Git 面板模型（v2.0 R3）：仓库状态 / 文件变更 / 提交历史。
//!
//! 数据只来自官方 `git` CLI 的标准输出（porcelain/统一 diff），上游 不做任何
//! 仓库内部结构解析——与红线①同一原则：只经官方接口，不侵入。

use serde::Serialize;
use ts_rs::TS;

/// 一个文件的双侧状态字母（porcelain XY：index 侧 / 工作区侧）。
/// 字母语义与 `git status --short` 一致（M/A/D/R/C/U/?/!/空格）。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitFileStatus {
    /// 仓库相对路径（重命名显示为 `old -> new`）
    pub path: String,
    /// index 侧状态字母（已暂存的内容）
    pub staged: String,
    /// 工作区侧状态字母（未暂存的改动）
    pub worktree: String,
}

/// `git status --porcelain=v1 --branch` 的结构化投影。
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitStatus {
    /// 当前分支（detached HEAD 时为 None）
    pub branch: Option<String>,
    /// 领先上游的提交数（无上游为 0）
    #[ts(type = "number")]
    pub ahead: i64,
    /// 落后上游的提交数
    #[ts(type = "number")]
    pub behind: i64,
    pub files: Vec<GitFileStatus>,
}

impl GitStatus {
    /// 是否有已暂存待提交的内容（index 侧字母非空非 `?`）。
    pub fn has_staged(&self) -> bool {
        self.files
            .iter()
            .any(|f| !f.staged.is_empty() && f.staged != " " && f.staged != "?")
    }
}

/// 一条提交历史（`git log` 行投影）。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitLogEntry {
    pub hash: String,
    pub subject: String,
    pub author: String,
    /// Unix 秒
    #[ts(type = "number")]
    pub at: i64,
}
