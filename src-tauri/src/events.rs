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

// ===== bench（代理工作台，vendor 自 Molto 的域层）=====

/// 流式会话事件（GUI 对话数据源；payload 为 Molto 归一化 StreamEvent）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
#[serde(transparent)]
pub struct BenchStreamEvent {
    pub event: molto_core::StreamEvent,
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
