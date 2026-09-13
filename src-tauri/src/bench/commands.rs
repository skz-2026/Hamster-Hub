//! bench 命令层：薄胶水（streaming/sessions/agents/runtime 选集；域层源自上游开源项目，整合时更名）。
//! 与上游项目的差异：错误统一 AppError；流式事件走 specta 强类型事件（rc.21 的
//! ipc Channel 尚无 JS 侧生成支持，docs "Channel — Coming soon"）。
#![allow(clippy::too_many_arguments)]

use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::bench::{bench_err, BenchContext};
use crate::error::AppError;
use crate::events::{BenchPtyExit, BenchStreamEvent, BenchStreamExit};
use crate::store::settings as settings_repo;
use crate::AppState;

/// settings.agent.cli_paths（用户手动指定的 CLI 路径）读取；读不到按空表处理
fn load_cli_paths(state: &State<'_, AppState>) -> std::collections::BTreeMap<String, String> {
    let Ok(conn) = state.db.lock() else {
        return Default::default();
    };
    settings_repo::load(&conn)
        .map(|s| s.agent.cli_paths)
        .unwrap_or_default()
}

/// 用户手动指定的 CLI 路径覆盖自动探测（含多版本择优结果）；
/// 路径失效（文件被移走/删除）时静默回退探测结果，不阻断启动。
fn apply_cli_override(
    spec: &mut hamster_core::RuntimeSpec,
    cli_paths: &std::collections::BTreeMap<String, String>,
    agent_id: &str,
) {
    if let Some(p) = cli_paths.get(agent_id) {
        if std::path::Path::new(p).is_file() {
            spec.program = p.clone();
        }
    }
}

// ===== agents / discovery =====

#[tauri::command]
#[specta::specta]
pub fn bench_scan_agents(
    ctx: State<'_, BenchContext>,
    state: State<'_, AppState>,
) -> Result<Vec<hamster_core::AgentInfo>, AppError> {
    let mut infos = hamster_core::scanner::scan(&ctx.registry);
    // 手动指定的路径优先：文件在 → 视为已安装，program 用所填路径
    let cli_paths = load_cli_paths(&state);
    for info in &mut infos {
        if let Some(p) = cli_paths.get(&info.id) {
            if std::path::Path::new(p).is_file() {
                info.installed = true;
                info.program = Some(p.clone());
            }
        }
    }
    Ok(infos)
}

#[tauri::command]
#[specta::specta]
pub fn bench_list_projects(ctx: State<'_, BenchContext>) -> Result<Vec<String>, AppError> {
    ctx.store.load_projects().map_err(bench_err)
}

/// 聚合各已安装 Agent 的历史工作目录（读会话记录的 cwd，Store 为空时的项目来源）。
/// 只保留磁盘上仍存在的目录，并跳过临时目录（e2e 沙箱会污染 ~/.claude/projects 等）。
#[tauri::command]
#[specta::specta]
pub fn bench_agent_workspaces(
    ctx: State<'_, BenchContext>,
) -> Result<Vec<hamster_core::WorkspaceRecord>, AppError> {
    let mut out: Vec<hamster_core::WorkspaceRecord> = Vec::new();
    for adapter in ctx.registry.list() {
        if adapter.detect().is_none() {
            continue;
        }
        for record in adapter.recent_workspaces() {
            let path = std::path::Path::new(&record.dir);
            if !path.is_dir() {
                continue;
            }
            if path.components().any(|c| {
                c.as_os_str().eq_ignore_ascii_case("Temp")
                    || c.as_os_str().eq_ignore_ascii_case("tmp")
            }) {
                continue;
            }
            if !out.iter().any(|r| r.dir == record.dir) {
                out.push(record);
            }
        }
    }
    Ok(out)
}

// ===== 流式通道（GUI 对话）=====

/// LiveStreamInfo → LiveSessionInfo（侧栏/工具栏复用同一形态；channel=stream）
fn as_live_info(info: &hamster_core::LiveStreamInfo) -> hamster_core::LiveSessionInfo {
    hamster_core::LiveSessionInfo {
        session_id: info.session_id.clone(),
        agent_id: info.agent_id.clone(),
        kind: hamster_core::SessionKind::Agent,
        channel: hamster_core::SessionChannel::Stream,
        project_dir: info.project_dir.clone(),
        running: info.running,
        exit_code: None,
        started_at: info.started_at,
        last_active_at: info.last_active_at,
        resume_key: info.resume_key.clone(),
    }
}

