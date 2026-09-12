//! hamster-mcp：Hamster 桌面 MCP server（desktop + browser-use + computer-use）。
//!
//! 定位：宿主 Agent 经 MCP stdio 使用这里的工具操作桌面（应用/文件/待办/音量 +
//! 浏览器 + 截图点击键入）。vendor 自上游项目（Apache-2.0，整合时更名），扩展 desktop 工具组、
//! computer use 安全门与调用审计。

pub mod browser;
pub mod computer;
pub mod desktop;
pub mod mcp;
pub mod server;

pub use browser::{find_browser_executable, VerifyBrowser};
pub use mcp::{handle_message, ServerState, ENV_CU_MODE, PROTOCOL_VERSION, SERVER_NAME};
pub use server::{selftest, serve_stdio};

#[cfg(test)]
/// 测试辅助：建一份带完整迁移链的内存主库（mcp.rs 的 desktop 用例注入用）
pub(crate) fn desktop_test_db() -> rusqlite::Connection {
    let c = rusqlite::Connection::open_in_memory().expect("内存库");
    c.execute_batch(include_str!("../../../migrations/0001_init.sql"))
        .expect("0001");
    c.execute_batch(include_str!("../../../migrations/0002_fileindex.sql"))
        .expect("0002");
    c.execute_batch(include_str!("../../../migrations/0003_apps_pinyin.sql"))
        .expect("0003");
    c
}
