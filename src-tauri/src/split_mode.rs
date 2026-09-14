//! 托管式分屏会话：桌面接管态下从 dock 右键「分屏添加」把窗口纳进来，由本模块
//! 持续维持分屏布局——每加一扇就整体重排一次（1 扇占左半 → 2 扇左右 → 3 扇三分
//! → 4 扇四分，见 `hamster_platform::tile::layout_for`），用户随时可以移出单扇、
//! 关掉单扇（剩下的自动补位）或整体退出（各自回到加入前的位置与显示状态）。
//!
//! 与「一次性铺窗」的区别就在这个**会话状态**：会话活着，分屏就是一份持续维护的
//! 桌面布局；会话结束，窗口全部物归原位。状态只在内存（进程重启后窗口早被系统
//! 重排过，回放陈旧矩形没有意义）。
//!
//! 分工：窗口状态读写与摆窗在 hamster-platform::tile（零 Tauri，带单测）；
//! 命令层 `commands/split.rs` 只做 app_key → exe 解析与错误映射。

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::desktop_mode;
use hamster_platform::tile::{self, WindowRestore};

/// 分屏可容纳的窗口数（与 UI「最多 4 个」一致，布局映射上限同此）
pub const CAPACITY: usize = tile::MAX_SLOTS;

/// 格与格之间的间隙（物理 px）。0 = 严丝合缝贴边：分屏要的是可用面积。
const TILE_GAP: i32 = 0;

/// 一扇被托管的窗口（对外快照）
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitMember {
    /// 窗口句柄数值（退出/移出/关闭都用它）
    pub window_id: u64,
    /// 来源应用（菜单据此判断「该应用是否已在分屏里」）
    pub app_key: String,
    /// 窗口标题（列表主文案；多开的应用靠它区分）
    pub title: String,
    /// 进程名（标题为空时的兜底文案）
    pub process: String,
    /// 窗口图标 PNG 的 data URL（空串时前端回退首字占位）
    pub png: String,
    /// 0 起的格序（UI 显示时 +1）
    pub slot: usize,
}

/// 分屏会话快照（`active=false` 时其余字段为空值）
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitState {
    pub active: bool,
    /// 布局 id（`tile::TileLayout::id`）；空串 = 无会话
    pub layout: String,
    /// 布局格数（空位数 = slots − members.len()）
    pub slots: usize,
    /// 上限（= 4）
    pub capacity: usize,
    pub members: Vec<SplitMember>,
}

impl SplitState {
    /// 无会话（也是退出后的返回值）
    fn idle() -> Self {
        Self {
            active: false,
            layout: String::new(),
            slots: 0,
            capacity: CAPACITY,
            members: Vec::new(),
        }
    }
}

/// 会话内一扇窗口 = 对外快照 + 加入前的窗口状态
struct Hosted {
    window_id: u64,
    app_key: String,
    title: String,
    process: String,
    png: String,
    restore: WindowRestore,
}

#[derive(Default)]
struct Session {
    /// 顺序 = 格序（第 i 扇在第 i 格）
    members: Vec<Hosted>,
}

/// 当前会话（None = 不在分屏中）。全局单例：分屏是整块桌面的一份布局，
/// 不存在「两个并行会话」，多开会互相抢窗口位置。
static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// 取会话锁。锁中毒（某次持锁 panic）不该让分屏功能整体瘫掉：拿回内部值继续。
fn lock() -> std::sync::MutexGuard<'static, Option<Session>> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

/// 会话 → 对外快照
fn snapshot(session: &Session) -> SplitState {
    let layout = tile::layout_for(session.members.len());
    SplitState {
        active: !session.members.is_empty(),
        layout: layout.map(|l| l.id().to_string()).unwrap_or_default(),
        slots: layout.map(|l| l.slot_count()).unwrap_or(0),
        capacity: CAPACITY,
        members: session
            .members
            .iter()
            .enumerate()
            .map(|(slot, m)| SplitMember {
                window_id: m.window_id,
                app_key: m.app_key.clone(),
                title: m.title.clone(),
                process: m.process.clone(),
                png: m.png.clone(),
                slot,
            })
            .collect(),
    }
}

