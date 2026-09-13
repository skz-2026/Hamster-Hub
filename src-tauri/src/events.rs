//! 强类型事件（命名：域://动作；payload 经 specta 生成 TS 类型）

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct CoreReady {
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct DesktopModeChanged {
    pub active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct AppIndexUpdated {
    pub count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct FileIndexUpdated {
    pub count: u32,
}

// ===== bench（代理工作台，源自上游（Apache-2.0，整合时更名）的域层）=====

/// 流式会话事件（GUI 对话数据源；payload 为 上游 归一化 StreamEvent）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
#[serde(transparent)]
pub struct BenchStreamEvent {
    pub event: hamster_core::StreamEvent,
}

/// 流式会话进程退出（前端刷新活会话列表）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct BenchStreamExit {
    pub session_id: String,
}

/// PTY 会话进程退出（TUI 会话）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct BenchPtyExit {
    pub session_id: String,
    pub exit_code: u32,
}

// ===== 待办提醒（core::reminder 后台调度 → 应用内提示）=====

/// 待办提醒到点（系统通知已同时发出）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct TodoReminder {
    pub id: u32,
    pub content: String,
    pub due_at: Option<i64>,
}

// ===== 番茄钟（core::focus 秒级计时 → 应用内倒计时）=====

/// 计时心跳（每秒）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct FocusTick {
    pub kind: String,
    pub remaining_secs: u32,
    pub paused: bool,
}

/// 一轮计时结束（专注/休息完成，历史已落库）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct FocusFinished {
    pub kind: String,
}

// ===== 应用自更新（commands::updater 下载安装进度）=====

/// 更新下载进度（done=true 表示安装包已就绪，应用即将退出交由安装器接管）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub done: bool,
}

// ===== 密码箱（闲置自动锁看门狗 → 前端锁屏覆盖层）=====

/// 密码箱已锁定（reason: manual | idle）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
pub struct VaultLocked {
    pub reason: String,
}
