//! 流式会话管理：AppServer 会话注册表（与 SessionManager 平行）。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use hamster_core::error::{HamsterError, Result};
use hamster_core::LiveStreamInfo;

use crate::appserver::{StreamExitCallback, StreamOptions, StreamSession};
use crate::session::lock_ok;

/// 空闲回收阈值（ui-design §4.3 用户约定 10 分钟）：运行中且超过该时长无活动
/// 的会话由 reaper 自动 kill。前端 `reclaimIfIdleSession` 只在关 tab 那一瞬间
/// 评估一次，关 tab 时仍活跃的会话若无人补刀，会一直漏到进程退出——reaper 即补刀。
const IDLE_REAP_MS: i64 = 10 * 60_000;
/// reaper 扫描周期
const REAP_TICK: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct StreamManager {
    sessions: Arc<Mutex<Vec<Arc<StreamSession>>>>,
}

impl StreamManager {
    pub fn new() -> Self {
        let mgr = Self::default();
        mgr.spawn_reaper();
        mgr
    }

    /// 空闲回收线程：随管理器常驻，周期扫描运行中会话，空闲超阈值即 kill。
    fn spawn_reaper(&self) {
        let sessions = Arc::clone(&self.sessions);
        let _ = std::thread::Builder::new()
            .name("stream-idle-reaper".into())
            .spawn(move || loop {
                std::thread::sleep(REAP_TICK);
                reap_idle(&sessions, now_ms());
            });
    }

    pub fn spawn(&self, opts: StreamOptions) -> Result<LiveStreamInfo> {
        let created = StreamSession::start(opts)?;
        let info = created.info();
        lock_ok(&self.sessions).push(created);
        Ok(info)
    }

    fn get(&self, session_id: &str) -> Result<Arc<StreamSession>> {
        lock_ok(&self.sessions)
            .iter()
            .find(|s| s.session_id() == session_id)
            .cloned()
            .ok_or_else(|| HamsterError::not_found(format!("流式会话 {session_id}")))
    }

    pub fn send(
        &self,
        session_id: &str,
        text: &str,
        model: Option<&str>,
        effort: Option<&str>,
    ) -> Result<()> {
        self.get(session_id)?.send_turn(text, model, effort)
    }

    pub fn interrupt(&self, session_id: &str) -> Result<()> {
        self.get(session_id)?.interrupt()
    }

    /// 会话级配置项修改（ACP：mode=官方 set_mode；其余=set_config）。
    pub fn set_config(&self, session_id: &str, config_id: &str, value: &str) -> Result<()> {
        self.get(session_id)?.set_config(config_id, value)
    }

    pub fn kill(&self, session_id: &str) -> Result<()> {
        self.get(session_id)?.kill()
    }

    pub fn info(&self, session_id: &str) -> Result<LiveStreamInfo> {
        Ok(self.get(session_id)?.info())
    }

    pub fn list(&self) -> Vec<LiveStreamInfo> {
        lock_ok(&self.sessions).iter().map(|s| s.info()).collect()
    }

    /// 应用退出时终止全部（best-effort）。
    pub fn kill_all(&self) {
        for s in lock_ok(&self.sessions).iter() {
            if s.is_running() {
                s.kill().ok();
            }
        }
    }
}

/// 回收判定（独立纯函数便于单测）：运行中且空闲时长达阈值。
fn should_reap(running: bool, last_active_at: i64, now_ms: i64) -> bool {
    running && now_ms - last_active_at >= IDLE_REAP_MS
}

/// 扫描并 kill 空闲会话。kill 幂等（进程已死则为无害操作）；单次失败静默，
/// 下一轮 `running` 仍为 true 会重试。
fn reap_idle(sessions: &Mutex<Vec<Arc<StreamSession>>>, now_ms: i64) {
    let targets: Vec<Arc<StreamSession>> = lock_ok(sessions)
        .iter()
        .filter(|s| {
            let info = s.info();
            should_reap(info.running, info.last_active_at, now_ms)
        })
        .cloned()
        .collect();
    for s in targets {
        let _ = s.kill();
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// StreamSession 需要 pub 的锁工具（appserver 内部使用同一约定）。
pub fn stream_exit_noop() -> StreamExitCallback {
    Box::new(|_| {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_stream_session_is_not_found() {
        let mgr = StreamManager::new();
        let err = mgr.send("nope", "hi", None, None).unwrap_err();
        assert!(matches!(err, HamsterError::NotFound { .. }));
        assert!(mgr.list().is_empty());
    }

    #[test]
    fn reap_decision_follows_running_and_idle_threshold() {
        let now = 1_000_000_000;
        // 运行中 + 刚活动：保留
        assert!(!should_reap(true, now - 1_000, now));
        // 运行中 + 空闲达阈值：回收
        assert!(should_reap(true, now - IDLE_REAP_MS, now));
        assert!(should_reap(true, now - IDLE_REAP_MS - 1, now));
        // 已退出：不回收（也无害）
        assert!(!should_reap(false, now - IDLE_REAP_MS * 10, now));
        // 时间倒流（时钟偏移）：视为未达阈值，不误杀
        assert!(!should_reap(true, now + 5_000, now));
    }

    #[test]
    fn reap_idle_on_empty_registry_is_noop() {
        let sessions = Mutex::new(Vec::new());
        reap_idle(&sessions, now_ms());
        assert!(sessions.lock().expect("reap scan").is_empty());
    }
}
