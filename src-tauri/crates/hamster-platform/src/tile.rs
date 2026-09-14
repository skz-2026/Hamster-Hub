//! 窗口平铺（Snap 式）：把多扇应用窗口按预设布局铺到显示器工作区。
//!
//! 面向「左边看微信、右边浏览器」这类多任务场景：布局是一组归一化格子
//! （左右 / 上下 / 三分 / 四分 / 主从），格子数封顶 4（与 UI 的「最多 4 个
//! 窗口」一致）；每格摆一扇窗口，格子顺序即窗口顺序。
//!
//! 两个易错点已内建：
//! - **最大化窗口必须先还原**：`SetWindowPos` 改的是「还原尺寸」，最大化态下
//!   摆窗不上屏（最小化同理，还原后才有真实矩形）；跨线程 `ShowWindow`
//!   可能异步生效，故还原后要轮询确认再摆。见 [`place`]。
//! - **不可见边框补偿**：Win10/11 的窗口外框含约 7px/边的不可见拖拽边框，
//!   直接按外框摆会让可视内容与格子边界错开。按 `DWMWA_EXTENDED_FRAME_BOUNDS`
//!   与 `GetWindowRect` 的差值外扩目标矩形，使**可视**边缘贴合格子。
//!
//! 托管式分屏（见 `split_mode`）还要「进得来、回得去」：加入前的窗口状态由
//! [`capture_restore`] 快照、退出/移出时由 [`apply_restore`] 回放——直接按矩形
//! 回放会把原本最大化的窗口留成浮窗，故最大化/最小化态要按 `showCmd` 语义还原。

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, GetWindowRect, IsIconic, IsWindow, IsZoomed, SetWindowPos, ShowWindow,
    HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_MAXIMIZE, SW_MINIMIZE,
    SW_RESTORE, WINDOWPLACEMENT,
};

/// 归一化格子矩形 (x, y, w, h)，单位 0..1，原点左上
pub type Rect01 = (f64, f64, f64, f64);

/// 平铺布局（IPC 用字符串 id，见 [`TileLayout::parse`]）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileLayout {
    /// 左右两分（左 | 右）
    LeftRight,
    /// 上下两分（上 / 下）
    TopBottom,
    /// 左右三分
    Cols3,
    /// 上下三分
    Rows3,
    /// 四分（2×2）
    Quad,
    /// 主从：左大 + 右侧上下两小
    MainLeft,
    /// 主从：上大 + 下方左右两小
    MainTop,
}

/// 格子数上限（与前端「最多 4 个窗口」一致）
pub const MAX_SLOTS: usize = 4;

/// 还原最大化/最小化窗口的确认轮询次数与间隔（见 [`place`]）
const MAX_RESTORE_POLLS: usize = 6;
const RESTORE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);

/// 托管式分屏的「窗口数 → 布局」映射：成员每加一个就整体重排一次，
/// 规则单一可预期（第 n 扇 = 第 n 份），不给用户猜布局的机会：
///
/// | 窗口数 | 布局 | 效果 |
/// |---|---|---|
/// | 1 | `LeftRight` 第 1 格 | 占左半，右半空着等下一扇（也是「加进来了」的即时反馈）|
/// | 2 | `LeftRight` | 左右各半 |
/// | 3 | `Cols3` | 三等分列 |
/// | 4 | `Quad` | 2×2 |
///
/// 0 扇无布局（会话结束）。
pub fn layout_for(count: usize) -> Option<TileLayout> {
    match count {
        0 => None,
        1 | 2 => Some(TileLayout::LeftRight),
        3 => Some(TileLayout::Cols3),
        4 => Some(TileLayout::Quad),
        _ => None,
    }
}

/// 全部布局（顺序 = 分屏面板的展示顺序：两分 → 三分 → 四分 → 主从）
pub const ALL_LAYOUTS: [TileLayout; 7] = [
    TileLayout::LeftRight,
    TileLayout::TopBottom,
    TileLayout::Cols3,
    TileLayout::Rows3,
    TileLayout::Quad,
    TileLayout::MainLeft,
    TileLayout::MainTop,
];

