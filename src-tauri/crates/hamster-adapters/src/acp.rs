//! ACP 通道型 Agent 适配器（v2.5）：数据驱动接入「原生支持 ACP」的 Agent。
//!
//! 背景（runtime-design.md §11.5）：ACP 方言在 runtime 已通用化——握手、会话、
//! 发轮次、通知归一化都不认具体 Agent。于是接入一个原生 ACP Agent 只剩三项
//! per-Agent 知识：**可执行名 / ACP 启动参数 / 已装探测依据**。
//! 本模块把这类 Agent 收敛为「一张纯数据表 + 一个泛型适配器」，避免为每个
//! Agent 复制一份 250 行样板（AGENTS.md §6 checklist 的等价轻量路径）。
//!
//! 边界（AGENTS.md §6.3 如实声明，不夸大）：
//! - **只接 Chat 结构化通道**：MCP / 规则 / Skills 配置面一律 `false`——
//!   这些 Agent 的配置格式未经本机实测，上游不接管、不猜测
//! - 程序名与参数以官方 registry
//!   （<https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json>）
//!   为准；`verified=false` 表示**未实机握手验证**（程序名/参数待复核）
//! - 续聊不声明 `resume_args`：ACP 走 `session/load{sessionId}`（协议层续聊，
//!   由 `agentCapabilities.loadSession` 门控），不是启动 argv

use std::path::{Path, PathBuf};

use hamster_core::adapter::{AgentAdapter, CommandSpec};
use hamster_core::error::{HamsterError, Result};
use hamster_core::model::agent::{Caps, InstallInfo};
use hamster_core::model::mcp::{McpServer, Scope};
use hamster_core::model::runtime::{LaunchOption, PromptInject, RuntimeSpec};
use hamster_core::model::streaming::StructuredChannel;

/// 一个 ACP 通道型 Agent 的接入规格（纯数据；新增 Agent = 表内加一项）。
pub struct AcpAgentSpec {
    /// 上游 内部 id（全小写，store/备份目录键）
    pub id: &'static str,
    /// 展示名
    pub name: &'static str,
    /// 候选可执行名（按序探测：npm 全局 → PATH；首个命中即用它启动）
    pub programs: &'static [&'static str],
    /// ACP 启动参数（直接拼在 program 之后）
    pub acp_args: &'static [&'static str],
    /// TUI 的模型覆盖 flag（官方参数，如 "--model"；经 RuntimeSpec.model 供
    /// 欢迎屏选择：TUI 走 plan_spawn 拼 argv，GUI 流式走会话配置
    /// `session/set_config_option` 应用）。None = 未实测，不声明（AGENTS.md §6
    /// 如实原则）。choices 恒为空：模型清单随登录的 provider 动态变化，
    /// 不静态枚举——GUI 会话内由 agent 经 configOptions 提供真实清单
    pub tui_model_flag: Option<&'static str>,
    /// 已装探测依据：home 下的配置目录（相对路径，任一存在即视为装过）
    pub config_dirs: &'static [&'static str],
    /// 官方 ACP registry 的 id（可追溯；None = registry 未收录）
    pub registry_id: Option<&'static str>,
    /// 已实机验证：握手 initialize + session/new 走通（本机实测）
    pub verified: bool,
    /// 备注（未验证项的依据/风险）
    pub note: &'static str,
}

