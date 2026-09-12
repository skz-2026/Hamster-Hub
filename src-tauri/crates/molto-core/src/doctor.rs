//! Doctor 体检：漂移 / 重复 Skill / 可合并 MCP / 项目级明文密钥。
//!
//! 全部只读检查；报告按严重度排序，可导出 Markdown（传播钩子）。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use crate::atomic::read_text;
use crate::error::Result;
use crate::hash::sha256_hex;
use crate::model::agent::AgentInfo;
use crate::model::mcp::{Scope, Transport};
use crate::registry::Registry;
use crate::store::{now_ts, Store, SyncState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DoctorLevel {
    /// 高危：明文密钥可能被提交
    Danger,
    /// 警告：漂移 / 重复 Skill
    Warning,
    /// 建议：可合并的 MCP
    Info,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DoctorItem {
    pub level: DoctorLevel,
    pub title: String,
    pub detail: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DoctorReport {
    pub checked_at: String,
    pub items: Vec<DoctorItem>,
}

const SECRET_KEY_MARKS: [&str; 5] = ["KEY", "TOKEN", "SECRET", "PASSWORD", "AUTH"];

fn looks_like_secret(key: &str) -> bool {
    let upper = key.to_uppercase();
    SECRET_KEY_MARKS.iter().any(|m| upper.contains(m))
}

fn transport_tag(t: &Transport) -> String {
    serde_json::to_string(t).unwrap_or_default()
}

pub fn run(store: &Store, registry: &Registry) -> Result<DoctorReport> {
    let mut items: Vec<DoctorItem> = vec![];
    let config = store.load_config()?;
    let scanned = crate::scanner::scan(registry);
    let installed: Vec<&AgentInfo> = scanned.iter().filter(|a| a.installed).collect();

    // 1) 漂移：已分发目标文件与 state 基准不一致
    let state = store.load_state()?;
    for target in &config.targets {
        if !target.enabled {
            continue;
        }
        let Ok(adapter) = registry.get(&target.adapter) else {
            continue;
        };
        let Some(path) = adapter.mcp_path(&target.scope) else {
            continue;
        };
        let key = SyncState::state_key(&target.adapter, &target.scope);
        let Some(record) = state.applied.get(&key) else {
            continue;
        };
        if let Ok(Some(current)) = read_text(&path) {
            if sha256_hex(&current) != record.hash {
                items.push(DoctorItem {
                    level: DoctorLevel::Warning,
                    title: format!("{} 的配置文件被外部修改", adapter.name()),
                    detail: "在 Molto 之外被改动过，分发前需要先处理漂移".into(),
                    paths: vec![path.to_string_lossy().to_string()],
                });
            }
        }
    }

    // 2) 重复安装的 Skill
    let mut skill_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for adapter in registry.list() {
        let Some(dir) = adapter.skills_dir() else {
            continue;
        };
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()) {
            let name = entry.file_name().to_string_lossy().to_string();
            skill_map
                .entry(name)
                .or_default()
                .push(dir.to_string_lossy().to_string());
        }
    }
    for (name, dirs) in &skill_map {
        if dirs.len() > 1 {
            items.push(DoctorItem {
                level: DoctorLevel::Warning,
                title: format!("Skill「{}」重复安装了 {} 份", name, dirs.len()),
                detail: "每次更新需要改多处；M1 的 Skills 同步可将其合并为一份".into(),
                paths: dirs.clone(),
            });
        }
    }

    // 3) 项目级配置中的明文密钥（可能被提交进 git）
    for project in store.load_projects()? {
        let scope = Scope::Project {
            dir: project.clone(),
        };
        for adapter in registry.list() {
            if !adapter.capabilities().project_mcp {
                continue;
            }
            let Some(path) = adapter.mcp_path(&scope) else {
                continue;
            };
            let Ok(Some(raw)) = read_text(&path) else {
                continue;
            };
            let Ok(servers) = adapter.read_mcp(&raw) else {
                continue;
            };
            let mut found: Vec<String> = vec![];
            for server in servers {
                let keys: Vec<String> = match &server.transport {
                    Transport::Stdio { env, .. } => env.keys().cloned().collect(),
                    Transport::Http { headers, .. } => headers.keys().cloned().collect(),
                };
                for k in keys {
                    if looks_like_secret(&k) {
                        found.push(format!("{}（{}）", k, server.name));
                    }
                }
            }
            if !found.is_empty() {
                items.push(DoctorItem {
                    level: DoctorLevel::Danger,
                    title: format!("项目级 MCP 配置含疑似密钥：{}", found.join("、")),
                    detail: "项目目录下的配置可能被提交进 git，建议只保留在用户级配置".into(),
                    paths: vec![path.to_string_lossy().to_string()],
                });
            }
        }
    }

    // 4) 可合并 MCP：同名同参数的 Server 出现在多个 Agent 配置中
    let mut server_map: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    for info in &installed {
        let Ok(adapter) = registry.get(&info.id) else {
            continue;
        };
        let Some(path) = adapter.mcp_path(&Scope::User) else {
            continue;
        };
        let Ok(Some(raw)) = read_text(&path) else {
            continue;
        };
        let Ok(servers) = adapter.read_mcp(&raw) else {
            continue;
        };
        for server in servers {
            server_map.entry(server.name.clone()).or_default().push((
                info.name.clone(),
                transport_tag(&server.transport),
                path.to_string_lossy().to_string(),
            ));
        }
    }
    for (name, entries) in &server_map {
        let agents: Vec<&String> = entries.iter().map(|(a, _, _)| a).collect();
        let unique_agents = agents.iter().collect::<std::collections::BTreeSet<_>>();
        if unique_agents.len() > 1 {
            let mut paths: Vec<String> = entries.iter().map(|(_, _, p)| p.clone()).collect();
            paths.sort();
            paths.dedup();
            items.push(DoctorItem {
                level: DoctorLevel::Info,
                title: format!(
                    "MCP「{}」在 {} 个 Agent 中重复配置",
                    name,
                    unique_agents.len()
                ),
                detail: "可纳入 Molto 统一管理，一处修改处处生效".into(),
                paths,
            });
        }
    }

    items.sort_by_key(|a| a.level);
    Ok(DoctorReport {
        checked_at: now_ts(),
        items,
    })
}

/// 导出为 Markdown 报告（Doctor 页「导出报告」用）。
pub fn export_markdown(report: &DoctorReport) -> String {
    let mut md = String::new();
    md.push_str("# Molto 多 Agent 配置体检报告\n\n");
    md.push_str(&format!("- 检查时间：{}\n", report.checked_at));
    md.push_str(&format!("- 发现问题：{} 项\n\n", report.items.len()));
    if report.items.is_empty() {
        md.push_str("未发现问题，配置状态健康。\n");
        return md;
    }
    for (level, label) in [
        (DoctorLevel::Danger, "高危"),
        (DoctorLevel::Warning, "警告"),
        (DoctorLevel::Info, "建议"),
    ] {
        let group: Vec<&DoctorItem> = report.items.iter().filter(|i| i.level == level).collect();
        if group.is_empty() {
            continue;
        }
        md.push_str(&format!("## {}\n\n", label));
        for item in group {
            md.push_str(&format!("### {}\n\n{}\n\n", item.title, item.detail));
            for path in &item.paths {
                md.push_str(&format!("- `{}`\n", path));
            }
            if !item.paths.is_empty() {
                md.push('\n');
            }
        }
    }
    md
}

/// 保存导出报告。
pub fn export_report(report: &DoctorReport, target: &Path) -> Result<String> {
    let md = export_markdown(report);
    crate::atomic::write_text_atomic(target, &md)?;
    Ok(target.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AgentAdapter, Caps, CommandSpec};
    use crate::model::agent::InstallInfo;
    use crate::model::mcp::McpServer;
    use std::path::PathBuf;

    struct SkillAdapter {
        skills: Option<PathBuf>,
        mcp: Option<PathBuf>,
    }
    impl AgentAdapter for SkillAdapter {
        fn id(&self) -> &'static str {
            "skillx"
        }
        fn name(&self) -> &'static str {
            "SkillX"
        }
        fn detect(&self) -> Option<InstallInfo> {
            None
        }
        fn capabilities(&self) -> Caps {
            Caps::default()
        }
        fn mcp_path(&self, _scope: &Scope) -> Option<PathBuf> {
            self.mcp.clone()
        }
        fn read_mcp(&self, raw: &str) -> Result<Vec<McpServer>> {
            let v: serde_json::Value = serde_json::from_str(raw).unwrap();
            Ok(v.get("mcpServers")
                .and_then(|m| m.as_object())
                .map(|m| {
                    m.keys()
                        .map(|k| McpServer {
                            name: k.clone(),
                            transport: Transport::Http {
                                url: "http://x".into(),
                                headers: BTreeMap::new(),
                            },
                            enabled: true,
                            disabled_tools: Vec::new(),
                        })
                        .collect()
                })
                .unwrap_or_default())
        }
        fn render_mcp(&self, _current: Option<&str>, _servers: &[McpServer]) -> Result<String> {
            Ok("{}".into())
        }
        fn skills_dir(&self) -> Option<PathBuf> {
            self.skills.clone()
        }
        fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
            CommandSpec {
                program: "x".into(),
                args: vec![],
                working_dir: project_dir.to_path_buf(),
            }
        }
    }

    #[test]
    fn duplicate_skills_reported() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a/skills");
        let b = dir.path().join("b/skills");
        std::fs::create_dir_all(a.join("find-skills")).unwrap();
        std::fs::create_dir_all(b.join("find-skills")).unwrap();
        let store = Store::new(dir.path().join("molto"));
        let reg = Registry::new(vec![
            Box::new(SkillAdapter {
                skills: Some(a),
                mcp: None,
            }),
            Box::new(SkillAdapter {
                skills: Some(b),
                mcp: None,
            }),
        ]);
        let report = run(&store, &reg).unwrap();
        assert!(report
            .items
            .iter()
            .any(|i| i.title.contains("find-skills") && i.title.contains("2 份")));
    }

    #[test]
    fn secret_key_detection() {
        assert!(looks_like_secret("API_KEY"));
        assert!(looks_like_secret("x-token"));
        assert!(!looks_like_secret("HOME"));
    }

    #[test]
    fn export_markdown_groups_by_level() {
        let report = DoctorReport {
            checked_at: now_ts(),
            items: vec![DoctorItem {
                level: DoctorLevel::Danger,
                title: "T".into(),
                detail: "D".into(),
                paths: vec!["p".into()],
            }],
        };
        let md = export_markdown(&report);
        assert!(md.contains("## 高危"));
        assert!(md.contains("`p`"));
    }
}
