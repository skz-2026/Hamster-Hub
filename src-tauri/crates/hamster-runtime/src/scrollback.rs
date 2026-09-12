//! scrollback 快照存储（runtime-design.md §10.2）。
//!
//! 前端 xterm serialize addon 把终端缓冲序列化为转义序列后经 command 存入
//! `<home>/.上游/runtime/<key>.scrollback`——重开/续聊同一逻辑会话时回放，
//! 恢复「看到的上下文」。存储键 = `resume_key`（agent 侧稳定 sessionKey，
//! 跨重启可对齐）缺省回落 pty session_id；非法字符的键确定性 hex 编码。
//! 这是 上游 自有 UI 数据的落盘，不触碰 Agent 会话文件（红线①/⑥：原子写、
//! 本地优先）。

use std::path::{Path, PathBuf};

use hamster_core::error::{HamsterError, Result};

/// 单会话快照上限（§10.2：1MB/会话，超出静默跳过保存）。
pub const MAX_SNAPSHOT_BYTES: usize = 1024 * 1024;
/// 目录总量上限：超出按 mtime LRU 清理最旧文件。
const MAX_TOTAL_BYTES: u64 = 50 * 1024 * 1024;

/// 键白名单字符：uuid 串之外一律拒绝落文件名（防路径穿越）。
fn plain_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 80
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !key.starts_with('.')
}

/// 存储键：合法 id 原样使用；否则确定性 hex 编码（agent 侧 sessionKey 可能
/// 含任意字符，直接落文件名有穿越风险）。
fn storage_key(id: &str) -> String {
    if plain_key(id) {
        return id.to_string();
    }
    let mut hexed = String::with_capacity(id.len() * 2 + 2);
    hexed.push_str("h-");
    for byte in id.as_bytes() {
        hexed.push_str(&format!("{byte:02x}"));
    }
    hexed
}

pub struct ScrollbackStore {
    root: PathBuf,
    total_limit: u64,
}

