//! dry-run 计划：对每个分发目标，渲染目标内容并与现状/上次分发 hash 比对，
//! 产出 Create / Update / NoChange / Drift 四类动作。此阶段不写任何文件。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{HamsterError, Result};
use crate::hash::sha256_hex;
use crate::model::mcp::Scope;
use crate::registry::Registry;
use crate::store::{Store, SyncState};
use crate::sync::diff::unified_diff;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum TargetAction {
    /// 目标文件不存在，将新建
    Create,
    /// 上次分发后文件未被动过，正常更新
    Update,
    /// 内容已一致，无需写入
    NoChange,
    /// 目标文件在 仓鼠Hub 之外被修改过，需用户三选一
    Drift,
}

/// 计划种类：MCP 配置文件 / 工具颗粒度权限文件 / 会话共享 skill（v2.4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PlanKind {
    Mcp,
    ToolConfig,
    Skill,
}

/// 跨 Agent 会话共享 skill 的目录名（architecture.md §6.1）。
pub const SESSION_SKILL_NAME: &str = "hamster-session-resolver";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FilePlan {
    pub adapter: String,
    /// mcp = MCP 配置文件；toolConfig = 工具颗粒度权限文件；skill = 会话共享 skill
    pub kind: PlanKind,
    pub scope: Scope,
    pub path: String,
    pub action: TargetAction,
    pub current_content: Option<String>,
    pub new_content: Option<String>,
    /// unified diff（Update / Drift 时提供）
    pub diff: Option<String>,
}

impl FilePlan {
    /// state.json 的键：不同 kind 分开记账（mcp / tools / skill 互不覆盖）。
    pub fn state_key(&self) -> String {
        match self.kind {
            PlanKind::Mcp => SyncState::state_key(&self.adapter, &self.scope),
            PlanKind::ToolConfig => {
                format!("{}/tools", SyncState::state_key(&self.adapter, &self.scope))
            }
            PlanKind::Skill => {
                format!("{}/skill", SyncState::state_key(&self.adapter, &self.scope))
            }
        }
    }
}

/// Create / Update / NoChange / Drift 判定（三类分发目标共用同一语义）。
/// `applied_hash` 为 state.json 里上次分发的基线 hash；无基线且现状存在 = 漂移。
fn eval_action(
    path: &str,
    current: &Option<String>,
    new_content: &str,
    applied_hash: Option<&String>,
) -> (TargetAction, Option<String>) {
    let Some(cur) = current else {
        return (TargetAction::Create, None);
    };
    // 仅尾部空白（结尾换行）不同视为无变化：语义等价，不值得落盘
    if cur.trim_end() == new_content.trim_end() {
        return (TargetAction::NoChange, None);
    }
    let is_drift = match applied_hash {
        Some(record) => record != &sha256_hex(cur),
        None => true,
    };
    let act = if is_drift {
        TargetAction::Drift
    } else {
        TargetAction::Update
    };
    (act, Some(unified_diff(path, cur, new_content)))
}

pub fn plan_target(
    store: &Store,
    registry: &Registry,
    adapter_id: &str,
    scope: &Scope,
) -> Result<Vec<FilePlan>> {
    let adapter = registry.get(adapter_id)?;
    let caps = adapter.capabilities();
    let scope_ok = match scope {
        Scope::User => caps.mcp,
        Scope::Project { .. } => caps.project_mcp,
    };
    if !scope_ok {
        return Err(HamsterError::Unsupported(format!(
            "{} 不支持 {} 级 MCP 分发",
            adapter.name(),
            match scope {
                Scope::User => "用户",
                Scope::Project { .. } => "项目",
            }
        )));
    }
    let path = adapter.mcp_path(scope).ok_or_else(|| {
        HamsterError::Unsupported(format!(
            "{} 未定义 {} 的 MCP 配置路径",
            adapter.name(),
            scope.key()
        ))
    })?;

    let config = store.load_config()?;
    let servers: Vec<_> = config
        .servers
        .iter()
        .filter(|s| s.enabled)
        .cloned()
        .collect();

    let current = crate::atomic::read_text(&path)?;
    let new_content = adapter.render_mcp(current.as_deref(), &servers)?;

    let state = store.load_state()?;
    let applied = state
        .applied
        .get(&SyncState::state_key(adapter_id, scope))
        .map(|r| r.hash.clone());
    let (action, diff) = eval_action(
        &path.to_string_lossy(),
        &current,
        &new_content,
        applied.as_ref(),
    );

    let mcp_plan = FilePlan {
        adapter: adapter_id.to_string(),
        kind: PlanKind::Mcp,
        scope: scope.clone(),
        path: path.to_string_lossy().to_string(),
        action,
        current_content: current,
        new_content: Some(new_content),
        diff,
    };

    let mut plans = vec![mcp_plan];

    // —— 工具颗粒度（v1.2）：支持该能力的 Agent 额外产出权限文件计划 ——
    if adapter.capabilities().tool_granularity {
        if let Some(tool_path) = adapter.tool_config_path(scope) {
            let tool_current = crate::atomic::read_text(&tool_path)?;
            let tool_new = adapter.render_tool_config(tool_current.as_deref(), &servers)?;
            if let Some(tool_new) = tool_new {
                let tool_key = format!("{}/tools", SyncState::state_key(adapter_id, scope));
                let tool_applied = state.applied.get(&tool_key).map(|r| r.hash.clone());
                let (tool_action, tool_diff) = eval_action(
                    &tool_path.to_string_lossy(),
                    &tool_current,
                    &tool_new,
                    tool_applied.as_ref(),
                );
                plans.push(FilePlan {
                    adapter: adapter_id.to_string(),
                    kind: PlanKind::ToolConfig,
                    scope: scope.clone(),
                    path: tool_path.to_string_lossy().to_string(),
                    action: tool_action,
                    current_content: tool_current,
                    new_content: Some(tool_new),
                    diff: tool_diff,
                });
            }
        }
    }

    // —— 跨 Agent 会话共享 skill（v2.4）：用户级分发，Caps.skills 为闸门 ——
    if matches!(scope, Scope::User) && adapter.capabilities().skills {
        if let Some(skill_dir) = adapter.skills_dir() {
            let skill_path = skill_dir.join(SESSION_SKILL_NAME).join("SKILL.md");
            let skill_current = crate::atomic::read_text(&skill_path)?;
            let skill_new = adapter.render_session_skill(skill_current.as_deref())?;
            if let Some(skill_new) = skill_new {
                let skill_key = format!("{}/skill", SyncState::state_key(adapter_id, scope));
                let skill_applied = state.applied.get(&skill_key).map(|r| r.hash.clone());
                let (skill_action, skill_diff) = eval_action(
                    &skill_path.to_string_lossy(),
                    &skill_current,
                    &skill_new,
                    skill_applied.as_ref(),
                );
                plans.push(FilePlan {
                    adapter: adapter_id.to_string(),
                    kind: PlanKind::Skill,
                    scope: scope.clone(),
                    path: skill_path.to_string_lossy().to_string(),
                    action: skill_action,
                    current_content: skill_current,
                    new_content: Some(skill_new),
                    diff: skill_diff,
                });
            }
        }
    }
    Ok(plans)
}