/// 接入清单（v2.5）。
///
/// `programs` / `acp_args` 取自官方 registry 的 distribution 字段：
/// npx 分发 → 取包名去 scope 后的 CLI 名 + args；binary 分发 → 取 `cmd` 文件名。
pub const SPECS: &[AcpAgentSpec] = &[
    AcpAgentSpec {
        id: "opencode",
        name: "OpenCode",
        programs: &["opencode"],
        acp_args: &["acp"],
        // TUI 官方 `-m/--model provider/model`（实测 1.18.3 --help）；
        // ACP 通道无此 flag（实测 `opencode acp` 传入 -m 进程无应答）
        tui_model_flag: Some("--model"),
        config_dirs: &[".opencode", ".config/opencode"],
        registry_id: Some("opencode"),
        verified: true,
        note: "本机实测 1.18.29（2026-09-13）：工具调用/续聊/编辑全链路；write 工具不带 diff 块（schema 允许）；session/new 有联网校验，弱网握手需容忍慢应答",
    },
    AcpAgentSpec {
        id: "kimi",
        name: "Kimi CLI",
        programs: &["kimi"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".kimi", ".kimi-code"],
        registry_id: Some("kimi"),
        // 握手通过；session/new 返回 Authentication required（未登录，协议正常）
        verified: true,
        note: "本机实测 0.36.1：ACP 应答正常，需先登录",
    },
    AcpAgentSpec {
        id: "cursor",
        name: "Cursor",
        programs: &["cursor-agent"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".cursor"],
        registry_id: Some("cursor"),
        verified: false,
        note: "本机实测 2026-09-06：initialize 返回 agentCapabilities（loadSession=true, image=true），session/new 未回包——需先 `agent login`（authMethods=cursor_login）后实聊；协议握手正常",
    },
    AcpAgentSpec {
        id: "copilot",
        name: "GitHub Copilot CLI",
        programs: &["copilot"],
        acp_args: &["--acp"],
        tui_model_flag: None,
        config_dirs: &[".copilot"],
        registry_id: Some("github-copilot-cli"),
        verified: false,
        note: "registry 记 --acp；另有 --acp --stdio 说法，待实机复核",
    },
    AcpAgentSpec {
        id: "qwen",
        name: "Qwen Code",
        programs: &["qwen"],
        acp_args: &["--acp", "--experimental-skills"],
        tui_model_flag: None,
        config_dirs: &[".qwen"],
        registry_id: Some("qwen-code"),
        verified: false,
        note: "registry args = --acp --experimental-skills",
    },
    AcpAgentSpec {
        id: "codebuddy",
        name: "Codebuddy Code",
        programs: &["codebuddy", "codebuddy-code"],
        acp_args: &["--acp"],
        tui_model_flag: None,
        config_dirs: &[".codebuddy"],
        registry_id: Some("codebuddy-code"),
        verified: false,
        note: "腾讯云官方 CLI（npx @tencent-ai/codebuddy-code）",
    },
    AcpAgentSpec {
        id: "auggie",
        name: "Auggie CLI",
        programs: &["auggie"],
        acp_args: &["--acp"],
        tui_model_flag: None,
        config_dirs: &[".augment"],
        registry_id: Some("auggie"),
        verified: false,
        note: "Augment Code；配置目录名待复核",
    },
    AcpAgentSpec {
        id: "cline",
        name: "Cline",
        programs: &["cline"],
        acp_args: &["--acp"],
        tui_model_flag: None,
        config_dirs: &[".cline"],
        registry_id: Some("cline"),
        verified: false,
        note: "npx cline --acp",
    },
    AcpAgentSpec {
        id: "goose",
        name: "Goose",
        programs: &["goose"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".config/goose", ".goose"],
        registry_id: Some("goose"),
        verified: false,
        note: "Block 开源 agent",
    },
    AcpAgentSpec {
        id: "kilo",
        name: "Kilo",
        programs: &["kilo"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".kilocode", ".kilo"],
        registry_id: Some("kilo"),
        verified: false,
        note: "npm 备选 @kilocode/cli acp",
    },
    AcpAgentSpec {
        id: "vibe",
        name: "Mistral Vibe",
        programs: &["vibe-acp", "vibe"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".vibe", ".config/vibe"],
        registry_id: Some("mistral-vibe"),
        verified: false,
        note: "registry 分发的可执行名即 vibe-acp，无附加参数",
    },
    AcpAgentSpec {
        id: "droid",
        name: "Factory Droid",
        programs: &["droid"],
        acp_args: &["exec", "--output-format", "acp-daemon"],
        tui_model_flag: None,
        config_dirs: &[".factory"],
        registry_id: Some("factory-droid"),
        verified: false,
        note: "参数形态特殊（exec --output-format acp-daemon）",
    },
    AcpAgentSpec {
        id: "devin",
        name: "Devin CLI",
        programs: &["devin"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".devin", ".config/devin"],
        registry_id: Some("devin"),
        verified: false,
        note: "Cognition",
    },
    AcpAgentSpec {
        id: "junie",
        name: "Junie",
        programs: &["junie"],
        acp_args: &["--acp=true"],
        tui_model_flag: None,
        config_dirs: &[".junie"],
        registry_id: Some("junie"),
        verified: false,
        note: "JetBrains；参数为 --acp=true",
    },
    AcpAgentSpec {
        id: "grok",
        name: "Grok Build",
        programs: &["grok"],
        acp_args: &["agent", "stdio"],
        tui_model_flag: None,
        config_dirs: &[".grok"],
        registry_id: Some("grok-build"),
        verified: false,
        note: "xAI；参数形态 = agent stdio",
    },
    AcpAgentSpec {
        id: "cortex",
        name: "Cortex Code",
        programs: &["cortex"],
        acp_args: &["acp", "serve"],
        tui_model_flag: None,
        config_dirs: &[".cortex"],
        registry_id: Some("cortex-code"),
        verified: false,
        note: "Snowflake；参数形态 = acp serve",
    },
    AcpAgentSpec {
        id: "poolside",
        name: "Poolside",
        programs: &["pool", "poolside"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".poolside"],
        registry_id: Some("poolside"),
        verified: false,
        note: "registry 分发可执行名 pool-<platform>",
    },
    AcpAgentSpec {
        id: "stakpak",
        name: "Stakpak",
        programs: &["stakpak"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".stakpak"],
        registry_id: Some("stakpak"),
        verified: false,
        note: "DevOps agent",
    },
    AcpAgentSpec {
        id: "fast-agent",
        name: "fast-agent",
        programs: &["fast-agent-acp", "fast-agent"],
        acp_args: &["-x"],
        tui_model_flag: None,
        config_dirs: &[".fast-agent"],
        registry_id: Some("fast-agent"),
        verified: false,
        note: "uvx fast-agent-acp==VERSION -x",
    },
    AcpAgentSpec {
        id: "openhands",
        name: "OpenHands",
        programs: &["openhands"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".openhands"],
        registry_id: None,
        verified: false,
        note: "⚠️ registry 未收录，程序名/参数待实机复核",
    },
    AcpAgentSpec {
        id: "kiro",
        name: "Kiro CLI",
        programs: &["kiro", "kiro-cli"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".kiro"],
        registry_id: None,
        verified: false,
        note: "⚠️ registry 未收录，参数待实机复核",
    },
    AcpAgentSpec {
        id: "minimax",
        name: "MiniMax Code",
        programs: &["minimax"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".minimax"],
        registry_id: None,
        verified: false,
        note: "⚠️ registry 未收录，程序名/参数待实机复核",
    },
    AcpAgentSpec {
        id: "amp",
        name: "Amp",
        programs: &["amp-acp", "amp"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".amp"],
        registry_id: Some("amp-acp"),
        verified: false,
        note: "registry 分发名 amp-acp（无附加参数）",
    },
    AcpAgentSpec {
        id: "glm",
        name: "GLM Agent",
        programs: &["glm-acp-agent", "glm"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".glm"],
        registry_id: Some("glm-acp-agent"),
        verified: false,
        note: "智谱 Z.ai；npx glm-acp-agent（无附加参数）",
    },
    AcpAgentSpec {
        id: "pi",
        name: "Pi",
        programs: &["pi-acp"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".pi"],
        registry_id: Some("pi-acp"),
        verified: false,
        note: "经 pi-acp 适配器（npx pi-acp）",
    },
    AcpAgentSpec {
        id: "agoragentic",
        name: "Agora Agentic",
        programs: &["agoragentic-mcp"],
        acp_args: &["--acp"],
        tui_model_flag: None,
        config_dirs: &[".agoragentic"],
        registry_id: Some("agoragentic-acp"),
        verified: false,
        note: "registry npx @agoragentic/mcp，args = --acp",
    },
    AcpAgentSpec {
        id: "antigravity",
        name: "Antigravity",
        programs: &["agy_acp_server"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".antigravity"],
        registry_id: Some("antigravity-acp"),
        verified: false,
        note: "Google Antigravity；binary agy_acp_server，Linux 需 --uid=",
    },
    AcpAgentSpec {
        id: "autohand",
        name: "Autohand",
        programs: &["autohand-acp", "autohand"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".autohand"],
        registry_id: Some("autohand"),
        verified: false,
        note: "registry npx @autohandai/autohand-acp",
    },
    AcpAgentSpec {
        id: "corust",
        name: "Corust Agent",
        programs: &["corust-agent-acp", "corust-agent"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".corust"],
        registry_id: Some("corust-agent"),
        verified: false,
        note: "registry binary corust-agent-acp",
    },
    AcpAgentSpec {
        id: "crow",
        name: "Crow CLI",
        programs: &["crow-cli"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".crow"],
        registry_id: Some("crow-cli"),
        verified: false,
        note: "registry binary crow-cli，args = acp",
    },
    AcpAgentSpec {
        id: "deepagents",
        name: "DeepAgents",
        programs: &["deepagents-acp", "deepagents"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".deepagents"],
        registry_id: Some("deepagents"),
        verified: false,
        note: "registry npx deepagents-acp（args 空）",
    },
    AcpAgentSpec {
        id: "dirac",
        name: "Dirac",
        programs: &["dirac-cli", "dirac"],
        acp_args: &["--acp"],
        tui_model_flag: None,
        config_dirs: &[".dirac"],
        registry_id: Some("dirac"),
        verified: false,
        note: "registry npx dirac-cli，args = --acp",
    },
    AcpAgentSpec {
        id: "dimcode",
        name: "DimCode",
        programs: &["dimcode"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".dimcode"],
        registry_id: Some("dimcode"),
        verified: false,
        note: "registry npx dimcode，args = acp",
    },
    AcpAgentSpec {
        id: "harn",
        name: "Harn",
        programs: &["harn"],
        acp_args: &["serve", "acp"],
        tui_model_flag: None,
        config_dirs: &[".harn"],
        registry_id: Some("harn"),
        verified: false,
        note: "registry binary harn，args = serve acp",
    },
    AcpAgentSpec {
        id: "minion",
        name: "Minion Code",
        programs: &["minion-code"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".minion"],
        registry_id: Some("minion-code"),
        verified: false,
        note: "registry uvx minion-code，args = acp",
    },
    AcpAgentSpec {
        id: "nova",
        name: "Nova",
        programs: &["nova"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".nova"],
        registry_id: Some("nova"),
        verified: false,
        note: "registry npx @compass-ai/nova，args = acp",
    },
    AcpAgentSpec {
        id: "qoder",
        name: "Qoder",
        programs: &["qodercli", "qoder"],
        acp_args: &["--acp"],
        tui_model_flag: None,
        config_dirs: &[".qoder"],
        registry_id: Some("qoder"),
        verified: false,
        note: "registry npx @qoder-ai/qodercli，args = --acp",
    },
    AcpAgentSpec {
        id: "sigit",
        name: "Sigit",
        programs: &["sigit"],
        acp_args: &[],
        tui_model_flag: None,
        config_dirs: &[".sigit"],
        registry_id: Some("sigit"),
        verified: false,
        note: "registry binary+npx @smbcloud/sigit",
    },
    AcpAgentSpec {
        id: "vtcode",
        name: "VTCode",
        programs: &["vtcode"],
        acp_args: &["acp"],
        tui_model_flag: None,
        config_dirs: &[".vtcode"],
        registry_id: Some("vtcode"),
        verified: false,
        note: "registry binary vtcode，args = acp（含 env 变量）",
    },
];

