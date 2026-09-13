//! dock 右键菜单弹窗：任务栏独立窗口只有一条高（68 逻辑 px），纵向菜单
//! 放不下；右键菜单弹成独立置顶小窗（Windows 任务栏自己的右键菜单也是
//! 独立 popup 窗口），主窗口被其它应用盖住时照样可见可点。
//!
//! 流程：任务栏窗口右键 → dock_menu_open（Rust 按 taskbar 条几何定位弹窗）
//! → 弹窗路由拉取/订阅载荷渲染 → 点选动作经 `hamster:dock-menu-action`
//! 广播给主窗口执行（remove 需要 layout commit，close/new 顺带失效主窗口
//! 查询让指示点即时变化）→ 失焦/ Esc / 任务栏任意点击收回。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;

/// 菜单载荷（dock-menu 弹窗渲染 + 定位所需；文案在弹窗内用 i18n 现算）
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DockMenuPayload {
    pub app_key: String,
    /// 右键锚点 x（任务栏窗口内逻辑 px，用于水平居中弹窗）
    pub x: f64,
    /// 定制组项才显示「移除」
    pub removable: bool,
    /// 运行中才显示「关闭窗口」
    pub running: bool,
}

/// 当前菜单载荷（dock-menu 弹窗挂载时拉取；打开/换目标时更新）
static PAYLOAD: Mutex<Option<DockMenuPayload>> = Mutex::new(None);

/// 菜单宽度（逻辑 px，与 DockAppMenu 的 w-44 对齐）
const MENU_W: f64 = 176.0;
/// 单项高度 + 容器上下留白（逻辑 px，与 MenuRow py-[7px] 对齐）
const ITEM_H: f64 = 34.0;
const PAD_H: f64 = 8.0;

/// 打开（或换目标重开）dock 右键菜单弹窗：水平钳进任务栏范围，
/// 垂直落在任务栏条上沿上方 8px。
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

    // 多开 +（运行中）关闭 +（定制项）移除
    let items = 1 + payload.removable as i32 + payload.running as i32;
    let w = (MENU_W * scale).round() as i32;
    let h = ((items as f64 * ITEM_H + PAD_H) * scale).round() as i32;
    let anchor = pos.x + (payload.x * scale).round() as i32;
    let x = (anchor - w / 2).clamp(pos.x + 8, pos.x + size.width as i32 - w - 8);
    let y = pos.y - h - (8.0 * scale).round() as i32;

    *PAYLOAD
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))? = Some(payload.clone());
    let _ = win.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = win.show();
    let _ = win.set_focus();
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

/// 收回弹窗（失焦/Esc/动作完成/退出桌面模式）
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
