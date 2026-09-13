//! 工作区控制（SPI_GET/SETWORKAREA）：桌面接管时把工作区扩为整屏。
//!
//! 隐藏系统任务栏后，它的 AppBar 预留仍会留在工作区上（最大化应用只铺到
//! 原工作区底，与全屏差一条）——需显式 SPI_SETWORKAREA 为整屏，最大化/新开
//! 的应用才会真正铺满（内容延伸到 dock 后面，dock 置顶覆盖）。
//! 退出接管时按可见任务栏的实际矩形重算还原（SW_HIDE/SW_SHOW 都不会让
//! shell 重算工作区，回放保存值可能传播上一轮的污染值）。

use windows::core::Result;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetSystemMetrics, GetWindowRect, SystemParametersInfoW, SM_CXSCREEN, SM_CYSCREEN,
    SPI_GETWORKAREA, SPI_SETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

/// 当前工作区 (left, top, right, bottom)；读取失败返回 None
pub fn get() -> Option<(i32, i32, i32, i32)> {
    let mut rect = RECT::default();
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut RECT as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
    };
    ok.then_some((rect.left, rect.top, rect.right, rect.bottom))
}

pub fn set(left: i32, top: i32, right: i32, bottom: i32) -> Result<()> {
    let rect = RECT {
        left,
        top,
        right,
        bottom,
    };
    unsafe {
        SystemParametersInfoW(
            SPI_SETWORKAREA,
            0,
            Some(&rect as *const RECT as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
}

/// 按当前**可见**主任务栏（Shell_TrayWnd）的实际矩形重算主屏工作区。
///
/// SW_HIDE/SW_SHOW 任务栏不会触发 shell 重算工作区，因此退出接管不能依赖
/// shell 自行恢复，也不能盲信进入前保存的值（可能被上一轮接管值污染）——
/// 以任务栏此刻真实占用的边条为准重算。主显示器原点恒为 (0,0)，
/// SM_CXSCREEN/SM_CYSCREEN 即其物理尺寸。任务栏不可见/找不到返回 None。
pub fn recompute_from_primary_taskbar() -> Option<(i32, i32, i32, i32)> {
    let cls = windows::core::HSTRING::from("Shell_TrayWnd");
    let tray = unsafe { FindWindowW(&cls, None) }.ok()?;
    let r = unsafe {
        let mut r = RECT::default();
        GetWindowRect(tray, &mut r).ok()?;
        r
    };
    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if w <= 0 || h <= 0 {
        return None;
    }
    // 贴边判定：横条按上下、竖条按左右，工作区 = 整屏减去任务栏所在边条
    if r.right - r.left >= w / 2 {
        if r.top < h / 2 {
            Some((0, r.bottom, w, h)) // 贴顶
        } else {
            Some((0, 0, w, r.top)) // 贴底（常规形态）
        }
    } else if r.left < w / 2 {
        Some((r.right, 0, w, h)) // 贴左
    } else {
        Some((0, 0, r.left, h)) // 贴右
    }
}