/// ACP 通道型 Agent 适配器实例（一个 spec + 一个 home）。
pub struct AcpAgentAdapter {
    home: PathBuf,
    spec: &'static AcpAgentSpec,
}

impl AcpAgentAdapter {
    pub fn new(home: &Path, spec: &'static AcpAgentSpec) -> Self {
        Self {
            home: home.to_path_buf(),
            spec,
        }
    }

    /// 探测到的启动程序：最佳可执行（多版本取最高；探测见 lib.rs resolve_best_program）。
    fn resolved_program(&self) -> String {
        self.spec
            .programs
            .iter()
            .find_map(|p| crate::resolve_best_program(p))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.spec.programs[0].to_string())
    }

    /// 可解析的最佳可执行路径（None = 本机没有）。
    fn best_program(&self) -> Option<std::path::PathBuf> {
        self.spec
            .programs
            .iter()
            .find_map(|p| crate::resolve_best_program(p))
    }

    /// detect 的可注入版本（测试注假 resolve，避免依赖测试机的 PATH）。
    fn detect_with(&self, resolve: impl Fn(&str) -> Option<PathBuf>) -> Option<InstallInfo> {
        let program = self.spec.programs.iter().find_map(|p| resolve(p))?;
        let config_root = self
            .spec
            .config_dirs
            .iter()
            .map(|d| self.home.join(d))
            .find(|p| p.is_dir())
            .unwrap_or(program);
        Some(InstallInfo {
            id: self.spec.id.into(),
            name: self.spec.name.into(),
            version: None,
            config_root: config_root.to_string_lossy().into_owned(),
        })
    }
}

