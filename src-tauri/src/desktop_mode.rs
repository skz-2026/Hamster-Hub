//! 桌面模式：完全接管整个桌面——隐藏 Windows 任务栏与桌面图标，
//! 主窗口铺满「屏幕减去底部任务栏条」，底部由**独立置顶 taskbar 窗口**完全替代系统任务栏：
//! 任务栏窗口 always-on-top，打开任何应用都不会遮挡它（持续渲染，和系统任务栏同语义）。
//! 启动/进入前对账残留快照（崩溃、断电、看门狗失守都可能留下隐藏态残留）；进入前落盘
//! 系统状态快照；正常退出按快照还原；主进程崩溃由 hamster-watchdog 凭快照兜底还原；
//! 进入期间每 2s 巡检，explorer 重启导致的任务栏复活会被重新隐藏。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::Manager;
use tauri_specta::Event;

use crate::events;

const PATROL_INTERVAL: Duration = Duration::from_secs(2);
/// 进入接管后的高危期时长：shell 对全屏标记窗口的工作区舞步（激活全屏窗时
/// 扩为整屏、失焦/重算时缩回预留）与 explorer 的 AppBar 预留写回集中在此
/// 窗口内不定期发生，任何一次「整屏→预留」的收缩都会把底边贴屏幕底的 dock
/// 窗口钳上抬一条系统任务栏高（~48px；第二轮进入尤其明显——主窗口从窗口化
/// 恢复再进全屏，触发更晚的写回）。期间以 150ms 高频重钉 dock，纠偏耗时
/// 低于感知阈值；之后转常规巡检兜底。
const GUARD_WINDOW: Duration = Duration::from_secs(10);
const GUARD_INTERVAL: Duration = Duration::from_millis(150);
/// 任务栏条高度（逻辑像素）：DockBar 桌面形态图标 ~51px + 底部留白，
/// 压紧后减少条上方的空区观感（hover 放大仍留有余量）
const TASKBAR_H_LOGICAL: f64 = 68.0;
/// 工作区改写后 explorer 会把任务栏 AppBar 预留异步写回（一次，实测 ~50-100ms）。
/// 落最终值前静置等它写完；之后不再有工作区变更，dock 窗口就不会在工作区
/// 变更时被钳回工作区底边（隐藏窗口底边超出工作区即被钳——底边 1080 超出
/// 预留底边 1032 时 dock 整体上抬一条任务栏高 ~48px，即「第二轮进入 dock
/// 上抬」问题）
const WORKAREA_SETTLE: Duration = Duration::from_millis(250);

static ACTIVE: AtomicBool = AtomicBool::new(false);

/// 进入接管前的工作区（退出时还原；主显示器）
static WORKAREA_SAVED: std::sync::Mutex<Option<(i32, i32, i32, i32)>> = std::sync::Mutex::new(None);

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

/// 工作区 = 整屏 − dock 条高（隐藏系统任务栏后其旧预留仍在且数值不对——
/// 必须显式设为我们 dock 的精确矩形：最大化/新开的 app 底边与 dock 顶
/// 严丝合缝，既不被 dock 遮挡、也无主窗口露边的缝隙）
fn expand_workarea(app: &tauri::AppHandle) {
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    let Ok(Some(mon)) = main.current_monitor() else {
        return;
    };
    let scale = main.scale_factor().unwrap_or(1.0);
    let tb_h = (TASKBAR_H_LOGICAL * scale).round() as i32;
    let mp = mon.position();
    let ms = mon.size();
    if let Err(e) = hamster_platform::workarea::set(
        mp.x,
        mp.y,
        mp.x + ms.width as i32,
        mp.y + ms.height as i32 - tb_h,
    ) {
        eprintln!("[desktop_mode] 工作区设为 dock 上沿失败：{e}");
    }
}

/// 启动对账：发现残留快照即无条件恢复并删除。上次会话若崩溃/断电/看门狗
/// 失守，系统可能仍处于隐藏态；若不先恢复，enter() 会把该隐藏态拍成
/// 「原始状态」存进新快照，退出时忠实还原到隐藏（副屏任务栏与桌面图标
/// 永久消失，仅主任务栏有按类名兜底可救）。正常路径无快照文件，是纯检查。
pub fn reconcile_stale_snapshot() {
    let Some(path) = hamster_platform::snapshot::snapshot_path() else {
        return;
    };
    if !path.exists() {
        return;
    }
    eprintln!("[desktop_mode] 发现残留快照（上次会话未正常还原？），先恢复系统状态");
    match hamster_platform::snapshot::read(&path) {
        Some(snap) => hamster_platform::snapshot::restore(&snap),
        None => {
            // 快照损坏时无从得知原始状态：无差别恢复（可见桌面优于黑屏），
            // 与 exit() 的无快照兜底同语义
            eprintln!("[desktop_mode] 残留快照损坏，无差别恢复任务栏与桌面图标");
            hamster_platform::taskbar::restore_all_by_class();
            let _ = hamster_platform::desktop_icons::set_hide_icons(0);
        }
    }
    hamster_platform::snapshot::remove(&path);
}

