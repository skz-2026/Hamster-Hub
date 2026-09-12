//! MCP server 入口（由主程序复用）：`hamster-hub.exe mcp serve`（stdio，默认）或
//! `hamster-hub.exe mcp selftest <url>`（冒烟）。
//!
//! 设计说明：MCP stdio 是 command 型注入——agent CLI 会把可执行文件作为子进程
//! 拉起并随会话回收，因此这里必然是一个独立「进程」；但不再需要独立 exe——
//! 主程序 argv 分发（src/main.rs）在 Tauri/单实例初始化之前进入本模块。

use std::io::{BufRead, Write as _};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use crate::mcp::{handle_message, ServerState};

/// MCP stdio 服务：stdin 每行一条 JSON-RPC，stdout 回帧；stdin EOF 正常退出
/// （ServerState/VerifyBrowser 的 Drop 兜底清理自管浏览器进程与临时目录）。
pub fn serve_stdio() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async {
        let state = Arc::new(Mutex::new(ServerState::new()));
        let stdin = std::io::stdin();
        let mut out = std::io::stdout();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(msg) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if let Some(resp) = handle_message(&state, &msg).await {
                let mut frame = serde_json::to_string(&resp).unwrap_or_default();
                frame.push('\n');
                if out.write_all(frame.as_bytes()).is_err() {
                    break;
                }
                let _ = out.flush();
            }
        }
    });
}

/// 冒烟自测：打开 URL → 快照 → 截图 → 关闭。全绿打印 SELFTEST-OK。
pub fn selftest(url: &str) -> bool {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async {
        let state = Arc::new(Mutex::new(ServerState::new()));
        let mut ok = true;
        for (tool, args) in [
            ("browser_open", serde_json::json!({ "url": url })),
            ("browser_snapshot", serde_json::json!({})),
            (
                "browser_screenshot",
                serde_json::json!({ "name": "selftest.png" }),
            ),
            ("browser_close", serde_json::json!({})),
        ] {
            let msg = serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "method": "tools/call",
                "params": { "name": tool, "arguments": args }
            });
            let resp = handle_message(&state, &msg).await;
            match resp {
                Some(v) => {
                    let err = v.pointer("/result/isError").and_then(Value::as_bool) == Some(true);
                    let full = v.pointer("/result/content").cloned().unwrap_or(Value::Null);
                    println!(
                        "[{tool}] err={err}\nRESP={}",
                        serde_json::to_string_pretty(&v).unwrap_or_default()
                    );
                    if err {
                        ok = false;
                    }
                    if tool == "browser_snapshot" {
                        println!("--- snapshot 前 40 行 ---");
                        let snap = full
                            .pointer("/0/text")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        for line in snap.lines().take(40) {
                            println!("{line}");
                        }
                    }
                }
                None => {
                    println!("[{tool}] 无响应");
                    ok = false;
                }
            }
        }
        if ok {
            println!("SELFTEST-OK");
        } else {
            println!("SELFTEST-FAILED");
        }
        ok
    })
}
