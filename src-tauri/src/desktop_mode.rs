//! 桌面模式：完全接管整个桌面（对齐水豚hub）——隐藏 Windows 任务栏与桌面图标，
//! 主窗口铺满「屏幕减去底部任务栏条」，底部由**独立置顶 taskbar 窗口**完全替代系统任务栏：
//! 任务栏窗口 always-on-top，打开任何应用都不会遮挡它（持续渲染，和系统任务栏同语义）。
//! 进入前落盘系统状态快照；正常退出按快照还原；主进程崩溃由 hamster-watchdog
//! 凭快照兜底还原；进入期间每 5s 巡检，explorer 重启导致的任务栏复活会被重新隐藏。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::Manager;
use tauri_specta::Event;

use crate::events;

const PATROL_INTERVAL: Duration = Duration::from_secs(5);
/// 任务栏条高度（逻辑像素）：DockBar 桌面形态 ~63px + 悬停放大上浮余量
const TASKBAR_H_LOGICAL: f64 = 72.0;

static ACTIVE: AtomicBool = AtomicBool::new(false);

/// 进入接管前的工作区（退出时还原；主显示器）
static WORKAREA_SAVED: std::sync::Mutex<Option<(i32, i32, i32, i32)>> = std::sync::Mutex::new(None);

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

/// 工作区扩为主窗口所在显示器的整屏（隐藏任务栏后其 AppBar 预留仍在，
/// 最大化应用只会铺到原工作区底——显式扩为整屏才有真全屏效果）
fn expand_workarea(app: &tauri::AppHandle) {
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    let Ok(Some(mon)) = main.current_monitor() else {
        return;
    };
    let mp = mon.position();
    let ms = mon.size();
    if let Err(e) =
        hamster_platform::workarea::set(mp.x, mp.y, mp.x + ms.width as i32, mp.y + ms.height as i32)
    {
        eprintln!("[desktop_mode] 工作区扩为整屏失败（最大化应用将留边）：{e}");
    }
}

pub fn enter(app: &tauri::AppHandle) -> Result<(), crate::error::AppError> {
    eprintln!("[desktop_mode] enter() called, active={}", is_active());
    if is_active() {
        return Ok(());
    }
    ACTIVE.store(true, Ordering::SeqCst);

    let pid = std::process::id();

    // ① 快照当前系统状态（任务栏可见性 + 桌面图标注册表值），崩溃兜底凭据
    if let Some(path) = hamster_platform::snapshot::snapshot_path() {
        let snap = hamster_platform::snapshot::capture(pid);
        if let Err(e) = hamster_platform::snapshot::write(&path, &snap) {
            eprintln!("[desktop_mode] 快照写入失败（看门狗兜底不可用）: {e}");
        }
    }

    // ② 隐藏任务栏 + 桌面图标（真正接管）；记住原工作区并扩为整屏
    //（隐藏后的 AppBar 预留会让最大化应用留一条边，水豚式全屏需要显式扩区）
    if let Ok(mut slot) = WORKAREA_SAVED.lock() {
        *slot = hamster_platform::workarea::get();
    }
    hamster_platform::taskbar::hide_all();
    expand_workarea(app);
    if let Err(e) = hamster_platform::desktop_icons::set_hide_icons(1) {
        eprintln!("[desktop_mode] 隐藏桌面图标失败: {e}");
    }

    // ③ 拉起看门狗（主进程死亡 → 按快照还原）；失败仅告警，正常退出路径不受影响
    spawn_watchdog(pid);

    // ④ 布局：主窗口铺满「整屏减去任务栏条」，taskbar 置顶窗贴底条（持续渲染不被遮挡）
    apply_takeover_layout(app);

    // ⑤ explorer 重启巡检：任务栏复活（Shell_TrayWnd 重新创建）则重新隐藏
    let patrol_app = app.clone();
    std::thread::spawn(move || {
        while ACTIVE.load(Ordering::SeqCst) {
            std::thread::sleep(PATROL_INTERVAL);
            if !ACTIVE.load(Ordering::SeqCst) {
                break;
            }
            if hamster_platform::taskbar::any_visible() {
                eprintln!("[desktop_mode] 检测到任务栏复活（explorer 重启？），重新隐藏");
                hamster_platform::taskbar::hide_all();
                // 任务栏复活→重隐的过程 shell 会重算工作区（回到留边状态），需再扩一次
                expand_workarea(&patrol_app);
            }
        }
    });

    let _ = events::DesktopModeChanged { active: true }.emit(app);
    eprintln!("[desktop_mode] enter() completed (deep takeover)");
    Ok(())
}

