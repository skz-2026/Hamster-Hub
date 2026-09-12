//! 桌面助手：bench 会话的仓鼠预设（M4「Agent 原生桌面」核心入口）。
//!
//! 与普通 bench 会话的差异：
//! 1. 不绑项目目录——cwd = store_root/assistant（助手数据目录）；
//! 2. 注入 hamster-desktop MCP server：**统一 HTTP 形态**（用户决策：HTTP 是
//!    MCP 规范传输，与具体 agent 无关）——claude 走 `--mcp-config` 内联 JSON，
//!    ACP 走 session/new 的 mcpServers（runtime 透传）；server 本体随主进程
//!    常驻（mcp_server.rs：127.0.0.1 + 每会话 Bearer 令牌）；
//! 3. persona：claude 走 `--append-system-prompt`，其它方言回退为首条 prompt 前缀；
//! 4. computer use 受 settings.agent.computer_use_enabled 门控（默认关，
//!    McpHub::set_cu_allowed 运行时联动）；全部工具调用经审计留证；
//! 5. 急停 = 吊销会话令牌（bench_stream_kill 时自动执行，调用立即 401）。

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, State};

use crate::bench::{bench_err, commands::spawn_stream_session, BenchContext};
use crate::error::AppError;
use crate::mcp_server::McpHub;
use crate::store;

// ===== 桌面 MCP 分发（hamster-core 同步引擎：按 agent 开关写入其配置文件）=====

/// 分发状态（agent_mcp_status 返回项）
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentMcpStatus {
    pub agent_id: String,
    pub agent_name: String,
    pub installed: bool,
    /// 该 agent 是否支持 MCP 配置分发
    pub mcp_capable: bool,
    /// 用户开关（settings.agent.mcp_agents）
    pub enabled: bool,
    /// 配置文件路径（mcp 不可用为空串）
    pub config_path: String,
}

/// 接入信息（设置页展示 + 供手动复制到任意 MCP 宿主）
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct McpAccessInfo {
    pub url: String,
    pub token: String,
    pub port: u16,
    /// 端口回退过（首选端口被占用，分发出去的固定 URL 当前不可用）
    pub port_fell_back: bool,
    pub default_port: u16,
}

/// 源 IR：hamster-desktop HTTP server（URL + 令牌来自 McpHub）
pub fn build_desktop_mcp_server(url: &str, token: &str) -> hamster_core::model::mcp::McpServer {
    use std::collections::BTreeMap;
    let mut headers = BTreeMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {token}"));
    hamster_core::model::mcp::McpServer {
        name: MCP_SERVER_KEY.to_string(),
        transport: hamster_core::model::mcp::Transport::Http {
            url: url.to_string(),
            headers,
        },
        enabled: true,
        disabled_tools: Vec::new(),
    }
}

/// 各 agent 的桌面 MCP 分发状态
#[tauri::command]
#[specta::specta]
pub fn agent_mcp_status(
    ctx: State<'_, BenchContext>,
    state: State<'_, crate::AppState>,
) -> Result<Vec<AgentMcpStatus>, AppError> {
    let settings = load_settings(&state)?;
    let infos = hamster_core::scanner::scan(&ctx.registry);
    Ok(infos
        .into_iter()
        .map(|info| {
            let adapter = match ctx.registry.get(&info.id) {
                Ok(a) => a,
                Err(_) => {
                    return AgentMcpStatus {
                        agent_id: info.id,
                        agent_name: info.name,
                        installed: info.installed,
                        mcp_capable: false,
                        enabled: false,
                        config_path: String::new(),
                    }
                }
            };
            AgentMcpStatus {
                enabled: settings.agent.mcp_agents.iter().any(|a| a == &info.id),
                config_path: adapter
                    .mcp_path(&hamster_core::model::mcp::Scope::User)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                mcp_capable: adapter.capabilities().mcp,
                agent_id: info.id,
                agent_name: info.name,
                installed: info.installed,
            }
        })
        .collect())
}

/// 接入信息（URL 含实际端口；端口回退时前端提示更新已分发配置）
#[tauri::command]
#[specta::specta]
pub fn agent_mcp_access_info(
    hub: State<'_, McpHub>,
    state: State<'_, crate::AppState>,
) -> Result<McpAccessInfo, AppError> {
    let settings = load_settings(&state)?;
    Ok(McpAccessInfo {
        url: hub.url(),
        token: settings.agent.mcp_user_token.unwrap_or_default(),
        port: hub.port(),
        port_fell_back: hub.port_fell_back(),
        default_port: crate::mcp_server::DEFAULT_MCP_PORT,
    })
}