/// 通道命令 = spec.program + channel.args；Windows npm shim 需经 cmd /c
/// （claude 流式方言例外：直接 spawn 原生 exe，cmd /c 包 shim 会静默挂起）
fn build_channel_command(
    spec: &hamster_core::RuntimeSpec,
    dialect: hamster_core::ProtocolDialect,
    channel_args: Vec<String>,
) -> (String, Vec<String>) {
    if cfg!(windows) && spec.windows_shim && dialect != hamster_core::ProtocolDialect::ClaudeStream
    {
        let mut a = vec!["/c".to_string(), spec.program.clone()];
        a.extend(channel_args);
        ("cmd.exe".into(), a)
    } else {
        (spec.program.clone(), channel_args)
    }
}

/// 创建流式会话（官方 headless 协议）。事件经 BenchStreamEvent 推送；
/// first_prompt 非空时握手后立即发起首轮。
/// bench_stream_create 参数包（specta 的 SpectaFn 上限 10 参，命令参数打包）
#[derive(Debug, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StreamCreateArgs {
    pub agent_id: String,
    pub project_dir: String,
    pub first_prompt: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub resume_key: Option<String>,
    pub fork: bool,
}

#[tauri::command]
#[specta::specta]
pub fn bench_stream_create(
    ctx: State<'_, BenchContext>,
    streams: State<'_, hamster_runtime::StreamManager>,
    app: AppHandle,
    state: State<'_, AppState>,
    args: StreamCreateArgs,
) -> Result<hamster_core::LiveSessionInfo, AppError> {
    let StreamCreateArgs {
        agent_id,
        project_dir,
        first_prompt,
        model,
        effort,
        resume_key,
        fork,
    } = args;
    let dir = project_dir.trim().to_string();
    if dir.is_empty() {
        return Err(bench_err(hamster_core::HamsterError::config_invalid(
            "请先选择项目目录",
        )));
    }
    let adapter = ctx.registry.get(&agent_id).map_err(bench_err)?;
    let mut spec = adapter.runtime().ok_or_else(|| {
        bench_err(hamster_core::HamsterError::Unsupported(format!(
            "{agent_id} 暂不支持内嵌对话"
        )))
    })?;
    apply_cli_override(&mut spec, &load_cli_paths(&state), &agent_id);
    let channel = spec.structured.clone().ok_or_else(|| {
        bench_err(hamster_core::HamsterError::Unsupported(format!(
            "{agent_id} 暂不支持结构化流式通道"
        )))
    })?;
    spawn_stream_session(
        &ctx,
        &streams,
        &app,
        agent_id,
        &dir,
        &spec,
        channel,
        Vec::new(),
        model,
        effort,
        resume_key,
        fork,
        first_prompt,
        Vec::new(),
        true,
    )
}

