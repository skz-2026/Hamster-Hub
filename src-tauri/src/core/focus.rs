//! 专注计时（番茄钟）：后端秒级计时线程，事件推送剩余时间；暂停不计入时长。
//! 状态经 tauri manage 共享（FocusState 可 Clone 进线程）；完成/停止落 store::focus 历史。

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use rusqlite::Connection;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::events::{FocusFinished, FocusTick};

pub const KIND_FOCUS: &str = "focus";
pub const KIND_BREAK: &str = "break";

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct FocusStatus {
    /// focus = 专注 / break = 休息
    pub kind: String,
    pub total_secs: u32,
    pub remaining_secs: u32,
    pub paused: bool,
    pub todo_id: Option<u32>,
}

pub struct Active {
    pub kind: &'static str,
    pub total_secs: u32,
    pub todo_id: Option<u32>,
    pub started_at: i64,
    pub remaining: Arc<AtomicU32>,
    pub paused: Arc<AtomicBool>,
    pub stop: Arc<AtomicBool>,
}

impl Active {
    fn status(&self) -> FocusStatus {
        FocusStatus {
            kind: self.kind.into(),
            total_secs: self.total_secs,
            remaining_secs: self.remaining.load(Ordering::SeqCst),
            paused: self.paused.load(Ordering::SeqCst),
            todo_id: self.todo_id,
        }
    }
}

/// 应用级计时状态（Default：无会话）
#[derive(Clone, Default)]
pub struct FocusState(pub Arc<Mutex<Option<Active>>>);

impl FocusState {
    pub fn status(&self) -> Option<FocusStatus> {
        let g = self.0.lock().ok()?;
        g.as_ref().map(|a| a.status())
    }

    fn clear(&self) {
        if let Ok(mut g) = self.0.lock() {
            *g = None;
        }
    }
}

/// 开始一轮（已有进行中的先按"手动停止"落库）；并拉起秒级 ticker
pub fn start(
    state: &FocusState,
    app: AppHandle,
    db_path: &Path,
    kind: &'static str,
    minutes: u32,
    todo_id: Option<u32>,
) -> FocusStatus {
    stop(state, db_path);
    let total = minutes.clamp(1, 180) * 60;
    let active = Active {
        kind,
        total_secs: total,
        todo_id,
        started_at: Utc::now().timestamp(),
        remaining: Arc::new(AtomicU32::new(total)),
        paused: Arc::new(AtomicBool::new(false)),
        stop: Arc::new(AtomicBool::new(false)),
    };
    let status = active.status();
    if let Ok(mut g) = state.0.lock() {
        *g = Some(active);
    }
    spawn_ticker(state.clone(), app, db_path.to_path_buf());
    status
}

pub fn set_paused(state: &FocusState, paused: bool) {
    if let Ok(g) = state.0.lock() {
        if let Some(a) = g.as_ref() {
            a.paused.store(paused, Ordering::SeqCst);
        }
    }
}

/// 手动停止：落一条净时长历史（暂停段不计），返回落库秒数
pub fn stop(state: &FocusState, db_path: &Path) -> Option<i64> {
    let (kind, elapsed, todo_id, started_at) = {
        let mut g = state.0.lock().ok()?;
        let a = g.as_mut()?;
        a.stop.store(true, Ordering::SeqCst);
        let elapsed = (a.total_secs - a.remaining.load(Ordering::SeqCst)) as i64;
        (a.kind, elapsed, a.todo_id, a.started_at)
    };
    state.clear();
    record(
        db_path,
        todo_id,
        kind,
        elapsed,
        started_at,
        Utc::now().timestamp(),
    );
    Some(elapsed)
}

fn record(
    db_path: &Path,
    todo_id: Option<u32>,
    kind: &str,
    seconds: i64,
    started_at: i64,
    ended_at: i64,
) {
    if seconds <= 0 {
        return;
    }
    match Connection::open(db_path) {
        Ok(conn) => {
            if let Err(e) =
                crate::store::focus::record(&conn, todo_id, kind, seconds, started_at, ended_at)
            {
                eprintln!("[focus] 专注历史落库失败: {e}");
            }
        }
        Err(e) => eprintln!("[focus] 专注历史打不开主库: {e}"),
    }
}

fn spawn_ticker(state: FocusState, app: AppHandle, db_path: std::path::PathBuf) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        // 短暂持锁：读标志、递减剩余；落库/清状态前先释放
        let info = {
            let Ok(g) = state.0.lock() else { break };
            let Some(a) = g.as_ref() else { break };
            if a.stop.load(Ordering::SeqCst) {
                break; // 手动停止已由 stop() 落库
            }
            if a.paused.load(Ordering::SeqCst) {
                None
            } else {
                Some((
                    a.kind,
                    a.remaining.fetch_sub(1, Ordering::SeqCst) - 1,
                    a.total_secs,
                    a.todo_id,
                    a.started_at,
                ))
            }
        };
        let Some((kind, left, total, todo_id, started_at)) = info else {
            continue;
        };
        let _ = FocusTick {
            kind: kind.into(),
            remaining_secs: left,
            paused: false,
        }
        .emit(&app);
        if left == 0 {
            record(
                &db_path,
                todo_id,
                kind,
                total as i64,
                started_at,
                Utc::now().timestamp(),
            );
            state.clear();
            let _ = FocusFinished { kind: kind.into() }.emit(&app);
            break;
        }
    });
}
