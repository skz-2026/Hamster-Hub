//! MCP stdio server：把 desktop/browser/computer 能力以 MCP 工具暴露给宿主 Agent。
//!
//! 协议：newline-delimited JSON-RPC 2.0（MCP stdio 规范）。
//! 支持 initialize / tools/list / tools/call / ping；每请求顺序处理。
//! 截图默认落 `HAMSTER_SHOTS`（默认系统临时目录），同时以 image content
//! 返回（headless agent 可直接"看"）。
//!
//! Hamster 扩展（vendor 后新增）：
//! - desktop 工具组（`desktop.rs`）：应用/文件/待办/音量，HAMSTER_DB_PATH 指向主库；
//! - computer_* 安全门：HAMSTER_CU_MODE 未开启（默认）时全部拒绝；
//! - 审计：HAMSTER_AUDIT_LOG 指向 JSONL，逐工具调用落盘（「agent 动了什么」可回看）。

use std::sync::Arc;

use base64::Engine as _;
use rusqlite::Connection;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::browser::VerifyBrowser;
use crate::computer::Computer;
use crate::desktop;
use crate::home;

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "hamster-desktop";
pub const SERVER_VERSION: &str = "0.1.0";

/// computer use 安全门的环境变量名（"1"/"true"/"on" = 放行，默认关）
pub const ENV_CU_MODE: &str = "HAMSTER_CU_MODE";

/// 全局服务状态（browser/computer/db 惰性创建）。
#[derive(Default)]
pub struct ServerState {
    browser: Option<VerifyBrowser>,
    computer: Option<Computer>,
    shots_dir: Option<std::path::PathBuf>,
    /// 主库连接（desktop 工具组；HAMSTER_DB_PATH 惰性打开）
    db: Option<Connection>,
    /// computer use 放行开关（默认 false = 拒绝）
    cu_allowed: bool,
    /// 审计 JSONL 路径（HAMSTER_AUDIT_LOG；未设置 = 不落盘）
    audit_path: Option<std::path::PathBuf>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            shots_dir: std::env::var("HAMSTER_SHOTS")
                .ok()
                .map(std::path::PathBuf::from),
            cu_allowed: matches!(
                std::env::var(ENV_CU_MODE).ok().as_deref(),
                Some("1" | "true" | "on")
            ),
            audit_path: std::env::var("HAMSTER_AUDIT_LOG")
                .ok()
                .map(std::path::PathBuf::from),
            ..Default::default()
        }
    }

    fn shots_dir(&self) -> std::path::PathBuf {
        self.shots_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("hamster-mcp-shots"))
    }

    /// 内嵌模式（HTTP server 随仓鼠Hub 主进程常驻）：直接注入连接与安全/审计配置，
    /// 不读环境变量。安全档位经 [`Self::set_cu_allowed`] 运行时更新（设置页开关即时生效）。
    pub fn new_embedded(
        db: Connection,
        cu_allowed: bool,
        audit_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            db: Some(db),
            cu_allowed,
            audit_path,
            ..Default::default()
        }
    }

    /// 运行时更新 computer use 安全档位（急停 = 置 false）
    pub fn set_cu_allowed(&mut self, allowed: bool) {
        self.cu_allowed = allowed;
    }

    /// 主库惰性打开（desktop 工具首次调用时）
    fn db_mut(&mut self) -> Result<&mut Connection, (i64, String)> {
        if self.db.is_none() {
            let path = std::env::var("HAMSTER_DB_PATH").map_err(|_| {
                (
                    -32000i64,
                    "HAMSTER_DB_PATH 未设置：desktop 工具仅在仓鼠Hub 注入的会话中可用".to_string(),
                )
            })?;
            self.db = Some(
                desktop::open_db(std::path::Path::new(&path))
                    .map_err(|e| (-32000i64, e.message().to_string()))?,
            );
        }
        Ok(self.db.as_mut().unwrap())
    }

    /// 审计落盘：每个 tools/call 一行 JSONL（失败静默——审计不能反过来影响工具语义）
    fn audit(&self, tool: &str, args: &Value, outcome: &Result<Vec<Content>, (i64, String)>) {
        let Some(path) = &self.audit_path else { return };
        let entry = json!({
            "ts": chrono_stamp(),
            "tool": tool,
            "args": args,
            "ok": outcome.is_ok(),
            "err": outcome.as_ref().err().map(|(_, m)| m),
        });
        use std::io::Write as _;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{entry}");
        }
    }
}

