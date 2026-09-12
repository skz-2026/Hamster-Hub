//! 系统状态快照：桌面模式进入前落盘，正常退出删除；看门狗凭它判断「未还原」并兜底恢复。

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemStateSnapshot {
    pub version: u32,
    pub pid: u32,
    pub created_at: i64,
    /// 快照时刻的任务栏窗口与可见性
    pub taskbars: Vec<crate::taskbar::TaskbarWindow>,
    /// 快照时刻的 HideIcons 注册表值（None = 未设置）
    pub hide_icons_reg: Option<u32>,
}

pub const SNAPSHOT_FILE: &str = "system-state.json";

/// 快照文件路径：与主程序共用 %APPDATA%\com.hamsterhub.app\
pub fn snapshot_path() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    Some(
        PathBuf::from(appdata)
            .join("com.hamsterhub.app")
            .join(SNAPSHOT_FILE),
    )
}

/// 捕获当前系统状态（不修改任何东西）
pub fn capture(pid: u32) -> SystemStateSnapshot {
    SystemStateSnapshot {
        version: 1,
        pid,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        taskbars: crate::taskbar::find_all(),
        hide_icons_reg: crate::desktop_icons::get_hide_icons(),
    }
}

pub fn write(path: &Path, snap: &SystemStateSnapshot) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(snap).map_err(|e| e.to_string())?;
    // 先写临时文件再原子改名，避免看门狗读到半截 JSON
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn read(path: &Path) -> Option<SystemStateSnapshot> {
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

pub fn remove(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// 按快照恢复系统状态（任务栏可见性 + 桌面图标）
pub fn restore(snap: &SystemStateSnapshot) {
    crate::taskbar::restore(&snap.taskbars);
    match snap.hide_icons_reg {
        Some(0) | None => {
            let _ = crate::desktop_icons::set_hide_icons(0);
        }
        Some(v) => {
            let _ = crate::desktop_icons::set_hide_icons(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_roundtrip() {
        let dir = std::env::temp_dir().join("hamster-platform-test");
        let path = dir.join(SNAPSHOT_FILE);
        let snap = capture(std::process::id());
        write(&path, &snap).unwrap();
        let back = read(&path).expect("读回快照失败");
        assert_eq!(back.pid, snap.pid);
        assert_eq!(back.taskbars.len(), snap.taskbars.len());
        remove(&path);
        assert!(read(&path).is_none());
    }
}
