// 后台看门狗不需要控制台：release 转 GUI 子系统（日志走 watchdog.log），
// 即使被用户手动双击也不弹黑框；debug 保留控制台便于本地排查
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! 仓鼠Hub 看门狗
//!
//! 由主进程在进入桌面模式时拉起：
//! - 快照文件存在 + 主进程已死 → 按快照还原系统状态，删除快照，退出
//! - 快照文件不存在（正常退出会删除）→ 直接退出
//! - 每 2s 轮询一次；最长存活 24h 自杀保护
//!
//! 注意：主进程死后其 stdio 管道即断，任何 eprintln 都会因 broken pipe panic，
//! 因此日志一律写 %APPDATA%\com.hamsterhub.app\watchdog.log。

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use hamster_platform::snapshot;
use windows::core::PWSTR;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};

fn log(msg: &str) {
    let mut path = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push("com.hamsterhub.app");
    let _ = std::fs::create_dir_all(&path);
    path.push("watchdog.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{} {msg}", chrono_like_now());
    }
}

fn chrono_like_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("[{secs}]")
}

fn main() {
    let pid: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if pid == 0 {
        std::process::exit(2);
    }
    let snap_path: PathBuf = snapshot::snapshot_path().unwrap_or_else(|| {
        std::process::exit(2);
    });
    log(&format!("watchdog started, main pid = {pid}"));

    let started = Instant::now();
    loop {
        std::thread::sleep(Duration::from_secs(2));

        // 快照不在 = 主进程已正常还原退出
        let Some(snap) = snapshot::read(&snap_path) else {
            log("snapshot gone, main exited cleanly");
            std::process::exit(0);
        };

        if !process_alive(snap.pid) {
            log(&format!(
                "main {} dead with snapshot present, restoring",
                snap.pid
            ));
            snapshot::restore(&snap);
            // 兜底：不信任快照细节，确保任务栏可见
            hamster_platform::taskbar::restore_all_by_class();
            snapshot::remove(&snap_path);
            std::process::exit(0);
        }

        // 自杀保护：主进程失联拉起看门狗却长期不退出（异常场景）
        if started.elapsed() > Duration::from_secs(24 * 3600) {
            log("24h timeout, exiting without restore");
            std::process::exit(3);
        }
    }
}

/// 主进程存活判定：PID 存在且可查、且镜像路径确属 hamster-hub。
/// （裸 OpenProcess 会把"死后被系统复用的 PID"误判为存活，导致永不还原。）
fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = windows::Win32::Foundation::CloseHandle(h);
        if ok.is_err() {
            return false;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
        path.contains("hamster-hub")
    }
}
