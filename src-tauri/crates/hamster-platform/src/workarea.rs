//! 工作区控制（SPI_GET/SETWORKAREA）：桌面接管时把工作区扩为整屏。
//!
//! 隐藏系统任务栏后，它的 AppBar 预留仍会留在工作区上（最大化应用只铺到
//! 原工作区底，与全屏差一条）——需显式 SPI_SETWORKAREA 为整屏，最大化/新开
//! 的应用才会真正铺满（水豚hub 同效果：内容延伸到 dock 后面，dock 置顶覆盖）。
//! 退出接管时还原进入前的工作区（任务栏重新显示时 shell 亦会自行重算）。

use windows::core::Result;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_GETWORKAREA, SPI_SETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
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
