//! 「托盘」命令：打开 Windows 原生托盘溢出窗口（dock 右端托盘按钮）。

use tauri::{AppHandle, Manager};

use crate::error::AppError;

/// 打开原生托盘溢出弹层（系统自带 UI，含全部后台托盘 app，可直接交互）。
///
/// 借壳显示系统任务栏的 ~1s 里先把 dock 切成不透明（见 [`OpaqueDockGuard`]），
/// 配合 open_overflow 内的置顶压制，全程看不到任务栏闪现，dock 不再「上浮」。
#[tauri::command]
#[specta::specta]
pub fn tray_open_overflow(app: AppHandle) -> Result<(), AppError> {
    let dock = app
        .get_webview_window("taskbar")
        .ok_or_else(|| AppError::new("TRAY_NO_DOCK", "taskbar 窗口不存在（未处于桌面模式）"))?;
    let dock_hwnd = dock
        .hwnd()
        .map_err(|e| AppError::new("TRAY_NO_DOCK", e.to_string()))?
        .0 as isize;

    // 闪现期 dock 临时不透明；守卫 Drop 时还原（含中途出错路径）
    let _opaque = OpaqueDockGuard::new(dock)?;
    // 给合成器一点时间把不透明帧呈现出来，再让系统任务栏现身
    std::thread::sleep(std::time::Duration::from_millis(80));

    hamster_platform::tray::open_overflow(dock_hwnd)
        .map_err(|e| AppError::new("TRAY_OPEN", e.to_string()))
}

/// 托盘闪现期「dock 临时不透明」守卫。
///
/// dock 的半透明渐变背景（~88% alpha）盖不住借壳显示的系统任务栏——透出来
/// 即「dock 上浮闪动」观感。构造时把 taskbar 窗口的 webview 背景切成全
/// 不透明深色：WebView2 原生属性（put_DefaultBackgroundColor），同步命令在
/// 主线程调用即时生效，无需前端重绘。Drop 时还原透明，任何提前返回路径都
/// 不会把 dock 留在不透明态。
struct OpaqueDockGuard {
    dock: tauri::WebviewWindow<tauri::Wry>,
}

impl OpaqueDockGuard {
    fn new(dock: tauri::WebviewWindow<tauri::Wry>) -> Result<Self, AppError> {
        // 与 DockBar 条底色同族的深色（TaskbarPage 渐变中段值）
        dock.set_background_color(Some(tauri::window::Color(17, 15, 21, 255)))
            .map_err(|e| AppError::new("TRAY_OPAQUE", e.to_string()))?;
        Ok(Self { dock })
    }
}

impl Drop for OpaqueDockGuard {
    fn drop(&mut self) {
        // 必须还原成 (0,0,0,0)（wry 建窗时 transparent 的原始值）；
        // 传 None 会被 runtime 映射成不透明白色，dock 会变白底
        let _ = self
            .dock
            .set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
    }
}
