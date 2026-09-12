//! PTY 会话：单个 Agent CLI 子进程的宿主。
//!
//! 交互边界（红线①「宿主零侵入」，runtime-design.md §3.1/§8）：
//! 仅 启动 / stdin 写入 / stdout 读取转发 / resize / kill 五类；
//! 环境继承用户进程，只补 TERM=xterm-256color（ConPTY 忽略之，Unix TUI 需要）。

use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};

use hamster_core::error::{HamsterError, Result};
use hamster_core::{LiveSessionInfo, SessionKind};

use crate::pty_err;
use crate::spawn::SpawnPlan;

/// 输出读取缓冲：单 chunk ≤16KB；转发前经 8ms/32KB 合帧（coalesce.rs）。
const READ_BUF: usize = 16 * 1024;

/// 退出码：wait 本身失败（如被信号杀死且拿不到状态）时的兜底。
const WAIT_FAILED_CODE: i32 = -1;

/// bracketed-paste 单块字节上限：超过则分块 + 限速（ConPTY 实测 ~64B/ms，
/// runtime-design §3.4；防止大 prompt 溢出输入缓冲）。
const PASTE_CHUNK_BYTES: usize = 512;
const PASTE_PACE_MS: u64 = 25;

const PASTE_HEAD: &[u8] = b"\x1b[200~";
const PASTE_TAIL: &[u8] = b"\x1b[201~";

/// 文本 → bracketed-paste 分块字节序列（按字符边界切，不撕裂 UTF-8）。
pub fn paste_chunks(text: &str) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    let mut current = PASTE_HEAD.to_vec();
    for ch in text.chars() {
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf).as_bytes();
        if current.len() + encoded.len() + PASTE_TAIL.len() > PASTE_CHUNK_BYTES {
            current.extend_from_slice(PASTE_TAIL);
            chunks.push(std::mem::take(&mut current));
            current = PASTE_HEAD.to_vec();
        }
        current.extend_from_slice(encoded);
    }
    current.extend_from_slice(PASTE_TAIL);
    chunks.push(current);
    chunks
}

/// stdout chunk 回调：原样字节流转发，不做任何语义改写。
pub type DataCallback = Box<dyn Fn(&[u8]) + Send + Sync>;
/// 退出回调 (session_id, exit_code)，wait 线程内恰好调用一次。
pub type ExitCallback = Box<dyn Fn(&str, i32) + Send + Sync>;

pub struct SpawnOptions {
    pub agent_id: String,
    /// rollout 会话键（resume 时非空）：写入会话信息供幂等裁决与侧栏聚合
    pub resume_key: Option<String>,
    /// 会话形态：Agent 对话（默认）或工作台通用终端
    pub kind: SessionKind,
    pub project_dir: PathBuf,
    pub plan: SpawnPlan,
    pub cols: u16,
    pub rows: u16,
    pub on_data: DataCallback,
    pub on_exit: ExitCallback,
}

struct SessionState {
    agent_id: String,
    kind: SessionKind,
    project_dir: PathBuf,
    resume_key: Option<String>,
    running: bool,
    exit_code: Option<i32>,
    started_at: i64,
    last_active_at: i64,
    /// 当前 PTY 几何（spawn 设定，resize 跟随；远程镜像端读取用）
    cols: u16,
    rows: u16,
}

pub struct PtySession {
    session_id: String,
    program: String,
    state: Mutex<SessionState>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    writer: Mutex<Box<dyn Write + Send>>,
    /// 直接子进程 pid（Windows 按进程树终止用；npm shim → node → agent.exe 的
    /// 孙进程不会被 ConPTY 关闭连带终结，孤儿 agent 会继续持有会话写锁）
    child_pid: Mutex<Option<u32>>,
    /// Option：kill 兜底时 take 掉并 Drop（关闭 ConPTY 终结子进程）
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    /// 输出汇（多播，支持视图重建后重新 attach）
    sinks: Arc<DataSinks>,
}

/// Mutex 中毒恢复：拿不到锁的极端场景（持锁线程 panic）下继续用原值，
/// 会话转发路径不应因毒锁永久卡死。
pub(crate) fn lock_ok<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 输出汇（多播）：同一会话可被多个视图先后附着（视图重建 / 重复 attach），
/// 合帧线程把每帧广播给全部汇；已关闭的汇由调用方自行摘除（写闭包容错）。
#[derive(Default)]
pub(crate) struct DataSinks {
    sinks: Mutex<Vec<DataCallback>>,
}

impl DataSinks {
    pub fn push(&self, cb: DataCallback) {
        lock_ok(&self.sinks).push(cb);
    }

