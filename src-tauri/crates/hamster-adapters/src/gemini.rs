//! Gemini CLI 适配器（v2.4：ACP 通用方言首个适配，runtime-design §11.5）。
//!
//! `~/.gemini/settings.json`：MCP 在顶层 `mcpServers` 键（JSON）。
//! 必须保真编辑——保留未知字段（hooks 等）与键序；禁止 serde 全量序列化回写。
//!
//! 会话面（checkpoint 索引 / resume）为 ⚠️ 未实测，按约定留空不夸大（R3+）；
//! 结构化通道走 ACP 通用方言（`gemini --experimental-acp`）。

use std::path::{Path, PathBuf};

use hamster_core::adapter::{AgentAdapter, CommandSpec};
use hamster_core::error::Result;
use hamster_core::model::agent::{Caps, InstallInfo};
use hamster_core::model::mcp::{McpServer, Scope};
use hamster_core::model::rules::RuleFile;
use hamster_core::model::runtime::{LaunchOption, PromptInject, RuntimeSpec};
use hamster_core::model::streaming::StructuredChannel;

use crate::jsonutil;

pub const ID: &str = "gemini";
pub const NAME: &str = "Gemini CLI";
const CONFIG_DIR: &str = ".gemini";
const CONFIG_FILE: &str = "settings.json";
const SERVERS_KEY: &[&str] = &["mcpServers"];
const SKILLS_DIR: &str = "skills";

pub struct GeminiAdapter {
    home: PathBuf,
}

impl GeminiAdapter {
    pub fn new(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
        }
    }

    fn config_root(&self) -> PathBuf {
        self.home.join(CONFIG_DIR)
    }

    fn settings_path(&self) -> PathBuf {
        self.config_root().join(CONFIG_FILE)
    }

    /// 读取 settings.json 中用户当前配置的 model，作为下拉默认值并入选项
    /// （只读；与 claude.rs configured_model 同一约定）。
    fn configured_model(&self) -> Option<String> {
        let raw = std::fs::read_to_string(self.settings_path()).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        v.get("model")
            .and_then(|m| m.as_str())
            .map(String::from)
            .filter(|s| !s.trim().is_empty())
    }

    fn model_launch_option(&self) -> LaunchOption {
        let configured = self.configured_model();
        let mut choices: Vec<String> = vec!["gemini-2.5-pro".into(), "gemini-2.5-flash".into()];
        if let Some(m) = &configured {
            if !choices.iter().any(|c| c == m) {
                choices.insert(0, m.clone());
            }
        }
        LaunchOption {
            choices,
            arg_template: "-m {v}".into(),
            default: configured,
            // ACP 通道经启动 flag -m 同样生效（app 层按 StructuredChannel.model_flag 拼）
            stream_override: true,
        }
    }
}

impl AgentAdapter for GeminiAdapter {
    fn id(&self) -> &'static str {
        ID
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn detect(&self) -> Option<InstallInfo> {
        // CLI 可解析才算已装：目录残留（试装/其他工具创建）不再误报
        crate::resolve_best_program("gemini")?;
        self.config_root().exists().then(|| InstallInfo {
            id: ID.into(),
            name: NAME.into(),
            version: None,
            config_root: self.config_root().to_string_lossy().to_string(),
        })
    }

    /// 本机 gemini CLI 的最佳路径（设置页展示；None = 未探测到）
    fn program_hint(&self) -> Option<String> {
        crate::resolve_best_program("gemini").map(|p| p.to_string_lossy().into_owned())
    }

    fn capabilities(&self) -> Caps {
        Caps {
            mcp: true,
            project_mcp: false,
            rules: true,
            skills: true,
            tool_granularity: false,
        }
    }

    fn mcp_path(&self, scope: &Scope) -> Option<PathBuf> {
        match scope {
            Scope::User => Some(self.settings_path()),
            Scope::Project { .. } => None,
        }
    }

    fn read_mcp(&self, raw: &str) -> Result<Vec<McpServer>> {
        jsonutil::read_servers(raw, SERVERS_KEY, &self.settings_path())
    }

    fn render_mcp(&self, current: Option<&str>, servers: &[McpServer]) -> Result<String> {
        jsonutil::render_servers(current, servers, SERVERS_KEY, &self.settings_path())
    }

    /// 指示.md：用户级 ~/.gemini/GEMINI.md；项目级 GEMINI.md（项目根）。
    fn rule_files(&self) -> Vec<RuleFile> {
        vec![RuleFile {
            name: "GEMINI.md".into(),
            path: self
                .config_root()
                .join("GEMINI.md")
                .to_string_lossy()
                .to_string(),
        }]
    }

