//! 统一分发引擎：plan（dry-run 预览）→ apply（唯一写入路径）。
//!
//! 红线（AGENTS.md §0/§3）：
//! - 所有对 Agent 配置文件的写入只发生在 [`apply`]：备份快照 → 原子写 → 记 history.jsonl
//! - Drift 三选一：覆盖 / 采纳为源 / 跳过，绝不静默覆盖
//! - Adapter 只做纯渲染，格式转换封死在适配层

pub mod apply;
pub mod diff;
pub mod plan;

use std::collections::BTreeMap;

use crate::backup::BackupManager;
use crate::error::Result;
use crate::model::mcp::Scope;
use crate::registry::Registry;
use crate::store::Store;

pub use apply::{AppliedFile, ApplyReport, DriftDecision};
pub use plan::{plan_target, FilePlan, PlanKind, TargetAction};

pub struct SyncEngine<'a> {
    pub store: &'a Store,
    pub backup: &'a BackupManager,
    pub registry: &'a Registry,
}

impl<'a> SyncEngine<'a> {
    pub fn new(store: &'a Store, backup: &'a BackupManager, registry: &'a Registry) -> Self {
        Self {
            store,
            backup,
            registry,
        }
    }

    /// dry-run：为每个目标生成 FilePlan（含工具颗粒度文件），不落任何盘。
    pub fn plan(&self, targets: &[(String, Scope)]) -> Result<Vec<FilePlan>> {
        let mut plans = vec![];
        for (adapter, scope) in targets {
            plans.extend(plan_target(self.store, self.registry, adapter, scope)?);
        }
        Ok(plans)
    }

    /// 唯一写入路径。Drift 项必须带决策，否则报错拒绝执行。
    pub fn apply(
        &self,
        plans: &[FilePlan],
        decisions: &BTreeMap<String, DriftDecision>,
    ) -> Result<ApplyReport> {
        apply::apply(self.store, self.backup, self.registry, plans, decisions)
    }
}