/// 流式会话 spawn 公共路径（bench_stream_create 与桌面助手 bench_assistant_create
/// 共用）：模型/effort flag 合并、cmd /c 包装、幂等裁决、事件回调装配、
/// 首条 prompt、项目目录登记。`extra_channel_args` = 会话级注入参数（如
/// claude 的 --mcp-config）；`mcp_servers` = ACP 会话级 MCP 注入。
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_stream_session(
    ctx: &BenchContext,
    streams: &hamster_runtime::StreamManager,
    app: &AppHandle,
    agent_id: String,
    dir: &str,
    spec: &hamster_core::RuntimeSpec,
    channel: hamster_core::StructuredChannel,
    extra_channel_args: Vec<String>,
    model: Option<String>,
    effort: Option<String>,
    resume_key: Option<String>,
    fork: bool,
    first_prompt: Option<String>,
    mcp_servers: Vec<serde_json::Value>,
    register_project: bool,
) -> Result<hamster_core::LiveSessionInfo, AppError> {
    let mut channel_args = channel.args.clone();
    channel_args.extend(extra_channel_args);
    if let (Some(flag), Some(m)) = (
        channel.model_flag.as_deref(),
        model.as_deref().map(str::trim).filter(|m| !m.is_empty()),
    ) {
        channel_args.extend([flag.to_string(), m.to_string()]);
    }
    let (program, args) = build_channel_command(spec, channel.dialect, channel_args);

    // spawn 幂等裁决：同一 rollout 已有活会话时直接返回它（单写者保证）
    let trimmed_key = resume_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty());
    if let Some(key) = trimmed_key {
        if let Some(existing) = streams
            .list()
            .into_iter()
            .find(|s| s.running && s.resume_key.as_deref() == Some(key))
        {
            return Ok(as_live_info(&existing));
        }
    }

    let event_app = app.clone();
    let on_event: hamster_runtime::EventCallback =
        Box::new(move |ev: &hamster_core::StreamEvent| {
            // 转发失败（前端已关闭）只丢事件，不影响会话
            let _ = BenchStreamEvent { event: ev.clone() }.emit(&event_app);
        });
    let exit_app = app.clone();
    let on_exit: hamster_runtime::appserver::StreamExitCallback =
        Box::new(move |session_id: &str| {
            let _ = BenchStreamExit {
                session_id: session_id.to_string(),
            }
            .emit(&exit_app);
        });

    // 迟绑定远程 hub 的部分不移植（远程访问属后续阶段）
    let info = streams
        .spawn(hamster_runtime::StreamOptions {
            agent_id,
            project_dir: std::path::Path::new(dir).to_path_buf(),
            channel,
            resume_key: resume_key.filter(|k| !k.trim().is_empty()),
            fork,
            model: model.clone().filter(|m| !m.trim().is_empty()),
            program,
            args,
            mcp_servers,
            on_event,
            on_exit,
        })
        .map_err(bench_err)?;

    // 首条 prompt：spawn 成功后立即发起首轮
    let first_prompt = first_prompt.filter(|p| !p.trim().is_empty());
    if let Some(prompt) = &first_prompt {
        streams
            .send(
                &info.session_id,
                prompt.trim(),
                model.as_deref().filter(|m| !m.trim().is_empty()),
                effort.as_deref().filter(|e| !e.trim().is_empty()),
            )
            .map_err(bench_err)?;
    }
    // 记住最近使用的项目目录（欢迎屏 datalist）
    if register_project {
        ctx.store.add_project(dir).map_err(bench_err)?;
    }
    Ok(as_live_info(&info))
}

// ===== PTY 终端（TUI resume / 混用模式：codex 等 resumeViaTui 会话的续聊通道）=====

/// bench_pty_create 参数包（specta 的 SpectaFn 上限 10 参，命令参数打包）
#[derive(Debug, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PtyCreateArgs {
    pub agent_id: String,
    pub project_dir: String,
    pub first_prompt: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub resume_key: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

/// 创建 PTY 会话（官方 CLI TUI + 官方 resume 参数）。输出经 ipc Channel 推送；
/// GUI 输入框的文字走 bench_pty_send_prompt（bracketed-paste 注入 stdin）。
#[tauri::command]
#[specta::specta]
pub fn bench_pty_create(
    ctx: State<'_, BenchContext>,
    sessions: State<'_, hamster_runtime::SessionManager>,
    app: AppHandle,
    state: State<'_, AppState>,
    args: PtyCreateArgs,
    on_data: tauri::ipc::Channel<Vec<u8>>,
) -> Result<hamster_core::LiveSessionInfo, AppError> {
    let PtyCreateArgs {
        agent_id,
        project_dir,
        first_prompt,
        model,
        effort,
        resume_key,
        cols,
        rows,
    } = args;
    let dir = project_dir.trim().to_string();
    if dir.is_empty() {
        return Err(bench_err(hamster_core::HamsterError::config_invalid(
            "请先选择项目目录",
        )));
    }
    // spawn 幂等裁决（单写者保证）：同一 rollout 已有活会话直接附着新输出汇后返回
    // （codex 会以 active-writer 拒绝第二个写入者；attach 让重建的视图接管输出）
    let trimmed_key = resume_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty());
    if let Some(key) = trimmed_key {
        if let Some(existing) = sessions
            .list()
            .into_iter()
            .find(|s| s.running && s.resume_key.as_deref() == Some(key))
        {
            let sid = existing.session_id.clone();
            sessions
                .attach(
                    &sid,
                    Box::new(move |bytes: &[u8]| {
                        let _ = on_data.send(bytes.to_vec());
                    }),
                )
                .map_err(bench_err)?;
            return Ok(existing);
        }
    }
    let adapter = ctx.registry.get(&agent_id).map_err(bench_err)?;
    let mut spec = adapter.runtime().ok_or_else(|| {
        bench_err(hamster_core::HamsterError::Unsupported(format!(
            "{agent_id} 暂不支持内嵌对话"
        )))
    })?;
    apply_cli_override(&mut spec, &load_cli_paths(&state), &agent_id);
    let overrides = hamster_core::LaunchOverrides {
        model,
        effort,
        resume_key: resume_key.clone(),
    };
    let plan =
        hamster_runtime::plan_spawn(&spec, first_prompt.as_deref(), &overrides, cfg!(windows));

    let on_data: hamster_runtime::DataCallback = Box::new(move |bytes: &[u8]| {
        // 转发失败（前端已关闭）只丢帧，不影响会话
        let _ = on_data.send(bytes.to_vec());
    });
    let exit_app = app.clone();
    let on_exit: hamster_runtime::session::ExitCallback =
        Box::new(move |session_id: &str, exit_code: i32| {
            let _ = BenchPtyExit {
                session_id: session_id.to_string(),
                exit_code: exit_code.max(0) as u32,
            }
            .emit(&exit_app);
        });

    let info = sessions
        .spawn(hamster_runtime::SpawnOptions {
            agent_id,
            resume_key: resume_key.filter(|k| !k.trim().is_empty()),
            kind: hamster_core::SessionKind::Agent,
            project_dir: std::path::Path::new(&dir).to_path_buf(),
            plan,
            cols,
            rows,
            on_data,
            on_exit,
        })
        .map_err(bench_err)?;
    // 记住最近使用的项目目录
    ctx.store.add_project(&dir).map_err(bench_err)?;
    Ok(info)
}

