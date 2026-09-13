//! Live 端到端验证：真实 CLI × 应用同款链路（adapters 声明 → StreamSession 握手/
//! 发轮 → 事件归一化）。覆盖 agent chat 的三条协议方言：
//! claude stream-json / codex app-server / ACP（opencode 代表 + kimi 鉴权路径）。
//!
//! 每个已验证 Agent 跑两轮：
//! ① 工具调用轮：要求真实执行 `echo <marker>`，断言「工具条目出现 + 回答含
//!    marker + 轮次正常收束」——验证权限放行、tool_call 归一化、轮结束判定；
//! ② 续聊轮：用①的会话键（claude --resume / codex thread/resume / ACP
//!    session/load）重开进程并追问 marker，验证协议层续聊。
//! 另有 ACP 文件编辑轮（断言文件真实落盘 + diff 解析）与 kimi 未登录路径
//! （断言优雅报错不悬挂）。
//!
//! 全部 `#[ignore]`：需要本机已装 CLI 且已登录，会产生**真实模型调用**。
//! 运行（串行，避免多 CLI 并发抢资源）：
//! `cargo test -p hamster-hub --test live_agent_chat -- --ignored --test-threads=1 --nocapture`
//! 调试原始协议行：先 `set HAMSTER_STREAM_DEBUG=1`。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hamster_adapters::build_registry;

use hamster_core::model::streaming::ProtocolDialect;
use hamster_core::{Registry, StreamEvent, StreamEventKind};
use hamster_runtime::{StreamOptions, StreamSession};

const TURN_TIMEOUT: Duration = Duration::from_secs(300);

// ===== 测试基座 =====

type Events = Arc<Mutex<Vec<StreamEvent>>>;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").expect("USERPROFILE"))
}

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "hamster-live-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&dir).expect("创建临时项目目录");
    dir
}

fn sink() -> (Events, hamster_runtime::EventCallback) {
    let evs: Events = Arc::new(Mutex::new(Vec::new()));
    let cb = {
        let evs = Arc::clone(&evs);
        Box::new(move |e: &StreamEvent| evs.lock().unwrap().push(e.clone()))
    };
    (evs, cb)
}

/// 与 src/bench/commands.rs `build_channel_command` 同款 Windows 包装
/// （extra = 模型覆盖等会话级启动旗标，先并入再包装）。
fn channel_command(
    spec: &hamster_core::RuntimeSpec,
    channel: &hamster_core::StructuredChannel,
    extra: Vec<String>,
) -> (String, Vec<String>) {
    let mut args = channel.args.clone();
    args.extend(extra);
    if cfg!(windows) && spec.windows_shim && channel.dialect != ProtocolDialect::ClaudeStream {
        let mut a = vec!["/c".to_string(), spec.program.clone()];
        a.append(&mut args);
        ("cmd.exe".into(), a)
    } else {
        (spec.program.clone(), args)
    }
}

/// 按应用真实路径 spawn：适配器 runtime() 声明 → StreamSession::start（含握手）。
/// model：claude 方言按 bench 层同款拼 channel.model_flag 进启动 argv；ACP 方言
/// 经 StreamOptions.model 走会话级 set_config_option（appserver 内处理）。
fn spawn_session(
    agent_id: &str,
    dir: &Path,
    resume_key: Option<&str>,
    model: Option<&str>,
    cb: hamster_runtime::EventCallback,
) -> Result<Arc<StreamSession>, hamster_core::HamsterError> {
    let reg: Registry = build_registry(&home());
    let adapter = reg.get(agent_id)?;
    let spec = adapter
        .runtime()
        .ok_or_else(|| hamster_core::HamsterError::Unsupported(format!("{agent_id} 无 runtime")))?;
    let channel = spec.structured.clone().ok_or_else(|| {
        hamster_core::HamsterError::Unsupported(format!("{agent_id} 无结构化通道"))
    })?;
    let extra = match (channel.model_flag.as_deref(), model) {
        (Some(flag), Some(m)) => vec![flag.to_string(), m.to_string()],
        _ => Vec::new(),
    };
    let (program, args) = channel_command(&spec, &channel, extra);
    StreamSession::start(StreamOptions {
        agent_id: agent_id.to_string(),
        project_dir: dir.to_path_buf(),
        channel,
        resume_key: resume_key.map(String::from),
        fork: false,
        model: model.map(String::from),
        program,
        args,
        mcp_servers: Vec::new(),
        on_event: cb,
        on_exit: Box::new(|_| {}),
    })
}

fn events_of(evs: &Events) -> Vec<StreamEvent> {
    evs.lock().unwrap().clone()
}