    pub fn broadcast(&self, bytes: &[u8]) {
        for cb in lock_ok(&self.sinks).iter() {
            cb(bytes);
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 启动一次 PTY 会话并挂好读/等待线程。失败时不留半成品（pair 随作用域释放）。
pub fn spawn(opts: SpawnOptions) -> Result<Arc<PtySession>> {
    let program = opts.plan.program.clone();
    if !opts.project_dir.is_dir() {
        return Err(HamsterError::Launch {
            program,
            message: format!("项目目录不存在：{}", opts.project_dir.display()),
        });
    }

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: opts.rows.max(1),
            cols: opts.cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| pty_err(&program, e))?;

    let mut cmd = CommandBuilder::new(&opts.plan.program);
    cmd.args(&opts.plan.args);
    cmd.cwd(&opts.project_dir);
    #[cfg(not(windows))]
    cmd.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| pty_err(&program, e))?;
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| pty_err(&program, e))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| pty_err(&program, e))?;
    // 0.8 版 clone_killer 直接返回 Box<dyn ChildKiller>（非 Result）
    let killer = child.clone_killer();

    let sinks = Arc::new(DataSinks::default());
    sinks.push(opts.on_data);
    let session = Arc::new(PtySession {
        session_id: uuid::Uuid::new_v4().to_string(),
        program: program.clone(),
        state: Mutex::new(SessionState {
            agent_id: opts.agent_id,
            kind: opts.kind,
            project_dir: opts.project_dir,
            resume_key: opts.resume_key,
            running: true,
            exit_code: None,
            started_at: now_ms(),
            last_active_at: now_ms(),
            cols: opts.cols.max(1),
            rows: opts.rows.max(1),
        }),
        killer: Mutex::new(killer),
        writer: Mutex::new(writer),
        child_pid: Mutex::new(child.process_id()),
        master: Mutex::new(Some(pair.master)),
        sinks,
    });

    spawn_reader_thread(&session.session_id, reader, Arc::clone(&session.sinks))?;
    spawn_wait_thread(&session, child, opts.on_exit)?;

    Ok(session)
}

