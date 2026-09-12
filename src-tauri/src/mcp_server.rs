//! 桌面 MCP server 的内嵌 HTTP 承载（M4「Agent 原生桌面」，用户决策 2026-09-12）。
//!
//! server 随主进程常驻：`127.0.0.1:<随机端口>/mcp` + 每会话 Bearer 令牌，
//! claude 走 `--mcp-config` 的 `{"type":"http"}` 注入（零进程、零侵入用户配置）。
//! 相比独立进程的红利：急停 = 吊销令牌、computer use 档位即时生效（设置页联动）、
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

/// 线程间共享：工具状态 + 会话令牌表（accept 线程与命令层各持一份 Arc）
struct Inner {
    state: Arc<AsyncMutex<ServerState>>,
    /// 活跃助手会话：sessionId → Bearer 令牌（会话被 kill 时吊销 = 急停）
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
    authorization: Option<String>,
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
    let mut authorization = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name == "content-length" {
            content_length = value.parse().map_err(|_| ())?;
        } else if name == "authorization" {
            authorization = Some(value.to_string());
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
        authorization,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// 请求路由（独立成函数便于测试）：鉴权 → 方法 → JSON-RPC 分发。
/// 令牌双通道：`Authorization: Bearer <t>` 头，或 `?token=<t>` 查询参数
///（兼容无法设置自定义头的 MCP 客户端）。
fn route(inner: &Inner, req: &Req) -> (u16, String) {
    let (path, query) = match req.path.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (req.path.as_str(), None),
    };
    if path != MCP_PATH {
        return not_found();
    }
    let token = req
        .authorization
        .as_deref()
        .and_then(|a| a.strip_prefix("Bearer "))
        .map(str::to_string)
        .or_else(|| {
            query.and_then(|q| {
                q.split('&')
                    .find_map(|kv| kv.strip_prefix("token=").map(str::to_string))
            })
        });
    let token_ok = token
        .map(|t| {
            inner
                .tokens
                .lock()
                .map(|set| set.contains(&t))
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if !token_ok {
        return (401, json_body(&json!({ "error": "unauthorized" })));
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
        Some(resp) => (200, resp.to_string()),
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
        McpHub::start(conn, false, None, None).expect("启动内嵌 server")
    }

    fn post(hub: &McpHub, token: Option<&str>, body: &str) -> (u16, String) {
        route(
            &hub.inner,
            &Req {
                method: "POST".into(),
                path: MCP_PATH.into(),
                authorization: token.map(|t| format!("Bearer {t}")),
                body: body.into(),
            },
        )
    }

    #[test]
    fn mint_authorizes_and_revoke_kills() {
        let hub = hub_with_mem_db();
        let token = hub.mint();
        hub.bind_session("s1", &token);
        let init = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
        }));
        // 有效令牌：initialize 正常回包
        let (status, body) = post(&hub, Some(&token), &init);
        assert_eq!(status, 200, "{body}");
        assert!(body.contains("protocolVersion"), "{body}");
        // 未知令牌 / 缺失令牌：401
        assert_eq!(post(&hub, Some("bogus"), &init).0, 401);
        assert_eq!(post(&hub, None, &init).0, 401);
        // 急停：吊销后原令牌立即失效
        hub.revoke_session("s1");
        assert_eq!(post(&hub, Some(&token), &init).0, 401);
    }

    #[test]
    fn token_via_query_param_is_accepted() {
        let hub = hub_with_mem_db();
        let token = hub.mint();
        let req = Req {
            method: "POST".into(),
            path: format!("{MCP_PATH}?token={token}"),
            authorization: None,
            body: json_body(&serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
            })),
        };
        let (status, body) = route(&hub.inner, &req);
        assert_eq!(
            status, 200,
            "查询参数令牌应可用（无 header 客户端兜底）：{body}"
        );
        // 令牌错误仍是 401
        let req = Req {
            path: format!("{MCP_PATH}?token=wrong"),
            ..req
        };
        assert_eq!(route(&hub.inner, &req).0, 401);
    }

    #[test]
    fn computer_tool_denied_then_allowed_via_set_cu_allowed() {
        let hub = hub_with_mem_db();
        let token = hub.mint();
        let click = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "computer_click", "arguments": { "x": 1, "y": 1 } }
        }));
        let (status, body) = post(&hub, Some(&token), &click);
        assert_eq!(
            status, 200,
            "安全门拒绝是 JSON-RPC 错误帧不是 HTTP 错误：{body}"
        );
        assert!(body.contains("-32001"), "应被安全门拒绝：{body}");
        // 设置页联动（内嵌模式的红利）：立即放行，无需重建会话
        hub.set_cu_allowed(true);
        let (status, body) = post(&hub, Some(&token), &click);
        assert_eq!(status, 200);
        assert!(!body.contains("-32001"), "开启后不应再被安全门拦截：{body}");
    }

    #[test]
    fn desktop_todo_roundtrip_over_http() {
        let hub = hub_with_mem_db();
        let token = hub.mint();
        let create = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "todo_create", "arguments": { "content": "给仓鼠添粮" } }
        }));
        let (status, body) = post(&hub, Some(&token), &create);
        assert_eq!(status, 200, "{body}");
        let list = json_body(&serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": { "name": "todo_list", "arguments": {} }
        }));
        let (_, body) = post(&hub, Some(&token), &list);
        assert!(body.contains("给仓鼠添粮"), "{body}");
    }

    #[test]
    fn wrong_path_method_and_bad_json() {
        let hub = hub_with_mem_db();
        let token = hub.mint();
        let req = |method: &str, path: &str, body: &str| {
            route(
                &hub.inner,
                &Req {
                    method: method.into(),
                    path: path.into(),
                    authorization: Some(format!("Bearer {token}")),
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
        let token = hub.mint();
        let (status, body) = post(
            &hub,
            Some(&token),
            &json_body(
                &serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            ),
        );
        assert_eq!(status, 202);
        assert!(body.is_empty());
    }
}
