//! 会话管理：活会话注册表，Tauri command 的唯一入口（薄 glue 的对端）。

use std::sync::{Arc, Mutex};

use hamster_core::error::{HamsterError, Result};
use hamster_core::LiveSessionInfo;

use crate::session::lock_ok;
use crate::session::{self, PtySession, SpawnOptions};

#[derive(Default)]
pub struct SessionManager {
    sessions: Mutex<Vec<Arc<PtySession>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 启动会话并注册；返回含生成 session_id 的初始信息。
    pub fn spawn(&self, opts: SpawnOptions) -> Result<LiveSessionInfo> {
        let created = session::spawn(opts)?;
        let info = created.info();
        lock_ok(&self.sessions).push(created);
        Ok(info)
    }

    /// 为活会话附着新的输出汇（视图重建 / 重复 attach；仓鼠Hub bench 混用模式用）
    pub fn attach(&self, session_id: &str, on_data: session::DataCallback) -> Result<()> {
        self.get(session_id)?.attach(on_data);
        Ok(())
    }

    fn get(&self, session_id: &str) -> Result<Arc<PtySession>> {
        lock_ok(&self.sessions)
            .iter()
            .find(|s| s.session_id() == session_id)
            .cloned()
            .ok_or_else(|| HamsterError::not_found(format!("会话 {session_id}")))
    }

    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<()> {
        self.get(session_id)?.write(data)
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<()> {
        self.get(session_id)?.resize(cols, rows)
    }

    /// 当前 PTY 几何（远程镜像端按此渲染，避免多端 fit 互抢尺寸）
    pub fn dims(&self, session_id: &str) -> Result<(u16, u16)> {
        Ok(self.get(session_id)?.dims())
    }

    /// GUI 输入通道（bracketed-paste 注入运行中会话）。
    pub fn send_prompt(&self, session_id: &str, text: &str) -> Result<()> {
        self.get(session_id)?.send_prompt(text)
    }

    pub fn kill(&self, session_id: &str) -> Result<()> {
        self.get(session_id)?.kill()
    }

    pub fn info(&self, session_id: &str) -> Result<LiveSessionInfo> {
        let session = self.get(session_id)?;
        Ok(session.info())
    }

    /// 全量快照（含已退出会话，前端渲染「已结束 + 重启入口」）。
    pub fn list(&self) -> Vec<LiveSessionInfo> {
        lock_ok(&self.sessions).iter().map(|s| s.info()).collect()
    }

    /// 应用退出时终止全部运行中会话（best-effort；ConPTY 句柄释放亦会兜底）。
    pub fn kill_all(&self) {
        for session in lock_ok(&self.sessions).iter() {
            if session.is_running() {
                session.kill().ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_session_is_not_found() {
        let mgr = SessionManager::new();
        let err = mgr.write("nope", b"x").unwrap_err();
        assert!(matches!(err, HamsterError::NotFound { .. }));
        assert!(mgr.list().is_empty());
    }
}