impl TileLayout {
    /// 布局 id（与 [`TileLayout::parse`] 互为逆运算）
    pub fn id(self) -> &'static str {
        match self {
            Self::LeftRight => "lr",
            Self::TopBottom => "tb",
            Self::Cols3 => "cols3",
            Self::Rows3 => "rows3",
            Self::Quad => "quad",
            Self::MainLeft => "mainLeft",
            Self::MainTop => "mainTop",
        }
    }

    /// 解析 IPC 传来的布局 id
    pub fn parse(id: &str) -> Option<Self> {
        ALL_LAYOUTS.into_iter().find(|l| l.id() == id)
    }

    /// 该布局的格子数（1..=4）
    pub fn slot_count(self) -> usize {
        self.rects().len()
    }

    /// 归一化格子矩形，顺序 = 窗口摆放顺序。
    /// 相邻格共用同一个边界值（按同一基准算整，非各自累加），避免舍入出缝。
    pub fn rects(self) -> Vec<Rect01> {
        const H: f64 = 0.5;
        const T1: f64 = 1.0 / 3.0;
        const T2: f64 = 2.0 / 3.0;
        match self {
            Self::LeftRight => vec![(0.0, 0.0, H, 1.0), (H, 0.0, H, 1.0)],
            Self::TopBottom => vec![(0.0, 0.0, 1.0, H), (0.0, H, 1.0, H)],
            Self::Cols3 => vec![(0.0, 0.0, T1, 1.0), (T1, 0.0, T1, 1.0), (T2, 0.0, T1, 1.0)],
            Self::Rows3 => vec![(0.0, 0.0, 1.0, T1), (0.0, T1, 1.0, T1), (0.0, T2, 1.0, T1)],
            Self::Quad => vec![
                (0.0, 0.0, H, H),
                (H, 0.0, H, H),
                (0.0, H, H, H),
                (H, H, H, H),
            ],
            Self::MainLeft => vec![(0.0, 0.0, H, 1.0), (H, 0.0, H, H), (H, H, H, H)],
            Self::MainTop => vec![(0.0, 0.0, 1.0, H), (0.0, H, H, H), (H, H, H, H)],
        }
    }
}

/// 归一化格子 → 屏幕物理像素矩形 (x, y, w, h)。
///
/// `area` = (left, top, right, bottom) 工作区；`gap` 为格与格之间的间隙像素
/// （0 = 严丝合缝贴边，也是默认值）。
pub fn slot_px(slot: Rect01, area: (i32, i32, i32, i32), gap: i32) -> (i32, i32, i32, i32) {
    let (l, t, r, b) = area;
    let aw = (r - l).max(0) as f64;
    let ah = (b - t).max(0) as f64;
    let (x01, y01, w01, h01) = slot;
    let x1 = l + (x01 * aw).round() as i32;
    let y1 = t + (y01 * ah).round() as i32;
    let x2 = l + ((x01 + w01) * aw).round() as i32;
    let y2 = t + ((y01 + h01) * ah).round() as i32;
    let half = gap / 2;
    (
        x1 + half,
        y1 + half,
        (x2 - x1 - gap).max(1),
        (y2 - y1 - gap).max(1),
    )
}

/// 窗口句柄数值 → HWND（与 [`crate::tray`] 同口径：对外一律用 u64 id）
fn hwnd_of(id: u64) -> HWND {
    HWND(id as *mut core::ffi::c_void)
}

/// 取窗口（id = 窗口句柄数值）所在显示器的工作区 (left, top, right, bottom)，
/// 已扣除该屏任务栏/AppBar。多屏下按窗口实际所在屏算，不会把副屏窗口搬到主屏。
pub fn workarea_for(id: u64) -> Option<(i32, i32, i32, i32)> {
    let hwnd = hwnd_of(id);
    if hwnd.0.is_null() {
        return None;
    }
    unsafe {
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if mon.0.is_null() {
            return None;
        }
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(mon, &mut info).as_bool() {
            return None;
        }
        let r = info.rcWork;
        Some((r.left, r.top, r.right, r.bottom))
    }
}

/// 加入分屏前的窗口状态快照（退出/移出时据此还原）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowRestore {
    /// 还原矩形（外框，left, top, right, bottom）；最大化窗取「还原尺寸」而非当前满屏矩形
    pub rect: (i32, i32, i32, i32),
    /// 当时是否最大化（回放时要重新最大化，否则原本最大化的窗口会被留成浮窗）
    pub maximized: bool,
    /// 当时是否最小化
    pub minimized: bool,
}

/// 窗口是否仍存活（托管会话里用户可能直接关掉分屏内的窗口）
pub fn is_alive(id: u64) -> bool {
    let hwnd = hwnd_of(id);
    !hwnd.0.is_null() && unsafe { IsWindow(Some(hwnd)).as_bool() }
}

