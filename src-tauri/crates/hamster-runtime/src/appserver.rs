//! 结构化流式通道：官方 headless 协议宿主（v2.0 R3，对齐 t3code）。
//!
//! 分层约定（AGENTS.md §2 依赖铁律）：本模块只认「协议方言」不认 Agent——
//! Agent → 方言的映射由 Adapter 以 `StructuredChannel` 纯数据声明。
//! 红线①：官方协议 = 官方支持的使用方式（runtime-design §8 允许清单延伸）。
//!
//! codex app-server v2 协议事实（实测 0.144.5，2026-08-31）：
//! - stdio 上每行一个 JSON-RPC 2.0 消息（非 LSP 头帧）
//! - 握手：initialize{clientInfo} → 响应；随后可选发 initialized 通知
//! - thread/start{cwd,model?} → result.thread.id；通知 thread/started
//! - turn/start{threadId,input:[{type:"text",text}],model?,effort?}
//!   → 通知 turn/started + item/* 流 + turn/completed
//! - turn/interrupt{threadId,turnId}（turnId 取自 turn/started）
//! - 关键通知：item/agentMessage/delta、item/reasoning/textDelta、
//!   item/started、item/completed{item:{type,...}}、turn/completed、error

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use hamster_core::error::{HamsterError, Result};
use hamster_core::{
    LiveStreamInfo, StreamConfigChoice, StreamConfigOption, StreamEvent, StreamEventDiff,
    StreamEventKind, StructuredChannel,
};

/// 事件回调（归一化后；转 Tauri Channel / 测试收集）。
pub type EventCallback = Box<dyn Fn(&StreamEvent) + Send + Sync>;
/// 通道进程退出回调 (session_id)。
pub type StreamExitCallback = Box<dyn Fn(&str) + Send + Sync>;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn lock_ok<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// claude 方言的 resume/fork 启动 flag（stream-json 无握手，续聊只能靠 argv 注入）。
/// 无 resume_key 时无从派生：fork 降级为新会话（与 codex None→thread/start 一致）。
pub fn claude_resume_flags(resume_key: Option<&str>, fork: bool) -> Vec<String> {
    let Some(key) = resume_key.map(str::trim).filter(|k| !k.is_empty()) else {
        return Vec::new();
    };
    let mut flags = vec!["--resume".to_string(), key.to_string()];
    if fork {
        flags.push("--fork-session".to_string());
    }
    flags
}

/// 欢迎屏动态模型清单探测（§11.5）：官方协议握手（initialize → session/new /
/// thread/start）后立即结束进程，返回 agent 声明的会话配置项（model/effort
/// 真实清单）。不发 prompt——不产生模型调用；session/new 会留一条空会话记录，
/// 属官方接口的正常使用。codex/claude 方言无 configOptions（返回空），
/// 它们的模型清单由 Adapter 静态声明。
pub fn probe_config_options(opts: StreamOptions) -> Result<Vec<StreamConfigOption>> {
    let session = StreamSession::start(opts)?;
    let options = lock_ok(&session.state).config_options.clone();
    let _ = session.kill();
    Ok(options)
}

pub struct StreamOptions {
    pub agent_id: String,
    pub project_dir: std::path::PathBuf,
    pub channel: StructuredChannel,
    /// 续聊：方言侧会话标识（codex threadId / claude sessionId）；None = 新会话
    pub resume_key: Option<String>,
    /// fork 语义：以 resume_key 为源派生新会话（原会话不动）
    /// codex = thread/fork；claude = --resume + --fork-session
    pub fork: bool,
    /// 启动级模型覆盖（claude 的 --model 是 spawn flag；codex 为 turn 参数）
    pub model: Option<String>,
    /// 完整启动命令（program + channel.args）
    pub program: String,
    pub args: Vec<String>,
    /// ACP 会话级 MCP 注入（session/new、session/load 的 mcpServers 参数；
    /// claude 方言走 spawn flag 不经此字段）。空 = 不注入（默认行为不变）。
    /// Hamster vendor 扩展：桌面助手会话经此注入 hamster-desktop MCP server。
    pub mcp_servers: Vec<serde_json::Value>,
    pub on_event: EventCallback,
    pub on_exit: StreamExitCallback,
}

struct StreamState {
    thread_id: String,
    turn_id: Option<String>,
    running: bool,
    turns: i64,
    started_at: i64,
    last_active_at: i64,
    /// ACP：进行中 `session/prompt` 的请求 id（响应 = 整轮结束）
    prompt_request_id: Option<i64>,
    /// ACP：agent 声明的 loadSession 能力（续聊门控）
    load_session: bool,
    /// ACP：session/new 应答的 configOptions 归一化（codex/claude 为空）
    config_options: Vec<StreamConfigOption>,
}

/// 一个流式会话：一个官方协议子进程 + 一个 thread。
pub struct StreamSession {
    session_id: String,
    agent_id: String,
    project_dir: std::path::PathBuf,
    resume_key: Option<String>,
    dialect: hamster_core::ProtocolDialect,
    state: Mutex<StreamState>,
    pending: Mutex<HashMap<i64, Arc<std::sync::mpsc::Sender<String>>>>,
    next_id: AtomicI64,
    child: Mutex<Child>,
    stdin: Mutex<Option<std::process::ChildStdin>>,
    /// 读线程之外的主动事件出口（ACP 无 turn/started 通知，发轮时即时补 TurnStarted）
    on_event: EventCallback,
}

