//! 各 Agent MCP 生效状态（v1.4）：引擎回读目标文件，展示用户级/项目级实际生效配置。
//! 只读；不产生任何写入。

use serde::Serialize;
use ts_rs::TS;

use crate::adapter::AgentAdapter;
use crate::atomic::read_text;
use crate::error::Result;
use crate::model::mcp::{McpServer, Scope};
use crate::model::workspace::WorkspaceRecord;
use crate::registry::Registry;

/// 单个作用域的生效状态。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScopeStatus {
    /// 该作用域配置文件路径（None = 该 Agent 不支持此作用域）
    pub path: Option<String>,
    /// 实际生效的 server（回读目标文件）
    pub servers: Vec<McpServer>,
}

/// 项目级条目：目录 + 该 Agent 在此项目的生效状态。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectStatus {
    pub dir: String,
    /// None = 该项目没有该 Agent 的项目级配置文件
    pub status: Option<ScopeStatus>,
}

/// 一个 Agent 的完整生效状态。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentMcpStatus {
    pub adapter: String,
    pub name: String,
    /// 用户级生效状态
    pub user: ScopeStatus,
    /// 项目级（仅 project_mcp 能力的 Agent；按 workspace 历史发现的项目逐个回读）
    pub projects: Vec<ProjectStatus>,
}

/// 聚合全部已装 Agent 的生效状态。
///
/// 项目目录来源：全部 Agent 的 recent_workspaces() 去重（跨 Agent 一起检查——
/// 用户在 Claude 用过的目录，也值得知道 Codex 在那里有没有项目级配置）。
pub fn agent_mcp_status(registry: &Registry) -> Result<Vec<AgentMcpStatus>> {
    let installed: Vec<&dyn AgentAdapter> = registry
        .list()
        .iter()
        .map(|a| a.as_ref())
        .filter(|a| a.detect().is_some())
        .collect();

    // 跨 Agent 去重的 workspace 目录（保留来源标注）
    let mut workspaces: Vec<WorkspaceRecord> = vec![];
    for adapter in &installed {
        for ws in adapter.recent_workspaces() {
            if !workspaces.iter().any(|w| same_dir(&w.dir, &ws.dir)) {
                workspaces.push(ws);
            }
        }
    }
    let workspaces = workspaces;

    let mut out = vec![];
    for adapter in &installed {
        let adapter: &dyn AgentAdapter = *adapter;
        // 用户级：读目标文件 → IRM
        let user = scope_status(adapter, &Scope::User)?;

        // 项目级：仅能力声明如实支持
        let mut projects = vec![];
        if adapter.capabilities().project_mcp {
            for ws in &workspaces {
                let scope = Scope::Project {
                    dir: normalize_dir(&ws.dir),
                };
                let status = scope_status(adapter, &scope)?;
                let exists = status
                    .path
                    .as_deref()
                    .map(|p| std::path::Path::new(p).exists())
                    .unwrap_or(false);
                if exists {
                    projects.push(ProjectStatus {
                        dir: ws.dir.clone(),
                        status: Some(status),
                    });
                }
            }
        }

        out.push(AgentMcpStatus {
            adapter: adapter.id().to_string(),
            name: adapter.name().to_string(),
            user,
            projects,
        });
    }
    Ok(out)
}

pub fn scope_status(adapter: &dyn AgentAdapter, scope: &Scope) -> Result<ScopeStatus> {
    match adapter.mcp_path(scope) {
        Some(path) => {
            let servers = match read_text(&path)? {
                Some(raw) => adapter.read_mcp(&raw)?,
                None => vec![],
            };
            Ok(ScopeStatus {
                path: Some(path.to_string_lossy().to_string()),
                servers,
            })
        }
        None => Ok(ScopeStatus {
            path: None,
            servers: vec![],
        }),
    }
}

/// 路径归一比较：统一分隔符 + 大小写（Windows 不敏感）后比对。
fn same_dir(a: &str, b: &str) -> bool {
    normalize_dir(a).eq_ignore_ascii_case(&normalize_dir(b))
}

fn normalize_dir(dir: &str) -> String {
    dir.replace('/', "\\").trim_end_matches('\\').to_string()
}