impl AgentAdapter for AcpAgentAdapter {
    fn id(&self) -> &'static str {
        self.spec.id
    }

    fn name(&self) -> &'static str {
        self.spec.name
    }

    /// 已装判定：可执行可解析（硬证据——launch 依赖它）→ 配置目录存在
    /// （软证据，仅决定 configRoot 展示）。纯文件系统探测，不运行任何外部命令。
    /// 目录存在不再单独算已装：IDE/试装残留（如 `.cursor`）会误报。
    fn detect(&self) -> Option<InstallInfo> {
        self.detect_with(crate::resolve_best_program)
    }

    /// 最佳可执行路径（设置页展示 + 手动指定路径的对照基线）
    fn program_hint(&self) -> Option<String> {
        self.best_program()
            .map(|p| p.to_string_lossy().into_owned())
    }

    /// 只接通道：MCP / 规则 / Skills / 工具颗粒度一律 false（未实测，不夸大）。
    fn capabilities(&self) -> Caps {
        Caps::default()
    }

    fn mcp_path(&self, _scope: &Scope) -> Option<PathBuf> {
        None
    }

    fn read_mcp(&self, _raw: &str) -> Result<Vec<McpServer>> {
        Err(HamsterError::Unsupported(format!(
            "{} 的 MCP 配置面未接入（本机未实测其格式）",
            self.spec.name
        )))
    }

    fn render_mcp(&self, _current: Option<&str>, _servers: &[McpServer]) -> Result<String> {
        Err(HamsterError::Unsupported(format!(
            "{} 的 MCP 配置面未接入（本机未实测其格式）",
            self.spec.name
        )))
    }

    /// TUI 形态：纯 CLI（无 ACP 参数，交给用户在终端里自己用）。
    fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
        CommandSpec {
            program: self.resolved_program(),
            args: vec![],
            working_dir: project_dir.to_path_buf(),
        }
    }

    /// ACP 通道画像：`structured.args` = spec 的 ACP 启动参数。
    /// model 仅在 spec 实测声明了 TUI flag 时给出（TUI 走 argv，GUI 流式经
    /// `session/set_config_option` 应用，choices 恒空——真实清单由 agent 在
    /// 会话 configOptions 提供）；effort/task_mode 一律 None（未实测不猜）。
    fn runtime(&self) -> Option<RuntimeSpec> {
        Some(RuntimeSpec {
            program: self.resolved_program(),
            args: vec![],
            // Windows：npm 安装的 CLI 多为 .cmd shim，经 cmd /c 才能稳定 spawn
            windows_shim: true,
            // ACP 通道不吃 argv prompt（走 session/prompt）；TUI 也不代用户输入
            prompt_inject: PromptInject::None,
            structured: Some(StructuredChannel {
                dialect: hamster_core::ProtocolDialect::Acp,
                // 无启动级模型 flag：模型经会话配置 session/set_config_option 应用
                model_flag: None,
                args: self.spec.acp_args.iter().map(|a| a.to_string()).collect(),
            }),
            model: self.spec.tui_model_flag.map(|flag| LaunchOption {
                choices: Vec::new(),
                arg_template: format!("{flag} {{v}}"),
                default: None,
                stream_override: true,
            }),
            effort: None,
            // 续聊走协议层 session/load，不是启动 argv
            resume_args: None,
            task_mode: None,
        })
    }
}