pub fn exit(app: &tauri::AppHandle) -> Result<(), crate::error::AppError> {
    eprintln!("[desktop_mode] exit() called, active={}", is_active());
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return Ok(());
    }

    // ① 按快照还原系统状态（任务栏可见性 + 桌面图标），并删除快照。
    // 快照删除后看门狗下一次轮询（≤2s）即自行退出。
    if let Some(path) = hamster_platform::snapshot::snapshot_path() {
        if let Some(snap) = hamster_platform::snapshot::read(&path) {
            hamster_platform::snapshot::restore(&snap);
        } else {
            // 无快照（写入失败过）→ 无差别恢复
            hamster_platform::taskbar::restore_all_by_class();
            let _ = hamster_platform::desktop_icons::set_hide_icons(0);
        }
        hamster_platform::snapshot::remove(&path);
    }

    // 还原工作区（进入前保存的值；任务栏重显后 shell 亦会自行重算，此处兜底）
    if let Ok(mut slot) = WORKAREA_SAVED.lock() {
        if let Some((l, t, r, b)) = slot.take() {
            let _ = hamster_platform::workarea::set(l, t, r, b);
        }
    }

    // 隐藏 taskbar 置顶窗（替代系统任务栏的渲染结束）
    if let Some(tb) = app.get_webview_window("taskbar") {
        let _ = tb.hide();
    }

    apply_windowed_mode(app);
    let _ = events::DesktopModeChanged { active: false }.emit(app);
    eprintln!("[desktop_mode] exit() completed");
    Ok(())
}

/// 看门狗可执行文件：主程序同目录（dev = target/debug，NSIS = 安装目录），
/// 兼容 resources 子目录布局
fn watchdog_exe_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    for rel in ["hamster-watchdog.exe", "resources\\hamster-watchdog.exe"] {
        let p = dir.join(rel);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn spawn_watchdog(pid: u32) {
    match watchdog_exe_path() {
        Some(p) => match std::process::Command::new(p).arg(pid.to_string()).spawn() {
            // Child 句柄直接丢弃（不 kill 不 wait）：看门狗生命周期由快照文件驱动
            Ok(_child) => eprintln!("[desktop_mode] watchdog started (main pid={pid})"),
            Err(e) => eprintln!("[desktop_mode] 看门狗拉起失败: {e}"),
        },
        None => {
            eprintln!("[desktop_mode] 未找到 hamster-watchdog.exe，跳过看门狗（崩溃兜底不可用）")
        }
    }
}

/// 接管布局：主窗口 = 无边框全屏（dock 独立置顶覆盖底部条）；
/// taskbar 置顶窗 = 底部条，任何应用窗口都无法遮挡它（系统任务栏同语义）。
///
/// 注意不能用 set_position/set_size 手摆「屏幕减 dock」矩形：无边框**可缩放**
/// 窗口自带 ~8px 隐形缩放边框（rect 含边框、client 内缩），手摆必留左/底缝隙——
/// set_fullscreen(true) 由系统铺满整屏，内容直达边缘（水豚hub 同效果）。
fn apply_takeover_layout(app: &tauri::AppHandle) {
    let main = app.get_webview_window("main");
    let mon = main
        .as_ref()
        .and_then(|w| w.current_monitor().ok().flatten());
    let scale = main
        .as_ref()
        .and_then(|w| w.scale_factor().ok())
        .unwrap_or(1.0);
    let tb_h = (TASKBAR_H_LOGICAL * scale).round() as i32;

    if let Some(w) = main {
        let _ = w.set_fullscreen(true);
        // 全屏目标屏 = 当前所在显示器（先把窗口挪到该屏原点再进全屏）
        if let Some(m) = &mon {
            let mp = m.position();
            let _ = w.set_position(tauri::PhysicalPosition::new(mp.x, mp.y));
        }
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }

    if let Some(tb) = app.get_webview_window("taskbar") {
        if let Some(m) = &mon {
            let mp = m.position();
            let ms = m.size();
            let _ = tb.set_position(tauri::PhysicalPosition::new(
                mp.x,
                mp.y + (ms.height as i32 - tb_h),
            ));
            let _ = tb.set_size(tauri::PhysicalSize::new(ms.width, tb_h as u32));
        }
        // 任务栏语义：可点击但不抢键盘焦点（不把主窗口/新开应用挤下去）
        let _ = tb.set_focusable(false);
        let _ = tb.set_always_on_top(true);
        let _ = tb.show();
    }
}

/// 还原为普通窗口
fn apply_windowed_mode(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_fullscreen(false);
        let _ = w.unmaximize();
        let _ = w.set_size(tauri::LogicalSize::new(1280.0, 800.0));
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
    }
}