impl ScrollbackStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            total_limit: MAX_TOTAL_BYTES,
        }
    }

    #[cfg(test)]
    fn with_total_limit(root: impl Into<PathBuf>, total_limit: u64) -> Self {
        Self {
            root: root.into(),
            total_limit,
        }
    }

    /// 快照存储键：resume_key（跨重启稳定的 agent sessionKey）优先，
    /// 无 resume 的会话（工作台终端、未落 rollout 的首聊）回落 pty session_id。
    pub fn storage_id(session_id: &str, resume_key: Option<&str>) -> String {
        storage_key(resume_key.unwrap_or(session_id))
    }

    fn key_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{key}.scrollback"))
    }

    /// 原子保存（temp + rename）。超过单会话上限时静默跳过（前端已按行数
    /// 截断，这里只做兜底）；保存后按总量 LRU 清理。
    pub fn save(&self, session_id: &str, resume_key: Option<&str>, data: &str) -> Result<()> {
        let key = Self::storage_id(session_id, resume_key);
        let path = self.key_path(&key);
        if data.len() > MAX_SNAPSHOT_BYTES {
            return Ok(());
        }
        std::fs::create_dir_all(&self.root).map_err(|e| HamsterError::io(self.root.clone(), e))?;
        let tmp = self.root.join(format!(".{key}.tmp"));
        std::fs::write(&tmp, data).map_err(|e| HamsterError::io(tmp.clone(), e))?;
        std::fs::rename(&tmp, &path).map_err(|e| HamsterError::io(path.clone(), e))?;
        self.evict_lru(&path);
        Ok(())
    }

    /// 读取快照；不存在或超上限（视为异常残留）返回 None。
    pub fn load(&self, session_id: &str, resume_key: Option<&str>) -> Result<Option<String>> {
        let path = self.key_path(&Self::storage_id(session_id, resume_key));
        match std::fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(HamsterError::io(path, e)),
            Ok(meta) => {
                if meta.len() > MAX_SNAPSHOT_BYTES as u64 {
                    return Ok(None);
                }
            }
        }
        std::fs::read_to_string(&path)
            .map(Some)
            .map_err(|e| HamsterError::io(path, e))
    }

    /// 总量超限时按 mtime 清理最旧快照（刚写入的不清；清理是尽力而为，
    /// 失败不影响保存结果）。
    fn evict_lru(&self, just_written: &Path) {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return;
        };
        let mut files: Vec<(PathBuf, std::time::SystemTime, u64)> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "scrollback"))
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                Some((e.path(), meta.modified().ok()?, meta.len()))
            })
            .collect();
        let total: u64 = files.iter().map(|(_, _, len)| len).sum();
        if total <= self.total_limit {
            return;
        }
        files.sort_by_key(|(_, modified, _)| *modified);
        let mut total = total;
        for (path, _, len) in files {
            if total <= self.total_limit {
                break;
            }
            if path == just_written {
                continue;
            }
            if std::fs::remove_file(&path).is_ok() {
                total = total.saturating_sub(len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_roundtrip_and_missing_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = ScrollbackStore::new(dir.path());
        assert_eq!(store.load("abc-123", None).unwrap(), None);
        store.save("abc-123", None, "\x1b[31mhello\x1b[0m").unwrap();
        assert_eq!(
            store.load("abc-123", None).unwrap().as_deref(),
            Some("\x1b[31mhello\x1b[0m")
        );
    }

    #[test]
    fn resume_key_wins_as_storage_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = ScrollbackStore::new(dir.path());
        // 以 resume_key 落盘：换一个新 pty session_id（resume 场景）仍能命中
        store.save("pty-uuid", Some("rollout-abc"), "buf").unwrap();
        assert_eq!(
            store
                .load("pty-uuid", Some("rollout-abc"))
                .unwrap()
                .as_deref(),
            Some("buf")
        );
        assert_eq!(
            store.load("rollout-abc", None).unwrap().as_deref(),
            Some("buf")
        );
    }

    #[test]
    fn exotic_keys_are_hex_encoded_deterministically() {
        assert_eq!(storage_key("abc-123"), "abc-123");
        let weird = "../evil key";
        let key = storage_key(weird);
        assert!(key.starts_with("h-"));
        assert_eq!(key, storage_key(weird)); // 确定性
        let dir = tempfile::tempdir().unwrap();
        let store = ScrollbackStore::new(dir.path());
        store.save("pty-uuid", Some(weird), "buf").unwrap();
        assert_eq!(
            store.load("pty-uuid", Some(weird)).unwrap().as_deref(),
            Some("buf")
        );
    }

    #[test]
    fn oversize_snapshot_skipped_silently() {
        let dir = tempfile::tempdir().unwrap();
        let store = ScrollbackStore::new(dir.path());
        let big = "x".repeat(MAX_SNAPSHOT_BYTES + 1);
        store.save("s1", None, &big).unwrap();
        assert_eq!(store.load("s1", None).unwrap(), None);
    }

    #[test]
    fn lru_evicts_oldest_when_total_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let store = ScrollbackStore::with_total_limit(dir.path(), 10);
        store.save("older", None, "123456").unwrap();
        // 保证 mtime 严格递增（同秒内 mtime 可能相同）
        std::thread::sleep(std::time::Duration::from_millis(20));
        store.save("newer", None, "123456").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        store.save("third", None, "123456").unwrap();
        // 每份 6B、上限 10B：第三份落盘时总量 18B，最旧优先清到上限以下
        // （older、newer 都被清，third 作为刚写入的保留）
        assert_eq!(store.load("older", None).unwrap(), None);
        assert_eq!(store.load("newer", None).unwrap(), None);
        assert!(store.load("third", None).unwrap().is_some());
    }

    #[test]
    fn just_written_never_evicted() {
        let dir = tempfile::tempdir().unwrap();
        let store = ScrollbackStore::with_total_limit(dir.path(), 5);
        store.save("only", None, "12345678").unwrap();
        assert!(store.load("only", None).unwrap().is_some());
    }
}
