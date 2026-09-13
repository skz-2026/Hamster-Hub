//! 桌面 MCP server 的内嵌 HTTP 承载（M4「Agent 原生桌面」，用户决策 2026-09-12）。
//!
//! server 随主进程常驻：`127.0.0.1:<固定端口>/mcp`，无鉴权（纯本地，用户决策
//! 2026-09-13；Origin 检查挡网页 drive-by），claude/codex 等 HTTP MCP 宿主
//! 零配置直连。急停 = bench_stream_kill 杀会话进程（进程死了工具调用自然停）。
//! 相比独立进程的红利：computer use 档位即时生效（设置页联动）、
//! 审计直写助手目录、直接复用主库连接（无跨进程 WAL 争用）。
//! 协议：MCP Streamable HTTP——POST JSON-RPC 单帧回（application/json），
//! 通知回 202，GET 流（SSE）按规范返回 405。ACP/codex 兜底走 stdio 子命令
//! （`hamster-hub.exe mcp serve`，见 main.rs 分发）。

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hamster_mcp::mcp::{handle_message, ServerState};
use serde_json::json;
use tokio::sync::Mutex as AsyncMutex;

use crate::error::AppError;

pub const MCP_PATH: &str = "/mcp";

/// 线程间共享：工具状态 + 会话登记（accept 线程与命令层各持一份 Arc）
struct Inner {
    state: Arc<AsyncMutex<ServerState>>,
    /// 主进程句柄（home_layout_set 写入后广播 hamster:layout-updated；stdio/测试为 None）
    app: Option<tauri::AppHandle>,
    /// 会话登记（鉴权移除后不再用于拦截；保留供审计与后续策略扩展）
    sessions: Mutex<HashMap<String, String>>,
    tokens: Mutex<HashSet<String>>,
}

/// 内嵌 MCP server 句柄（app.manage）
pub struct McpHub {
    inner: Arc<Inner>,
    port: u16,
    /// 首选端口被占用回退了随机端口（分发出去的固定 URL 当前不可用）
    fell_back: bool,
}

/// 桌面 MCP 默认端口（外部 agent 配置里的 url 需要跨重启稳定，故用固定默认值；
/// 被占用时回退随机端口并在设置页显示实际 URL）
pub const DEFAULT_MCP_PORT: u16 = 47613;

impl McpHub {
    /// 绑定 127.0.0.1 并拉起 accept 线程（每连接一线程；工具调用的串行语义由
    /// ServerState 内部锁保证）。`preferred_port`（设置 mcp_port，None = 默认）
    /// 绑定失败时回退随机端口——外部配置依赖固定端口，设置页展示实际 URL。
    /// `audit_path` = 助手目录 audit.jsonl。
    pub fn start(
        db: rusqlite::Connection,
        cu_allowed: bool,
        audit_path: Option<std::path::PathBuf>,
        preferred_port: Option<u16>,
        app: Option<tauri::AppHandle>,
    ) -> Result<Self, AppError> {
        let want = preferred_port.unwrap_or(DEFAULT_MCP_PORT);
        let (listener, port, fell_back) = match TcpListener::bind(("127.0.0.1", want)) {
            Ok(l) => {
                let port = l
                    .local_addr()
                    .map_err(|e| AppError::io(e.to_string()))?
                    .port();
                (l, port, false)
            }
            Err(bind_err) => {
                let l = TcpListener::bind("127.0.0.1:0")
                    .map_err(|e| AppError::new("MCP_HTTP_BIND", e.to_string()))?;
                let port = l
                    .local_addr()
                    .map_err(|e| AppError::io(e.to_string()))?
                    .port();
                eprintln!(
                    "[mcp] 端口 {want} 被占用（{bind_err}），回退随机端口 {port}——\
                     已分发到 agent 配置里的 URL 需要在设置页更新"
                );
                (l, port, true)
            }
        };
        let inner = Arc::new(Inner {
            app,
            state: Arc::new(AsyncMutex::new(ServerState::new_embedded(
                db, cu_allowed, audit_path,
            ))),
            sessions: Mutex::new(HashMap::new()),
            tokens: Mutex::new(HashSet::new()),
        });
        let inner_for_thread = inner.clone();
        std::thread::Builder::new()
            .name("mcp-http".into())
            .spawn(move || accept_loop(listener, inner_for_thread))
            .map_err(|e| AppError::new("MCP_HTTP_SPAWN", e.to_string()))?;
        Ok(Self {
            inner,
            port,
            fell_back,
        })
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}{MCP_PATH}", self.port)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// 端口是否回退过（true = 分发出去的固定端口 URL 当前不可用）
    pub fn port_fell_back(&self) -> bool {
        self.fell_back
    }

    /// 注册用户级长效令牌（外部 MCP 宿主接入；启动时与轮换后调用）
    pub fn register_user_token(&self, token: &str) {
        self.inner
            .tokens
            .lock()
            .expect("tokens 表")
            .insert(token.to_string());
    }