/// 按当前成员数整体重排（加窗/移出/补位后都要走一次）。
///
/// 接管态还要把主窗压回 z 序最底：它是整屏的「桌面」，属普通窗，不压会盖住分屏窗。
fn retile(app: &AppHandle, session: &Session) {
    let Some(layout) = tile::layout_for(session.members.len()) else {
        return;
    };
    let rects = layout.rects();
    let takeover = desktop_mode::is_active();
    // 工作区来源与一次性铺窗同口径：接管态用我们写过的 SPI 值（整屏 − 置顶任务栏条），
    // 否则按第一扇窗口所在显示器取 rcWork（多屏下不把副屏窗口搬到主屏）
    let area = if takeover {
        hamster_platform::workarea::get()
    } else {
        tile::workarea_for(session.members[0].window_id).or_else(hamster_platform::workarea::get)
    };
    let Some(area) = area else {
        return;
    };
    for (i, m) in session.members.iter().enumerate() {
        let (x, y, w, h) = tile::slot_px(rects[i], area, TILE_GAP);
        tile::place(m.window_id, x, y, w, h);
    }
    if takeover {
        if let Some(main) = app.get_webview_window("main") {
            if let Ok(hwnd) = main.hwnd() {
                tile::send_to_bottom(hwnd.0 as isize);
            }
        }
    }
}

/// 把一个应用的窗口加入分屏：取它**尚未在分屏里**的最上层窗口（多开的应用
/// 反复调用可依次加进来）。加入的那扇会置前——用户刚点的就是它。
pub fn add(app: &AppHandle, exe: &str, app_key: &str) -> Result<SplitState, String> {
    let mut guard = lock();
    let mut session = guard.take().unwrap_or_default();
    if session.members.len() >= CAPACITY {
        *guard = Some(session);
        return Err(format!("分屏已满（最多 {CAPACITY} 个窗口）"));
    }
    let taken: Vec<u64> = session.members.iter().map(|m| m.window_id).collect();
    let hit = hamster_platform::tray::list_by_process(std::process::id(), exe)
        .into_iter()
        .find(|w| !taken.contains(&w.id));
    let Some(hit) = hit else {
        *guard = Some(session);
        return Err("该应用没有可加入分屏的窗口".into());
    };
    // 快照必须在摆窗之前取：摆完再取，存下的就是分屏里的矩形，退出时「还原」等于没还原
    let Some(restore) = tile::capture_restore(hit.id) else {
        *guard = Some(session);
        return Err("读取窗口状态失败，无法加入分屏".into());
    };
    session.members.push(Hosted {
        window_id: hit.id,
        app_key: app_key.to_string(),
        title: hit.title,
        process: exe.to_string(),
        png: hit.png,
        restore,
    });
    retile(app, &session);
    let state = snapshot(&session);
    *guard = Some(session);
    let _ = hamster_platform::tray::activate(hit.id);
    Ok(state)
}

/// 把某扇窗口移出分屏：窗口回到加入前的位置/状态，剩下的成员补位。
/// 最后一扇移出即会话结束。
pub fn remove(app: &AppHandle, window_id: u64) -> SplitState {
    let mut guard = lock();
    let mut ended = false;
    if let Some(session) = guard.as_mut() {
        if let Some(pos) = session
            .members
            .iter()
            .position(|m| m.window_id == window_id)
        {
            let m = session.members.remove(pos);
            tile::apply_restore(m.window_id, &m.restore);
        }
        if session.members.is_empty() {
            ended = true;
        } else {
            retile(app, session);
        }
    }
    if ended {
        *guard = None;
        return SplitState::idle();
    }
    guard
        .as_ref()
        .map(snapshot)
        .unwrap_or_else(SplitState::idle)
}

/// 退出分屏：全部窗口还原到加入前的位置/状态，会话清空。幂等。
pub fn exit() -> SplitState {
    if let Some(session) = lock().take() {
        // 正序还原：与加入顺序一致，退出后的 z 序和加入前接近
        for m in session.members.iter() {
            tile::apply_restore(m.window_id, &m.restore);
        }
    }
    SplitState::idle()
}

/// 读会话快照，顺带自愈：用户可能在分屏之外直接关掉某扇窗口（任务栏、应用自身
/// 退出），这里把它从会话剔除，剩下的按新成员数补位；剔光了就结束会话。
/// 前端在分屏期间轮询本函数（每 ≤4 个句柄一次 IsWindow，开销可忽略）。
pub fn state(app: &AppHandle) -> SplitState {
    let mut guard = lock();
    let mut ended = false;
    if let Some(session) = guard.as_mut() {
        let before = session.members.len();
        session.members.retain(|m| tile::is_alive(m.window_id));
        if session.members.is_empty() {
            ended = true;
        } else if session.members.len() != before {
            retile(app, session);
        }
    }
    if ended {
        *guard = None;
        return SplitState::idle();
    }
    guard
        .as_ref()
        .map(snapshot)
        .unwrap_or_else(SplitState::idle)
}
