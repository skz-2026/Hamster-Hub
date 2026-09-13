//! 应用索引查询与启动

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::core::appindex::{self, AppEntry};
use crate::error::AppError;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn app_list(state: State<'_, AppState>) -> Result<Vec<AppEntry>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    appindex::list(&conn)
}

/// 拼音/首字母/名称检索（Spotlight 应用路）
#[tauri::command]
#[specta::specta]
pub fn app_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<AppEntry>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    appindex::search(&conn, &query, limit.unwrap_or(8))
}

/// app_key → exec_target（app_meta 单行查询）
fn exec_target_of(conn: &rusqlite::Connection, app_key: &str) -> Option<String> {
    conn.query_row(
        "SELECT exec_target FROM app_meta WHERE app_key = ?1",
        [app_key],
        |r| r.get(0),
    )
    .ok()
}

/// 启动实现：new_instance = true 时跳过「已运行则激活」，强制再开（dock 右键「多开」）
fn launch(state: State<'_, AppState>, app_key: String, new_instance: bool) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let Some(target) = exec_target_of(&conn, &app_key) else {
        return Err(AppError::validate(format!("应用不在索引中: {app_key}")));
    };
    let target_path = Path::new(&target);
    // 已运行则激活现有窗口（浏览器等单进程多窗应用，重复 shell_open 会再开新窗）
    if !new_instance && hamster_platform::shell::activate_running(target_path) {
        appindex::log_usage(&conn, &app_key);
        return Ok(());
    }
    if !hamster_platform::shell::shell_open(target_path) {
        return Err(AppError::io(format!("启动失败: {target}")));
    }
    appindex::log_usage(&conn, &app_key);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn app_launch(state: State<'_, AppState>, app_key: String) -> Result<(), AppError> {
    launch(state, app_key, false)
}

/// 多开：绕过「已运行激活现有窗口」，直接 shell_open 再开一个。
/// 文件夹/记事本/浏览器得到新窗口；单实例应用由其自身去重（等效激活）。
/// 顺带自学习多开能力：已存在窗口时再开、1.5s 后窗数没涨 → 记为单实例，
/// 菜单不再提供「多开」；涨了 → 移出名单重新放行（见 app_multi_flags）。
#[tauri::command]
#[specta::specta]
pub fn app_launch_new(state: State<'_, AppState>, app_key: String) -> Result<(), AppError> {
    // 学习基线：多开前该 exe 的可见窗口数（解析不出 exe / 冷启动 n0=0 时不学）
    let baseline = (|| -> Option<(String, usize)> {
        let conn = state.db.lock().ok()?;
        let target = exec_target_of(&conn, &app_key)?;
        let exe = hamster_platform::shell::cached_exe_file_name(Path::new(&target))?;
        let n = hamster_platform::tray::count_visible_by_process(std::process::id(), &exe);
        Some((exe, n))
    })();
    let r = launch(state, app_key.clone(), true);
    if r.is_ok() {
        if let Some((exe, n0)) = baseline.filter(|(_, n)| *n >= 1) {
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                let n1 = hamster_platform::tray::count_visible_by_process(std::process::id(), &exe);
                if let Ok(mut guard) = SINGLE_INSTANCE.lock() {
                    let set = guard.get_or_insert_with(Default::default);
                    if n1 > n0 {
                        set.remove(&app_key);
                    } else {
                        set.insert(app_key);
                    }
                }
            });
        }
    }
    r
}

/// 运行时学到的单实例应用（app_key）；内置名单见 SINGLE_INSTANCE_EXE。
/// Option 包一层：HashSet::new 非 const，首写时 get_or_insert_with 惰性建。
static SINGLE_INSTANCE: std::sync::Mutex<Option<std::collections::HashSet<String>>> =
    std::sync::Mutex::new(None);

/// 出名单即确定单实例的常见应用（exe 文件名，小写）。
/// 名单内的应用不提供「多开」——点了一定收敛回已有窗口，入口就是谎言。
const SINGLE_INSTANCE_EXE: &[&str] = &[
    "wechat.exe",
    "weixin.exe",
    "qq.exe",
    "ntqq.exe",
    "steam.exe",
    "telegram.exe",
    "spotify.exe",
    "dingtalk.exe",
    "feishu.exe",
    "lark.exe",
    "slack.exe",
];

/// 查询 keys 中已知不支持多开的应用（内置名单 + 运行时自学习并集），
/// 菜单据此隐藏「多开应用」项。
#[tauri::command]
#[specta::specta]
pub async fn app_multi_flags(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<Vec<String>, AppError> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    let mut targets: Vec<(String, String)> = Vec::new();
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        let placeholders = vec!["?"; keys.len()].join(",");
        let sql =
            format!("SELECT app_key, exec_target FROM app_meta WHERE app_key IN ({placeholders})");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::io(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(keys.iter()), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::io(e.to_string()))?;
        for row in rows.flatten() {
            targets.push(row);
        }
    }
    tauri::async_runtime::spawn_blocking(move || {
        let learned = SINGLE_INSTANCE
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_default();
        targets
            .into_iter()
            .filter(|(key, target)| {
                if learned.contains(key) {
                    return true;
                }
                hamster_platform::shell::cached_exe_file_name(Path::new(target))
                    .is_some_and(|exe| SINGLE_INSTANCE_EXE.contains(&exe.as_str()))
            })
            .map(|(key, _)| key)
            .collect::<Vec<String>>()
    })
    .await
    .map_err(|e| AppError::io(format!("多开能力查询失败: {e}")))
}