    /// 轮换用户令牌：旧令牌立即失效（设置页「重新生成」）
    pub fn rotate_user_token(&self, old: &str, new: &str) {
        let mut tokens = self.inner.tokens.lock().expect("tokens 表");
        tokens.remove(old);
        tokens.insert(new.to_string());
    }

    /// 铸造 Bearer 令牌（立即生效；spawn 成功后经 bind_session 绑定会话供急停）
    pub fn mint(&self) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        self.inner
            .tokens
            .lock()
            .expect("tokens 表")
            .insert(token.clone());
        token
    }

    /// 令牌 ↔ 会话绑定（急停按 sessionId 吊销）
    pub fn bind_session(&self, session_id: &str, token: &str) {
        self.inner
            .sessions
            .lock()
            .expect("sessions 表")
            .insert(session_id.to_string(), token.to_string());
    }

    /// 吊销一枚令牌（spawn 失败清理）
    pub fn revoke_token(&self, token: &str) {
        self.inner.tokens.lock().expect("tokens 表").remove(token);
    }

    /// 急停：吊销会话令牌——之后该会话的所有 MCP 调用立即 401
    pub fn revoke_session(&self, session_id: &str) {
        if let Some(token) = self
            .inner
            .sessions
            .lock()
            .expect("sessions 表")
            .remove(session_id)
        {
            self.inner.tokens.lock().expect("tokens 表").remove(&token);
        }
    }

    /// computer use 安全档位运行时更新（设置页开关即时生效，无需重启会话）
    pub fn set_cu_allowed(&self, allowed: bool) {
        tauri::async_runtime::block_on(async {
            self.inner.state.lock().await.set_cu_allowed(allowed);
        });
    }
}

fn accept_loop(listener: TcpListener, inner: Arc<Inner>) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let inner_for_conn = inner.clone();
        // 每连接一线程：MCP 客户端会并发 POST；串行工具语义在 ServerState 锁内
        let _ = std::thread::Builder::new()
            .name("mcp-http-conn".into())
            .spawn(move || handle_conn(stream, inner_for_conn));
    }
}

fn handle_conn(mut stream: TcpStream, inner: Arc<Inner>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(120)));
    let req = match read_request(&mut stream) {
        Ok(r) => r,
        Err(()) => return, // 超时/断开/坏帧：直接关闭（客户端按连接失败重试）
    };
    let (status, body) = route(&inner, &req);
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

struct Req {
    method: String,
    path: String,
    /// 浏览器跨域请求必带（drive-by 防线的判定依据）；MCP 宿主不会带
    origin: Option<String>,
    body: String,
}

/// 最小 HTTP/1.1 读取：请求行 + 头 + 按 Content-Length 的 body
fn read_request(stream: &mut TcpStream) -> Result<Req, ()> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut chunk).map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
        if buf.len() > 1 << 20 {
            return Err(()); // 头部异常过大
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or(())?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(())?.to_string();
    let path = parts.next().ok_or(())?.to_string();
    let mut content_length = 0usize;
    let mut origin = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name == "content-length" {
            content_length = value.parse().map_err(|_| ())?;
        } else if name == "origin" {
            origin = Some(value.to_string());
        }
    }
    if content_length > 8 << 20 {
        return Err(()); // body 上限 8MB（截图回包方向不经过这里）
    }
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk).map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);
    Ok(Req {
        method,
        path,
        origin,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// 请求路由（独立成函数便于测试）：来源检查 → 方法 → JSON-RPC 分发。
/// 无鉴权（纯本地服务，用户决策 2026-09-13）：CLI 类 MCP 宿主免配 token/头。
/// 唯一保留的防线是 Origin 检查——浏览器跨域 POST 必带 Origin 头而 MCP 宿主
/// 不会带，挡掉恶意网页对 localhost 的 drive-by 调用（launch/todo 等是可写工具）。
fn route(inner: &Inner, req: &Req) -> (u16, String) {
    let (status, body) = route_inner(inner, req);
    // 临时访问日志（排查宿主连通性；量小落 %TEMP%，排查完可删）
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::temp_dir().join("hamster-mcp-access.log"))
    {
        use std::io::Write as _;
        let _ = writeln!(
            f,
            "{} {} origin={:?} body_len={} -> {}",
            req.method,
            req.path,
            req.origin,
            req.body.len(),
            status
        );
    }
    (status, body)
}

fn route_inner(inner: &Inner, req: &Req) -> (u16, String) {
    let (path, _query) = match req.path.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (req.path.as_str(), None),
    };
    if path != MCP_PATH {
        return not_found();
    }
    if let Some(origin) = &req.origin {
        let local =
            origin.starts_with("http://localhost") || origin.starts_with("http://127.0.0.1");
        if !local {
            return (403, json_body(&json!({ "error": "cross-origin blocked" })));
        }
    }
    if req.method != "POST" {
        // GET（SSE 服务器流）未提供：规范允许 405
        return (405, json_body(&json!({ "error": "method not allowed" })));
    }
    let Ok(msg) = serde_json::from_str::<serde_json::Value>(&req.body) else {
        return (400, json_body(&json!({ "error": "invalid json" })));
    };
    let resp = tauri::async_runtime::block_on(handle_message(&inner.state, &msg));
    match resp {
        // 通知帧（无 id）：202 Accepted 空体（Streamable HTTP 规范）
        None => (202, String::new()),
        Some(resp) => {
            // 主屏布局写入成功 → 广播刷新（前端 useHomeLayout 监听同款事件即时重取；
            // stdio/测试模式 app=None 不广播）
            if msg
                .pointer("/params/name")
                .and_then(serde_json::Value::as_str)
                == Some("home_layout_set")
                && resp.get("error").is_none()
            {
                if let Some(app) = &inner.app {
                    use tauri::Emitter;
                    let _ = app.emit("hamster:layout-updated", ());
                }
            }
            (200, resp.to_string())
        }
    }
}

