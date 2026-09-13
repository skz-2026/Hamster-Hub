//! 仓鼠Hub Rust 核心装配
//!
//! 分层（见 docs/04-architecture.md §3）：commands(薄层) → core/desktop_mode → store/platform。
//! M1 范围：桌面模式（系统接管三件套 + iOS 主屏）、应用索引与启动、KV 布局存储。

mod bench;
#[cfg(test)]
mod benchmarks;
mod commands;
mod core;
mod desktop_mode;
mod error;
mod events;
mod mcp_server;
mod plugins;
mod store;

use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_specta::{collect_commands, collect_events, Event};

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub db_path: std::path::PathBuf,
    pub icons_dir: std::path::PathBuf,
    /// 密码箱会话（主密钥仅存内存；锁定/退出即丢弃）
    pub vault: Mutex<core::vault::VaultSession>,
}

impl AppState {
    /// 索引根目录（按当前设置展开 ~；文件打开白名单也用它）
    pub fn settings_snapshot_roots(&self) -> Vec<std::path::PathBuf> {
        let Ok(conn) = self.db.lock() else {
            return vec![];
        };
        store::settings::load(&conn)
            .map(|s| core::fileindex::roots_from(&s.file_index.roots))
            .unwrap_or_default()
    }

    pub fn settings_snapshot_max_files(&self) -> u32 {
        let Ok(conn) = self.db.lock() else {
            return 100_000;
        };
        store::settings::load(&conn)
            .map(|s| s.file_index.max_files)
            .unwrap_or(100_000)
    }
}

fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::ai::ai_chat,
            commands::control::volume_get,
            commands::control::volume_set,
            commands::settings::settings_load,
            commands::settings::settings_save,
            commands::settings::agent_cli_path_set,
            commands::settings::kv_get,
            commands::settings::kv_set,
            commands::system::app_health,
            commands::system::window_set_pinned,
            commands::system::open_url,
            commands::system::wallpaper_image_import,
            commands::apps::app_list,
            commands::apps::app_launch,
            commands::apps::app_launch_new,
            commands::apps::app_close,
            commands::apps::apps_running,
            commands::apps::app_search,
            // dock 右键菜单弹窗（任务栏条内放不下纵向菜单，弹独立置顶小窗）
            commands::dock::dock_menu_open,
            commands::dock::dock_menu_payload,
            commands::dock::dock_menu_hide,
            commands::todo::todo_list,
            commands::todo::todo_create,
            commands::todo::todo_toggle,
            commands::todo::todo_delete,
            commands::todo::todo_set_due,
            commands::todo::todo_set_recur,
            // 密码箱（字段级加密：Argon2id + AES-256-GCM；密钥不出后端）
            commands::vault::vault_status,
            commands::vault::vault_setup,
            commands::vault::vault_unlock,
            commands::vault::vault_lock,
            commands::vault::vault_set_auto_lock,
            commands::vault::vault_change_password,
            commands::vault::vault_item_list,
            commands::vault::vault_item_search,
            commands::vault::vault_item_create,
            commands::vault::vault_item_update,
            commands::vault::vault_item_toggle_favorite,
            commands::vault::vault_item_delete,
            commands::vault::vault_item_reveal,
            commands::vault::vault_copy_field,
            commands::vault::vault_password_generate,
            commands::vault::vault_password_strength,
            // 番茄钟（后端计时 + 历史聚合）
            commands::focus::focus_start,
            commands::focus::focus_break,
            commands::focus::focus_pause,
            commands::focus::focus_resume,
            commands::focus::focus_stop,
            commands::focus::focus_status,
            commands::focus::focus_history,
            commands::note::note_list,
            commands::note::note_create,
            commands::note::note_update,
            commands::note::note_toggle_pin,
            commands::note::note_delete,
            commands::note::countdown_custom_list,
            commands::note::countdown_custom_create,
            commands::note::countdown_custom_delete,
            commands::dashboard::weather_get,
            commands::dashboard::weather_set_city,
            commands::dashboard::countdown_list,
            commands::dashboard::recent_files,
            commands::dashboard::top_apps,
            commands::sysinfo::system_stats,
            commands::sysinfo::process_list,
            commands::sysinfo::process_kill,
            commands::files::file_search,
            commands::files::file_index_refresh,
            commands::files::open_path,
            commands::files::reveal_in_explorer,
            commands::desktop::desktop_mode_enter,
            commands::desktop::desktop_mode_exit,
            commands::desktop::desktop_mode_is_active,
            commands::desktop::start_menu_open,
            // bench（代理工作台，上游域层整合 MVP）
            bench::commands::bench_scan_agents,
            bench::commands::bench_list_projects,
            bench::commands::bench_agent_workspaces,
            bench::commands::bench_stream_create,
            bench::commands::bench_stream_send,
            bench::commands::bench_stream_interrupt,
            bench::commands::bench_stream_kill,
            bench::commands::bench_list_stream_sessions,
            bench::commands::bench_list_live_sessions,
            bench::commands::bench_pty_create,
            bench::commands::bench_pty_write,
            bench::commands::bench_pty_send_prompt,
            bench::commands::bench_pty_resize,
            bench::commands::bench_pty_kill,
            bench::commands::bench_list_history_sessions,
            bench::commands::bench_search_sessions,
            bench::commands::bench_session_messages,
            bench::commands::bench_list_session_messages,
            bench::commands::bench_latest_indexed_session,
            bench::commands::bench_index_status,
            bench::commands::bench_index_refresh,
            bench::commands::bench_reindex,
            bench::commands::bench_session_delete,
            // 桌面助手（Agent 原生桌面：persona + hamster-desktop MCP 注入）
            bench::assistant::bench_assistant_create,
            // 桌面 MCP 分发（按 agent 开关写入配置文件）+ 接入信息
            bench::assistant::agent_mcp_status,
            bench::assistant::agent_mcp_set_enabled,
            bench::assistant::agent_mcp_access_info,
            // UI 插件（主屏小组件扩展）
            plugins::plugin_list,
            plugins::plugin_read_code,
            plugins::plugin_storage_get,
            plugins::plugin_storage_set,
            plugins::plugin_delete,
            plugins::plugin_bridge_call,
            // 托盘管理（dock 右侧系统托盘区）
            commands::tray::tray_open_overflow,
            // 应用自更新（GitHub Releases latest.json）
            commands::updater::update_check,
            commands::updater::update_install,
        ])
        .events(collect_events![
            events::CoreReady,
            events::DesktopModeChanged,
            events::AppIndexUpdated,
            events::FileIndexUpdated,
            events::BenchStreamEvent,
            events::BenchStreamExit,
            events::BenchPtyExit,
            events::TodoReminder,
            events::FocusTick,
            events::FocusFinished,
            events::VaultLocked,
            events::UpdateProgress
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
}

/// 导出 IPC TypeScript 类型（dev 启动与 cargo test export_bindings 时执行）
#[cfg(any(debug_assertions, test))]
fn export_bindings(builder: &tauri_specta::Builder<tauri::Wry>) {
    use specta_typescript::Typescript;
    let out = std::path::Path::new("../src/shared/types/ipc.ts");
    if let Some(dir) = out.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    builder
        .export(
            // 64 位整数（会话时间戳等）按 number 导出，避免 TS bigint
            Typescript::default().bigint(specta_typescript::BigIntExportBehavior::Number),
            out,
        )
        .expect("导出 IPC TypeScript 类型失败");
}

pub use mcp_server::McpHub;

/// `hamster-hub.exe mcp serve`：桌面 MCP server 的 stdio 承载（ACP/codex 兜底，
/// claude 主线走内嵌 HTTP——见 mcp_server.rs）。argv 分发在 Tauri/单实例
/// 初始化之前，因此不会与 GUI 实例冲突。
pub fn mcp_serve() {
    hamster_mcp::serve_stdio();
}

/// `hamster-hub.exe mcp selftest <url>`：browser-use 冒烟（真实浏览器链路自检）
pub fn mcp_selftest(url: &str) -> bool {
    hamster_mcp::selftest(url)
}

pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        // 单实例：二次启动时唤起已有主窗口
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        // bench 选择项目目录的原生文件夹选择器
        .plugin(tauri_plugin_dialog::init())
        // 待办提醒的系统通知（Rust 侧直发）
        .plugin(tauri_plugin_notification::init())
        // 密码箱：剪贴板写入与到时自动清除（Rust 侧直调，不走 JS 权限）
        .plugin(tauri_plugin_clipboard_manager::init())
        // 应用自更新（GitHub Releases）：检查/安装经 commands::updater 的 Rust 命令
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            // 桌面接管对账：上次会话崩溃/断电/看门狗失守可能留下隐藏态残留
            // 快照，启动即恢复，避免用户面对无任务栏无图标的桌面（见 desktop_mode）
            desktop_mode::reconcile_stale_snapshot();

            // 本地存储（迁移在 open 内完成）
            let db_path = app
                .path()
                .app_data_dir()
                .map_err(|e| error::AppError::io(e.to_string()))?
                .join("hamsterhub.db");
            let db = store::db::open_at(&db_path)?;
            let icons_dir = db_path.parent().unwrap().join("icons");
            app.manage(AppState {
                db: Mutex::new(db),
                db_path,
                icons_dir,
                vault: Mutex::new(core::vault::VaultSession::locked()),
            });
            // 系统信息监控（内存/磁盘/CPU/温度/进程）
            app.manage(commands::sysinfo::SharedMonitor::new(
                commands::sysinfo::SystemMonitor::new(),
            ));
            // 番茄钟计时状态（秒级 ticker 线程共享）
            app.manage(core::focus::FocusState::default());

            // bench（代理工作台）：上游 域层装配（sessions.db 独立自管，不入主库迁移链）
            let bench_data = app
                .path()
                .app_data_dir()
                .map_err(|e| error::AppError::io(e.to_string()))?;
            let bench_ctx = bench::BenchContext::init(&bench_data)?;
            let bench_index = hamster_index::IndexStore::open(
                &bench_ctx.store_root.join("index").join("sessions.db"),
            )
            .map_err(bench::bench_err)?;
            app.manage(bench_ctx);
            app.manage(bench_index);
            app.manage(hamster_runtime::StreamManager::new());
            app.manage(hamster_runtime::SessionManager::new());

            // UI 插件：首启自举内置示例（用户可直接改插件目录里的文件即时改效果）
            if let Err(e) = plugins::ensure_examples(
                &app.path()
                    .app_data_dir()
                    .map_err(|e| error::AppError::io(e.to_string()))?,
            ) {
                eprintln!("[plugins] 示例插件写入失败: {e}");
            }

            // 桌面 MCP server 内嵌 HTTP 承载（M4）：127.0.0.1 端口 + 双层令牌
            //（用户级长效令牌 = 外部宿主接入；会话令牌 = bench 助手，急停可吊销）
            {
                let state: tauri::State<AppState> = app.state();
                let assistant_dir = bench_data
                    .join("agent")
                    .join(bench::assistant::ASSISTANT_DIR);
                let _ = std::fs::create_dir_all(&assistant_dir);
                let (cu_allowed, pref_port, user_token) = {
                    let conn = state
                        .db
                        .lock()
                        .map_err(|e| error::AppError::poison(e.to_string()))?;
                    let mut s = store::settings::load(&conn)?;
                    let cu = s.agent.computer_use_enabled;
                    // 首启无用户令牌：生成并持久化（外部 agent 配置跨重启有效）
                    if s.agent
                        .mcp_user_token
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty()
                    {
                        s.agent.mcp_user_token = Some(uuid::Uuid::new_v4().to_string());
                        store::settings::save(&conn, &s)?;
                    }
                    let token = s.agent.mcp_user_token.unwrap_or_default();
                    (cu, s.agent.mcp_port, token)
                };
                // 内嵌 server 独享一条主库连接（与 AppState 的 Mutex 连接并存，WAL 并发）
                let conn = hamster_mcp::desktop::open_db(&state.db_path)?;
                let hub = McpHub::start(
                    conn,
                    cu_allowed,
                    Some(assistant_dir.join("audit.jsonl")),
                    pref_port,
                    Some(app.handle().clone()),
                )?;
                hub.register_user_token(&user_token);
                app.manage(hub);
            }

            // 全局热键（从设置读取；注册失败只降级告警，不阻塞启动）
            let (spot_hotkey, desk_hotkey) = {
                let state: tauri::State<AppState> = app.state();
                let conn = state
                    .db
                    .lock()
                    .map_err(|e| error::AppError::poison(e.to_string()))?;
                let s = store::settings::load(&conn)?;
                (s.search.hotkey, s.behavior.desktop_mode_hotkey)
            };
            let gs = app.global_shortcut();
            if let Err(e) = gs.on_shortcut(
                spot_hotkey.to_lowercase().as_str(),
                |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_spotlight(app);
                    }
                },
            ) {
                eprintln!(
                    "[hotkey] {spot_hotkey} 注册失败（可能被占用），Spotlight 请用托盘菜单: {e}"
                );
            }
            if let Err(e) = gs.on_shortcut(
                desk_hotkey.to_lowercase().as_str(),
                |app, _shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let result = if desktop_mode::is_active() {
                        desktop_mode::exit(app)
                    } else {
                        desktop_mode::enter(app)
                    };
                    if let Err(e) = result {
                        eprintln!("[hotkey] 桌面模式切换失败: {e}");
                    }
                    refresh_tray_menu(app);
                },
            ) {
                eprintln!("[hotkey] {desk_hotkey} 注册失败: {e}");
            }

            // 托盘：左键显示主窗口，右键菜单（桌面模式项按状态重建）
            let tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().expect("缺少应用图标").clone())
                .tooltip("仓鼠Hub")
                .menu(&build_tray_menu(app.handle())?)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main(app),
                    "spotlight" => toggle_spotlight(app),
                    "enter-desktop" => {
                        if let Err(e) = desktop_mode::enter(app) {
                            eprintln!("[desktop_mode] 进入失败: {e}");
                        }
                        refresh_tray_menu(app);
                    }
                    "exit-desktop" => {
                        if let Err(e) = desktop_mode::exit(app) {
                            eprintln!("[desktop_mode] 退出失败: {e}");
                        }
                        refresh_tray_menu(app);
                    }
                    "quit" => {
                        let _ = desktop_mode::exit(app);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            let _ = tray;

            // 后台应用索引扫描（含图标缓存提取）
            let state: tauri::State<AppState> = app.state();
            core::appindex::spawn_scan(
                app.handle().clone(),
                state.db_path.clone(),
                state.icons_dir.clone(),
            );

            // 后台文件索引扫描（FTS5 + 拼音）+ 目录变更 watcher
            let roots = state.settings_snapshot_roots();
            let max_files = state.settings_snapshot_max_files();
            core::fileindex::spawn_scan(
                app.handle().clone(),
                state.db_path.clone(),
                roots.clone(),
                max_files,
            );
            core::fileindex::spawn_watcher(
                app.handle().clone(),
                state.db_path.clone(),
                roots,
                max_files,
            );

            // 待办提醒调度（独立连接轮询到点提醒 → 系统通知 + 应用内事件）
            core::reminder::spawn(app.handle().clone(), state.db_path.clone());

            // 密码箱自动锁看门狗：闲置超时 → 丢弃密钥并广播（前端弹锁屏覆盖层）
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(10));
                    let state: tauri::State<AppState> = handle.state();
                    let locked = {
                        let Ok(mut session) = state.vault.lock() else {
                            continue;
                        };
                        session.lock_if_idle()
                    };
                    if locked {
                        let _ = events::VaultLocked {
                            reason: "idle".to_string(),
                        }
                        .emit(&handle);
                    }
                });
            }

            // 启动即进入桌面模式（打开应用直接全屏工作台 + 底部 Dock）
            {
                let enable_launch_desktop = {
                    let conn = state
                        .db
                        .lock()
                        .map_err(|e| error::AppError::poison(e.to_string()))?;
                    store::settings::load(&conn)?
                        .behavior
                        .desktop_mode_on_launch
                };
                if enable_launch_desktop {
                    if let Err(e) = desktop_mode::enter(app.handle()) {
                        eprintln!("[desktop_mode] 启动自动进入失败: {e}");
                    }
                    refresh_tray_menu(app.handle());
                }
            }

            // 主窗口不透明（tauri.conf transparent 已关）：亚克力在部分机器/背景色下
            // 会把整窗渲染成半透明灰，配色全乱。毛玻璃观感由 CSS 背景色承担，
            // Spotlight 覆盖层窗口仍保持透明毛玻璃。

            // 核心就绪事件（前端 TitleBar 展示版本）
            events::CoreReady {
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
            .emit(app)?;

            Ok(())
        })
        // 关闭 = 隐藏到托盘；桌面模式下退出托盘菜单走「退出」并还原系统
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("仓鼠Hub 启动失败");
}

