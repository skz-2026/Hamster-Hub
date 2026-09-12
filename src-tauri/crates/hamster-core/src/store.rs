//! ~/.上游 存储：config.json（单一事实源）、state.json（分发 hash 基准）、
//! history.jsonl（append-only 变更史）、projects.json（项目登记）。
//!
//! 自身配置保持纯文本可审计；写入一律原子化。

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::atomic::{read_text, write_text_atomic};
use crate::error::{HamsterError, Result};
use crate::model::mcp::{McpServer, Scope};

pub const CONFIG_FILE: &str = "config.json";
pub const STATE_FILE: &str = "state.json";
pub const HISTORY_FILE: &str = "history.jsonl";
pub const PROJECTS_FILE: &str = "projects.json";
pub const BACKUPS_DIR: &str = "backups";

/// 单一事实源。frontend 通过 get/save_mcp_source 读写。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct StoreConfig {
    pub version: u32,
    pub servers: Vec<McpServer>,
    pub targets: Vec<SyncTarget>,
}

/// 分发目标：把源分发到哪个 Agent 的哪个作用域。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncTarget {
    pub adapter: String,
    pub scope: Scope,
    pub enabled: bool,
}

impl StoreConfig {
    pub fn ensure_targets(&mut self, adapter_ids: &[&str]) {
        for id in adapter_ids {
            let key = (id.to_string(), Scope::User);
            if !self
                .targets
                .iter()
                .any(|t| (t.adapter.clone(), t.scope.clone()) == key)
            {
                self.targets.push(SyncTarget {
                    adapter: id.to_string(),
                    scope: Scope::User,
                    enabled: true,
                });
            }
        }
    }

    pub fn upsert_server(&mut self, server: McpServer) {
        match self.servers.iter_mut().find(|s| s.name == server.name) {
            Some(existing) => *existing = server,
            None => self.servers.push(server),
        }
    }
}

/// 上次分发的 hash 基准（漂移检测依据）。键："<adapter>/<scope_key>"。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncState {
    pub applied: BTreeMap<String, AppliedRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedRecord {
    pub path: String,
    pub hash: String,
    pub at: String,
}

impl SyncState {
    pub fn state_key(adapter: &str, scope: &Scope) -> String {
        format!("{}/{}", adapter, scope.key())
    }
}

/// 变更史条目（history.jsonl 一行一条）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub ts: String,
    /// apply | adopt | rollback
    pub kind: String,
    pub adapter: String,
    pub scope: String,
    pub path: String,
    /// create | update | adopt | restore
    pub action: String,
    pub backup: Option<String>,
    pub hash: Option<String>,
}