impl StreamSession {
    /// spawn 协议进程并完成 initialize + thread/start 握手。
    pub fn start(opts: StreamOptions) -> Result<Arc<Self>> {
        // claude 方言：.cmd shim（npm 转发器）在 Rust 管道 spawn 下会静默挂起
        // （实测 2.1.251）——解析到同包原生 claude.exe 直接启动
        let program = if opts.channel.dialect == hamster_core::ProtocolDialect::ClaudeStream {
            resolve_native(&opts.program).unwrap_or_else(|| opts.program.clone())
        } else {
            opts.program.clone()
        };
        let mut cmd = Command::new(&program);
        if std::env::var("HAMSTER_STREAM_DEBUG").ok().as_deref() == Some("1") {
            eprintln!("[stream-spawn] program={program} args={:?}", opts.args);
        }
        cmd.args(&opts.args)
            .current_dir(&opts.project_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // claude 方言的 resume/fork 是启动 flag（无握手可走），必须 spawn 前合入；
        // codex/ACP 的续聊走下方协议层 thread/resume、session/load
        if opts.channel.dialect == hamster_core::ProtocolDialect::ClaudeStream {
            cmd.args(claude_resume_flags(opts.resume_key.as_deref(), opts.fork));
        }
        let mut child = cmd.spawn().map_err(|e| HamsterError::Launch {
            program: program.clone(),
            message: format!("启动结构化通道失败：{e}"),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| HamsterError::Launch {
            program: opts.program.clone(),
            message: "通道进程无 stdout".into(),
        })?;
        let stdin = child.stdin.take().ok_or_else(|| HamsterError::Launch {
            program: opts.program.clone(),
            message: "通道进程无 stdin".into(),
        })?;
        let stderr = child.stderr.take();

        // cwd 先行提取（后续 opts 整体 move 进 session）
        let cwd = opts.project_dir.display().to_string().replace('/', "\\");
        let program_name = opts.program.clone();

        let session = Arc::new(Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: opts.agent_id,
            project_dir: opts.project_dir.clone(),
            resume_key: opts.resume_key.clone(),
            dialect: opts.channel.dialect,
            state: Mutex::new(StreamState {
                thread_id: String::new(),
                turn_id: None,
                running: true,
                turns: 0,
                started_at: now_ms(),
                last_active_at: now_ms(),
                prompt_request_id: None,
                load_session: false,
                config_options: Vec::new(),
            }),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicI64::new(1),
            child: Mutex::new(child),
            stdin: Mutex::new(Some(stdin)),
            on_event: opts.on_event,
        });

        spawn_reader_thread(&session, stdout, opts.on_exit)?;
        if let Some(err_out) = stderr {
            let debug_raw = std::env::var("HAMSTER_STREAM_DEBUG").ok().as_deref() == Some("1");
            std::thread::Builder::new()
                .name(format!("stream-stderr-{}", session.session_id))
                .spawn(move || {
                    let reader = std::io::BufReader::new(err_out);
                    for line in reader.lines() {
                        let Ok(line) = line else { break };
                        let t = line.trim();
                        if !t.is_empty()
                            && (debug_raw || t.contains("error") || t.contains("Error"))
                        {
                            eprintln!("[stream-stderr] {t}");
                        }
                    }
                })
                .ok();
        }

        if session.dialect == hamster_core::ProtocolDialect::ClaudeStream {
            // claude stream-json：无握手；resume/fork 已于 spawn 前注入 argv；
            // session_id 由 system/init 事件异步回填（读线程处理）
            return Ok(session);
        }

        // —— ACP 握手（runtime-design §11.5）：initialize → session/new|session/load ——
        if session.dialect == hamster_core::ProtocolDialect::Acp {
            let init = session.request(
                "initialize",
                serde_json::json!({
                    "protocolVersion": 1,
                    "clientCapabilities": { "fs": { "readTextFile": false, "writeTextFile": false } }
                }),
            )?;
            let load_session = init
                .get("agentCapabilities")
                .and_then(|a| a.get("loadSession"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            lock_ok(&session.state).load_session = load_session;

            if opts.fork {
                return Err(HamsterError::Unsupported(
                    "ACP 协议无 fork：请直接开新会话".into(),
                ));
            }
            let (thread_id, config_options) = match opts
                .resume_key
                .as_deref()
                .map(str::trim)
                .filter(|k| !k.is_empty())
            {
                Some(key) => {
                    if !load_session {
                        return Err(HamsterError::Unsupported(
                            "该 Agent 的 ACP 不支持续聊（loadSession=false）".into(),
                        ));
                    }
                    let resp = session.request(
                        "session/load",
                        serde_json::json!({ "sessionId": key, "cwd": cwd, "mcpServers": opts.mcp_servers }),
                    )?;
                    let tid = resp
                        .get("sessionId")
                        .and_then(|i| i.as_str())
                        .unwrap_or(key)
                        .to_string();
                    (tid, parse_acp_config_options(&resp))
                }
                None => {
                    let resp = session.request(
                        "session/new",
                        serde_json::json!({ "cwd": cwd, "mcpServers": opts.mcp_servers }),
                    )?;
                    let tid = resp
                        .get("sessionId")
                        .and_then(|i| i.as_str())
                        .unwrap_or_default()
                        .to_string();
                    (tid, parse_acp_config_options(&resp))
                }
            };
            if thread_id.is_empty() {
                return Err(HamsterError::Launch {
                    program: program_name,
                    message: format!("session/new 未返回 sessionId：{init}"),
                });
            }
            {
                let mut st = lock_ok(&session.state);
                st.thread_id = thread_id;
                st.config_options = config_options;
            }
            // GUI 流式的模型覆盖（§11.6）：会话就绪后、首轮 prompt 前经
            // session/set_config_option 应用；agent 校验失败即启动失败
            if let Some(m) = opts
                .model
                .as_deref()
                .map(str::trim)
                .filter(|m| !m.is_empty())
            {
                session.apply_model_override(m)?;
            }
            return Ok(session);
        }

        // —— codex 方言握手 ——
        // 握手：initialize（clientInfo 为协议要求的最小字段）
        let init = session.request(
            "initialize",
            serde_json::json!({ "clientInfo": { "name": "上游", "version": "0.1.0" } }),
        )?;
        let _ = init; // serverInfo/codexHome，暂不消费
        session.notify("initialized", serde_json::json!({}))?;

        // thread/start（新会话）/ thread/resume{threadId}（续聊）/
        // thread/fork{threadId}（派生新会话，协议实测 0.144.5）
        let resp = match opts
            .resume_key
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
        {
            Some(thread_id) if opts.fork => {
                session.request("thread/fork", serde_json::json!({ "threadId": thread_id }))?
            }
            Some(thread_id) => session.request(
                "thread/resume",
                serde_json::json!({ "threadId": thread_id }),
            )?,
            None => session.request("thread/start", serde_json::json!({ "cwd": cwd }))?,
        };
        let thread_id = resp
            .get("thread")
            .and_then(|t| t.get("id"))
            .and_then(|i| i.as_str())
            .unwrap_or_default()
            .to_string();
        if thread_id.is_empty() {
            return Err(HamsterError::Launch {
                program: program_name,
                message: format!("thread/start 未返回 thread.id：{resp}"),
            });
        }
        lock_ok(&session.state).thread_id = thread_id;
        Ok(session)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn thread_id(&self) -> String {
        lock_ok(&self.state).thread_id.clone()
    }

    pub fn is_running(&self) -> bool {
        lock_ok(&self.state).running
    }

    pub fn info(&self) -> LiveStreamInfo {
        let st = lock_ok(&self.state);
        LiveStreamInfo {
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
            project_dir: self.project_dir.display().to_string(),
            thread_id: st.thread_id.clone(),
            running: st.running,
            turns: st.turns,
            started_at: st.started_at,
            last_active_at: st.last_active_at,
            resume_key: self.resume_key.clone(),
            config_options: st.config_options.clone(),
        }
    }

    /// 发起一轮对话（input 为 [{type:"text",text}]；model/effort 为官方覆盖项）。
    pub fn send_turn(&self, text: &str, model: Option<&str>, effort: Option<&str>) -> Result<()> {
        if !self.is_running() {
            return Err(HamsterError::Other("会话已结束，无法发送".into()));
        }
        // claude：发轮次只需 stdin 用户行；thread_id 校验仅 codex 需要
        let thread_id = self.thread_id();
        if self.dialect != hamster_core::ProtocolDialect::ClaudeStream && thread_id.is_empty() {
            return Err(HamsterError::Other("通道尚未就绪（无 thread）".into()));
        }
        if self.dialect == hamster_core::ProtocolDialect::ClaudeStream {
            // claude：用户轮次是一行 JSONL 写入 stdin（--input-format stream-json）
            let user = serde_json::json!({
                "type": "user",
                "message": { "role": "user", "content": [{ "type": "text", "text": text }] },
            });
            return self.write_json(&user);
        }
        if self.dialect == hamster_core::ProtocolDialect::Acp {
            // ACP：session/prompt 的响应在整轮结束时才返回——写入后立即返回不等
            // （模型覆盖已在会话创建时经 set_config_option 应用；effort 无对应，忽略）。
            // TurnStarted 由本函数即时发出（ACP 无 turn/started 通知）。
            let id = self.next_id.fetch_add(1, Ordering::SeqCst);
            lock_ok(&self.state).prompt_request_id = Some(id);
            self.write_json(&serde_json::json!({
                "jsonrpc": "2.0", "id": id, "method": "session/prompt",
                "params": {
                    "sessionId": thread_id,
                    "prompt": [{ "type": "text", "text": text }],
                }
            }))?;
            (self.on_event)(&StreamEvent {
                session_id: self.session_id.clone(),
                item_id: String::new(),
                at: now_ms(),
                kind: StreamEventKind::TurnStarted,
                text: String::new(),
                tool_name: None,
                status: None,
                diff: None,
            });
            return Ok(());
        }
        let mut params = serde_json::json!({
            "threadId": thread_id,
            "input": [{ "type": "text", "text": text }],
        });
        if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
            params["model"] = serde_json::json!(m.trim());
        }
        if let Some(e) = effort.filter(|e| !e.trim().is_empty()) {
            params["effort"] = serde_json::json!(e.trim());
        }
        // turn/start 是 Request（必须带 id，实测通知会被忽略）；响应为空载荷且立即返回
        let _ = self.request("turn/start", params)?;
        Ok(())
    }

    /// 停止当前轮。codex：turn/interrupt；claude：print 模式无中断协议，
    /// 结束进程（--resume sessionId 可继续，会话已持久化）；ACP：session/cancel
    /// 通知（无响应）——agent 随后对挂起 prompt 回 stopReason="cancelled"，
    /// 读线程据此收轮。
    pub fn interrupt(&self) -> Result<()> {
        if self.dialect == hamster_core::ProtocolDialect::ClaudeStream {
            return self.kill();
        }
        if self.dialect == hamster_core::ProtocolDialect::Acp {
            let pending = lock_ok(&self.state).prompt_request_id.is_some();
            if !pending {
                return Ok(()); // 无进行中轮次：无害成功
            }
            let thread_id = lock_ok(&self.state).thread_id.clone();
            return self.notify(
                "session/cancel",
                serde_json::json!({ "sessionId": thread_id }),
            );
        }
        let st = lock_ok(&self.state);
        let turn_id = match &st.turn_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return Ok(()), // 无进行中轮次：无害成功
        };
        let thread_id = st.thread_id.clone();
        drop(st);
        self.request(
            "turn/interrupt",
            serde_json::json!({ "threadId": thread_id, "turnId": turn_id }),
        )
        .map(|_| ())
    }

    /// 会话级配置项修改（ACP，§11.5）：mode 走官方 `session/set_mode`
    /// （实测 opencode 1.18.x 生效，plan 模式拒写验证通过）；其余项走 ACP
    /// schema 正名方法 `session/set_config_option`——此前误调
    /// `session/set_config`（不存在的方法名，opencode 报 Method not found，
    /// 曾被误记为上游 #31750 未实现）；2026-09-06 实测 set_config_option 在
    /// opencode 1.18.3 切换 model 生效（T3 Code 同款用法）。应答回带最新
    /// configOptions，据此同步本地 current_value。
    pub fn set_config(&self, config_id: &str, value: &str) -> Result<()> {
        let thread_id = lock_ok(&self.state).thread_id.clone();
        if thread_id.is_empty() {
            return Err(HamsterError::Other("通道尚未就绪（无 thread）".into()));
        }
        if config_id == "mode" {
            self.request(
                "session/set_mode",
                serde_json::json!({ "sessionId": thread_id, "modeId": value }),
            )?;
        } else {
            let resp = self.request(
                "session/set_config_option",
                serde_json::json!({ "sessionId": thread_id, "configId": config_id, "value": value }),
            )?;
            // 应答携带全量 configOptions：按 agent 权威值刷新本地态
            let fresh = parse_acp_config_options(&resp);
            if !fresh.is_empty() {
                lock_ok(&self.state).config_options = fresh;
            }
        }
        if let Some(c) = lock_ok(&self.state)
            .config_options
            .iter_mut()
            .find(|c| c.id == config_id)
        {
            c.current_value = value.to_string();
        }
        Ok(())
    }

    /// 会话创建后、首轮前的模型覆盖（ACP）：目标配置项由 agent 在
    /// configOptions 里声明（category=="model"，兜底 id=="model"），未声明
    /// 即显式报错。agent 校验失败（如未知模型值）原样透传为启动失败。
    fn apply_model_override(&self, model: &str) -> Result<()> {
        let config_id = lock_ok(&self.state)
            .config_options
            .iter()
            .find(|c| c.category.as_deref() == Some("model") || c.id == "model")
            .map(|c| c.id.clone());
        match config_id {
            Some(id) => self.set_config(&id, model),
            None => Err(HamsterError::Other(format!(
                "该 Agent 未声明模型配置项，无法为会话指定模型 {model}"
            ))),
        }
    }

    /// 终止通道进程（返回时已确认退出或兜底等待）。
    pub fn kill(&self) -> Result<()> {
        // Windows：先按进程树强杀（cmd shim → node → agent.exe 的孙进程不随
        // 直接子进程死亡，孤儿会继续持有 codex 的 thread writer 锁）
        #[cfg(windows)]
        {
            let pid = lock_ok(&self.child).id();
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .creation_flags(0x0800_0000) // CREATE_NO_WINDOW：不闪黑框
                .status();
        }
        {
            let mut child = lock_ok(&self.child);
            let _ = child.kill();
        }
        lock_ok(&self.state).running = false;
        for _ in 0..20 {
            if lock_ok(&self.child)
                .try_wait()
                .map(|w| w.is_some())
                .unwrap_or(true)
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }

    /// 发请求并等待响应（读线程回填 pending 通道）。冷启动可达数秒——由调用方超时控制。
    fn request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        lock_ok(&self.pending).insert(id, Arc::new(tx));
        self.write_json(&serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params
        }))?;
        match rx.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(raw) => {
                let v: serde_json::Value = serde_json::from_str(&raw)
                    .map_err(|e| HamsterError::Other(format!("协议响应解析失败：{e}")))?;
                if let Some(err) = v.get("error") {
                    let _ = lock_ok(&self.pending).remove(&id);
                    return Err(HamsterError::Other(format!(
                        "{method} 失败：{}",
                        err.get("message").and_then(|m| m.as_str()).unwrap_or("?")
                    )));
                }
                Ok(v.get("result").cloned().unwrap_or(serde_json::Value::Null))
            }
            Err(_) => {
                lock_ok(&self.pending).remove(&id);
                Err(HamsterError::Other(format!("{method} 响应超时（30s）")))
            }
        }
    }

    fn notify(&self, method: &str, params: serde_json::Value) -> Result<()> {
        self.write_json(&serde_json::json!({
            "jsonrpc": "2.0", "method": method, "params": params
        }))
    }

    fn write_json(&self, msg: &serde_json::Value) -> Result<()> {
        let mut guard = lock_ok(&self.stdin);
        let Some(stdin) = guard.as_mut() else {
            return Ok(()); // 已退出：输入无处可去，无害成功
        };
        let line = format!("{}\n", msg);
        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|e| HamsterError::Launch {
                program: self.agent_id.clone(),
                message: format!("写入结构化通道失败：{e}"),
            })
    }

    /// ACP server→client 请求应答（runtime-design §11.5）：
    /// session/request_permission 自动选择第一个 allow_once 选项（MVP 无审批 UI，
    /// R4 接交互）；其余方法回 JSON-RPC error，防止 agent 无限等待。
    fn answer_acp_server_request(&self, id: i64, method: &str, params: &serde_json::Value) {
        let response = match method {
            "session/request_permission" => {
                let option_id = params
                    .get("options")
                    .and_then(|o| o.as_array())
                    .and_then(|opts| {
                        opts.iter()
                            .find(|o| o.get("kind").and_then(|k| k.as_str()) == Some("allow_once"))
                            .or_else(|| opts.first())
                    })
                    .and_then(|o| o.get("optionId"))
                    .and_then(|i| i.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                serde_json::json!({
                    "jsonrpc": "2.0", "id": id,
                    "result": { "outcome": { "outcome": "selected", "optionId": option_id } }
                })
            }
            other => serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": -32601, "message": format!("上游 未支持该 agent 侧请求：{other}") }
            }),
        };
        let _ = self.write_json(&response);
    }
}