fn json_body(v: &serde_json::Value) -> String {
    v.to_string()
}

fn not_found() -> (u16, String) {
    (404, json_body(&json!({ "error": "not found" })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub_with_mem_db() -> McpHub {
        // 带完整迁移链的内存主库（与 hamster-mcp 的 desktop 测试同款；跨 crate
        // 不能复用其 #[cfg(test)] 项，这里独立建）
        let conn = rusqlite::Connection::open_in_memory().expect("内存库");
        conn.execute_batch(include_str!("../migrations/0001_init.sql"))
            .expect("0001");
        conn.execute_batch(include_str!("../migrations/0002_fileindex.sql"))
            .expect("0002");
        conn.execute_batch(include_str!("../migrations/0003_apps_pinyin.sql"))
            .expect("0003");
        McpHub::start(conn, false, None, None, None).expect("启动内嵌 server")
    }

    fn post(hub: &McpHub, body: &str) -> (u16, String) {
        route(
            &hub.inner,
            &Req {
                method: "POST".into(),
                path: MCP_PATH.into(),
                origin: None,
                body: body.into(),
            },
        )
    }

    #[test]
    fn open_endpoint_accepts_initialize_without_auth() {
        let hub = hub_with_mem_db();
        let init = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
        }));
        // 无鉴权：不带任何令牌/头即可 initialize（纯本地，用户决策 2026-09-13）
        let (status, body) = post(&hub, &init);
        assert_eq!(status, 200, "{body}");
        assert!(body.contains("protocolVersion"), "{body}");
    }

    #[test]
    fn cross_origin_browser_posts_are_blocked() {
        let hub = hub_with_mem_db();
        let init = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
        }));
        // 浏览器跨域 drive-by（Origin 非本地）→ 403
        let req = Req {
            method: "POST".into(),
            path: MCP_PATH.into(),
            origin: Some("https://evil.example".into()),
            body: init,
        };
        let (status, body) = route(&hub.inner, &req);
        assert_eq!(status, 403, "{body}");
        // 本机来源（同机页面）放行
        let req = Req {
            origin: Some("http://localhost:5173".into()),
            ..req
        };
        assert_eq!(route(&hub.inner, &req).0, 200);
    }

    #[test]
    fn computer_tool_denied_then_allowed_via_set_cu_allowed() {
        let hub = hub_with_mem_db();
        let click = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "computer_click", "arguments": { "x": 1, "y": 1 } }
        }));
        let (status, body) = post(&hub, &click);
        assert_eq!(
            status, 200,
            "安全门拒绝是 JSON-RPC 错误帧不是 HTTP 错误：{body}"
        );
        assert!(body.contains("-32001"), "应被安全门拒绝：{body}");
        // 设置页联动（内嵌模式的红利）：立即放行，无需重建会话
        hub.set_cu_allowed(true);
        let (status, body) = post(&hub, &click);
        assert_eq!(status, 200);
        assert!(!body.contains("-32001"), "开启后不应再被安全门拦截：{body}");
    }

    #[test]
    fn desktop_todo_roundtrip_over_http() {
        let hub = hub_with_mem_db();
        let create = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "todo_create", "arguments": { "content": "给仓鼠添粮" } }
        }));
        let (status, body) = post(&hub, &create);
        assert_eq!(status, 200, "{body}");
        let list = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": { "name": "todo_list", "arguments": {} }
        }));
        let (_, body) = post(&hub, &list);
        assert!(body.contains("给仓鼠添粮"), "{body}");
    }

    #[test]
    fn wrong_path_method_and_bad_json() {
        let hub = hub_with_mem_db();
        let req = |method: &str, path: &str, body: &str| {
            route(
                &hub.inner,
                &Req {
                    method: method.into(),
                    path: path.into(),
                    origin: None,
                    body: body.into(),
                },
            )
            .0
        };
        assert_eq!(req("POST", "/other", "{}"), 404);
        assert_eq!(req("GET", MCP_PATH, ""), 405);
        assert_eq!(req("POST", MCP_PATH, "not json"), 400);
    }

    #[test]
    fn notifications_get_202() {
        let hub = hub_with_mem_db();
        let (status, body) = post(
            &hub,
            &json_body(
                &serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            ),
        );
        assert_eq!(status, 202);
        assert!(body.is_empty());
    }
}