/// 可执行探测统一走 `crate::resolve_best_program`（npm 全局 → PATH 收集候选，
/// 多版本按 `--version` 择优，带 TTL 缓存）；本模块不再单独维护一份查找逻辑。

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn spec(id: &str) -> &'static AcpAgentSpec {
        SPECS.iter().find(|s| s.id == id).expect("清单中应存在")
    }

    /// 清单完整性：id 唯一小写、名非空、至少一个候选程序。
    #[test]
    fn specs_are_well_formed() {
        let mut seen = HashSet::new();
        for s in SPECS {
            assert!(!s.id.is_empty() && !s.name.is_empty(), "id/name 不得为空");
            assert_eq!(s.id, s.id.to_lowercase(), "id 必须全小写：{}", s.id);
            assert!(seen.insert(s.id), "id 重复：{}", s.id);
            assert!(!s.programs.is_empty(), "{} 需至少一个候选程序", s.id);
            assert!(s.name.chars().count() <= 24, "展示名过长：{}", s.name);
        }
    }

    /// 已实机验证的 Agent 必须有非空 ACP 参数（否则启动的是 TUI 不是协议服务端）。
    #[test]
    fn verified_specs_declare_acp_args() {
        for s in SPECS.iter().filter(|s| s.verified) {
            assert!(
                !s.acp_args.is_empty(),
                "{} 标了 verified 却无 ACP 启动参数",
                s.id
            );
        }
        assert!(SPECS.iter().any(|s| s.verified), "应至少有一个已验证项");
    }

    /// 与既有 Adapter 的 id 不冲突（claude/codex/gemini/zcode 已由专设适配器接管）。
    #[test]
    fn no_id_collision_with_builtin_adapters() {
        let builtin: HashSet<&str> = ["claude", "codex", "gemini", "zcode"].into();
        for s in SPECS {
            assert!(!builtin.contains(s.id), "id 与内置适配器冲突：{}", s.id);
        }
    }

    /// 配置目录探测：有可执行时配置目录优先作为 config_root。
    #[test]
    fn detect_prefers_config_dir() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".config/opencode")).unwrap();
        let adapter = AcpAgentAdapter::new(home.path(), spec("opencode"));
        let info = adapter
            .detect_with(|_| Some(PathBuf::from("/bin/opencode")))
            .expect("可执行 + 配置目录应判定为已装");
        assert_eq!(info.id, "opencode");
        assert!(info.config_root.ends_with(".config/opencode"));
    }

    /// 回归：只有配置目录残留（IDE/试装）而无可执行 → 不再误报为已装。
    #[test]
    fn config_dir_alone_is_not_installed() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let detected = AcpAgentAdapter::new(home.path(), spec("cursor"))
            .detect_with(|_| None)
            .is_some();
        assert!(!detected, "无可执行文件时不得判定为已装");
    }

    /// 无配置目录时 config_root 回退为解析到的可执行路径。
    #[test]
    fn detect_falls_back_to_program_path() {
        let home = tempfile::tempdir().unwrap();
        let info = AcpAgentAdapter::new(home.path(), spec("opencode"))
            .detect_with(|p| (p == "opencode").then(|| PathBuf::from("/usr/local/bin/opencode")))
            .expect("可执行可解析即已装");
        assert_eq!(info.config_root, "/usr/local/bin/opencode");
    }

    /// 未装（无目录也无程序）→ detect 为 None，不污染扫描清单。
    #[test]
    fn undetected_agent_is_hidden() {
        let home = tempfile::tempdir().unwrap();
        let a = AcpAgentAdapter::new(home.path(), spec("minimax"));
        // 沙箱 home 下无 .minimax；程序名 minimax 极不可能存在
        if crate::resolve_best_program("minimax").is_none() {
            assert!(a.detect().is_none());
        }
    }

    /// runtime 画像：ACP 方言 + spec 参数 + Windows shim；能力面如实为 false。
    /// model 仅对实测声明了 tui_model_flag 的 spec 给出（opencode 首例）：
    /// TUI 走 argv、GUI 流式经 set_config_option（stream_override=true），
    /// choices 恒空（真实清单由 agent 在会话 configOptions 提供）。
    #[test]
    fn runtime_declares_acp_channel_and_honest_caps() {
        for s in SPECS {
            let a = AcpAgentAdapter::new(Path::new("/h"), s);
            let spec_rt = a.runtime().expect("ACP Agent 必须有 runtime 画像");
            let channel = spec_rt.structured.expect("必须有结构化通道");
            assert_eq!(channel.dialect, hamster_core::ProtocolDialect::Acp);
            assert_eq!(
                channel.args,
                s.acp_args.iter().map(|x| x.to_string()).collect::<Vec<_>>()
            );
            assert!(channel.model_flag.is_none(), "ACP 通道无启动级模型 flag");
            assert!(spec_rt.windows_shim, "Windows npm shim 需 cmd /c 包装");
            match (s.tui_model_flag, spec_rt.model.as_ref()) {
                (None, None) => {}
                (Some(flag), Some(opt)) => {
                    assert_eq!(opt.arg_template, format!("{flag} {{v}}"));
                    assert!(opt.choices.is_empty(), "模型清单动态，不得静态枚举");
                    assert!(opt.stream_override, "GUI 流式经 set_config_option 应用");
                    assert_eq!(opt.default, None);
                }
                (None, Some(_)) => panic!("{} 未实测 flag 却声明了 model", s.id),
                (Some(_), None) => panic!("{} 声明了 flag 却未给出 model", s.id),
            }
            assert!(spec_rt.effort.is_none(), "未实测的推理强度参数不得声明");
            assert!(spec_rt.resume_args.is_none(), "ACP 续聊走协议层");
            assert_eq!(spec_rt.prompt_inject, PromptInject::None);
            let caps = a.capabilities();
            assert!(
                !caps.mcp && !caps.project_mcp && !caps.rules && !caps.skills,
                "{} 未实测的配置面必须声明为 false",
                s.id
            );
        }
    }

    /// opencode 是 tui_model_flag 首例：TUI `--model {v}`（实测 1.18.3）。
    #[test]
    fn opencode_declares_verified_tui_model_flag() {
        let a = AcpAgentAdapter::new(Path::new("/h"), spec("opencode"));
        let model = a
            .runtime()
            .expect("runtime")
            .model
            .expect("opencode 应声明 TUI 模型覆盖");
        assert_eq!(model.arg_template, "--model {v}");
    }

    /// 配置面未接入：调用应显式报 Unsupported，而不是静默返回空。
    #[test]
    fn config_surface_is_explicitly_unsupported() {
        let a = AcpAgentAdapter::new(Path::new("/h"), spec("opencode"));
        assert!(matches!(
            a.read_mcp("{}"),
            Err(HamsterError::Unsupported(_))
        ));
        assert!(a.mcp_path(&Scope::User).is_none());
        assert!(a.skills_dir().is_none());
        assert!(a.rule_files().is_empty());
    }
}