/// 关闭应用：对其 exe 名下全部可见窗口投递 WM_CLOSE（温和关闭，应用可弹
/// 保存确认）。解析不出 exe（UWP/文件夹）或没有可见窗口时报错。
#[tauri::command]
#[specta::specta]
pub fn app_close(state: State<'_, AppState>, app_key: String) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let Some(target) = exec_target_of(&conn, &app_key) else {
        return Err(AppError::validate(format!("应用不在索引中: {app_key}")));
    };
    let Some(exe) = hamster_platform::shell::cached_exe_file_name(Path::new(&target)) else {
        return Err(AppError::validate(format!("无法解析应用进程: {target}")));
    };
    drop(conn);
    let sent = hamster_platform::tray::close_all_by_process(std::process::id(), &exe);
    if sent == 0 {
        return Err(AppError::validate(format!(
            "应用当前没有可见窗口: {app_key}"
        )));
    }
    Ok(())
}

/// 一扇应用窗口（dock 悬停卡片行）
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AppWindowInfo {
    /// 窗口句柄数值（app_window_activate 目标）
    pub id: u64,
    pub title: String,
    pub minimized: bool,
    /// 窗口图标 PNG 的 data URL（空串时前端回退应用图标/首字占位）
    pub png: String,
}

/// 枚举应用的全部可见顶层窗口（z 序，最上层在前）——dock 悬停卡片用。
/// 解析不出 exe（UWP/文件夹）返回空表。
#[tauri::command]
#[specta::specta]
pub async fn app_windows(
    state: State<'_, AppState>,
    app_key: String,
) -> Result<Vec<AppWindowInfo>, AppError> {
    let target = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        exec_target_of(&conn, &app_key)
    };
    let Some(target) = target else {
        return Err(AppError::validate(format!("应用不在索引中: {app_key}")));
    };
    tauri::async_runtime::spawn_blocking(move || {
        let Some(exe) = hamster_platform::shell::cached_exe_file_name(Path::new(&target)) else {
            return Vec::new();
        };
        hamster_platform::tray::list_by_process(std::process::id(), &exe)
            .into_iter()
            .map(|w| AppWindowInfo {
                id: w.id,
                title: w.title,
                minimized: w.minimized,
                png: w.png,
            })
            .collect()
    })
    .await
    .map_err(|e| AppError::io(format!("窗口枚举失败: {e}")))
}

/// 前置（必要时还原）指定窗口——dock 悬停卡片点选目标窗口
#[tauri::command]
#[specta::specta]
pub fn app_window_activate(id: u64) -> Result<(), AppError> {
    hamster_platform::tray::activate(id).map_err(|e| AppError::io(format!("窗口前置失败: {e}")))
}

/// 关闭单扇窗口（dock 悬停卡片行 X）：对该 hwnd 投递 WM_CLOSE 温和关闭
#[tauri::command]
#[specta::specta]
pub fn app_window_close(id: u64) -> Result<(), AppError> {
    if hamster_platform::tray::close_window(id) {
        Ok(())
    } else {
        Err(AppError::io(format!("窗口关闭失败（可能已关）: hwnd={id}")))
    }
}

/// dock 运行态轮询：返回 keys 中当前有可见窗口的应用 app_key。
/// DB 查询留在调用线程（毫秒级）；lnk 解析 + EnumWindows 放线程池，不占主线程。
#[tauri::command]
#[specta::specta]
pub async fn apps_running(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<Vec<String>, AppError> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    let mut targets: Vec<(String, String)> = Vec::new();
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        let placeholders = vec!["?"; keys.len()].join(",");
        let sql =
            format!("SELECT app_key, exec_target FROM app_meta WHERE app_key IN ({placeholders})");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::io(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(keys.iter()), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::io(e.to_string()))?;
        for row in rows.flatten() {
            targets.push(row);
        }
    }
    tauri::async_runtime::spawn_blocking(move || {
        let procs = hamster_platform::tray::visible_process_set(std::process::id());
        targets
            .into_iter()
            .filter(|(_, target)| {
                hamster_platform::shell::cached_exe_file_name(Path::new(target))
                    .is_some_and(|exe| procs.contains(&exe))
            })
            .map(|(key, _)| key)
            .collect::<Vec<String>>()
    })
    .await
    .map_err(|e| AppError::io(format!("运行态枚举失败: {e}")))
}
