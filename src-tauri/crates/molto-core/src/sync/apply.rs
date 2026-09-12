//! apply：唯一写入路径。固定顺序：① 备份快照 → ② 原子写 → ③ 记 history.jsonl。
//! Drift 处理三选一：覆盖 / 采纳为源 / 跳过；无决策直接拒绝。

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::backup::{BackupManager, BackupMeta};
use crate::error::{MoltoError, Result};
use crate::hash::sha256_hex;
use crate::model::mcp::Scope;
use crate::registry::Registry;
use crate::store::{now_ts, HistoryEntry, Store, SyncState};
use crate::sync::plan::{FilePlan, PlanKind, TargetAction};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DriftDecision {
    /// 以源为准覆盖外部修改
    Overwrite,
    /// 采纳外部修改为新源（读回目标文件并更新源配置）
    AdoptExternal,
    /// 本次跳过该文件
    Skip,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppliedFile {
    pub adapter: String,
    pub path: String,
    /// write | adopt | skip
    pub action: String,
    pub backup: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApplyReport {
    pub applied: Vec<AppliedFile>,
    pub skipped: Vec<AppliedFile>,
    pub message: String,
}

pub fn apply(
    store: &Store,
    backup: &BackupManager,
    registry: &Registry,
    plans: &[FilePlan],
    decisions: &BTreeMap<String, DriftDecision>,
) -> Result<ApplyReport> {
    let mut applied = vec![];
    let mut skipped = vec![];

    for plan in plans {
        match plan.action {
            TargetAction::NoChange => continue,
            TargetAction::Create | TargetAction::Update => {
                applied.push(write_with_backup(store, backup, plan)?);
            }
            TargetAction::Drift => {
                let decision = decisions.get(&plan.path).ok_or_else(|| MoltoError::Drift {
                    path: Path::new(&plan.path).to_path_buf(),
                })?;
                match decision {
                    DriftDecision::Overwrite => {
                        applied.push(write_with_backup(store, backup, plan)?);
                    }
                    DriftDecision::AdoptExternal => {
                        applied.push(adopt_external(store, registry, backup, plan)?);
                    }
                    DriftDecision::Skip => skipped.push(AppliedFile {
                        adapter: plan.adapter.clone(),
                        path: plan.path.clone(),
                        action: "skip".into(),
                        backup: None,
                    }),
                }
            }
        }
    }

    let message = format!(
        "已写入 {} 个文件，采纳 {} 个，跳过 {} 个",
        applied.iter().filter(|f| f.action != "adopt").count(),
        applied.iter().filter(|f| f.action == "adopt").count(),
        skipped.len(),
    );
    Ok(ApplyReport {
        applied,
        skipped,
        message,
    })
}

/// ① 快照 → ② 原子写 → ③ 记历史 + 更新 state hash。
fn write_with_backup(
    store: &Store,
    backup: &BackupManager,
    plan: &FilePlan,
) -> Result<AppliedFile> {
    let new_content = plan
        .new_content
        .as_deref()
        .ok_or_else(|| MoltoError::Other(format!("计划缺少新内容：{}", plan.path)))?;
    let path = Path::new(&plan.path);
    let meta = backup.snapshot(&plan.adapter, &plan.scope, path)?;
    crate::atomic::write_text_atomic(path, new_content)?;
    let hash = sha256_hex(new_content);
    store.record_applied_with_key(&plan.state_key(), path, hash.clone())?;
    store.append_history(&HistoryEntry {
        ts: now_ts(),
        kind: "apply".into(),
        adapter: plan.adapter.clone(),
        scope: plan.scope.key(),
        path: plan.path.clone(),
        action: match plan.action {
            TargetAction::Create => "create".into(),
            _ => "update".into(),
        },
        backup: meta.as_ref().map(|m| m.backup_path.clone()),
        hash: Some(hash),
    })?;
    Ok(AppliedFile {
        adapter: plan.adapter.clone(),
        path: plan.path.clone(),
        action: "write".into(),
        backup: meta.map(|m| m.backup_path),
    })
}

/// 「采纳为源」：外部修改进入源配置（按 Server 名 upsert），
/// 并把外部状态登记为已分发基线，不写目标文件。
fn adopt_external(
    store: &Store,
    registry: &Registry,
    backup: &BackupManager,
    plan: &FilePlan,
) -> Result<AppliedFile> {
    let current = plan
        .current_content
        .as_deref()
        .ok_or_else(|| MoltoError::Other(format!("漂移文件不存在：{}", plan.path)))?;
    let adapter = registry.get(&plan.adapter)?;

    // 安全网：采纳前也快照一次
    let meta = backup.snapshot(&plan.adapter, &plan.scope, Path::new(&plan.path))?;

    if plan.kind == PlanKind::Mcp {
        // 工具配置文件没有源语义，只有 MCP 文件才导入源
        let external = adapter.read_mcp(current)?;
        let mut config = store.load_config()?;
        for server in external {
            config.upsert_server(server);
        }
        store.save_config(&config)?;
    }

    let hash = sha256_hex(current);
    store.record_applied_with_key(&plan.state_key(), Path::new(&plan.path), hash.clone())?;
    store.append_history(&HistoryEntry {
        ts: now_ts(),
        kind: "adopt".into(),
        adapter: plan.adapter.clone(),
        scope: plan.scope.key(),
        path: plan.path.clone(),
        action: "adopt".into(),
        backup: meta.as_ref().map(|m| m.backup_path.clone()),
        hash: Some(hash),
    })?;
    Ok(AppliedFile {
        adapter: plan.adapter.clone(),
        path: plan.path.clone(),
        action: "adopt".into(),
        backup: meta.map(|m| m.backup_path),
    })
}

/// 回滚：恢复快照内容，并把恢复后的内容登记为已分发基线。
pub fn rollback(store: &Store, backup: &BackupManager, meta: &BackupMeta) -> Result<()> {
    let content = backup.read_snapshot(&meta.backup_path)?;
    let scope = if meta.scope.starts_with("project:") {
        Scope::Project {
            dir: meta.scope.trim_start_matches("project:").to_string(),
        }
    } else {
        Scope::User
    };
    let safety = backup.restore(meta)?;

    let hash = sha256_hex(&content);
    store.record_applied(
        &meta.adapter,
        &scope,
        Path::new(&meta.original_path),
        hash.clone(),
    )?;
    store.append_history(&HistoryEntry {
        ts: now_ts(),
        kind: "rollback".into(),
        adapter: meta.adapter.clone(),
        scope: scope.key(),
        path: meta.original_path.clone(),
        action: "restore".into(),
        backup: safety.map(|m| m.backup_path),
        hash: Some(hash),
    })?;
    Ok(())
}

/// 供引擎测试与 doctor 使用的状态查询辅助。
pub fn applied_hash(store: &Store, adapter: &str, scope: &Scope) -> Result<Option<String>> {
    let state = store.load_state()?;
    Ok(state
        .applied
        .get(&SyncState::state_key(adapter, scope))
        .map(|r| r.hash.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AgentAdapter, Caps, CommandSpec};
    use crate::model::agent::InstallInfo;
    use crate::model::mcp::{McpServer, Transport};
    use crate::store::StoreConfig;
    use crate::sync::SyncEngine;
    use std::path::{Path as StdPath, PathBuf};

    /// JSON 型假 Adapter：把 servers 写成固定结构，便于引擎测试。
    struct JsonAdapter {
        root: PathBuf,
    }
    impl AgentAdapter for JsonAdapter {
        fn id(&self) -> &'static str {
            "json"
        }
        fn name(&self) -> &'static str {
            "Json"
        }
        fn detect(&self) -> Option<InstallInfo> {
            None
        }
        fn capabilities(&self) -> Caps {
            Caps {
                mcp: true,
                project_mcp: false,
                rules: false,
                skills: false,
                tool_granularity: false,
            }
        }
        fn mcp_path(&self, _scope: &Scope) -> Option<PathBuf> {
            Some(self.root.join("cfg.json"))
        }
        fn read_mcp(&self, raw: &str) -> Result<Vec<McpServer>> {
            let v: serde_json::Value = serde_json::from_str(raw)
                .map_err(|e| MoltoError::json_parse(self.root.join("cfg.json"), e.to_string()))?;
            let mut out = vec![];
            if let Some(obj) = v.get("mcpServers").and_then(|m| m.as_object()) {
                for (name, sv) in obj {
                    out.push(McpServer {
                        name: name.clone(),
                        transport: Transport::Stdio {
                            command: sv
                                .get("command")
                                .and_then(|c| c.as_str())
                                .unwrap_or_default()
                                .into(),
                            args: vec![],
                            env: BTreeMap::new(),
                        },
                        enabled: true,
                        disabled_tools: Vec::new(),
                    });
                }
            }
            Ok(out)
        }
        fn render_mcp(&self, current: Option<&str>, servers: &[McpServer]) -> Result<String> {
            let mut root: serde_json::Value = match current {
                Some(c) if !c.trim().is_empty() => serde_json::from_str(c).map_err(|e| {
                    MoltoError::json_parse(self.root.join("cfg.json"), e.to_string())
                })?,
                _ => serde_json::json!({ "keepMe": true }),
            };
            let mut map = serde_json::Map::new();
            for s in servers {
                if let Transport::Stdio { command, .. } = &s.transport {
                    map.insert(s.name.clone(), serde_json::json!({ "command": command }));
                }
            }
            root["mcpServers"] = serde_json::Value::Object(map);
            Ok(serde_json::to_string_pretty(&root).unwrap())
        }
        fn launch_cmd(&self, project_dir: &StdPath) -> CommandSpec {
            CommandSpec {
                program: "x".into(),
                args: vec![],
                working_dir: project_dir.to_path_buf(),
            }
        }
    }

    fn fixture() -> (tempfile::TempDir, Store, BackupManager, Registry) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("molto"));
        let backup = BackupManager::new(dir.path().join("molto").join("backups"));
        let registry = Registry::new(vec![Box::new(JsonAdapter {
            root: dir.path().to_path_buf(),
        })]);
        (dir, store, backup, registry)
    }

    fn server(name: &str, command: &str) -> McpServer {
        McpServer {
            name: name.into(),
            transport: Transport::Stdio {
                command: command.into(),
                args: vec![],
                env: BTreeMap::new(),
            },
            enabled: true,
            disabled_tools: Vec::new(),
        }
    }

    #[test]
    fn full_cycle_create_then_nochange_then_update() {
        let (_d, store, backup, registry) = fixture();
        let mut cfg = StoreConfig::default();
        cfg.servers.push(server("a", "cmd-a"));
        store.save_config(&cfg).unwrap();

        let engine = SyncEngine::new(&store, &backup, &registry);
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        assert_eq!(plans[0].action, TargetAction::Create);

        let report = engine.apply(&plans, &BTreeMap::new()).unwrap();
        assert_eq!(report.applied.len(), 1);
        assert_eq!(report.applied[0].action, "write");

        // 再次 plan：无变化
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        assert_eq!(plans[0].action, TargetAction::NoChange);

        // 改源后：Update
        let mut cfg = store.load_config().unwrap();
        cfg.servers[0] = server("a", "cmd-a2");
        store.save_config(&cfg).unwrap();
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        assert_eq!(plans[0].action, TargetAction::Update);
        assert!(plans[0].diff.as_deref().unwrap().contains("cmd-a2"));
    }

    #[test]
    fn drift_requires_decision_and_skip_writes_nothing() {
        let (_d, store, backup, registry) = fixture();
        let mut cfg = StoreConfig::default();
        cfg.servers.push(server("a", "cmd-a"));
        store.save_config(&cfg).unwrap();

        let engine = SyncEngine::new(&store, &backup, &registry);
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        engine.apply(&plans, &BTreeMap::new()).unwrap();

        // 外部修改目标文件 → 漂移
        let target = registry
            .get("json")
            .unwrap()
            .mcp_path(&Scope::User)
            .unwrap();
        std::fs::write(
            &target,
            "{\"keepMe\":false,\"mcpServers\":{\"ext\":{\"command\":\"ext\"}}}",
        )
        .unwrap();
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        assert_eq!(plans[0].action, TargetAction::Drift);

        // 无决策 → 拒绝
        let err = engine.apply(&plans, &BTreeMap::new()).unwrap_err();
        assert!(matches!(err, MoltoError::Drift { .. }));

        // Skip → 不写
        let mut decisions = BTreeMap::new();
        decisions.insert(plans[0].path.clone(), DriftDecision::Skip);
        let report = engine.apply(&plans, &decisions).unwrap();
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "{\"keepMe\":false,\"mcpServers\":{\"ext\":{\"command\":\"ext\"}}}"
        );
    }

    #[test]
    fn adopt_external_becomes_source() {
        let (_d, store, backup, registry) = fixture();
        let mut cfg = StoreConfig::default();
        cfg.servers.push(server("a", "cmd-a"));
        store.save_config(&cfg).unwrap();

        let engine = SyncEngine::new(&store, &backup, &registry);
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        engine.apply(&plans, &BTreeMap::new()).unwrap();

        // 外部加了 ext、改了 a
        let target = registry
            .get("json")
            .unwrap()
            .mcp_path(&Scope::User)
            .unwrap();
        std::fs::write(
            &target,
            "{\"keepMe\":true,\"mcpServers\":{\"a\":{\"command\":\"cmd-ext\"},\"ext\":{\"command\":\"ext-cmd\"}}}",
        )
        .unwrap();

        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        assert_eq!(plans[0].action, TargetAction::Drift);
        let mut decisions = BTreeMap::new();
        decisions.insert(plans[0].path.clone(), DriftDecision::AdoptExternal);
        engine.apply(&plans, &decisions).unwrap();

        let cfg = store.load_config().unwrap();
        let names: Vec<&str> = cfg.servers.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"ext"));
        let a = cfg.servers.iter().find(|s| s.name == "a").unwrap();
        match &a.transport {
            Transport::Stdio { command, .. } => assert_eq!(command, "cmd-ext"),
            _ => panic!("transport mismatch"),
        }
        // 采纳后不再漂移（外部格式差异会以 Update 呈现，属正常）
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        assert_ne!(plans[0].action, TargetAction::Drift);
    }

    #[test]
    fn rollback_restores_and_rebases_state() {
        let (_d, store, backup, registry) = fixture();
        let mut cfg = StoreConfig::default();
        cfg.servers.push(server("a", "v1"));
        store.save_config(&cfg).unwrap();

        let engine = SyncEngine::new(&store, &backup, &registry);
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        engine.apply(&plans, &BTreeMap::new()).unwrap();
        let target = registry
            .get("json")
            .unwrap()
            .mcp_path(&Scope::User)
            .unwrap();
        assert!(std::fs::read_to_string(&target).unwrap().contains("v1"));

        cfg.servers[0] = server("a", "v2");
        store.save_config(&cfg).unwrap();
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        engine.apply(&plans, &BTreeMap::new()).unwrap();
        assert!(std::fs::read_to_string(&target).unwrap().contains("v2"));

        // 回滚到 v1 的快照
        let backups = backup.list().unwrap();
        let v1 = backups
            .iter()
            .find(|m| backup.read_snapshot(&m.backup_path).unwrap().contains("v1"))
            .unwrap()
            .clone();
        rollback(&store, &backup, &v1).unwrap();
        let restored = std::fs::read_to_string(&target).unwrap();
        assert!(restored.contains("v1"));
        // 回滚后不漂移（state 已重置为恢复内容；源仍为 v2 → 差异以 Update 呈现）
        let plans = engine.plan(&[("json".into(), Scope::User)]).unwrap();
        assert_ne!(plans[0].action, TargetAction::Drift);
    }
}