    fn project_rule_name(&self) -> Option<String> {
        Some("GEMINI.md".into())
    }

    /// Gemini 官方 skills 目录（本机实测存在）。
    fn skills_dir(&self) -> Option<PathBuf> {
        Some(self.config_root().join(SKILLS_DIR))
    }

    /// 跨 Agent 会话共享 skill（v2.4）：~/.gemini/skills/hamster-session-resolver/SKILL.md
    fn render_session_skill(&self, current: Option<&str>) -> Result<Option<String>> {
        Ok(Some(crate::session_skill::render(current)?))
    }

    fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
        CommandSpec {
            program: crate::resolve_program("gemini"),
            args: vec![],
            working_dir: project_dir.to_path_buf(),
        }
    }

    /// v2.4：gemini CLI 接受位置参数作为首条 prompt（`gemini "消息"`）；
    /// 结构化通道 = ACP 通用方言（`--experimental-acp`）。
    /// resume（PTY `--resume` / checkpoint 索引）⚠️ 未实测，按约定留空（R3+）。
    fn runtime(&self) -> Option<RuntimeSpec> {
        Some(RuntimeSpec {
            program: crate::resolve_program("gemini"),
            args: vec![],
            windows_shim: true,
            prompt_inject: PromptInject::Argv,
            structured: Some(StructuredChannel {
                dialect: hamster_core::ProtocolDialect::Acp,
                model_flag: Some("-m".into()),
                args: vec!["--experimental-acp".into()],
            }),
            model: Some(self.model_launch_option()),
            effort: None, // 官方无独立推理强度启动参数
            resume_args: None,
            task_mode: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_SETTINGS: &str = r#"{
  "model": "gemini-2.5-pro",
  "hooks": { "BeforeAgent": [{ "hooks": [{ "type": "command", "command": "hook.cmd" }] }] },
  "mcpServers": {
    "fs": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-fs", "D:/pub"],
      "env": { "DEBUG": "1" }
    },
    "search": {
      "url": "https://mcp.example.com/search",
      "unknownField": "keep-me"
    }
  }
}"#;

    fn adapter() -> GeminiAdapter {
        GeminiAdapter::new(Path::new("/h"))
    }

    /// 往返：read → render → read 结果不变，未知字段/键序原样保留。
    #[test]
    fn roundtrip_read_render_read() {
        let original = adapter().read_mcp(FIXTURE_SETTINGS).unwrap();
        assert_eq!(original.len(), 2);

        let out = adapter()
            .render_mcp(Some(FIXTURE_SETTINGS), &original)
            .unwrap();
        let reparsed = adapter().read_mcp(&out).unwrap();
        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].name, original[0].name);

        // hooks / unknownField 等与 上游无关的内容原样保留
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("hooks").is_some(), "hooks 应保留");
        assert_eq!(v["mcpServers"]["search"]["unknownField"], "keep-me");
        // 键序：model 仍在最前（preserve_order）
        let out_obj = out.trim_start_matches('{').trim();
        assert!(out_obj.starts_with("\"model\""), "顶层键序应保留：{out}");
    }

    #[test]
    fn runtime_declares_acp_channel_and_skills() {
        let a = adapter();
        let caps = a.capabilities();
        assert!(caps.skills);
        assert!(caps.mcp);
        let spec = a.runtime().expect("gemini 应有 runtime 画像");
        let channel = spec.structured.expect("应有结构化通道");
        assert_eq!(channel.dialect, hamster_core::ProtocolDialect::Acp);
        assert_eq!(channel.args, vec!["--experimental-acp".to_string()]);
        assert!(spec.resume_args.is_none(), "resume 未实测不夸大");
        assert_eq!(a.skills_dir().unwrap(), PathBuf::from("/h/.gemini/skills"));
        // session resolver skill 内容与全局模板一致
        let skill = a.render_session_skill(None).unwrap().unwrap();
        assert!(skill.contains("hamster://session/<agent>/<sessionKey>"));
    }

    #[test]
    fn model_options_merge_configured_default() {
        // configured default 来自真实落盘的 settings.json（tempdir 沙箱）
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".gemini")).unwrap();
        std::fs::write(home.path().join(".gemini/settings.json"), FIXTURE_SETTINGS).unwrap();
        let spec = GeminiAdapter::new(home.path()).runtime().unwrap();
        let model = spec.model.unwrap();
        assert_eq!(model.default.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(
            model.choices.first().map(String::as_str),
            Some("gemini-2.5-pro")
        );
        assert!(model.choices.contains(&"gemini-2.5-flash".into()));
    }
}