impl PtySession {
    /// 附着新的输出汇（视图重建 / 重复 attach：新通道接管输出广播）
    pub fn attach(&self, on_data: DataCallback) {
        self.sinks.push(on_data);
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// GUI 输入通道（runtime-design §3.6 混用模式）：把文本以 bracketed-paste
    /// 包裹后分块写入 stdin，再补回车——与用户在 TUI 里手动粘贴等价，
    /// Agent 收到的就是一次普通粘贴 + 回车（官方接口，零注入）。
    pub fn send_prompt(&self, text: &str) -> Result<()> {
        if !self.is_running() {
            return Err(HamsterError::Other("会话已结束，无法发送".into()));
        }
        for chunk in paste_chunks(text) {
            self.write(&chunk)?;
            if chunk.len() >= PASTE_CHUNK_BYTES {
                std::thread::sleep(Duration::from_millis(PASTE_PACE_MS));
            }
        }
        self.write(b"\r")
    }

    /// 用户输入 / 首条 prompt 注入直达 stdin，不落盘、不改写。
    ///
    /// ConPTY 的 conin 偶发返回携带残留错误码的伪失败（如 os error 0）：
    /// 短重试两次；会话已退出时输入无处可去，视为无害成功。
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let mut last_err: Option<HamsterError> = None;
        for attempt in 0..3u32 {
            let result = lock_ok(&self.writer).write_all(data);
            match result {
                Ok(()) => {
                    lock_ok(&self.writer).flush().ok();
                    lock_ok(&self.state).last_active_at = now_ms();
                    return Ok(());
                }
                Err(e) => {
                    last_err = Some(pty_err(&self.program, e));
                    if !self.is_running() {
                        return Ok(());
                    }
                    std::thread::sleep(Duration::from_millis(30 * (attempt as u64 + 1)));
                }
            }
        }
        match last_err {
            Some(_) if !self.is_running() => Ok(()),
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        {
            let mut st = lock_ok(&self.state);
            st.cols = cols.max(1);
            st.rows = rows.max(1);
        }
        let guard = lock_ok(&self.master);
        match guard.as_ref() {
            Some(master) => master
                .resize(PtySize {
                    rows: rows.max(1),
                    cols: cols.max(1),
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| pty_err(&self.program, e)),
            None => Ok(()), // 已兜底关闭，无需 resize
        }
    }

    /// 终止子进程，返回时子进程已确认退出（或兜底关闭后短期内确认）。
    ///
    /// 先尝试优雅退出：给 Agent TUI 发两连 Ctrl+C（codex/claude 的官方退出方式），
    /// 让它自行清理运行态——thread writer 锁被强杀会残留，导致该会话之后所有
    /// resume 立即失败。~2s 内没退再走强制路径。
    ///
    /// portable-pty 0.8.1 的 `WinChildKiller::kill` 判断反转（TerminateProcess 成功
    /// 时返回 Err(残留 last_error)），因此忽略其返回值，以会话状态为准。
    pub fn kill(&self) -> Result<()> {
        let graceful = lock_ok(&self.state).kind == SessionKind::Agent && self.is_running();
        if graceful {
            let _ = self.write(b"\x03");
            std::thread::sleep(Duration::from_millis(500));
            if self.is_running() {
                let _ = self.write(b"\x03");
            }
            for _ in 0..8 {
                if !self.is_running() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        }

        // Windows：按进程树强杀兜底。直接子进程只是 cmd shim，TerminateProcess
        // 不会连带孙进程（agent 本体），孤儿会继续持有会话写锁。
        #[cfg(windows)]
        if let Some(pid) = *lock_ok(&self.child_pid) {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .creation_flags(0x0800_0000) // CREATE_NO_WINDOW：不闪黑框
                .status();
        }
        lock_ok(&self.killer).kill().ok();
        lock_ok(&self.state).last_active_at = now_ms();

        for _ in 0..30 {
            if !self.is_running() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        lock_ok(&self.master).take();
        for _ in 0..20 {
            if !self.is_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        lock_ok(&self.state).running
    }

    /// 当前 PTY 几何（远程镜像端按此渲染，避免多端 fit 互抢尺寸）
    pub fn dims(&self) -> (u16, u16) {
        let st = lock_ok(&self.state);
        (st.cols, st.rows)
    }

    pub fn info(&self) -> LiveSessionInfo {
        let st = lock_ok(&self.state);
        LiveSessionInfo {
            session_id: self.session_id.clone(),
            agent_id: st.agent_id.clone(),
            kind: st.kind,
            channel: hamster_core::SessionChannel::Pty,
            project_dir: st.project_dir.display().to_string(),
            running: st.running,
            exit_code: st.exit_code,
            started_at: st.started_at,
            last_active_at: st.last_active_at,
            resume_key: st.resume_key.clone(),
        }
    }
}

fn spawn_reader_thread(
    session_id: &str,
    mut reader: Box<dyn Read + Send>,
    sinks: Arc<DataSinks>,
) -> Result<()> {
    // 读线程 → 通道 → 合帧线程：合帧需要超时冲刷，阻塞读给不了这个时机
    // （§3.3：击键回显最多攒 8ms，构建输出按 32KB 上限合并）
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let read_name = format!("pty-read-{session_id}");
    std::thread::Builder::new()
        .name(read_name)
        .spawn(move || {
            let mut buf = vec![0u8; READ_BUF];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF：子进程侧已关闭
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break; // 合帧线程已退出（回调方消失）
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        })
        .map_err(|e| HamsterError::Other(format!("PTY 读取线程启动失败：{e}")))?;

    let name = format!("pty-coalesce-{session_id}");
    std::thread::Builder::new()
        .name(name)
        .spawn(move || {
            crate::coalesce::run_coalesce(rx, &mut |bytes| sinks.broadcast(bytes));
        })
        .map(|_| ())
        .map_err(|e| HamsterError::Other(format!("PTY 合帧线程启动失败：{e}")))
}

fn spawn_wait_thread(
    session: &Arc<PtySession>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    on_exit: ExitCallback,
) -> Result<()> {
    let session = Arc::clone(session);
    std::thread::Builder::new()
        .name(format!("pty-wait-{}", session.session_id))
        .spawn(move || {
            let mut child = child;
            let code = child
                .wait()
                .map(|status| status.exit_code() as i32)
                .unwrap_or(WAIT_FAILED_CODE);
            {
                let mut st = lock_ok(&session.state);
                st.running = false;
                st.exit_code = Some(code);
            }
            on_exit(&session.session_id, code);
        })
        .map(|_| ())
        .map_err(|e| HamsterError::Other(format!("PTY 等待线程启动失败：{e}")))
}

#[cfg(test)]
mod paste_tests {
    use super::*;

    #[test]
    fn paste_chunks_wraps_and_splits_on_char_boundary() {
        let chunks = paste_chunks("hi");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].starts_with(PASTE_HEAD));
        assert!(chunks[0].ends_with(PASTE_TAIL));

        // 600 个中文（各 3 字节 UTF-8）必然跨块，且去掉包裹标记后必须还原原文
        let big = "测".repeat(600);
        let chunks = paste_chunks(&big);
        assert!(chunks.len() >= 3);
        let mut joined = String::new();
        for c in &chunks {
            let text = String::from_utf8(c.clone()).unwrap();
            let text = text
                .trim_start_matches(std::str::from_utf8(PASTE_HEAD).unwrap())
                .trim_end_matches(std::str::from_utf8(PASTE_TAIL).unwrap());
            joined.push_str(text);
        }
        assert_eq!(joined, big);
    }
}
