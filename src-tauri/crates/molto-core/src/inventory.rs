//! Agent 配置清单聚合（v1.5 Agent 详情页数据源）。
//! 全部只读：用户级 skills/MCP/规则 + 各 workspace 的项目级 MCP/规则。

use serde::Serialize;
use ts_rs::TS;

use crate::adapter::AgentAdapter;
use crate::atomic::read_text;
use crate::error::Result;
use crate::model::rules::SkillInfo;
use std::path::Path;

use crate::status::{scope_status, ScopeStatus};
use crate::Registry;

/// 用户级清单。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UserInventory {
    pub skills: Vec<SkillInfo>,
    pub mcp: ScopeStatus,
    pub rules: Vec<RuleStatus>,
}

/// 规则文件状态（存在 + 内容前若干行，供预览）。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RuleStatus {
    pub name: String,
    pub path: String,
    pub exists: bool,
}

/// 项目级清单。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectInventory {
    pub dir: String,
    /// 项目级 MCP（.mcp.json 等），None = 无配置文件
    pub mcp: Option<ScopeStatus>,
    /// 项目级规则文件（AGENTS.md / CLAUDE.md），None = 无
    pub rule: Option<RuleStatus>,
}

/// 一个 Agent 的完整清单。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentInventory {
    pub adapter: String,
    pub name: String,
    pub user: UserInventory,
    pub workspaces: Vec<ProjectInventory>,
}

/// 聚合全部已装 Agent 的清单。
pub fn agent_inventory(registry: &Registry) -> Result<Vec<AgentInventory>> {
    // workspace 目录跨 Agent 去重共享
    let mut workspaces: Vec<String> = vec![];
    for adapter in registry.list() {
        let adapter: &dyn AgentAdapter = adapter.as_ref();
        if adapter.detect().is_none() {
            continue;
        }
        for ws in adapter.recent_workspaces() {
            let norm = ws.dir.replace('/', "\\").trim_end_matches('\\').to_string();
            if !workspaces.iter().any(|w| w.eq_ignore_ascii_case(&norm)) {
                workspaces.push(norm);
            }
        }
    }

    let mut out = vec![];
    for adapter in registry.list() {
        let adapter: &dyn AgentAdapter = adapter.as_ref();
        if adapter.detect().is_none() {
            continue;
        }
        let user = UserInventory {
            skills: list_skills(adapter.skills_dir()),
            mcp: scope_status(adapter, &crate::model::mcp::Scope::User)?,
            rules: adapter
                .rule_files()
                .into_iter()
                .map(|r| RuleStatus {
                    name: r.name,
                    exists: std::path::Path::new(&r.path).exists(),
                    path: r.path,
                })
                .collect(),
        };

        let mut wss = vec![];
        if adapter.capabilities().project_mcp || adapter.project_rule_name().is_some() {
            for dir in &workspaces {
                let scope = crate::model::mcp::Scope::Project { dir: dir.clone() };
                let mcp = if adapter.capabilities().project_mcp {
                    match adapter.mcp_path(&scope) {
                        Some(p) if p.exists() => Some(scope_status(adapter, &scope)?),
                        _ => None,
                    }
                } else {
                    None
                };
                let rule = adapter.project_rule_name().map(|name| {
                    let path = std::path::Path::new(dir).join(&name);
                    RuleStatus {
                        name,
                        exists: path.exists(),
                        path: path.to_string_lossy().to_string(),
                    }
                });
                if mcp.is_some() || rule.as_ref().is_some_and(|r| r.exists) {
                    wss.push(ProjectInventory {
                        dir: dir.clone(),
                        mcp,
                        rule,
                    });
                }
            }
        }

        out.push(AgentInventory {
            adapter: adapter.id().to_string(),
            name: adapter.name().to_string(),
            user,
            workspaces: wss,
        });
    }
    Ok(out)
}

/// skills 目录扫描：子目录名即 skill 名（SKILL.md 规范）。
pub fn list_skills(dir: Option<std::path::PathBuf>) -> Vec<SkillInfo> {
    let Some(dir) = dir else { return vec![] };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut out = vec![];
    for e in entries.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()) {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        out.push(SkillInfo {
            name: name.clone(),
            path: e.path().join("SKILL.md").to_string_lossy().to_string(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// 读取文本文件前 n 行（规则文件预览用）。
pub fn read_head(path: &str, lines: usize) -> Result<String> {
    let raw = read_text(Path::new(path))?.unwrap_or_default();
    Ok(raw.lines().take(lines).collect::<Vec<_>>().join("\n"))
}
