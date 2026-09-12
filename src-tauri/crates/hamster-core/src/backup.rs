//! 备份与回滚：写入前快照到 ~/.hamster/backups/<agent>/<timestamp>/，默认每 Agent 保留 20 份。
//!
//! 红线：任何对外部配置的写入之前，必须先经过这里。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::atomic::{read_text, write_text_atomic};
use crate::error::{HamsterError, Result};
use crate::model::mcp::Scope;
use crate::store::now_ts;

/// 每个默认保留的快照份数。
pub const RETAIN_PER_AGENT: usize = 20;

/// 快照元数据（frontend 展示 + 回滚定位）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupMeta {
    /// 快照目录名（时间戳），全局唯一
    pub id: String,
    pub adapter: String,
    pub scope: String,
    pub original_path: String,
    pub backup_path: String,
    pub created_at: String,
}

pub struct BackupManager {
    root: PathBuf,
}

impl BackupManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn agent_dir(&self, adapter: &str) -> PathBuf {
        self.root.join(adapter)
    }

    /// 快照 id：时间戳（微秒）+ 进程内单调计数，保证同毫秒内不碰撞且按名排序即时序。
    fn next_snapshot_id() -> String {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!(
            "{}-{:04}",
            chrono::Local::now().format("%Y%m%d-%H%M%S%.6f"),
            n
        )
    }

    /// 快照当前文件。原文件不存在（首次创建）返回 Ok(None)。
    pub fn snapshot(
        &self,
        adapter: &str,
        scope: &Scope,
        original_path: &Path,
    ) -> Result<Option<BackupMeta>> {
        let content = match read_text(original_path)? {
            Some(c) => c,
            None => return Ok(None),
        };
        let id = Self::next_snapshot_id();
        let dir = self.agent_dir(adapter).join(&id);
        fs::create_dir_all(&dir).map_err(|e| HamsterError::io(&dir, e))?;

        let file_name = original_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "config".to_string());
        let content_path = dir.join(&file_name);
        fs::write(&content_path, content.as_bytes())
            .map_err(|e| HamsterError::io(&content_path, e))?;

        let meta = BackupMeta {
            id,
            adapter: adapter.to_string(),
            scope: scope.key(),
            original_path: original_path.to_string_lossy().to_string(),
            backup_path: content_path.to_string_lossy().to_string(),
            created_at: now_ts(),
        };
        let meta_path = dir.join("meta.json");
        let raw = serde_json::to_string_pretty(&meta)
            .map_err(|e| HamsterError::Other(format!("序列化快照元数据失败：{}", e)))?;
        fs::write(&meta_path, format!("{}\n", raw)).map_err(|e| HamsterError::io(&meta_path, e))?;

        self.prune(adapter)?;
        Ok(Some(meta))
    }

    /// 超出保留数时删最旧。
    fn prune(&self, adapter: &str) -> Result<()> {
        let dir = self.agent_dir(adapter);
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(HamsterError::io(&dir, e)),
        };
        let mut ids: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        ids.sort();
        while ids.len() > RETAIN_PER_AGENT {
            let oldest = ids.remove(0);
            let path = dir.join(oldest);
            if let Err(e) = fs::remove_dir_all(&path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return Err(HamsterError::io(&path, e));
                }
            }
        }
        Ok(())
    }

    /// 列出全部快照（按创建时间倒序）。
    pub fn list(&self) -> Result<Vec<BackupMeta>> {
        let mut out = vec![];
        let agents = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(HamsterError::io(&self.root, e)),
        };
        for agent in agents.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()) {
            let snapshots = match fs::read_dir(agent.path()) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for snap in snapshots
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
            {
                let meta_path = snap.path().join("meta.json");
                if let Ok(Some(raw)) = read_text(&meta_path) {
                    if let Ok(meta) = serde_json::from_str::<BackupMeta>(&raw) {
                        out.push(meta);
                    }
                }
            }
        }
        // 快照 id（微秒+计数）是严格时序；created_at 秒级精度在同秒内不稳定
        out.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(out)
    }

    pub fn read_snapshot(&self, backup_path: &str) -> Result<String> {
        read_text(Path::new(backup_path))?
            .ok_or_else(|| HamsterError::not_found(format!("快照文件 {}", backup_path)))
    }

    /// 回滚：先把当前状态再快照一次（安全网），再原子恢复。
    /// 返回安全网快照（可能为 None：目标文件当前不存在）。
    pub fn restore(&self, meta: &BackupMeta) -> Result<Option<BackupMeta>> {
        let content = self.read_snapshot(&meta.backup_path)?;
        let original = Path::new(&meta.original_path);
        let scope = Scope::User; // scope 仅用于安全网快照目录归类，此处不参与路径推导
        let safety = self.snapshot(&meta.adapter, &scope, original)?;
        write_text_atomic(original, &content)?;
        Ok(safety)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, tempfile::TempDir, BackupManager, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let original = target.path().join("config.json");
        let mgr = BackupManager::new(root.path());
        (root, target, mgr, original)
    }

    #[test]
    fn snapshot_and_restore_roundtrip() {
        let (_root, _target, mgr, original) = setup();
        std::fs::write(&original, "v1").unwrap();
        let meta = mgr
            .snapshot("claude", &Scope::User, &original)
            .unwrap()
            .unwrap();

        std::fs::write(&original, "v2").unwrap();
        let safety = mgr.restore(&meta).unwrap();
        // 恢复成功 + 安全网记录了 v2
        assert_eq!(std::fs::read_to_string(&original).unwrap(), "v1");
        let safety = safety.unwrap();
        assert_eq!(mgr.read_snapshot(&safety.backup_path).unwrap(), "v2");
    }

    #[test]
    fn snapshot_missing_file_is_none() {
        let (_root, _target, mgr, original) = setup();
        assert!(mgr
            .snapshot("codex", &Scope::User, &original)
            .unwrap()
            .is_none());
    }

    #[test]
    fn prune_keeps_latest_20() {
        let (_root, _target, mgr, original) = setup();
        std::fs::write(&original, "content").unwrap();
        for _ in 0..25 {
            mgr.snapshot("gemini", &Scope::User, &original)
                .unwrap()
                .unwrap();
            // 文件名时间戳精度 3ms，强制推进避免同 id
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let list = mgr
            .list()
            .unwrap()
            .into_iter()
            .filter(|m| m.adapter == "gemini")
            .count();
        assert_eq!(list, RETAIN_PER_AGENT);
    }

    #[test]
    fn list_empty_root_ok() {
        let (_root, _target, mgr, _) = setup();
        assert!(mgr.list().unwrap().is_empty());
    }
}