/// 开/关某 agent 的桌面 MCP 分发：
/// 开 = 源 upsert（HTTP IR）+ target 启用 → plan/apply 写入其配置文件；
/// 关 = 源临时置 disabled → 仅对该 target apply（渲染省略 = 摘除条目）→ 恢复源。
/// Drift（基线外的既有配置）统一 Overwrite：render 本身「读现有→合并」，
/// 用户已有条目保留，apply 自动落备份可回滚。
#[tauri::command]
#[specta::specta]
pub fn agent_mcp_set_enabled(
    ctx: State<'_, BenchContext>,
    hub: State<'_, McpHub>,
    state: State<'_, crate::AppState>,
    agent_id: String,
    enabled: bool,
) -> Result<(), AppError> {
    use hamster_core::sync::{DriftDecision, SyncEngine, TargetAction};
    use std::collections::BTreeMap;

    let adapter = ctx.registry.get(&agent_id).map_err(bench_err)?;
    if adapter
        .mcp_path(&hamster_core::model::mcp::Scope::User)
        .is_none()
    {
        return Err(bench_err(hamster_core::HamsterError::Unsupported(format!(
            "{agent_id} 不支持 MCP 配置文件分发"
        ))));
    }

    let mut settings = load_settings(&state)?;
    let token = ensure_user_token(&mut settings, &state, &hub);
    let url = hub.url();

    let mut cfg = ctx.store.load_config().map_err(bench_err)?;
    if enabled {
        cfg.upsert_server(build_desktop_mcp_server(&url, &token));
        cfg.ensure_targets(&[agent_id.as_str()]);
        set_target_enabled(&mut cfg, &agent_id, true);
    } else {
        // 摘除：源临时 disabled → 渲染不含条目；target 一并停用（后续同步不再触及）
        let mut off = build_desktop_mcp_server(&url, &token);
        off.enabled = false;
        cfg.upsert_server(off);
        set_target_enabled(&mut cfg, &agent_id, false);
    }
    ctx.store.save_config(&cfg).map_err(bench_err)?;

    let engine = SyncEngine::new(&ctx.store, &ctx.backup, &ctx.registry);
    let plans = engine
        .plan(std::slice::from_ref(&(
            agent_id.clone(),
            hamster_core::model::mcp::Scope::User,
        )))
        .map_err(bench_err)?;
    let mut decisions = BTreeMap::new();
    for plan in &plans {
        if plan.action == TargetAction::Drift {
            decisions.insert(plan.path.clone(), DriftDecision::Overwrite);
        }
    }
    let report = engine.apply(&plans, &decisions).map_err(bench_err)?;

    // 关闭语义：恢复源 enabled=true（本次只 apply 了该 target，其他 agent 文件未触碰）
    if !enabled {
        let mut cfg = ctx.store.load_config().map_err(bench_err)?;
        cfg.upsert_server(build_desktop_mcp_server(&url, &token));
        ctx.store.save_config(&cfg).map_err(bench_err)?;
    }

    // 开关状态落库（单一事实源 = settings.agent.mcp_agents）
    settings.agent.mcp_agents.retain(|a| a != &agent_id);
    if enabled {
        settings.agent.mcp_agents.push(agent_id.clone());
    }
    persist_settings(&state, &settings)?;

    if !report.message.is_empty() {
        eprintln!("[mcp-sync] {agent_id}: {}", report.message);
    }
    Ok(())
}

fn set_target_enabled(cfg: &mut hamster_core::store::StoreConfig, agent_id: &str, enabled: bool) {
    if let Some(t) = cfg
        .targets
        .iter_mut()
        .find(|t| t.adapter == agent_id && t.scope == hamster_core::model::mcp::Scope::User)
    {
        t.enabled = enabled;
    }
}

fn load_settings(state: &crate::AppState) -> Result<store::config::Settings, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::settings::load(&conn)
}

fn persist_settings(
    state: &crate::AppState,
    settings: &store::config::Settings,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::settings::save(&conn, settings).map(|_| ())
}