pub fn now_ts() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 默认根：HAMSTER_HOME 环境变量覆盖 home 目录（沙箱/测试用），否则 ~/.上游。
    pub fn default_root() -> PathBuf {
        Self::default_home().join(".上游")
    }

    /// 用户 home（HAMSTER_HOME 覆盖）：adapters 以 <home>/.claude 等探测，
    /// CLI 用它组装 registry（与 default_root 同一来源，避免两套逻辑）。
    pub fn default_home() -> PathBuf {
        match std::env::var("HAMSTER_HOME") {
            Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
            _ => std::env::home_dir().unwrap_or_else(|| PathBuf::from(".")),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join(CONFIG_FILE)
    }

    pub fn backups_root(&self) -> PathBuf {
        self.root.join(BACKUPS_DIR)
    }

    /// 首次使用检测（Onboarding 判断依据）。
    pub fn has_config(&self) -> bool {
        self.config_path().exists()
    }

    pub fn load_config(&self) -> Result<StoreConfig> {
        match read_text(&self.config_path())? {
            None => Ok(StoreConfig::default()),
            Some(raw) => serde_json::from_str(&raw)
                .map_err(|e| HamsterError::json_parse(self.config_path(), e.to_string())),
        }
    }

    pub fn save_config(&self, config: &StoreConfig) -> Result<()> {
        let raw = serde_json::to_string_pretty(config)
            .map_err(|e| HamsterError::Other(format!("序列化源配置失败：{}", e)))?;
        write_text_atomic(&self.config_path(), &format!("{}\n", raw))
    }

    pub fn load_state(&self) -> Result<SyncState> {
        match read_text(&self.root.join(STATE_FILE))? {
            None => Ok(SyncState::default()),
            Some(raw) => serde_json::from_str(&raw)
                .map_err(|e| HamsterError::json_parse(self.root.join(STATE_FILE), e.to_string())),
        }
    }

    pub fn record_applied(
        &self,
        adapter: &str,
        scope: &Scope,
        path: &Path,
        hash: String,
    ) -> Result<()> {
        self.record_applied_with_key(&SyncState::state_key(adapter, scope), path, hash)
    }

    /// 指定键记账：工具颗粒度文件与 MCP 文件分开（`<base>/tools`）。
    pub fn record_applied_with_key(&self, key: &str, path: &Path, hash: String) -> Result<()> {
        let mut state = self.load_state()?;
        state.applied.insert(
            key.to_string(),
            AppliedRecord {
                path: path.to_string_lossy().to_string(),
                hash,
                at: now_ts(),
            },
        );
        let raw = serde_json::to_string_pretty(&state)
            .map_err(|e| HamsterError::Other(format!("序列化状态失败：{}", e)))?;
        write_text_atomic(&self.root.join(STATE_FILE), &format!("{}\n", raw))
    }

    pub fn append_history(&self, entry: &HistoryEntry) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|e| HamsterError::io(&self.root, e))?;
        let line = serde_json::to_string(entry)
            .map_err(|e| HamsterError::Other(format!("序列化历史失败：{}", e)))?;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join(HISTORY_FILE))
            .map_err(|e| HamsterError::io(self.root.join(HISTORY_FILE), e))?;
        writeln!(f, "{}", line).map_err(|e| HamsterError::io(self.root.join(HISTORY_FILE), e))?;
        Ok(())
    }

    /// 项目登记（启动器用）。去重 + 排序。
    pub fn load_projects(&self) -> Result<Vec<String>> {
        match read_text(&self.root.join(PROJECTS_FILE))? {
            None => Ok(vec![]),
            Some(raw) => serde_json::from_str(&raw).map_err(|e| {
                HamsterError::json_parse(self.root.join(PROJECTS_FILE), e.to_string())
            }),
        }
    }

    pub fn add_project(&self, dir: &str) -> Result<Vec<String>> {
        let mut projects = self.load_projects()?;
        if !projects.iter().any(|p| p == dir) {
            projects.push(dir.to_string());
            projects.sort();
            let raw = serde_json::to_string_pretty(&projects)
                .map_err(|e| HamsterError::Other(format!("序列化项目失败：{}", e)))?;
            write_text_atomic(&self.root.join(PROJECTS_FILE), &format!("{}\n", raw))?;
        }
        Ok(projects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        (dir, store)
    }

    #[test]
    fn config_roundtrip_and_default() {
        let (_d, store) = temp_store();
        assert!(!store.has_config());
        let cfg = store.load_config().unwrap();
        assert!(cfg.servers.is_empty());

        let mut cfg = StoreConfig {
            version: 1,
            ..Default::default()
        };
        cfg.servers.push(McpServer {
            name: "fs".into(),
            transport: crate::model::mcp::Transport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into(), "@modelcontextprotocol/server-fs".into()],
                env: BTreeMap::new(),
            },
            enabled: true,
            disabled_tools: Vec::new(),
        });
        store.save_config(&cfg).unwrap();
        assert!(store.has_config());
        let loaded = store.load_config().unwrap();
        assert_eq!(loaded.servers.len(), 1);
        assert_eq!(loaded.servers[0].name, "fs");
    }

    #[test]
    fn state_record_and_history_append() {
        let (_d, store) = temp_store();
        let scope = Scope::User;
        store
            .record_applied("claude", &scope, Path::new("/x/.claude.json"), "abc".into())
            .unwrap();
        let state = store.load_state().unwrap();
        assert_eq!(
            state.applied[&SyncState::state_key("claude", &scope)].hash,
            "abc"
        );

        store
            .append_history(&HistoryEntry {
                ts: now_ts(),
                kind: "apply".into(),
                adapter: "claude".into(),
                scope: "user".into(),
                path: "/x/.claude.json".into(),
                action: "update".into(),
                backup: None,
                hash: Some("abc".into()),
            })
            .unwrap();
        store
            .append_history(&HistoryEntry {
                ts: now_ts(),
                kind: "apply".into(),
                adapter: "codex".into(),
                scope: "user".into(),
                path: "/x/config.toml".into(),
                action: "create".into(),
                backup: None,
                hash: Some("def".into()),
            })
            .unwrap();
        let raw = fs::read_to_string(store.root().join(HISTORY_FILE)).unwrap();
        assert_eq!(raw.lines().count(), 2);
    }

    #[test]
    fn ensure_targets_idempotent() {
        let mut cfg = StoreConfig::default();
        cfg.ensure_targets(&["claude", "codex"]);
        cfg.ensure_targets(&["claude"]);
        assert_eq!(cfg.targets.len(), 2);
    }

    #[test]
    fn projects_dedup_sorted() {
        let (_d, store) = temp_store();
        store.add_project("D:\\work\\b").unwrap();
        let _p = store.add_project("D:\\work\\a").unwrap();
        let p = store.add_project("D:\\work\\b").unwrap();
        assert_eq!(p, vec!["D:\\work\\a", "D:\\work\\b"]);
    }
}