/// 托盘菜单文案（随 settings.behavior.language 切换；缺省回落简体中文）
struct TrayLabels {
    open: &'static str,
    spotlight: &'static str,
    enter_desktop: &'static str,
    enter_desktop_on: &'static str,
    exit_desktop: &'static str,
    quit: &'static str,
}

fn tray_labels(lang: &str) -> TrayLabels {
    if lang == "en" {
        TrayLabels {
            open: "Open HamsterHub",
            spotlight: "Spotlight Search",
            enter_desktop: "Enter Desktop Mode",
            enter_desktop_on: "Desktop Mode (Active)",
            exit_desktop: "Exit Desktop Mode",
            quit: "Quit",
        }
    } else if lang == "zh-TW" {
        TrayLabels {
            open: "開啟倉鼠Hub",
            spotlight: "Spotlight 搜尋",
            enter_desktop: "進入桌面模式",
            enter_desktop_on: "進入桌面模式（已開啟）",
            exit_desktop: "離開桌面模式",
            quit: "結束",
        }
    } else {
        TrayLabels {
            open: "打开仓鼠Hub",
            spotlight: "Spotlight 搜索",
            enter_desktop: "进入桌面模式",
            enter_desktop_on: "进入桌面模式（已开启）",
            exit_desktop: "退出桌面模式",
            quit: "退出",
        }
    }
}

fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let lang = {
        let state: tauri::State<AppState> = app.state();
        let lock = state.db.lock();
        match lock {
            Ok(conn) => store::settings::load(&conn)
                .map(|s| s.behavior.language)
                .unwrap_or_default(),
            Err(_) => String::new(),
        }
    };
    let l = tray_labels(&lang);
    let open = MenuItem::with_id(app, "open", l.open, true, None::<&str>)?;
    let spotlight = MenuItem::with_id(app, "spotlight", l.spotlight, true, None::<&str>)?;
    let (enter, exit) = if desktop_mode::is_active() {
        (
            MenuItem::with_id(
                app,
                "enter-desktop",
                l.enter_desktop_on,
                false,
                None::<&str>,
            )?,
            MenuItem::with_id(app, "exit-desktop", l.exit_desktop, true, None::<&str>)?,
        )
    } else {
        (
            MenuItem::with_id(app, "enter-desktop", l.enter_desktop, true, None::<&str>)?,
            MenuItem::with_id(app, "exit-desktop", l.exit_desktop, false, None::<&str>)?,
        )
    };
    let quit = MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?;
    Menu::with_items(app, &[&open, &spotlight, &enter, &exit, &quit])
}

pub(crate) fn refresh_tray_menu(app: &tauri::AppHandle) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Ok(menu) = build_tray_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Spotlight 覆盖层显隐切换
fn toggle_spotlight(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("spotlight") {
        match w.is_visible() {
            Ok(true) => {
                let _ = w.hide();
            }
            _ => {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// `pnpm codegen:ipc` 的入口：生成/刷新前端 IPC 类型
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