/// 快照窗口当前状态（矩形 + 最大化/最小化），供分屏退出时还原。
/// 取不到 `GetWindowPlacement` 时退回 `GetWindowRect`（至少位置对）。
pub fn capture_restore(id: u64) -> Option<WindowRestore> {
    let hwnd = hwnd_of(id);
    if !is_alive(id) {
        return None;
    }
    unsafe {
        let maximized = IsZoomed(hwnd).as_bool();
        let minimized = IsIconic(hwnd).as_bool();
        // rcNormalPosition 是「还原后」的矩形：最大化窗口当前矩形是满屏的，
        // 直接存它会导致退出分屏后那扇窗再也回不到原来的大小
        let mut wp = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ptMinPosition: POINT::default(),
            ptMaxPosition: POINT::default(),
            ..Default::default()
        };
        let rect = if GetWindowPlacement(hwnd, &mut wp).is_ok() {
            let r = wp.rcNormalPosition;
            (r.left, r.top, r.right, r.bottom)
        } else {
            let mut r = RECT::default();
            if GetWindowRect(hwnd, &mut r).is_err() {
                return None;
            }
            (r.left, r.top, r.right, r.bottom)
        };
        Some(WindowRestore {
            rect,
            maximized,
            minimized,
        })
    }
}

/// 把窗口还原到 [`capture_restore`] 时的矩形与显示状态。
/// 先退回普通态再摆矩形（否则 SetWindowPos 只改「还原尺寸」不上屏），
/// 最后按快照重新最大化/最小化。
pub fn apply_restore(id: u64, r: &WindowRestore) -> bool {
    let hwnd = hwnd_of(id);
    if !is_alive(id) {
        return false;
    }
    unsafe {
        ensure_normal(hwnd);
        let (l, t, right, bottom) = r.rect;
        let ok = SetWindowPos(
            hwnd,
            None,
            l,
            t,
            (right - l).max(1),
            (bottom - t).max(1),
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .is_ok();
        if r.maximized {
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
        } else if r.minimized {
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
        }
        ok
    }
}

/// 若窗口处于最大化/最小化态则退回普通态并等它生效。
/// 跨线程 `ShowWindow` 可能异步生效，轮询确认（最多 60ms）——否则
/// 「先把最大化的浏览器铺进左半屏」这类最常见用法会静默失效。
fn ensure_normal(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() || IsZoomed(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            for _ in 0..MAX_RESTORE_POLLS {
                if !IsIconic(hwnd).as_bool() && !IsZoomed(hwnd).as_bool() {
                    break;
                }
                std::thread::sleep(RESTORE_POLL_INTERVAL);
            }
        }
    }
}

