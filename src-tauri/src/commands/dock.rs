//! dock 弹窗（独立置顶小窗）：任务栏独立窗口只有一条高（68 逻辑 px），
//! 纵向内容放不下；右键菜单和悬停窗口卡片都弹成独立置顶小窗
//! （Windows 任务栏自己的右键菜单/悬停预览也是独立 popup 窗口），
//! 主窗口被其它应用盖住时照样可见可点。
//!
//! 流程：任务栏窗口右键/悬停 → dock_menu_open（Rust 按 taskbar 条几何定位
//! 弹窗）→ 弹窗路由拉取/订阅载荷渲染（kind 区分 menu / hover）→
//! 菜单动作经 `hamster:dock-menu-action` 广播给渲染 DockBar 的窗口执行；
//! 悬停卡片点选直接 app_window_activate（纯 Rust，无需中转）。
//! 收回：失焦 / Esc / 任务栏任意点击 / 悬停离开（hover）。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::apps::AppWindowInfo;
use crate::error::AppError;

/// 弹窗载荷（dock-menu 窗口渲染 + 定位所需；文案在弹窗内用 i18n 现算）
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DockMenuPayload {
    /// "menu" = 右键菜单；"hover" = 悬停窗口卡片
    pub kind: String,
    pub app_key: String,
    /// 锚点 x（任务栏窗口内逻辑 px，用于水平居中弹窗）
    pub x: f64,
    /// 定制组项才显示「移除」（menu）
    pub removable: bool,
    /// 运行中才显示「关闭窗口」（menu）
    pub running: bool,
    /// 支持多开才显示「多开应用」（menu；单实例应用点了只会收敛回已有窗口）
    pub multi: bool,
    /// 悬停卡片窗口清单（hover；menu 为空）
    pub windows: Vec<AppWindowInfo>,
}

/// 当前弹窗载荷（dock-menu 窗口挂载时拉取；打开/换目标时更新）
static PAYLOAD: Mutex<Option<DockMenuPayload>> = Mutex::new(None);

/// 菜单宽度（逻辑 px，与 DockAppMenu 的 w-44 对齐）
const MENU_W: f64 = 176.0;
/// 悬停卡片宽度（逻辑 px，窗口标题需要更多横向空间）
const HOVER_W: f64 = 280.0;
/// 菜单单项高度（逻辑 px，与 MenuRow py-[7px] 对齐）
const MENU_ITEM_H: f64 = 34.0;
/// 悬停卡片单项高度（逻辑 px，与 DockWindowsCard 行高对齐）
const HOVER_ITEM_H: f64 = 40.0;
/// 容器上下留白（逻辑 px）
const PAD_H: f64 = 8.0;
/// 悬停卡片最大高度（逻辑 px；窗口更多时内部滚动）
const HOVER_MAX_H: f64 = 440.0;

/// 打开（或换目标重开）dock 弹窗：水平钳进任务栏范围，垂直落在任务栏条
/// 上沿上方 8px。menu 抢焦点（失焦即收）；hover 不抢焦点（跟随悬停）。
#[tauri::command]
#[specta::specta]
pub fn dock_menu_open(app: AppHandle, payload: DockMenuPayload) -> Result<(), AppError> {
    let tb = app
        .get_webview_window("taskbar")
        .ok_or_else(|| AppError::new("DOCK_MENU", "taskbar 窗口不存在（未处于桌面模式）"))?;
    let win = app
        .get_webview_window("dock-menu")
        .ok_or_else(|| AppError::new("DOCK_MENU", "dock-menu 窗口不存在"))?;

    let scale = tb.scale_factor().unwrap_or(1.0);
    let pos = tb
        .outer_position()
        .map_err(|e| AppError::new("DOCK_MENU", e.to_string()))?;
    let size = tb
        .outer_size()
        .map_err(|e| AppError::new("DOCK_MENU", e.to_string()))?;

    // 尺寸按 kind：菜单 = 条目数 × 行高；悬停卡片 = 窗口数 × 行高（封顶滚动）
    let (w_logical, h_logical) = if payload.kind == "hover" {
        let rows = payload.windows.len().max(1) as f64;
        (
            HOVER_W,
            ((rows * HOVER_ITEM_H + PAD_H) * scale).min(HOVER_MAX_H * scale) / scale,
        )
    } else {
        let items = payload.multi as i32 + payload.removable as i32 + payload.running as i32;
        (MENU_W, items as f64 * MENU_ITEM_H + PAD_H)
    };
    let w = (w_logical * scale).round() as i32;
    let h = (h_logical * scale).round() as i32;
    let anchor = pos.x + (payload.x * scale).round() as i32;
    let x = (anchor - w / 2).clamp(pos.x + 8, pos.x + size.width as i32 - w - 8);
    let y = pos.y - h - (8.0 * scale).round() as i32;

    let is_menu = payload.kind != "hover";
    *PAYLOAD
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))? = Some(payload.clone());
    let _ = win.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    // hover 卡片不抢焦点：任务栏本就不抢，卡片抢了会把刚聚焦的应用顶下去
    let _ = win.set_focusable(is_menu);
    let _ = win.show();
    if is_menu {
        let _ = win.set_focus();
    }
    // 弹窗 webview 启动即挂载常驻，监听器已就绪；重开换目标靠事件推送
    let _ = app.emit_to("dock-menu", "hamster:dock-menu-payload", payload);
    Ok(())
}

/// 弹窗挂载时拉取当前载荷（之后靠 payload 事件增量更新）
#[tauri::command]
#[specta::specta]
pub fn dock_menu_payload() -> Result<Option<DockMenuPayload>, AppError> {
    PAYLOAD
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))
        .map(|g| g.clone())
}

/// 收回弹窗（失焦/Esc/动作完成/退出桌面模式/悬停离开）
#[tauri::command]
#[specta::specta]
pub fn dock_menu_hide(app: AppHandle) -> Result<(), AppError> {
    if let Some(w) = app.get_webview_window("dock-menu") {
        let _ = w.hide();
    }
    if let Ok(mut g) = PAYLOAD.lock() {
        *g = None;
    }
    Ok(())
}