/// 令牌兜底：理论上 setup 已生成；此处补齐并持久化 + 注册进 hub
fn ensure_user_token(
    settings: &mut store::config::Settings,
    state: &crate::AppState,
    hub: &McpHub,
) -> String {
    let existing = settings
        .agent
        .mcp_user_token
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    if !existing.is_empty() {
        return existing;
    }
    let token = uuid::Uuid::new_v4().to_string();
    settings.agent.mcp_user_token = Some(token.clone());
    if let Err(e) = persist_settings(state, settings) {
        eprintln!("[mcp-sync] 用户令牌持久化失败: {e}");
    }
    hub.register_user_token(&token);
    token
}

/// MCP server 键名（--mcp-config 的 servers 键 / ACP server 名 /
/// claude allowedTools 的 `mcp__<key>` 前缀，三处必须一致）
pub const MCP_SERVER_KEY: &str = "hamster-desktop";

/// 助手工作目录（store_root 下）：audit.jsonl / 会话 cwd
pub const ASSISTANT_DIR: &str = "assistant";

pub const DEFAULT_PERSONA: &str =
    "你是「囤囤」，仓鼠Hub 的桌面助手：亲切、简短、不装萌卖傻，默认说中文。\
你运行在用户的 Windows 桌面助手里，可以使用 hamster-desktop 提供的工具直接操作这台电脑：\
搜索并启动应用（app_search / app_launch）、搜索并打开文件（file_search / file_open）、\
管理待办（todo_list / todo_create / todo_set_done）、查询与调节音量（volume_get / volume_set）、整理 iOS 主屏图标（home_apps_list 看应用 → home_layout_get 看现状 → home_layout_set 写整理结果，写完主屏立即刷新）；\
需要网页信息时用 browser_* 工具；操作图形界面用 computer_* 工具（须用户显式开启）。\
原则：优先用结构化工具而不是模拟点击；执行有副作用的操作前先复述你要做什么；\
一次只做用户交代的事，顺手优化要先问。";

/// bench_assistant_create 参数包
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantCreateArgs {
    pub first_prompt: Option<String>,
    pub model: Option<String>,
    /// 指定代理；缺省按 claude → zcode → 首个已安装顺序解析
    pub agent_id: Option<String>,
}

/// 桌面助手会话信息（返回给前端；附注入摘要便于调试与展示）
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSessionInfo {
    pub session: hamster_core::LiveSessionInfo,
    pub agent_id: String,
    /// 本会话是否注入了桌面 MCP server
    pub mcp_injected: bool,
    /// 本会话 computer use 是否放行（未放行时 computer_* 工具被 server 拒绝）
    pub computer_use_enabled: bool,
}

/// claude `--mcp-config` 的内联 JSON（HTTP 内嵌承载：URL + 每会话令牌）
pub fn mcp_http_config_value(url: &str, token: &str) -> serde_json::Value {
    serde_json::json!({
        "mcpServers": {
            MCP_SERVER_KEY: {
                "type": "http",
                "url": url,
                "headers": { "Authorization": format!("Bearer {token}") },
            }
        }
    })
}

/// ACP session/new 的 mcpServers 条目（同承载，ACP 规范的 http kind 字段）
pub fn acp_http_entry(url: &str, token: &str) -> serde_json::Value {
    serde_json::json!({
        "name": MCP_SERVER_KEY,
        "kind": "http",
        "url": url,
        "headers": { "Authorization": format!("Bearer {token}") },
    })
}

/// claude 会话级注入参数：persona + MCP 配置（内联 JSON）+ 仅放行本 server 的工具
/// （headless 下未放行工具会被自动拒绝；不放行 Bash/Edit 等宿主工具）
pub fn claude_injection_args(mcp_config_json: &str, persona: &str) -> Vec<String> {
    vec![
        "--append-system-prompt".into(),
        persona.into(),
        "--mcp-config".into(),
        mcp_config_json.into(),
        "--allowedTools".into(),
        format!("mcp__{MCP_SERVER_KEY}"),
    ]
}

/// 非 claude 方言的 persona 回退：首条 prompt 前缀（通用、零协议依赖）
pub fn prompt_with_persona(persona: &str, prompt: &str) -> String {
    format!("[系统人设，请全程遵守]\n{persona}\n\n[用户]\n{prompt}")
}