/// 工具清单（名称 + 描述 + 输入 schema）。
pub fn tool_definitions() -> Value {
    let obj = |name: &str, desc: &str, props: Value, required: Value| {
        json!({
            "name": name,
            "description": desc,
            "inputSchema": {
                "type": "object",
                "properties": props,
                "required": required,
            }
        })
    };
    json!([
        obj("browser_open", "启动浏览器并打开 URL（页面已存在时复用）",
            json!({ "url": { "type": "string", "description": "要打开的 http(s) 地址" } }),
            json!(["url"])),
        obj("browser_navigate", "当前标签页导航到 URL，返回页面标题",
            json!({ "url": { "type": "string" } }), json!(["url"])),
        obj("browser_snapshot", "页面可访问性树快照：缩进结构 + 交互元素 [ref] 编号（点击前必看）",
            json!({}), json!([])),
        obj("browser_click", "点击 snapshot 里的 [ref] 元素",
            json!({ "ref": { "type": "integer", "description": "snapshot 返回的 [ref] 编号" } }),
            json!(["ref"])),
        obj("browser_type", "向当前聚焦元素键入文本（先点击输入框；clear=true 先全选清空再输入）",
            json!({ "text": { "type": "string" }, "clear": { "type": "boolean", "description": "输入前清空已有内容（默认 false）" } }), json!(["text"])),
        obj("browser_press_key", "按键：Enter/Tab/Escape/ArrowUp/ArrowDown/ArrowLeft/ArrowRight/Backspace/Delete/Home/End/PageUp/PageDown/Space；支持组合键如 ctrl+a、ctrl+shift+tab",
            json!({ "key": { "type": "string" } }), json!(["key"])),
        obj("browser_screenshot", "页面截图；默认保存到截图目录并在返回中给出路径与预览",
            json!({ "name": { "type": "string", "description": "文件名（可选，默认时间戳）" } }),
            json!([])),
        obj("browser_close", "关闭浏览器并清理临时会话目录（server 退出时也会自动清理）",
            json!({}), json!([])),
        obj("computer_screenshot", "桌面全屏截图（覆盖浏览器外的场景）",
            json!({ "name": { "type": "string" } }), json!([])),
        obj("computer_click", "桌面绝对坐标点击（right=true 右键）",
            json!({ "x": { "type": "integer" }, "y": { "type": "integer" }, "right": { "type": "boolean" } }),
            json!(["x", "y"])),
        obj("computer_type", "桌面当前焦点键入文本",
            json!({ "text": { "type": "string" } }), json!(["text"])),
        obj("computer_key", "桌面按键（键表同 browser_press_key，支持 ctrl+c 等组合）",
            json!({ "key": { "type": "string" } }), json!(["key"])),
        // —— desktop 工具组（仓鼠Hub 本地能力）——
        obj("app_search", "按名称/拼音/首字母搜索本机已索引的应用（返回 app_key）",
            json!({ "query": { "type": "string" }, "limit": { "type": "integer", "description": "返回条数上限，默认 8" } }),
            json!(["query"])),
        obj("app_launch", "启动应用（app_search 返回的 app_key）",
            json!({ "app_key": { "type": "string" } }), json!(["app_key"])),
        obj("file_search", "按文件名/拼音搜索本机已索引的文件（返回完整路径）",
            json!({ "query": { "type": "string" }, "limit": { "type": "integer", "description": "返回条数上限，默认 10" } }),
            json!(["query"])),
        obj("file_open", "打开文件（仅限索引根目录白名单内的路径）",
            json!({ "path": { "type": "string" } }), json!(["path"])),
        obj("todo_list", "列出仓鼠Hub 待办（未完成在前）",
            json!({ "limit": { "type": "integer", "description": "默认 20" } }), json!([])),
        obj("todo_create", "新建一条待办（due_date 格式 YYYY-MM-DD，可省略）",
            json!({ "content": { "type": "string" }, "due_date": { "type": "string" } }),
            json!(["content"])),
        obj("todo_set_done", "勾选/取消待办（done=true 完成）",
            json!({ "id": { "type": "integer" }, "done": { "type": "boolean" } }),
            json!(["id", "done"])),
        obj("volume_get", "查询系统主音量（0.0-1.0 与静音状态）",
            json!({}), json!([])),
        obj("volume_set", "设置系统主音量（level 0.0-1.0；mute 可选，缺省保持当前静音状态）",
            json!({ "level": { "type": "number" }, "mute": { "type": "boolean" } }),
            json!(["level"])),
        // —— home 工具组（iOS 主屏布局：agent 自动整理图标）——
        obj("home_layout_get", "读取仓鼠Hub 主屏布局（version/wallpaper/pages 槽位/dock/folders；未配置返回空）。配合 home_apps_list 使用：先看现状再给整理方案",
            json!({}), json!([])),
        obj("home_apps_list", "列出本机已索引的全部应用（app_key/名称/类型/启动次数，高频在前）。整理主屏前必读：app_key 用于布局槽位，启动次数可决定哪些放首页/dock",
            json!({}), json!([])),
        obj("home_layout_set", "写入整理后的主屏布局并即时刷新（整体替换）。结构：{version:1, wallpaper, pages:[[槽位]]（单页≤35，槽位=\"app:应用key\"/\"folder:文件夹id\"/\"widget:组件类型\"）, dock:[原始appKey]（≤6）, folders:{文件夹id:{name,apps:[原始appKey]}}}。注意：槽位用 app: 前缀，dock 和文件夹 apps 用原始 key；不存在的应用会被自动剔除",
            json!({ "layout": { "type": "object", "description": "完整布局对象" } }),
            json!(["layout"])),
    ])
}

