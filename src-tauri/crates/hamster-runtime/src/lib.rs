//! 上游 Runtime（v2.0）：PTY 会话宿主。
//!
//! 职责（runtime-design.md §3）：以 PTY 拉起官方 CLI、stdin 写入、stdout 原样转发、
//! resize、终止、退出上报。不感知具体 Agent（启动画像来自 core 的 RuntimeSpec）；
//! 不写任何 Agent 配置文件（红线①「宿主零侵入」）。
//!
//! 输出转发按 8ms/32KB 合帧（§3.3）：高吞吐（构建输出）不满屏打爆
//! ipc Channel，低延迟（击键回显）最多攒一个窗口期。

pub mod appserver;
#[cfg(test)]
mod appserver_tests;
pub mod coalesce;
pub mod manager;
pub mod scrollback;
pub mod session;
pub mod shell;
pub mod spawn;
pub mod stream_manager;

pub use appserver::{EventCallback, StreamOptions, StreamSession};
pub use manager::SessionManager;
pub use scrollback::ScrollbackStore;
pub use session::{DataCallback, PtySession, SpawnOptions};
pub use shell::shell_plan;
pub use spawn::{plan_spawn, SpawnPlan};
pub use stream_manager::StreamManager;

use hamster_core::error::HamsterError;

/// portable-pty 的错误统一映射为 HamsterError::Launch（保持全库单一错误类型）。
pub(crate) fn pty_err(program: &str, source: impl std::fmt::Display) -> HamsterError {
    HamsterError::Launch {
        program: program.to_string(),
        message: source.to_string(),
    }
}