/// 默认代理解析：claude（结构化注入最完整）→ zcode → 首个已装且支持
/// 结构化通道的代理。都没有则报错引导。
fn pick_agent_id(
    registry: &hamster_core::Registry,
    preferred: Option<&str>,
) -> Result<String, AppError> {
    let supports = |a: &dyn hamster_core::AgentAdapter| {
        a.detect().is_some()
            && a.runtime()
                .as_ref()
                .and_then(|r| r.structured.as_ref())
                .is_some()
    };
    if let Some(id) = preferred.map(str::trim).filter(|s| !s.is_empty()) {
        let a = registry.get(id).map_err(bench_err)?;
        if !supports(a) {
            return Err(bench_err(hamster_core::HamsterError::Unsupported(format!(
                "{id} 未安装或不支持结构化通道，无法担任桌面助手"
            ))));
        }
        return Ok(id.to_string());
    }
    for id in ["claude", "zcode"] {
        if let Ok(a) = registry.get(id) {
            if supports(a) {
                return Ok(id.to_string());
            }
        }
    }
    registry
        .list()
        .iter()
        .find(|a| supports(a.as_ref()))
        .map(|a| a.id().to_string())
        .ok_or_else(|| {
            bench_err(hamster_core::HamsterError::Unsupported(
                "未找到已安装的 Agent CLI：请先安装 Claude Code / ZCode 等其中一个".into(),
            ))
        })
}

