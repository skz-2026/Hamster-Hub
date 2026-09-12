// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut args = std::env::args().skip(1);
    // 桌面 MCP server 的 stdio 承载（ACP/codex 兜底 + 手动调试）。argv 分发
    // 必须在 Tauri/单实例初始化之前——子进程不进 GUI，与运行中实例互不干扰。
    if args.next().as_deref() == Some("mcp") {
        match args.next().as_deref() {
            Some("selftest") => {
                let url = args.next().unwrap_or_else(|| "https://example.com".into());
                let ok = hamster_hub_lib::mcp_selftest(&url);
                std::process::exit(if ok { 0 } else { 1 });
            }
            _ => hamster_hub_lib::mcp_serve(),
        }
        return;
    }
    hamster_hub_lib::run()
}
