//! 应用设置（settings.json 心智模型，M0 落 SQLite settings 表）
//! 前端经 IPC 读写；serde(default) 保证旧数据向前兼容。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Default)]
#[serde(default)]
pub struct Settings {
    pub appearance: Appearance,
    pub search: Search,
    pub file_index: FileIndex,
    pub weather: Weather,
    pub ai: Ai,
    pub behavior: Behavior,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct Appearance {
    pub theme: String,
    pub accent: String,
    pub glass: String,
    pub font_scale: f64,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            accent: "#FF8A3D".into(),
            glass: "acrylic".into(),
            font_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct Search {
    pub hotkey: String,
    pub search_apps: bool,
    pub search_files: bool,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Space".into(),
            search_apps: true,
            search_files: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct FileIndex {
    pub roots: Vec<String>,
    pub max_files: u32, // specta 禁 i64（TS 无 BigInt），10 万量级 u32 足够
}

impl Default for FileIndex {
    fn default() -> Self {
        Self {
            roots: vec![
                "~/Desktop".into(),
                "~/Documents".into(),
                "~/Downloads".into(),
                "~/Pictures".into(),
                "~/Videos".into(),
            ],
            max_files: 100_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct Weather {
    pub provider: String,
    pub city_id: String,
    pub qweather_key: Option<String>,
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            provider: "open-meteo".into(),
            city_id: String::new(),
            qweather_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct Ai {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Default for Ai {
    fn default() -> Self {
        Self {
            base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            model: String::new(),
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct Behavior {
    pub autostart: bool,
    pub start_minimized: bool,
    pub language: String,
    /// 桌面模式切换热键（tauri global-shortcut 格式，重启生效）
    pub desktop_mode_hotkey: String,
    /// 启动即进入桌面模式（对齐水豚hub：打开应用直接全屏工作台 + 底部 Dock）
    pub desktop_mode_on_launch: bool,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            autostart: false,
            start_minimized: false,
            language: "zh-CN".into(),
            desktop_mode_hotkey: "Ctrl+Alt+D".into(),
            desktop_mode_on_launch: true,
        }
    }
}