/// 创建桌面助手会话：解析默认代理 → 铸造令牌 → 注入 HTTP MCP 配置 → spawn。
#[tauri::command]
#[specta::specta]
pub fn bench_assistant_create(
    ctx: State<'_, BenchContext>,
    streams: State<'_, hamster_runtime::StreamManager>,
    hub: State<'_, McpHub>,
    app: AppHandle,
    state: State<'_, crate::AppState>,
    args: AssistantCreateArgs,
) -> Result<AssistantSessionInfo, AppError> {
    let settings = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        store::settings::load(&conn)?
    };
    let cu_enabled = settings.agent.computer_use_enabled;
    let persona = if settings.agent.assistant_persona.trim().is_empty() {
        DEFAULT_PERSONA.to_string()
    } else {
        settings.agent.assistant_persona.clone()
    };

    let agent_id = pick_agent_id(&ctx.registry, args.agent_id.as_deref())?;
    let adapter = ctx.registry.get(&agent_id).map_err(bench_err)?;
    let spec = adapter.runtime().ok_or_else(|| {
        bench_err(hamster_core::HamsterError::Unsupported(format!(
            "{agent_id} 暂不支持内嵌对话"
        )))
    })?;
    let channel = spec.structured.clone().ok_or_else(|| {
        bench_err(hamster_core::HamsterError::Unsupported(format!(
            "{agent_id} 暂不支持结构化流式通道"
        )))
    })?;

    // 助手工作目录（会话 cwd；审计文件由 McpHub 启动时落位）
    let assistant_dir = ctx.store_root.join(ASSISTANT_DIR);
    std::fs::create_dir_all(&assistant_dir).map_err(|e| AppError::io(e.to_string()))?;

    // 令牌先行铸造（注入配置要用）；spawn 失败回收，成功后绑定会话供急停
    let token = hub.mint();

    // 统一 HTTP 形态注入：claude = spawn flag（内联 JSON）；其它 = ACP mcpServers
    // （运行时透传 session/new）+ persona 前缀回退。Bash 等宿主工具不在
    // allowedTools 内，headless 自动拒绝。
    let url = hub.url();
    let (extra_args, mcp_servers, first_prompt) =
        if channel.dialect == hamster_core::ProtocolDialect::ClaudeStream {
            let cfg = mcp_http_config_value(&url, &token).to_string();
            (
                claude_injection_args(&cfg, &persona),
                Vec::new(),
                args.first_prompt,
            )
        } else {
            (
                Vec::new(),
                vec![acp_http_entry(&url, &token)],
                args.first_prompt.map(|p| prompt_with_persona(&persona, &p)),
            )
        };

    let spawned = spawn_stream_session(
        &ctx,
        &streams,
        &app,
        agent_id.clone(),
        &assistant_dir.display().to_string(),
        &spec,
        channel,
        extra_args,
        args.model,
        None,
        None,
        false,
        first_prompt,
        mcp_servers,
        false,
    );
    let session = match spawned {
        Ok(s) => s,
        Err(e) => {
            hub.revoke_token(&token);
            return Err(e);
        }
    };
    hub.bind_session(&session.session_id, &token);

    Ok(AssistantSessionInfo {
        session,
        agent_id,
        mcp_injected: true,
        computer_use_enabled: cu_enabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_config_has_type_url_and_auth_header() {
        let cfg = mcp_http_config_value("http://127.0.0.1:45678/mcp", "tok-1");
        let server = cfg
            .pointer("/mcpServers/hamster-desktop")
            .expect("server 键必须是 hamster-desktop");
        assert_eq!(server.get("type").and_then(|v| v.as_str()), Some("http"));
        assert_eq!(
            server.get("url").and_then(|v| v.as_str()),
            Some("http://127.0.0.1:45678/mcp")
        );
        assert_eq!(
            server
                .pointer("/headers/Authorization")
                .and_then(|v| v.as_str()),
            Some("Bearer tok-1")
        );
    }

    #[test]
    fn acp_entry_uses_http_kind() {
        let e = acp_http_entry("http://127.0.0.1:1/mcp", "t");
        assert_eq!(e.get("name").and_then(|v| v.as_str()), Some(MCP_SERVER_KEY));
        assert_eq!(e.get("kind").and_then(|v| v.as_str()), Some("http"));
        assert!(e.get("url").is_some());
    }

    #[test]
    fn claude_args_pair_flags_with_values() {
        let cfg = mcp_http_config_value("http://127.0.0.1:1/mcp", "t").to_string();
        let args = claude_injection_args(&cfg, "人设");
        // flag/value 必须成对出现，否则 spawn 直接失败
        assert_eq!(args.len() % 2, 0, "参数应两两成对：{args:?}");
        for pair in args.chunks(2) {
            assert!(pair[0].starts_with("--"), "奇数位应是 flag：{pair:?}");
        }
        assert!(args.contains(&"--mcp-config".to_string()));
        assert!(args.contains(&"mcp__hamster-desktop".to_string()));
        // 内联 JSON 串必须能被解析（--mcp-config 接受文件路径或字符串）
        let cfg_arg = args
            .iter()
            .zip(args.iter().skip(1))
            .find(|(f, _)| f.as_str() == "--mcp-config")
            .map(|(_, v)| v)
            .expect("应有 --mcp-config");
        assert!(serde_json::from_str::<serde_json::Value>(cfg_arg).is_ok());
    }

    #[test]
    fn persona_prefix_wraps_user_prompt() {
        let p = prompt_with_persona("人设A", "帮我开微信");
        assert!(p.starts_with("[系统人设"));
        assert!(p.ends_with("帮我开微信"));
    }

    #[test]
    fn desktop_server_ir_is_http_with_bearer() {
        let s = build_desktop_mcp_server("http://127.0.0.1:47613/mcp", "tok");
        assert_eq!(s.name, MCP_SERVER_KEY);
        assert!(s.enabled, "分发源必须 enabled");
        match s.transport {
            hamster_core::model::mcp::Transport::Http { url, headers } => {
                assert_eq!(url, "http://127.0.0.1:47613/mcp");
                assert_eq!(
                    headers.get("Authorization").map(String::as_str),
                    Some("Bearer tok")
                );
            }
            _ => panic!("桌面 MCP 必须是 Http 传输"),
        }
    }

    #[test]
    fn toggle_off_disables_source_and_target() {
        use hamster_core::store::StoreConfig;
        let mut cfg = StoreConfig::default();
        cfg.upsert_server(build_desktop_mcp_server("http://x/mcp", "t"));
        cfg.ensure_targets(&["claude"]);
        set_target_enabled(&mut cfg, "claude", true);
        assert!(cfg.targets[0].enabled);

        // 摘除变换：源 disabled（渲染省略 = 从该 agent 配置移除）+ target 停用
        let mut off = build_desktop_mcp_server("http://x/mcp", "t");
        off.enabled = false;
        cfg.upsert_server(off);
        set_target_enabled(&mut cfg, "claude", false);
        assert!(!cfg.servers[0].enabled, "源 disabled = 渲染省略条目");
        assert!(!cfg.targets[0].enabled, "target 停用 = 后续同步不触及");
    }
}