/// 等到出现新的 TurnCompleted（基线 = baseline 之后的第一个）；返回是否等到。
fn wait_turn_done(evs: &Events, baseline: usize, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let snapshot = events_of(evs);
        let from = baseline.min(snapshot.len());
        if snapshot[from..]
            .iter()
            .any(|e| e.kind == StreamEventKind::TurnCompleted)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

fn baseline_len(evs: &Events) -> usize {
    events_of(evs).len()
}

/// 断言：①发出后 Exit 未先于 TurnCompleted（进程中途死掉=链路断裂）。
fn assert_no_premature_exit(evs: &[StreamEvent]) {
    if let Some(pos) = evs.iter().position(|e| e.kind == StreamEventKind::Exit) {
        assert!(
            evs[..pos]
                .iter()
                .any(|e| e.kind == StreamEventKind::TurnCompleted),
            "通道进程在轮次结束前退出：{:?}",
            events_brief(&evs[..=pos])
        );
    }
}

/// 回答正文：AgentDone 权威全文优先，其次拼接 AgentDelta。
fn reply_text(evs: &[StreamEvent]) -> String {
    let done = evs
        .iter()
        .filter(|e| e.kind == StreamEventKind::AgentDone)
        .map(|e| e.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    if !done.trim().is_empty() {
        return done;
    }
    evs.iter()
        .filter(|e| e.kind == StreamEventKind::AgentDelta)
        .map(|e| e.text.clone())
        .collect::<Vec<_>>()
        .join("")
}

fn events_brief(evs: &[StreamEvent]) -> Vec<String> {
    evs.iter()
        .map(|e| {
            format!(
                "{:?} tool={:?} status={:?} text={:?}",
                e.kind,
                e.tool_name,
                e.status,
                e.text.chars().take(120).collect::<String>()
            )
        })
        .collect()
}

/// ①+② 主体：工具调用轮 + 续聊追问轮。model：会话级模型覆盖（None = CLI 默认）。
fn live_tool_call_and_resume(agent_id: &str, model: Option<&str>) {
    std::env::set_var("HAMSTER_STREAM_DEBUG", "1");
    let marker = format!("HAMSTER-LIVE-{}", agent_id.to_uppercase());
    let dir = scratch_dir(agent_id);
    let prompt = format!(
        "Run the shell command `echo {marker}` (you must actually execute it, do not simulate) \
and then reply with the command's exact output on its own line."
    );

    // ① 工具调用轮
    let (evs, cb) = sink();
    let session = spawn_session(agent_id, &dir, None, model, cb).expect("spawn + 握手应成功");
    let base = baseline_len(&evs);
    session.send_turn(&prompt, None, None).expect("发轮应成功");
    assert!(
        wait_turn_done(&evs, base, TURN_TIMEOUT),
        "工具调用轮 300s 未收束：{:?}",
        events_brief(&events_of(&evs)[base..])
    );
    let evs1 = events_of(&evs)[base..].to_vec();
    assert_no_premature_exit(&evs1);

    let tool_items: Vec<&StreamEvent> = evs1
        .iter()
        .filter(|e| e.kind == StreamEventKind::ToolItem)
        .collect();
    assert!(
        !tool_items.is_empty(),
        "[{agent_id}] 未观察到任何工具调用条目（工具被拒或未执行）：{:?}",
        events_brief(&evs1)
    );
    assert!(
        tool_items
            .iter()
            .any(|e| e.status.as_deref() != Some("failed")),
        "[{agent_id}] 工具调用全部 failed：{:?}",
        events_brief(&evs1)
    );
    let reply = reply_text(&evs1);
    assert!(
        reply.contains(&marker),
        "[{agent_id}] 回答未包含 echo 输出 {marker}：{reply:.400}"
    );

    // ② 续聊轮：杀掉旧进程，用会话键重开（协议层续聊）
    let key = session.thread_id();
    assert!(!key.is_empty(), "[{agent_id}] 无会话键可供续聊");
    session.kill().ok();
    let (evs2, cb2) = sink();
    let resumed = spawn_session(agent_id, &dir, Some(&key), model, cb2).expect("续聊 spawn 应成功");
    let base2 = baseline_len(&evs2);
    resumed
        .send_turn(
            "In the first message of this conversation I asked you to echo a token. \
Reply with ONLY that token, nothing else.",
            None,
            None,
        )
        .expect("续聊发轮应成功");
    assert!(
        wait_turn_done(&evs2, base2, TURN_TIMEOUT),
        "续聊轮 300s 未收束：{:?}",
        events_brief(&events_of(&evs2)[base2..])
    );
    let reply2 = reply_text(&events_of(&evs2)[base2..]);
    assert!(
        reply2.contains(&marker),
        "[{agent_id}] 续聊上下文丢失，回答未含 {marker}：{reply2:.400}"
    );
    resumed.kill().ok();
    let _ = std::fs::remove_dir_all(&dir);
}

// ===== 各方言 live 用例 =====

/// claude stream-json：工具调用 + --resume 续聊（用 CLI 默认模型；2026-09-13
/// 起本机 settings.json 默认模型为 glm-5.3，deepseek-v4-pro 已在 BigModel 下线）。
#[test]
#[ignore = "live：真实调用 claude CLI，需已登录；手动运行"]
fn live_claude_tool_call_and_resume() {
    live_tool_call_and_resume("claude", None);
}

/// codex app-server：thread/start → turn/start 工具调用 + thread/resume 续聊。
#[test]
#[ignore = "live：真实调用 codex CLI，需已登录；手动运行"]
fn live_codex_tool_call_and_resume() {
    live_tool_call_and_resume("codex", None);
}

/// opencode（ACP 通道代表）：session/new → session/prompt 工具调用 + session/load 续聊。
#[test]
#[ignore = "live：真实调用 opencode CLI，需已配置模型；手动运行"]
fn live_opencode_tool_call_and_resume() {
    live_tool_call_and_resume("opencode", None);
}

/// ACP 文件编辑轮：tool_call content 的 diff 块解析 + 文件真实落盘。
#[test]
#[ignore = "live：真实调用 opencode CLI；手动运行"]
fn live_opencode_edit_emits_diff_and_writes_file() {
    std::env::set_var("HAMSTER_STREAM_DEBUG", "1");
    let dir = scratch_dir("opencode-edit");
    let (evs, cb) = sink();
    let session = spawn_session("opencode", &dir, None, None, cb).expect("spawn + 握手应成功");
    let base = baseline_len(&evs);
    session
        .send_turn(
            "Create a file named hm-live.txt in the current directory with exactly one line: LIVE-OK. \
Reply with just: done",
            None,
            None,
        )
        .expect("发轮应成功");
    assert!(
        wait_turn_done(&evs, base, TURN_TIMEOUT),
        "编辑轮 300s 未收束：{:?}",
        events_brief(&events_of(&evs)[base..])
    );
    let evs1 = events_of(&evs)[base..].to_vec();
    assert_no_premature_exit(&evs1);

    // 文件真实落盘（端到端硬证据）
    let written = std::fs::read_to_string(dir.join("hm-live.txt")).unwrap_or_default();
    assert!(
        written.trim().contains("LIVE-OK"),
        "[opencode] 文件未按指令落盘（content={written:?}）：{:?}",
        events_brief(&evs1)
    );
    // diff 结构化解析：ACP schema 允许 content 不带 diff 块（opencode 1.18.29
    // 的 write 工具实测只回文本 content）——有则校验内容，无则记录跳过
    let diffs: Vec<_> = evs1.iter().filter_map(|e| e.diff.as_ref()).collect();
    if diffs.is_empty() {
        println!(
            "[opencode] 该版本 tool_call 未携带 diff 块（schema 允许），跳过 diff 校验：{:?}",
            events_brief(&evs1)
        );
    } else {
        assert!(
            diffs.iter().any(|d| d.new_text.contains("LIVE-OK")),
            "[opencode] diff 块内容不符：{:?}",
            diffs
        );
    }
    session.kill().ok();
    let _ = std::fs::remove_dir_all(&dir);
}

/// kimi（ACP）未登录路径：握手可用，但发轮应得到明确的鉴权错误，不悬挂。
#[test]
#[ignore = "live：真实调用 kimi CLI；手动运行"]
fn live_kimi_unauthenticated_fails_fast_with_clear_error() {
    std::env::set_var("HAMSTER_STREAM_DEBUG", "1");
    let dir = scratch_dir("kimi");
    let outcome = (|| -> Result<String, String> {
        let (evs, cb) = sink();
        match spawn_session("kimi", &dir, None, None, cb) {
            Err(e) => Err(format!("spawn 阶段失败（协议回传）：{e}")),
            Ok(session) => {
                let base = baseline_len(&evs);
                session
                    .send_turn("Say hi", None, None)
                    .map_err(|e| e.to_string())?;
                if wait_turn_done(&evs, base, Duration::from_secs(120)) {
                    let tail = events_of(&evs)[base..].to_vec();
                    let err = tail
                        .iter()
                        .find(|e| e.kind == StreamEventKind::Error)
                        .map(|e| e.text.clone());
                    session.kill().ok();
                    Ok(match err {
                        Some(m) => format!("轮次收束且带错误事件：{m}"),
                        None => "轮次正常收束（kimi 已可登录使用）".into(),
                    })
                } else {
                    session.kill().ok();
                    Err("120s 内既未收束也无错误事件（悬挂）".into())
                }
            }
        }
    })();
    let _ = std::fs::remove_dir_all(&dir);
    let msg = match outcome {
        Ok(m) => m,
        Err(m) => m,
    };
    println!("[kimi] {msg}");
    // 优雅路径 = 明确报错（spawn 期鉴权失败 / 错误事件）或正常收束；悬挂才判失败
    let graceful =
        msg.contains("spawn 阶段失败") || msg.contains("错误事件") || msg.contains("正常收束");
    assert!(graceful, "[kimi] 未登录路径不优雅：{msg}");
}

// ===== 快速盘点（不发模型调用）：当前机器上 agent chat 可用面 =====

/// 打印本机已装且带结构化通道的 Agent 及其方言（回归 detector 声明）。
#[test]
#[ignore = "盘点用：手动运行查看本机可用 Agent 清单"]
fn inventory_local_agents_with_structured_channel() {
    let reg = build_registry(&home());
    for a in reg.list() {
        let detected = a.detect().is_some();
        let dialect = a.runtime().and_then(|r| r.structured.map(|c| c.dialect));
        if detected {
            println!("[inventory] {:<10} dialect={:?}", a.id(), dialect);
        }
    }
}