fn spawn_reader_thread(
    session: &Arc<StreamSession>,
    stdout: std::process::ChildStdout,
    on_exit: StreamExitCallback,
) -> Result<()> {
    let session = Arc::clone(session);
    std::thread::Builder::new()
        .name(format!("stream-read-{}", session.session_id))
        .spawn(move || {
            // 协议调试：HAMSTER_STREAM_DEBUG=1 时原样输出每个原始行到 stderr
            let debug_raw = std::env::var("HAMSTER_STREAM_DEBUG").ok().as_deref() == Some("1");
            eprintln!(
                "[stream-reader] started dialect={:?} debug_raw={debug_raw}",
                session.dialect
            );
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if debug_raw {
                    eprintln!("[stream-raw] {trimmed}");
                }
                let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                    continue; // 协议噪声行：跳过（fail-safe）
                };
                if session.dialect == hamster_core::ProtocolDialect::ClaudeStream {
                    // claude stream-json：行即事件（无 JSON-RPC 包装）
                    let mapped = normalize_claude(&session.session_id, &v);
                    if mapped.thread_id_filled {
                        let tid = v.get("session_id").and_then(|i| i.as_str()).unwrap_or("");
                        if !tid.is_empty() {
                            lock_ok(&session.state).thread_id = tid.to_string();
                        }
                    }
                    if mapped.turn_completed {
                        let mut st = lock_ok(&session.state);
                        st.turn_id = None;
                        st.turns += 1;
                        st.last_active_at = now_ms();
                    }
                    for mut ev in mapped.events {
                        ev.at = now_ms();
                        (session.on_event)(&ev);
                    }
                    continue;
                }
                // —— ACP（JSON-RPC）：先分流 server→client 请求与 prompt 响应 ——
                if session.dialect == hamster_core::ProtocolDialect::Acp {
                    let method = v.get("method").and_then(|m| m.as_str());
                    let req_id = v.get("id").and_then(|i| i.as_i64());
                    // server→client 请求（method + id）：必须应答，否则 agent 挂起等回复
                    if let (Some(method), Some(req_id)) = (method, req_id) {
                        let params = v.get("params").cloned().unwrap_or(serde_json::Value::Null);
                        session.answer_acp_server_request(req_id, method, &params);
                        continue;
                    }
                    // 响应：回填等待者；prompt 响应 = 整轮结束（ACP 无 turn 通知）
                    if let Some(req_id) = req_id {
                        let was_prompt = {
                            let mut st = lock_ok(&session.state);
                            let was = st.prompt_request_id == Some(req_id);
                            if was {
                                st.prompt_request_id = None;
                            }
                            was
                        };
                        if let Some(tx) = lock_ok(&session.pending).remove(&req_id) {
                            let _ = tx.send(trimmed.to_string());
                        }
                        if was_prompt {
                            let err_text = v
                                .get("error")
                                .and_then(|e| e.get("message"))
                                .and_then(|m| m.as_str())
                                .map(String::from);
                            {
                                let mut st = lock_ok(&session.state);
                                st.turn_id = None;
                                st.turns += 1;
                                st.last_active_at = now_ms();
                            }
                            if let Some(m) = err_text {
                                (session.on_event)(&StreamEvent {
                                    session_id: session.session_id.clone(),
                                    item_id: String::new(),
                                    at: now_ms(),
                                    kind: StreamEventKind::Error,
                                    text: m,
                                    tool_name: None,
                                    status: None,
                                    diff: None,
                                });
                            }
                            (session.on_event)(&StreamEvent {
                                session_id: session.session_id.clone(),
                                item_id: String::new(),
                                at: now_ms(),
                                kind: StreamEventKind::TurnCompleted,
                                text: String::new(),
                                tool_name: None,
                                status: None,
                                diff: None,
                            });
                        }
                        continue;
                    }
                    // 其余（method 无 id）= 通知，落公共归一化路径
                }
                // —— codex 方言（JSON-RPC）——
                // 响应：回填等待者
                if let Some(id) = v.get("id").and_then(|i| i.as_i64()) {
                    if let Some(tx) = lock_ok(&session.pending).remove(&id) {
                        let _ = tx.send(trimmed.to_string());
                    }
                    continue;
                }
                // 通知：按方言归一化后逐条回调（状态副作用在此应用）
                if let Some(method) = v.get("method").and_then(|m| m.as_str()) {
                    let params = v.get("params").cloned().unwrap_or(serde_json::Value::Null);
                    let mapped =
                        normalize_event(&session.session_id, session.dialect, method, &params);
                    {
                        let mut st = lock_ok(&session.state);
                        if let Some(turn_id) = mapped.turn_started {
                            st.turn_id = Some(turn_id);
                        }
                        if mapped.turn_completed {
                            st.turn_id = None;
                            st.turns += 1;
                        }
                        if mapped.touch {
                            st.last_active_at = now_ms();
                        }
                    }
                    for mut ev in mapped.events {
                        ev.at = now_ms(); // 归一化纯函数不取时，回调侧补真实时刻
                        (session.on_event)(&ev);
                    }
                }
            }
            // EOF：进程退出
            {
                let mut st = lock_ok(&session.state);
                st.running = false;
                st.last_active_at = now_ms();
            }
            if debug_raw {
                eprintln!("[stream-raw] EOF 进程退出");
            }
            (session.on_event)(&StreamEvent {
                session_id: session.session_id.clone(),
                item_id: String::new(),
                at: now_ms(),
                kind: StreamEventKind::Exit,
                text: String::new(),
                tool_name: None,
                status: None,
                diff: None,
            });
            on_exit(&session.session_id);
        })
        .map(|_| ())
        .map_err(|e| HamsterError::Other(format!("流式读取线程启动失败：{e}")))
}

