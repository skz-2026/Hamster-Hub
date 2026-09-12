//! 控制中心命令：主音量读写（控制中心滑条后端）。

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct VolumeState {
    /// 0.0 - 1.0
    pub level: f32,
    pub muted: bool,
}

#[tauri::command]
#[specta::specta]
pub fn volume_get() -> Result<VolumeState, AppError> {
    hamster_platform::volume::get()
        .map(|v| VolumeState {
            level: v.level,
            muted: v.muted,
        })
        .map_err(|e| AppError::new("AUDIO", e))
}

#[tauri::command]
#[specta::specta]
pub fn volume_set(level: f32, muted: bool) -> Result<(), AppError> {
    hamster_platform::volume::set(level, muted).map_err(|e| AppError::new("AUDIO", e))
}