/// 把窗口摆到屏幕物理矩形。摆成功（含被系统部分接受）返回 true，
/// 句柄无效或 SetWindowPos 被拒（UIPI 拦提权窗等）返回 false。
pub fn place(id: u64, x: i32, y: i32, w: i32, h: i32) -> bool {
    let hwnd = hwnd_of(id);
    if hwnd.0.is_null() {
        return false;
    }
    unsafe {
        // 最大化/最小化态先还原：否则 SetWindowPos 只改「还原尺寸」，画面不动
        ensure_normal(hwnd);
        let m = frame_margins(hwnd);
        SetWindowPos(
            hwnd,
            None,
            x - m.0,
            y - m.1,
            w + m.0 + m.2,
            h + m.1 + m.3,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .is_ok()
    }
}

/// 外框相对**可视**区域多出的像素：(左, 上, 右, 下)。
/// 取不到 DWM 扩展边界、或数值不可信（负值/超大）时返回全 0（不补偿）。
fn frame_margins(hwnd: HWND) -> (i32, i32, i32, i32) {
    unsafe {
        let mut outer = RECT::default();
        if GetWindowRect(hwnd, &mut outer).is_err() {
            return (0, 0, 0, 0);
        }
        let mut vis = RECT::default();
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut vis as *mut RECT as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_err()
        {
            return (0, 0, 0, 0);
        }
        let m = (
            vis.left - outer.left,
            vis.top - outer.top,
            outer.right - vis.right,
            outer.bottom - vis.bottom,
        );
        if [m.0, m.1, m.2, m.3].iter().any(|v| *v < 0 || *v > 64) {
            return (0, 0, 0, 0);
        }
        m
    }
}

/// 把窗口压到 z 序最底（桌面接管态把全屏主窗变回「壁纸底」用）。
pub fn send_to_bottom(hwnd: isize) {
    let h = HWND(hwnd as *mut core::ffi::c_void);
    if h.0.is_null() {
        return;
    }
    unsafe {
        let _ = SetWindowPos(
            h,
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1000×1000 工作区，方便断言整数边界
    const AREA: (i32, i32, i32, i32) = (0, 0, 1000, 1000);

    fn px(slot: Rect01) -> (i32, i32, i32, i32) {
        slot_px(slot, AREA, 0)
    }

    #[test]
    fn parse_all_layouts() {
        for layout in ALL_LAYOUTS {
            // id ↔ 布局 双向一致（前端只认 id 字符串，写错就静默不生效）
            assert_eq!(
                TileLayout::parse(layout.id()),
                Some(layout),
                "{}",
                layout.id()
            );
        }
        assert_eq!(TileLayout::parse("nope"), None);
    }

    #[test]
    fn slot_count_never_exceeds_four() {
        for layout in ALL_LAYOUTS {
            assert!(
                layout.slot_count() >= 2 && layout.slot_count() <= MAX_SLOTS,
                "{}: {}",
                layout.id(),
                layout.slot_count()
            );
        }
    }

    #[test]
    fn slots_cover_area_without_gap_or_overlap() {
        // 左右两分：两格拼起来正好是整块工作区
        let r = TileLayout::LeftRight.rects();
        assert_eq!(px(r[0]), (0, 0, 500, 1000));
        assert_eq!(px(r[1]), (500, 0, 500, 1000));

        // 四分：四格互不重叠且铺满
        let q = TileLayout::Quad.rects();
        assert_eq!(px(q[0]), (0, 0, 500, 500));
        assert_eq!(px(q[1]), (500, 0, 500, 500));
        assert_eq!(px(q[2]), (0, 500, 500, 500));
        assert_eq!(px(q[3]), (500, 500, 500, 500));
    }

    #[test]
    fn thirds_share_edges_exactly() {
        // 三分之一布局最容易因舍入出缝：相邻格右边界必须等于下一格左边界
        let r = TileLayout::Cols3.rects();
        let a = px(r[0]);
        let b = px(r[1]);
        let c = px(r[2]);
        assert_eq!(a.0 + a.2, b.0);
        assert_eq!(b.0 + b.2, c.0);
        assert_eq!(c.0 + c.2, 1000);
        assert_eq!(a.2, 333);
        assert_eq!(c.2, 333);
    }

    #[test]
    fn main_slave_layouts() {
        // 左大 + 右侧上下两小
        let r = TileLayout::MainLeft.rects();
        assert_eq!(px(r[0]), (0, 0, 500, 1000));
        assert_eq!(px(r[1]), (500, 0, 500, 500));
        assert_eq!(px(r[2]), (500, 500, 500, 500));
    }

    #[test]
    fn offset_area_and_gap() {
        // 工作区有偏移（副屏）时边界跟着平移
        let area = (1920, -200, 3200, 880);
        let r = TileLayout::LeftRight.rects();
        assert_eq!(slot_px(r[0], area, 0), (1920, -200, 640, 1080));
        assert_eq!(slot_px(r[1], area, 0), (2560, -200, 640, 1080));

        // 间隙：每格四周各缩 half，相邻两格之间留出 gap，且不越界
        let g = 16;
        let a = slot_px(r[0], AREA, g);
        let b = slot_px(r[1], AREA, g);
        assert_eq!(a, (8, 8, 484, 984));
        assert_eq!(b.0 - (a.0 + a.2), g);
        assert_eq!(b.0 + b.2, 1000 - 8);
    }

    #[test]
    fn degenerate_area_clamps_to_positive_size() {
        // 工作区反了（right < left）也不产出 0/负尺寸矩形
        let r = TileLayout::LeftRight.rects();
        let (_, _, w, h) = slot_px(r[0], (0, 0, 0, 0), 0);
        assert!(w >= 1 && h >= 1);
    }

    #[test]
    fn layout_for_grows_with_member_count() {
        // 成员数与布局的映射是 UI「分屏 n/4」文案的依据，改了这里要同步文案
        assert_eq!(layout_for(0), None);
        assert_eq!(layout_for(1), Some(TileLayout::LeftRight));
        assert_eq!(layout_for(2), Some(TileLayout::LeftRight));
        assert_eq!(layout_for(3), Some(TileLayout::Cols3));
        assert_eq!(layout_for(4), Some(TileLayout::Quad));
        // 超上限无布局（调用方按 CAPACITY 拦在前面，这里只保证不越界取 rects）
        assert_eq!(layout_for(5), None);
    }

    #[test]
    fn layout_for_always_has_room_for_every_member() {
        // 关键不变量：n 扇窗口必须能塞进 layout_for(n) 的格子里——
        // 否则 retile 里 rects[i] 会越界 panic
        for n in 1..=MAX_SLOTS {
            let layout = layout_for(n).expect("布局");
            assert!(
                layout.slot_count() >= n,
                "{n} 扇塞进 {} 格",
                layout.slot_count()
            );
            assert!(layout.slot_count() <= MAX_SLOTS);
        }
    }

    #[test]
    fn first_member_takes_left_half() {
        // 第 1 扇占左半：右半空着，是「再加一扇」的可见入口
        let r = layout_for(1).expect("布局").rects();
        assert_eq!(px(r[0]), (0, 0, 500, 1000));
    }
}