/// 方言通知归一化结果：事件 + 会话状态副作用（调用方应用）。
pub struct MappedEvents {
    pub events: Vec<StreamEvent>,
    /// turn/started 携带的 turn id（interrupt 用）
    pub turn_started: Option<String>,
    pub turn_completed: bool,
    /// 是否刷新 last_active_at
    pub touch: bool,
}

/// ACP `session/new|load` 应答的 `configOptions` 归一化（纯函数）。
/// mode 项走 `session/set_mode`、model 项走 `session/set_config_option`
/// （均在 opencode 1.18.3 实测生效，T3 Code 同款）；其余项暂只读展示。
pub(crate) fn parse_acp_config_options(resp: &serde_json::Value) -> Vec<StreamConfigOption> {
    resp.get("configOptions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|co| {
                    let id = co.get("id")?.as_str()?.to_string();
                    let name = co
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or(&id)
                        .to_string();
                    let current_value = co
                        .get("currentValue")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let category = co
                        .get("category")
                        .and_then(|c| c.as_str())
                        .map(String::from);
                    let choices = co
                        .get("options")
                        .and_then(|v| v.as_array())
                        .map(|opts| {
                            opts.iter()
                                .filter_map(|o| {
                                    let value = o.get("value")?.as_str()?.to_string();
                                    let name = o
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or(&value)
                                        .to_string();
                                    Some(StreamConfigChoice { value, name })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    Some(StreamConfigOption {
                        settable: id == "mode" || category.as_deref() == Some("model"),
                        id,
                        name,
                        category,
                        current_value,
                        choices,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// ACP `tool_call(_update)` 的 `content[]` 中首个 diff 块 → 结构化 diff
///（形状见 ACP schema：{path, oldText?, newText}；无 diff 块返回 None）。
fn parse_acp_tool_diff(content: Option<&serde_json::Value>) -> Option<StreamEventDiff> {
    let arr = content?.as_array()?;
    for item in arr {
        if item.get("type").and_then(|t| t.as_str()) != Some("diff") {
            continue;
        }
        let path = item.get("path")?.as_str()?.to_string();
        let new_text = item
            .get("newText")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string();
        let old_text = item
            .get("oldText")
            .and_then(|t| t.as_str())
            .map(String::from);
        return Some(StreamEventDiff {
            path,
            old_text,
            new_text,
        });
    }
    None
}

/// 方言通知 → 归一化事件（纯函数，无锁无 IO；协议回归测试直接调用）。
pub fn normalize_event(
    session_id: &str,
    dialect: hamster_core::ProtocolDialect,
    method: &str,
    params: &serde_json::Value,
) -> MappedEvents {
    let mut out = MappedEvents {
        events: Vec::new(),
        turn_started: None,
        turn_completed: false,
        touch: false,
    };
    let single = |out: &mut MappedEvents, kind: StreamEventKind, item_id: &str, text: String| {
        out.events.push(StreamEvent {
            session_id: session_id.to_string(),
            item_id: item_id.to_string(),
            at: 0,
            kind,
            text,
            tool_name: None,
            status: None,
            diff: None,
        });
    };
    // 方言分支：codex / ACP 走 JSON-RPC 通知，claude 走行事件（在 reader 单独处理）
    match method {
        "turn/started" => {
            out.turn_started = params
                .get("turn")
                .and_then(|t| t.get("id"))
                .and_then(|i| i.as_str())
                .map(String::from);
            out.touch = true;
            single(&mut out, StreamEventKind::TurnStarted, "", String::new());
        }
        "turn/completed" => {
            out.turn_completed = true;
            out.touch = true;
            single(&mut out, StreamEventKind::TurnCompleted, "", String::new());
        }
        "item/agentMessage/delta" => {
            out.touch = true;
            single(
                &mut out,
                StreamEventKind::AgentDelta,
                params.get("itemId").and_then(|i| i.as_str()).unwrap_or(""),
                params
                    .get("delta")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string(),
            );
        }
        "item/reasoning/textDelta" | "item/reasoning/summaryTextDelta" => {
            out.touch = true;
            single(
                &mut out,
                StreamEventKind::ReasoningDelta,
                params.get("itemId").and_then(|i| i.as_str()).unwrap_or(""),
                params
                    .get("delta")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string(),
            );
        }
        "item/completed" => {
            let item = params
                .get("item")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let item_id = item
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            let ty = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let ev = match ty {
                "userMessage" => Some(StreamEvent {
                    session_id: session_id.to_string(),
                    item_id: item_id.clone(),
                    at: 0,
                    kind: StreamEventKind::UserEcho,
                    text: user_message_text(&item),
                    tool_name: None,
                    status: None,
                    diff: None,
                }),
                "agentMessage" => Some(StreamEvent {
                    session_id: session_id.to_string(),
                    item_id: item_id.clone(),
                    at: 0,
                    kind: StreamEventKind::AgentDone,
                    text: item
                        .get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string(),
                    tool_name: None,
                    status: None,
                    diff: None,
                }),
                "reasoning" => Some(StreamEvent {
                    session_id: session_id.to_string(),
                    item_id: item_id.clone(),
                    at: 0,
                    kind: StreamEventKind::ReasoningDone,
                    text: reasoning_text(&item),
                    tool_name: None,
                    status: None,
                    diff: None,
                }),
                "commandExecution" | "fileChange" | "mcpToolCall" | "dynamicToolCall" => {
                    let mut e = StreamEvent {
                        session_id: session_id.to_string(),
                        item_id: item_id.clone(),
                        at: 0,
                        kind: StreamEventKind::ToolItem,
                        text: tool_detail(&item),
                        tool_name: Some(tool_label(&item, ty)),
                        status: item
                            .get("status")
                            .and_then(|s| s.as_str())
                            .map(String::from),
                        diff: None,
                    };
                    let _ = &mut e;
                    Some(e)
                }
                "errorItem" => Some(StreamEvent {
                    session_id: session_id.to_string(),
                    item_id: item_id.clone(),
                    at: 0,
                    kind: StreamEventKind::Error,
                    text: item
                        .get("message")
                        .and_then(|m| m.as_str())
                        .or_else(|| item.get("text").and_then(|t| t.as_str()))
                        .unwrap_or("未知错误")
                        .to_string(),
                    tool_name: None,
                    status: None,
                    diff: None,
                }),
                _ => None, // plan/webSearch 等次要条目：R4 再渲染
            };
            if let Some(e) = ev {
                out.touch = true;
                out.events.push(e);
            }
        }
        "error" => {
            out.events.push(StreamEvent {
                session_id: session_id.to_string(),
                item_id: String::new(),
                at: 0,
                kind: StreamEventKind::Error,
                text: params
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("协议错误")
                    .to_string(),
                tool_name: None,
                status: None,
                diff: None,
            });
        }
        // —— ACP：session/update 通知（runtime-design §11.5）——
        "session/update" if dialect == hamster_core::ProtocolDialect::Acp => {
            let upd = params
                .get("update")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let upd_ty = upd
                .get("sessionUpdate")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            // 条目 id：tool_call 系取 toolCallId；消息/思考增量取 messageId
            // （opencode 实测携带）。注意 message 与 thought 部分共用同一个
            // messageId（opencode 一条助手消息含思考+文本两部分）——必须按角色
            // 加命名空间前缀，否则助手文本会被前端 findRow 并进思考行；
            // 两者皆缺时回退空 id（前端「并入最后一条流式行」兜底）
            let raw_id = upd
                .get("toolCallId")
                .and_then(|i| i.as_str())
                .or_else(|| upd.get("messageId").and_then(|i| i.as_str()))
                .map(String::from);
            let mk = |kind: StreamEventKind, item_id: String, text: String| StreamEvent {
                session_id: session_id.to_string(),
                item_id,
                at: 0,
                kind,
                text,
                tool_name: None,
                status: None,
                diff: None,
            };
            match upd_ty {
                "agent_message_chunk" => {
                    out.touch = true;
                    let text = acp_content_text(upd.get("content"));
                    if !text.is_empty() {
                        let key = raw_id.map(|id| format!("msg:{id}")).unwrap_or_default();
                        out.events.push(mk(StreamEventKind::AgentDelta, key, text));
                    }
                }
                "agent_thought_chunk" => {
                    out.touch = true;
                    let text = acp_content_text(upd.get("content"));
                    if !text.is_empty() {
                        let key = raw_id.map(|id| format!("think:{id}")).unwrap_or_default();
                        out.events
                            .push(mk(StreamEventKind::ReasoningDelta, key, text));
                    }
                }
                "tool_call" | "tool_call_update" => {
                    out.touch = true;
                    let title = upd.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let kind_label = upd.get("kind").and_then(|k| k.as_str()).unwrap_or("tool");
                    let mut e = mk(
                        StreamEventKind::ToolItem,
                        raw_id.unwrap_or_default(),
                        title.chars().take(2000).collect(),
                    );
                    e.tool_name = Some(if title.is_empty() {
                        kind_label.to_string()
                    } else {
                        title.to_string()
                    });
                    e.status = upd.get("status").and_then(|s| s.as_str()).map(String::from);
                    e.diff = parse_acp_tool_diff(upd.get("content"));
                    out.events.push(e);
                }
                _ => {} // user_message_chunk（前端乐观回显）/ plan / available_commands 等：R4
            }
        }
        _ => {} // thread/started、mcp startup、token usage 等：暂不透传
    }
    out
}

/// ACP ContentBlock（{type:"text",text} 或 ContentBlock[]）→ 纯文本。
fn acp_content_text(content: Option<&serde_json::Value>) -> String {
    let mut out = String::new();
    match content {
        Some(serde_json::Value::Array(items)) => {
            for it in items {
                if it.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(t) = it.get("text").and_then(|t| t.as_str()) {
                        out.push_str(t);
                    }
                }
            }
        }
        Some(v) => {
            if let Some(t) = v
                .get("text")
                .and_then(|t| t.as_str())
                .filter(|_| v.get("type").and_then(|ty| ty.as_str()) == Some("text"))
            {
                out.push_str(t);
            }
        }
        None => {}
    }
    out.chars().take(2000).collect()
}

/// userMessage.content = UserInput[]（取 text 项拼接）。
fn user_message_text(item: &serde_json::Value) -> String {
    let mut out = String::new();
    if let Some(items) = item.get("content").and_then(|c| c.as_array()) {
        for it in items {
            if it.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = it.get("text").and_then(|x| x.as_str()) {
                    out.push_str(t);
                }
            }
        }
    }
    out.chars().take(2000).collect()
}

/// npm .cmd shim → 同包原生 exe 解析：
/// `<dir>/claude.cmd` 的同级 `node_modules/<pkg>/bin/claude.exe`。
/// claude 2.x 是原生二进制，shim 仅转发；Rust 管道 spawn shim 会静默挂起。
fn resolve_native(program: &str) -> Option<String> {
    let p = std::path::Path::new(program);
    if p.extension().and_then(|e| e.to_str()) != Some("cmd") {
        return None;
    }
    let dir = p.parent()?;
    let exe = dir
        .join("node_modules")
        .join("@anthropic-ai")
        .join("claude-code")
        .join("bin")
        .join("claude.exe");
    exe.is_file().then(|| exe.to_string_lossy().into_owned())
}

/// claude stream-json 归一化结果。
pub struct ClaudeMapped {
    pub events: Vec<StreamEvent>,
    /// system/init 行：回填 session_id（thread_id）
    pub thread_id_filled: bool,
    pub turn_completed: bool,
}

/// claude `--output-format stream-json` 事件 → 归一化事件（纯函数）。
/// 形状取自本机实测（claude 2.1.251，--include-partial-messages）。
pub fn normalize_claude(session_id: &str, v: &serde_json::Value) -> ClaudeMapped {
    let mut out = ClaudeMapped {
        events: Vec::new(),
        thread_id_filled: false,
        turn_completed: false,
    };
    let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let mk = |kind: StreamEventKind, item_id: &str, text: String| StreamEvent {
        session_id: session_id.to_string(),
        item_id: item_id.to_string(),
        at: 0,
        kind,
        text,
        tool_name: None,
        status: None,
        diff: None,
    };
    match ty {
        // init：回填 session_id
        "system" if v.get("subtype").and_then(|x| x.as_str()) == Some("init") => {
            out.thread_id_filled = true;
        }
        // 部分 delta：text_delta = 助手增量；thinking_delta = 思考增量
        "stream_event" => {
            let ev = v.get("event").cloned().unwrap_or(serde_json::Value::Null);
            let ev_ty = ev.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let idx = ev.get("index").and_then(|i| i.as_i64()).unwrap_or(0);
            let item_id = format!(
                "blk-{}-{}",
                v.get("session_id").and_then(|i| i.as_str()).unwrap_or("s"),
                idx
            );
            if ev_ty == "content_block_delta" {
                let delta = ev.get("delta").cloned().unwrap_or(serde_json::Value::Null);
                match delta.get("type").and_then(|t| t.as_str()) {
                    Some("text_delta") => {
                        let text = delta.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        if !text.is_empty() {
                            out.events.push(mk(
                                StreamEventKind::AgentDelta,
                                &item_id,
                                text.to_string(),
                            ));
                        }
                    }
                    Some("thinking_delta") => {
                        let text = delta.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
                        if !text.is_empty() {
                            out.events.push(mk(
                                StreamEventKind::ReasoningDelta,
                                &item_id,
                                text.to_string(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        // 完整 assistant 消息：text/thinking/tool_use 块 → 权威行
        "assistant" => {
            let msg = v.get("message").cloned().unwrap_or(serde_json::Value::Null);
            let mid = msg
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("m")
                .to_string();
            if let Some(blocks) = msg.get("content").and_then(|c| c.as_array()) {
                for (i, b) in blocks.iter().enumerate() {
                    let bty = b.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    let item_id = format!("{mid}-{i}");
                    match bty {
                        "text" => {
                            let text = b.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            if !text.trim().is_empty() {
                                out.events.push(mk(
                                    StreamEventKind::AgentDone,
                                    &item_id,
                                    text.to_string(),
                                ));
                            }
                        }
                        "thinking" => {
                            let text = b.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
                            if !text.trim().is_empty() {
                                out.events.push(mk(
                                    StreamEventKind::ReasoningDone,
                                    &item_id,
                                    text.to_string(),
                                ));
                            }
                        }
                        "tool_use" => {
                            let mut e = mk(
                                StreamEventKind::ToolItem,
                                &item_id,
                                b.get("input")
                                    .map(|i| serde_json::to_string(i).unwrap_or_default())
                                    .unwrap_or_default(),
                            );
                            e.tool_name = b.get("name").and_then(|n| n.as_str()).map(String::from);
                            e.status = Some("completed".into());
                            out.events.push(e);
                        }
                        _ => {}
                    }
                }
            }
        }
        // 轮次结束（带最终文本与用量）
        "result" => {
            out.turn_completed = true;
            out.events
                .push(mk(StreamEventKind::TurnCompleted, "", String::new()));
            if let Some(text) = v.get("result").and_then(|t| t.as_str()) {
                if !text.trim().is_empty() {
                    out.events
                        .push(mk(StreamEventKind::AgentDone, "final", text.to_string()));
                }
            }
        }
        _ => {} // system.status/thinking_tokens、user(tool_result) 等：不渲染
    }
    out
}

/// reasoning.content/summary = string[]（拼接，截断）。
fn reasoning_text(item: &serde_json::Value) -> String {
    let mut out = String::new();
    for key in ["content", "summary"] {
        if let Some(parts) = item.get(key).and_then(|c| c.as_array()) {
            for p in parts {
                if let Some(s) = p.as_str() {
                    out.push_str(s);
                }
            }
        }
    }
    out.chars().take(6000).collect()
}

fn tool_label(item: &serde_json::Value, ty: &str) -> String {
    match ty {
        "commandExecution" => item
            .get("command")
            .and_then(|c| c.as_str())
            .and_then(|c| c.split_whitespace().next())
            .map(String::from)
            .or_else(|| Some("command".into())),
        "fileChange" => Some("fileChange".into()),
        "mcpToolCall" => Some(format!(
            "{}.{}",
            item.get("server").and_then(|s| s.as_str()).unwrap_or("mcp"),
            item.get("tool").and_then(|t| t.as_str()).unwrap_or("tool")
        )),
        _ => Some(ty.to_string()),
    }
    .unwrap_or_else(|| ty.to_string())
}

fn tool_detail(item: &serde_json::Value) -> String {
    match item.get("type").and_then(|t| t.as_str()) {
        Some("commandExecution") => item
            .get("command")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .chars()
            .take(2000)
            .collect(),
        Some("fileChange") => item
            .get("changes")
            .map(|c| serde_json::to_string(c).unwrap_or_default())
            .unwrap_or_default()
            .chars()
            .take(2000)
            .collect(),
        _ => serde_json::to_string(&item)
            .unwrap_or_default()
            .chars()
            .take(2000)
            .collect(),
    }
}
