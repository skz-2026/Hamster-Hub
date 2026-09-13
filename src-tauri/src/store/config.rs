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
    /// Agent 域（桌面助手 / computer use，M4）
    pub agent: Agent,
}

/// Agent 域设置：桌面助手 persona、computer use 安全开关与桌面 MCP 分发
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct Agent {
    /// computer use 总开关：允许 agent 经桌面 MCP server 操作真实鼠标键盘
    /// （HAMSTER_CU_MODE 注入；默认关，安全红线见 docs/03 §3.4）
    pub computer_use_enabled: bool,
    /// 桌面助手 persona（追加为 agent system prompt；空 = 内置仓鼠默认）
    pub assistant_persona: String,
    /// 桌面 MCP 端口（None = 默认 47613；被占用时回退随机端口，实际值见设置页）
    pub mcp_port: Option<u16>,
    /// 用户级长效令牌：写入各 agent 配置文件供外部 MCP 宿主接入
    /// （None = 首启自动生成；区别于 bench 助手会话的一次性令牌）
    pub mcp_user_token: Option<String>,
    /// 已开启桌面 MCP 分发的 agent id（写入其配置文件；移除 = 从配置摘除条目）
    pub mcp_agents: Vec<String>,
    /// 桌面助手默认代理（空 = 自动：claude → zcode → 首个已装且支持结构化通道）
    pub assistant_agent_id: String,
    /// 桌面助手默认模型（空 = agent CLI 自身默认）
    pub assistant_model: String,
    /// 桌面助手默认推理强度（空 = agent 默认；选项来自各 agent launchOptions）
    pub assistant_effort: String,
    /// 用户手动指定的 Agent CLI 程序路径（agentId → 绝对路径）：优先于自动探测，
    /// 供绿色版/自拷贝的 CLI 注册进应用。写入口 agent_cli_path_set（先校验再落库）
    pub cli_paths: std::collections::BTreeMap<String, String>,
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