/// 处理一条 JSON-RPC 消息；通知类返回 None（不回帧）。
pub async fn handle_message(state: &Arc<Mutex<ServerState>>, msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(Value::as_str)?.to_string();
    let id = msg.get("id").cloned();
    let is_notification = id.is_none();

    let result: std::result::Result<Value, (i64, String)> = match method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
        })),
        "notifications/initialized" | "initialized" => {
            return None; // 通知：不回帧
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let name = msg
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let args = msg
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or(json!({}));
            call_tool(state, &name, &args)
                .await
                .map(|content| content_to_value(&content))
        }
        other => Err((-32601, format!("未知方法：{other}"))),
    };

    if is_notification {
        return None;
    }
    Some(match result {
        Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    })
}

async fn call_tool(
    state: &Arc<Mutex<ServerState>>,
    name: &str,
    args: &Value,
) -> std::result::Result<Vec<Content>, (i64, String)> {
    let outcome = call_tool_inner(state, name, args).await;
    // 审计在锁外读路径快照即可（audit_path 不变），锁内已释放
    state.lock().await.audit(name, args, &outcome);
    outcome
}

async fn call_tool_inner(
    state: &Arc<Mutex<ServerState>>,
    name: &str,
    args: &Value,
) -> std::result::Result<Vec<Content>, (i64, String)> {
    let arg = |k: &str| args.get(k).cloned();
    let arg_str = |k: &str| args.get(k).and_then(Value::as_str).map(String::from);
    let outcome: std::result::Result<Vec<Content>, (i64, String)> = match name {
        "browser_open" => {
            let url = arg_str("url").unwrap_or_default();
            let headless = arg("headless").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut st = state.lock().await;
            if st.browser.is_none() {
                st.browser = Some(
                    VerifyBrowser::open(&url, headless)
                        .await
                        .map_err(tool_error)?,
                );
                return Ok(vec![text_content(format!("浏览器已启动并打开 {url}"))]);
            }
            let b = st.browser.as_mut().unwrap();
            let title = b.navigate(&url).await.map_err(tool_error)?;
            Ok(vec![text_content(format!(
                "已导航到 {url}（标题：{title}）"
            ))])
        }
        "browser_navigate" => {
            let url = arg_str("url").unwrap_or_default();
            let mut st = state.lock().await;
            let b = browser_mut(&mut st)?;
            let title = b.navigate(&url).await.map_err(tool_error)?;
            Ok(vec![text_content(format!(
                "已导航到 {url}（标题：{title}）"
            ))])
        }
        "browser_snapshot" => {
            let mut st = state.lock().await;
            let b = browser_mut(&mut st)?;
            let snap = b.snapshot().await.map_err(tool_error)?;
            Ok(vec![text_content(snap)])
        }
        "browser_click" => {
            let r = arg("ref").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let mut st = state.lock().await;
            let b = browser_mut(&mut st)?;
            b.click(r).await.map_err(tool_error)?;
            Ok(vec![text_content(format!(
                "已点击 [ref {r}]。如需查看最新状态请再次 browser_snapshot。"
            ))])
        }
        "browser_type" => {
            let text = arg_str("text").unwrap_or_default();
            let clear = arg("clear").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut st = state.lock().await;
            let b = browser_mut(&mut st)?;
            b.type_text(&text, clear).await.map_err(tool_error)?;
            Ok(vec![text_content(format!(
                "已输入 {} 个字符{}",
                text.len(),
                if clear {
                    "（已先清空原内容）"
                } else {
                    ""
                }
            ))])
        }
        "browser_press_key" => {
            let key = arg_str("key").unwrap_or_default();
            let mut st = state.lock().await;
            let b = browser_mut(&mut st)?;
            b.press_key(&key).await.map_err(tool_error)?;
            Ok(vec![text_content(format!("已按键 {key}"))])
        }
        "browser_screenshot" => {
            let default_name = format!("shot-{}.png", chrono_stamp());
            let name = arg_str("name").unwrap_or(default_name);
            let path = state.lock().await.shots_dir().join(sanitize(&name));
            let (path_str, size) = {
                let mut st = state.lock().await;
                let b = browser_mut(&mut st)?;
                b.screenshot(&path).await.map_err(tool_error)?
            };
            Ok(shot_content(&path_str, &size))
        }
        "computer_screenshot" => {
            ensure_cu_allowed(state).await?;
            let default_name = format!("shot-{}.png", chrono_stamp());
            let name = arg_str("name").unwrap_or(default_name);
            let path = state.lock().await.shots_dir().join(sanitize(&name));
            let (path_str, size) = {
                let mut st = state.lock().await;
                if st.computer.is_none() {
                    st.computer = Some(Computer::new().map_err(tool_error)?);
                }
                let c = st.computer.as_mut().unwrap();
                c.screenshot(&path).map_err(tool_error)?
            };
            Ok(shot_content(&path_str, &size))
        }
        "browser_close" => {
            let mut st = state.lock().await;
            match st.browser.take() {
                Some(b) => {
                    b.close().await;
                    Ok(vec![text_content("浏览器已关闭，临时会话目录已清理")])
                }
                None => Ok(vec![text_content("浏览器未启动，无需关闭")]),
            }
        }
        "computer_click" => {
            ensure_cu_allowed(state).await?;
            let x = arg("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = arg("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let right = arg("right").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut st = state.lock().await;
            if st.computer.is_none() {
                st.computer = Some(Computer::new().map_err(tool_error)?);
            }
            let c = st.computer.as_mut().unwrap();
            c.click(x, y, right).map_err(tool_error)?;
            Ok(vec![text_content(format!("已在 ({x},{y}) 点击"))])
        }
        "computer_type" => {
            ensure_cu_allowed(state).await?;
            let text = arg_str("text").unwrap_or_default();
            let mut st = state.lock().await;
            if st.computer.is_none() {
                st.computer = Some(Computer::new().map_err(tool_error)?);
            }
            let c = st.computer.as_mut().unwrap();
            c.type_text(&text).map_err(tool_error)?;
            Ok(vec![text_content(format!("已输入 {} 个字符", text.len()))])
        }
        "computer_key" => {
            ensure_cu_allowed(state).await?;
            let key = arg_str("key").unwrap_or_default();
            let mut st = state.lock().await;
            if st.computer.is_none() {
                st.computer = Some(Computer::new().map_err(tool_error)?);
            }
            let c = st.computer.as_mut().unwrap();
            c.press_key(&key).map_err(tool_error)?;
            Ok(vec![text_content(format!("已按键 {key}"))])
        }
        // —— desktop 工具组 ——
        "app_search" => {
            let q = arg_str("query").unwrap_or_default();
            let limit = arg("limit").and_then(|v| v.as_i64()).unwrap_or(8);
            let mut st = state.lock().await;
            let hits = desktop::app_search(st.db_mut()?, &q, limit).map_err(tool_error)?;
            Ok(vec![text_content(app_hits_text(&q, &hits))])
        }
        "app_launch" => {
            let key = arg_str("app_key").unwrap_or_default();
            let mut st = state.lock().await;
            let target = desktop::app_launch(st.db_mut()?, &key).map_err(tool_error)?;
            Ok(vec![text_content(format!("已启动：{target}"))])
        }
        "file_search" => {
            let q = arg_str("query").unwrap_or_default();
            let limit = arg("limit").and_then(|v| v.as_i64()).unwrap_or(10);
            let mut st = state.lock().await;
            let hits = desktop::file_search(st.db_mut()?, &q, limit).map_err(tool_error)?;
            Ok(vec![text_content(file_hits_text(&q, &hits))])
        }
        "file_open" => {
            let path = arg_str("path").unwrap_or_default();
            let mut st = state.lock().await;
            desktop::file_open(st.db_mut()?, &path).map_err(tool_error)?;
            Ok(vec![text_content(format!("已打开：{path}"))])
        }
        "todo_list" => {
            let limit = arg("limit").and_then(|v| v.as_i64()).unwrap_or(20);
            let mut st = state.lock().await;
            let items = desktop::todo_list(st.db_mut()?, limit).map_err(tool_error)?;
            if items.is_empty() {
                return Ok(vec![text_content("待办列表为空")]);
            }
            let body = items
                .iter()
                .map(|t| {
                    format!(
                        "#{} [{}] {}{}",
                        t.id,
                        if t.done { "已完成" } else { "未完成" },
                        t.content,
                        t.due_date
                            .as_deref()
                            .map(|d| format!("（截止 {d}）"))
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            Ok(vec![text_content(body)])
        }
        "todo_create" => {
            let content = arg_str("content").unwrap_or_default();
            let due = arg_str("due_date").filter(|s| !s.trim().is_empty());
            let mut st = state.lock().await;
            let id =
                desktop::todo_create(st.db_mut()?, &content, due.as_deref()).map_err(tool_error)?;
            Ok(vec![text_content(format!(
                "已创建待办 #{id}：{}{}",
                content.trim(),
                due.map(|d| format!("（截止 {d}）")).unwrap_or_default()
            ))])
        }
        "todo_set_done" => {
            let id = arg("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let done = arg("done").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut st = state.lock().await;
            desktop::todo_set_done(st.db_mut()?, id, done).map_err(tool_error)?;
            Ok(vec![text_content(format!(
                "待办 #{id} 已标记为{}",
                if done { "已完成" } else { "未完成" }
            ))])
        }
        "volume_get" => {
            let (level, muted) = desktop::volume_get().map_err(tool_error)?;
            Ok(vec![text_content(format!(
                "主音量：{:.0}%{}",
                level * 100.0,
                if muted { "（已静音）" } else { "" }
            ))])
        }
        "volume_set" => {
            let level = arg("level").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let mute = arg("mute").and_then(|v| v.as_bool());
            desktop::volume_set(level, mute).map_err(tool_error)?;
            Ok(vec![text_content(format!(
                "主音量已设为 {:.0}%{}",
                level.clamp(0.0, 1.0) * 100.0,
                mute.map(|m| if m {
                    "（静音）"
                } else {
                    "（取消静音）"
                })
                .unwrap_or("")
            ))])
        }
        // —— home 工具组（主屏布局；写入口在承载层广播刷新事件）——
        "home_layout_get" => {
            let mut st = state.lock().await;
            match home::layout_get(st.db_mut()?).map_err(tool_error)? {
                Some(raw) => Ok(vec![text_content(raw)]),
                None => Ok(vec![text_content(
                    "主屏尚未配置布局（当前使用默认布局）。可用 home_layout_set 写入整理结果。",
                )]),
            }
        }
        "home_apps_list" => {
            let mut st = state.lock().await;
            let apps = home::apps_list(st.db_mut()?).map_err(tool_error)?;
            if apps.is_empty() {
                return Ok(vec![text_content("应用索引为空（尚未完成索引扫描）")]);
            }
            let body = apps
                .iter()
                .map(|a| {
                    format!(
                        "{}（{}，启动 {} 次）\n  app_key: {}",
                        a.display_name, a.kind, a.use_count, a.app_key
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            Ok(vec![text_content(format!(
                "共 {} 个应用（高频在前）：\n{body}",
                apps.len()
            ))])
        }
        "home_layout_set" => {
            let layout = arg("layout").unwrap_or(Value::Null);
            let mut st = state.lock().await;
            home::layout_set(st.db_mut()?, &layout.to_string()).map_err(tool_error)?;
            Ok(vec![text_content(
                "布局已写入，主屏已刷新。建议 home_layout_get 复核一遍。",
            )])
        }
        other => Err((-32602, format!("未知工具：{other}"))),
    };
    outcome
}

/// computer use 安全门：默认拒绝（设置显式开启后放行）
async fn ensure_cu_allowed(
    state: &Arc<Mutex<ServerState>>,
) -> std::result::Result<(), (i64, String)> {
    if state.lock().await.cu_allowed {
        return Ok(());
    }
    Err((
        -32001,
        "computer use 未开启：请在仓鼠Hub 设置 → Agent → 允许操作电脑中显式开启后重试".to_string(),
    ))
}

fn app_hits_text(query: &str, hits: &[desktop::AppHit]) -> String {
    if hits.is_empty() {
        return format!("未找到匹配「{query}」的应用");
    }
    let body = hits
        .iter()
        .map(|h| format!("{}（app_key: {}）", h.display_name, h.app_key))
        .collect::<Vec<_>>()
        .join("\n");
    format!("找到 {} 个应用：\n{body}", hits.len())
}

fn file_hits_text(query: &str, hits: &[desktop::FileHit]) -> String {
    if hits.is_empty() {
        return format!("未找到匹配「{query}」的文件");
    }
    let body = hits
        .iter()
        .map(|h| format!("{}（目录：{}）\n  完整路径：{}", h.name, h.dir, h.path))
        .collect::<Vec<_>>()
        .join("\n");
    format!("找到 {} 个文件：\n{body}", hits.len())
}

/// 工具执行内容（text 或 image）。
pub enum Content {
    Text(String),
    Image { data: String, mime_type: String },
}

pub fn text_content(text: impl Into<String>) -> Content {
    Content::Text(text.into())
}

fn content_to_value(content: &[Content]) -> Value {
    let items: Vec<Value> = content
        .iter()
        .map(|c| match c {
            Content::Text(t) => json!({ "type": "text", "text": t }),
            Content::Image { data, mime_type } => json!({
                "type": "image", "data": data, "mimeType": mime_type
            }),
        })
        .collect();
    json!({ "content": items, "isError": false })
}

/// 截图统一回包：路径 + 尺寸文本，正文以 image content 返回（agent 可直接看图）。
fn shot_content(path: &std::path::Path, size: &str) -> Vec<Content> {
    let bytes = std::fs::read(path).unwrap_or_default();
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    vec![
        text_content(format!("截图已保存：{}\n尺寸：{}", path.display(), size)),
        Content::Image {
            data: b64,
            mime_type: "image/png".into(),
        },
    ]
}

fn browser_mut(st: &mut ServerState) -> std::result::Result<&mut VerifyBrowser, (i64, String)> {
    st.browser
        .as_mut()
        .ok_or_else(|| (-32602, "浏览器未启动：先调用 browser_open".to_string()))
}

fn tool_error(e: hamster_core::HamsterError) -> (i64, String) {
    (-32000, e.message())
}

fn sanitize(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !s.ends_with(".png") {
        s.push_str(".png");
    }
    s
}

fn chrono_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn call(state: &Arc<Mutex<ServerState>>, method: &str, params: Value) -> Value {
        let msg = json!({
            "jsonrpc": "2.0", "id": 1,
            "method": method,
            "params": params
        });
        handle_message(state, &msg).await.expect("应有响应帧")
    }

    #[test]
    fn sanitize_keeps_safe_filename_and_appends_png() {
        assert_eq!(sanitize("shot"), "shot.png");
        assert_eq!(sanitize("a.png"), "a.png");
        assert_eq!(sanitize("a b/c:d.png"), "a_b_c_d.png");
    }

    #[tokio::test]
    async fn initialize_returns_protocol_info() {
        let state = Arc::new(Mutex::new(ServerState::new()));
        let resp = call(&state, "initialize", json!({})).await;
        assert_eq!(
            resp.pointer("/result/protocolVersion")
                .and_then(Value::as_str),
            Some(PROTOCOL_VERSION)
        );
        assert_eq!(
            resp.pointer("/result/serverInfo/name")
                .and_then(Value::as_str),
            Some(SERVER_NAME)
        );
    }

    #[tokio::test]
    async fn notifications_return_no_frame() {
        let state = Arc::new(Mutex::new(ServerState::new()));
        let msg = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(handle_message(&state, &msg).await.is_none());
    }

    #[tokio::test]
    async fn tools_list_contains_all_tools() {
        let state = Arc::new(Mutex::new(ServerState::new()));
        let resp = call(&state, "tools/list", json!({})).await;
        let tools = resp
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .expect("tools 数组");
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(Value::as_str))
            .collect();
        assert_eq!(names.len(), 24, "工具总数应为 24：{names:?}");
        for expected in [
            "browser_open",
            "browser_navigate",
            "browser_snapshot",
            "browser_click",
            "browser_type",
            "browser_press_key",
            "browser_screenshot",
            "browser_close",
            "computer_screenshot",
            "computer_click",
            "computer_type",
            "computer_key",
            "app_search",
            "app_launch",
            "file_search",
            "file_open",
            "todo_list",
            "todo_create",
            "todo_set_done",
            "volume_get",
            "volume_set",
            "home_layout_get",
            "home_apps_list",
            "home_layout_set",
        ] {
            assert!(names.contains(&expected), "缺少工具 {expected}");
        }
    }

    #[tokio::test]
    async fn computer_tools_denied_by_default() {
        // 默认（未设置 HAMSTER_CU_MODE）全部 computer_* 拒绝，错误码 -32001
        let state = Arc::new(Mutex::new(ServerState::default()));
        for (name, args) in [
            ("computer_screenshot", json!({})),
            ("computer_click", json!({ "x": 1, "y": 1 })),
            ("computer_type", json!({ "text": "hi" })),
            ("computer_key", json!({ "key": "Enter" })),
        ] {
            let resp = call(
                &state,
                "tools/call",
                json!({ "name": name, "arguments": args }),
            )
            .await;
            assert_eq!(
                resp.pointer("/error/code").and_then(Value::as_i64),
                Some(-32001),
                "{name} 未开启时应被安全门拒绝"
            );
        }
        // 显式放行后不再被安全门拦截（点击会真动鼠标，这里只验证门逻辑放行分支，
        // 用 cu_allowed=true + 未初始化 Computer 的报错路径区分于安全门错误码）
        state.lock().await.cu_allowed = true;
        let resp = call(
            &state,
            "tools/call",
            json!({ "name": "computer_screenshot", "arguments": {} }),
        )
        .await;
        let code = resp.pointer("/error/code").and_then(Value::as_i64);
        assert_ne!(code, Some(-32001), "开启后不应再被安全门拦截：{resp}");
    }

    #[tokio::test]
    async fn desktop_todo_roundtrip_via_mcp() {
        let state = Arc::new(Mutex::new(ServerState::default()));
        // 直接注入内存库（免 env 依赖），走完整 JSON-RPC 臂
        state.lock().await.db = Some(crate::desktop_test_db());
        let resp = call(
            &state,
            "tools/call",
            json!({ "name": "todo_create", "arguments": { "content": "给仓鼠添粮" } }),
        )
        .await;
        assert!(resp.get("error").is_none(), "{resp}");
        let resp = call(
            &state,
            "tools/call",
            json!({ "name": "todo_list", "arguments": {} }),
        )
        .await;
        let text = resp
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(text.contains("给仓鼠添粮"), "{text}");
    }

    #[tokio::test]
    async fn unknown_method_and_tool_error() {
        let state = Arc::new(Mutex::new(ServerState::new()));
        let resp = call(&state, "no/such/method", json!({})).await;
        assert_eq!(
            resp.pointer("/error/code").and_then(Value::as_i64),
            Some(-32601)
        );

        let resp = call(
            &state,
            "tools/call",
            json!({ "name": "no_such_tool", "arguments": {} }),
        )
        .await;
        assert_eq!(
            resp.pointer("/error/code").and_then(Value::as_i64),
            Some(-32602)
        );
    }

    #[tokio::test]
    async fn browser_tools_require_open_first() {
        let state = Arc::new(Mutex::new(ServerState::new()));
        for name in ["browser_navigate", "browser_snapshot", "browser_screenshot"] {
            let resp = call(
                &state,
                "tools/call",
                json!({ "name": name, "arguments": {} }),
            )
            .await;
            assert_eq!(
                resp.pointer("/error/code").and_then(Value::as_i64),
                Some(-32602),
                "{name} 未启动浏览器应报参数错误"
            );
        }
        // 关闭未启动的浏览器是幂等成功，不是错误
        let resp = call(&state, "tools/call", json!({ "name": "browser_close" })).await;
        assert!(resp.get("result").is_some());
        assert!(resp.pointer("/error").is_none());
    }

    #[tokio::test]
    async fn home_layout_roundtrip_via_mcp() {
        let state = Arc::new(Mutex::new(ServerState::default()));
        state.lock().await.db = Some(crate::desktop_test_db());
        let layout = json!({
            "version": 1, "wallpaper": "midnight",
            "pages": [["app:C:\\lnk\\wechat.lnk", "widget:clock"]],
            "dock": [], "folders": {}
        });
        let resp = call(
            &state,
            "tools/call",
            json!({ "name": "home_layout_set", "arguments": { "layout": layout } }),
        )
        .await;
        assert!(resp.get("error").is_none(), "{resp}");
        let resp = call(
            &state,
            "tools/call",
            json!({ "name": "home_layout_get", "arguments": {} }),
        )
        .await;
        let text = resp
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(text.contains("midnight"), "{text}");
        assert!(text.contains("wechat"), "{text}");
        // 非法布局：本 server 的工具错误走 JSON-RPC error 帧（-32000），消息可自修正
        let resp = call(
            &state,
            "tools/call",
            json!({ "name": "home_layout_set", "arguments": { "layout": { "version": 2 } } }),
        )
        .await;
        assert_eq!(
            resp.pointer("/error/code").and_then(Value::as_i64),
            Some(-32000)
        );
        assert!(
            resp.pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .contains("version"),
            "{resp}"
        );
    }
}