#[tauri::command]
#[specta::specta]
pub fn bench_pty_write(
    sessions: State<'_, hamster_runtime::SessionManager>,
    session_id: String,
    data: String,
) -> Result<(), AppError> {
    sessions
        .write(&session_id, data.as_bytes())
        .map_err(bench_err)
}

/// GUI 输入通道：文本以 bracketed-paste 注入运行中会话（混用模式，上游设计文档 runtime-design §3.6）
#[tauri::command]
#[specta::specta]
pub fn bench_pty_send_prompt(
    sessions: State<'_, hamster_runtime::SessionManager>,
    session_id: String,
    text: String,
) -> Result<(), AppError> {
    sessions.send_prompt(&session_id, &text).map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub fn bench_pty_resize(
    sessions: State<'_, hamster_runtime::SessionManager>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), AppError> {
    sessions.resize(&session_id, cols, rows).map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub fn bench_pty_kill(
    sessions: State<'_, hamster_runtime::SessionManager>,
    session_id: String,
) -> Result<(), AppError> {
    sessions.kill(&session_id).map_err(bench_err)
}

/// PTY 活会话（与流式列表并存；侧栏合并去重）
#[tauri::command]
#[specta::specta]
pub fn bench_list_live_sessions(
    sessions: State<'_, hamster_runtime::SessionManager>,
) -> Result<Vec<hamster_core::LiveSessionInfo>, AppError> {
    Ok(sessions.list())
}

#[tauri::command]
#[specta::specta]
pub fn bench_stream_send(
    streams: State<'_, hamster_runtime::StreamManager>,
    session_id: String,
    text: String,
    model: Option<String>,
    effort: Option<String>,
) -> Result<(), AppError> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(());
    }
    streams
        .send(
            &session_id,
            &text,
            model.as_deref().filter(|m| !m.trim().is_empty()),
            effort.as_deref().filter(|e| !e.trim().is_empty()),
        )
        .map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub fn bench_stream_interrupt(
    streams: State<'_, hamster_runtime::StreamManager>,
    session_id: String,
) -> Result<(), AppError> {
    streams.interrupt(&session_id).map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub fn bench_stream_kill(
    streams: State<'_, hamster_runtime::StreamManager>,
    hub: State<'_, crate::mcp_server::McpHub>,
    session_id: String,
) -> Result<(), AppError> {
    // 急停联动：桌面助手会话结束的同时吊销其 MCP 令牌（在途调用立即 401）
    hub.revoke_session(&session_id);
    streams.kill(&session_id).map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub fn bench_list_stream_sessions(
    streams: State<'_, hamster_runtime::StreamManager>,
) -> Result<Vec<hamster_core::LiveSessionInfo>, AppError> {
    Ok(streams.list().iter().map(as_live_info).collect())
}

// ===== 会话历史与索引（Recall）=====

#[tauri::command]
#[specta::specta]
pub fn bench_list_history_sessions(
    ctx: State<'_, BenchContext>,
) -> Result<Vec<hamster_core::SessionSummary>, AppError> {
    let mut all: Vec<hamster_core::SessionSummary> = Vec::new();
    for adapter in ctx.registry.list() {
        all.extend(adapter.recent_sessions());
    }
    all.sort_by_key(|s| std::cmp::Reverse(s.last_active_at));
    Ok(all)
}

/// 检索前触发一轮快速增量摄取（未变更文件仅 stat + 游标比对，短暂阻塞可接受——
/// 与上游同款：async 命令体内直调 rusqlite，State 借用不跨 spawn）
#[tauri::command]
#[specta::specta]
pub async fn bench_search_sessions(
    ctx: State<'_, BenchContext>,
    index: State<'_, hamster_index::IndexStore>,
    query: hamster_core::SearchQuery,
) -> Result<Vec<hamster_core::SearchHit>, AppError> {
    let _ = hamster_index::ingest_all(&ctx.registry, &index);
    index.search(&query).map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub async fn bench_index_status(
    index: State<'_, hamster_index::IndexStore>,
) -> Result<hamster_core::IndexStatus, AppError> {
    index.stats().map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub async fn bench_index_refresh(
    ctx: State<'_, BenchContext>,
    index: State<'_, hamster_index::IndexStore>,
) -> Result<hamster_core::IndexStatus, AppError> {
    hamster_index::ingest_all(&ctx.registry, &index).map_err(bench_err)?;
    index.stats().map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub async fn bench_reindex(
    ctx: State<'_, BenchContext>,
    index: State<'_, hamster_index::IndexStore>,
) -> Result<hamster_core::IndexStatus, AppError> {
    index.rebuild().map_err(bench_err)?;
    hamster_index::ingest_all(&ctx.registry, &index).map_err(bench_err)?;
    index.stats().map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub fn bench_session_messages(
    ctx: State<'_, BenchContext>,
    index: State<'_, hamster_index::IndexStore>,
    agent: String,
    session_key: String,
    around_seq: usize,
    window: usize,
) -> Result<Vec<hamster_core::SnapshotMessage>, AppError> {
    // 读取前先快速增量摄取（GUI resume 播种历史的前提；游标使未变更文件近零成本）
    let _ = hamster_index::ingest_all(&ctx.registry, &index);
    index
        .messages_around(&agent, &session_key, around_seq as i64, window as i64)
        .map_err(bench_err)
}

#[tauri::command]
#[specta::specta]
pub fn bench_list_session_messages(
    ctx: State<'_, BenchContext>,
    index: State<'_, hamster_index::IndexStore>,
    agent: String,
    session_key: String,
    from_seq: i64,
    limit: usize,
) -> Result<hamster_core::SessionMessagesPage, AppError> {
    let _ = hamster_index::ingest_all(&ctx.registry, &index);
    index
        .messages_page(&agent, &session_key, from_seq, limit as i64)
        .map_err(bench_err)
}

/// 某代理在项目下最新入索引的会话键（登记表条目缺 sessionKey 时的恢复回退）
#[tauri::command]
#[specta::specta]
pub fn bench_latest_indexed_session(
    ctx: State<'_, BenchContext>,
    index: State<'_, hamster_index::IndexStore>,
    agent: String,
    project_dir: String,
) -> Result<Option<hamster_core::SessionSummary>, AppError> {
    let _ = hamster_index::ingest_all(&ctx.registry, &index);
    Ok(index
        .latest_for_project(&agent, &project_dir)
        .map_err(bench_err)?
        .map(|(key, title)| hamster_core::SessionSummary {
            agent,
            session_key: key,
            project_path: project_dir,
            title,
            last_active_at: 0,
            source_path: String::new(),
            resumable: false,
            resume_via_tui: false,
        }))
}

/// 删除一条历史会话：备份源文件（上游红线②，备份后删）→ 删除 → 清索引派生数据
#[tauri::command]
#[specta::specta]
pub fn bench_session_delete(
    ctx: State<'_, BenchContext>,
    index: State<'_, hamster_index::IndexStore>,
    agent: String,
    session_key: String,
) -> Result<(), AppError> {
    let Some(src) = index
        .source_path_for(&agent, &session_key)
        .map_err(bench_err)?
    else {
        return Err(bench_err(hamster_core::HamsterError::not_found(format!(
            "会话 {agent}/{session_key}"
        ))));
    };
    let path = std::path::Path::new(&src);
    if path.exists() {
        let scope = hamster_core::Scope::User;
        ctx.backup
            .snapshot(&agent, &scope, path)
            .map_err(|e| AppError::new("BENCH_BACKUP", format!("删除前备份失败：{e}")))?;
        std::fs::remove_file(path)
            .map_err(|e| bench_err(hamster_core::HamsterError::io(path, e)))?;
    }
    index.reset_file(&agent, &src).map_err(bench_err)
}