pub fn enter(app: &tauri::AppHandle) -> Result<(), crate::error::AppError> {
    eprintln!("[desktop_mode] enter() called, active={}", is_active());
    if is_active() {
        return Ok(());
    }
    ACTIVE.store(true, Ordering::SeqCst);

    let pid = std::process::id();

    // ⓪ 启动对账：清掉上次会话可能的残留，确保下面拍到的是系统真实
    // 原始状态而非上一次接管的隐藏态（污染场景见 reconcile 注释）
    reconcile_stale_snapshot();

    // ① 快照当前系统状态（任务栏可见性 + 桌面图标注册表值），崩溃兜底凭据
    if let Some(path) = hamster_platform::snapshot::snapshot_path() {
        let snap = hamster_platform::snapshot::capture(pid);
        if let Err(e) = hamster_platform::snapshot::write(&path, &snap) {
            eprintln!("[desktop_mode] 快照写入失败（看门狗兜底不可用）: {e}");
        }
    }

    // ② 隐藏任务栏 + 桌面图标（真正接管）；记住原工作区并设为「整屏 − dock」
    //（旧预留数值不对：不是被 dock 遮挡就是留缝隙，见 expand_workarea）
    if let Ok(mut slot) = WORKAREA_SAVED.lock() {
        *slot = hamster_platform::workarea::get();
    }
    hamster_platform::taskbar::hide_all();
    if let Err(e) = hamster_platform::desktop_icons::set_hide_icons(1) {
        eprintln!("[desktop_mode] 隐藏桌面图标失败: {e}");
    }

    // ③ 拉起看门狗（主进程死亡 → 按快照还原）；失败仅告警，正常退出路径不受影响
    spawn_watchdog(pid);

    // ③½ Win+D 守卫：接管期间吞掉系统「显示桌面」（否则全屏主窗被最小化，
    // 露出无图标无任务栏的黑屏），改为仓鼠版「显示桌面」——清开别的窗口、
    // 仓鼠桌面主页回到眼前，接管保持不动（仓鼠桌面就是桌面）
    {
        let guard_app = app.clone();
        hamster_platform::wind_guard::install(Box::new(move || {
            hamster_platform::wind_guard::minimize_all_except(std::process::id());
            if let Some(w) = guard_app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
            // active:true 驱动前端路由回 /desktop（已在桌面主页则是空操作）
            let _ = events::DesktopModeChanged { active: true }.emit(&guard_app);
        }));
    }

    // ④ 布局：主窗口先铺满（立即可见）；工作区静置两段落值后再摆 dock——
    // dock 的定位/显示必须在最后一次工作区变更之后，否则会被钳上抬
    //（见 WORKAREA_SETTLE 注释）
    fullscreen_main(app);
    settle_workarea(app);
    place_taskbar(app);

    // ⑤ 巡检线程（两阶段）：前 GUARD_WINDOW 内每 GUARD_INTERVAL 重钉一次
    // dock（钳制发生到纠正 ≤150ms，肉眼基本无感）；之后转 PATROL_INTERVAL
    // 常规巡检。explorer 重启导致任务栏复活时顺带重隐 + 重落工作区。
    let patrol_app = app.clone();
    std::thread::spawn(move || {
        let fast_until = std::time::Instant::now() + GUARD_WINDOW;
        loop {
            let interval = if std::time::Instant::now() < fast_until {
                GUARD_INTERVAL
            } else {
                PATROL_INTERVAL
            };
            std::thread::sleep(interval);
            if !ACTIVE.load(Ordering::SeqCst) {
                break;
            }
            // 托盘 open_overflow 正借壳显示任务栏时不可动它，否则 chevron
            // Invoke 被打断、弹层打不开（见 hamster_platform::tray::SHELL_FLASH）
            if hamster_platform::taskbar::any_visible()
                && !hamster_platform::tray::is_shell_flash_in_progress()
            {
                eprintln!("[desktop_mode] 检测到任务栏复活（explorer 重启？），重新隐藏");
                hamster_platform::taskbar::hide_all();
                // 任务栏复活→重隐的过程 shell 会重算工作区（回到留边状态），
                // 同样存在 AppBar 预留写回竞争：两段落值后再重钉 dock
                settle_workarea(&patrol_app);
            }
            // 重钉 dock 位置：任何来源的工作区收缩都可能把底边超界的 dock
            // 窗口钳回工作区底边（place_taskbar 内部位置正确时会跳过，无抖动）
            place_taskbar(&patrol_app);
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

    // ⓪ 先撤 Win+D 守卫：还原期间按键立即交还系统原生行为
    hamster_platform::wind_guard::stop();

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

    // 还原工作区：按还原后「真实可见的任务栏矩形」重算（上面已先把任务栏恢复显示）。
    // 不能直接回放进入前的保存值——SW_HIDE/SW_SHOW 都不会让 shell 重算工作区，
    // 若上一轮退出时残留过接管值，下一轮 enter 会把污染值存进快照、代代相传。
    // 任务栏查找失败（极端：explorer 刚死）才退回保存值。
    if let Ok(mut slot) = WORKAREA_SAVED.lock() {
        let saved = slot.take();
        let target = hamster_platform::workarea::recompute_from_primary_taskbar().or(saved);
        if let Some((l, t, r, b)) = target {
            let _ = hamster_platform::workarea::set(l, t, r, b);
        }
    }

    // 隐藏 taskbar 置顶窗（替代系统任务栏的渲染结束）；
    // dock 右键菜单弹窗也是置顶窗，不跟着收会一直浮在桌面上
    if let Some(tb) = app.get_webview_window("taskbar") {
        let _ = tb.hide();
    }
    if let Some(dm) = app.get_webview_window("dock-menu") {
        let _ = dm.hide();
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
        Some(p) => {
            let mut cmd = std::process::Command::new(p);
            cmd.arg(pid.to_string());
            // 看门狗是控制台子系统程序：发布版主进程是 GUI 子系统（无控制台），
            // 直接拉起会给它新分配一个控制台窗口（用户看到黑框）。CREATE_NO_WINDOW
            // 压住——看门狗日志本就写 watchdog.log，不依赖控制台。
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            }
            match cmd.spawn() {
                // Child 句柄直接丢弃（不 kill 不 wait）：看门狗生命周期由快照文件驱动
                Ok(_child) => eprintln!("[desktop_mode] watchdog started (main pid={pid})"),
                Err(e) => eprintln!("[desktop_mode] 看门狗拉起失败: {e}"),
            }
        }
        None => {
            eprintln!("[desktop_mode] 未找到 hamster-watchdog.exe，跳过看门狗（崩溃兜底不可用）")
        }
    }
}

/// 工作区两段落值：第一次设置后 explorer 会把任务栏 AppBar 预留写回
/// （SW_HIDE 不注销 AppBar），静置等它写完再落最终值。此后接管期内不再有
/// 工作区变更，dock 窗口（隐藏中、底边超出工作区）就不会在变更瞬间被系统
/// 钳回工作区底边——那会让 dock 整体上抬一条系统任务栏的高度。
fn settle_workarea(app: &tauri::AppHandle) {
    expand_workarea(app);
    std::thread::sleep(WORKAREA_SETTLE);
    expand_workarea(app);
}

/// 接管布局第一步：主窗口无边框全屏铺满所在显示器（立即生效可见）。
///
/// 注意不能用 set_position/set_size 手摆「屏幕减 dock」矩形：无边框**可缩放**
/// 窗口自带 ~8px 隐形缩放边框（rect 含边框、client 内缩），手摆必留左/底缝隙——
/// set_fullscreen(true) 由系统铺满整屏，内容直达边缘。
fn fullscreen_main(app: &tauri::AppHandle) {
    let main = app.get_webview_window("main");
    let mon = main
        .as_ref()
        .and_then(|w| w.current_monitor().ok().flatten());
    if let Some(w) = main {
        // 全屏目标屏 = 当前所在显示器（先把窗口挪到该屏原点再进全屏）。
        // set_position 必须在 set_fullscreen 之前：tao 的 set_position 会
        // 解除最大化（对 MAXIMIZED 标志做 SW_RESTORE），放在全屏之后会把
        // 全屏打回最大化前的旧尺寸（最大化进接管时必现的缩窗 bug）。
        if let Some(m) = &mon {
            let mp = m.position();
            let _ = w.set_position(tauri::PhysicalPosition::new(mp.x, mp.y));
        }
        let _ = w.set_fullscreen(true);
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 接管布局第二步：taskbar 置顶窗贴屏幕底部条（持续渲染不被遮挡）。
/// 必须在 settle_workarea 之后调用（定位落在最后一次工作区变更之后，
/// 见 settle_workarea 注释）；巡检每轮也重钉一次作兜底。
fn place_taskbar(app: &tauri::AppHandle) {
    let main = app.get_webview_window("main");
    let mon = main
        .as_ref()
        .and_then(|w| w.current_monitor().ok().flatten());
    let scale = main
        .as_ref()
        .and_then(|w| w.scale_factor().ok())
        .unwrap_or(1.0);
    let tb_h = (TASKBAR_H_LOGICAL * scale).round() as i32;

    if let Some(tb) = app.get_webview_window("taskbar") {
        if let Some(m) = &mon {
            let mp = m.position();
            let ms = m.size();
            // 高频巡检复用本函数：位置已正确时不再 set_position，避免对
            // 每个tick都发异步 SetWindowPos 造成渲染层反复重排
            let target = tauri::PhysicalPosition::new(mp.x, mp.y + (ms.height as i32 - tb_h));
            if tb.outer_position().map(|p| p != target).unwrap_or(true) {
                let _ = tb.set_position(target);
                let _ = tb.set_size(tauri::PhysicalSize::new(ms.width, tb_h as u32));
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_without_snapshot_is_noop() {
        // 无残留快照必须是纯检查（不动任务栏、不写注册表）；测试机若恰有
        // 残留快照，本函数的恢复语义正是期望行为
        reconcile_stale_snapshot();
        assert!(!hamster_platform::snapshot::snapshot_path()
            .unwrap()
            .exists());
    }
}
