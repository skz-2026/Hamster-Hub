//! 核心抽象：AgentAdapter trait。
//!
//! 约定（AGENTS.md §3）：
//! - Adapter 是纯函数层：`read_mcp` / `render_mcp` 只做格式转换，**禁止任何 IO**。
//!   现有文件内容由引擎读取后以 `current` 参数传入，写盘只发生在 `SyncEngine.apply()`。
//! - 格式保真是红线：不得用 serde 全量序列化回写，必须保留未知字段、键序与注释。

use std::path::{Path, PathBuf};

use crate::error::Result;
pub use crate::model::agent::Caps;
use crate::model::agent::InstallInfo;
use crate::model::mcp::{McpServer, Scope};

pub trait AgentAdapter: Send + Sync {
    /// 全小写稳定标识（"claude" / "codex" / …），同时是 store/state/备份目录的键。
    fn id(&self) -> &'static str;

    /// 展示名。
    fn name(&self) -> &'static str;

    /// 是否已安装（只读探测，不运行任何外部命令）。
    fn detect(&self) -> Option<InstallInfo>;

    /// 本机可解析的 CLI 程序路径（设置页展示与手动指定路径的对照基线；
    /// None = 未探测到）。多版本共存时为按版本择优的结果。
    fn program_hint(&self) -> Option<String> {
        None
    }

    /// 能力声明，必须如实。
    fn capabilities(&self) -> Caps;

    /// 该作用域下 MCP 配置文件路径；不支持返回 None。
    fn mcp_path(&self, scope: &Scope) -> Option<PathBuf>;

    /// 读：目标格式 → IRM。`raw` 为引擎读入的文件全文。
    fn read_mcp(&self, raw: &str) -> Result<Vec<McpServer>>;

    /// 渲染：IRM → 目标文件全文（纯函数）。
    /// `current` 为目标文件现有内容（不存在为 None），必须在此基础上合并，
    /// 保真保留与 上游无关的字段/注释/键序。
    fn render_mcp(&self, current: Option<&str>, servers: &[McpServer]) -> Result<String>;

    /// Skills 目录（M1 启用；现在仅 Doctor 用于重复安装检测）。
    /// 工具颗粒度目标文件路径（v1.2）。不支持返回 None。
    /// Claude Code：~/.claude/settings.json；不支持该能力的 Agent 不要实现（默认 None）。
    fn tool_config_path(&self, _scope: &Scope) -> Option<PathBuf> {
        None
    }

    /// 工具颗粒度渲染（v1.2）：把 servers 中的 disabledTools 合并进现有权限文件。
    /// 纯函数；返回 Ok(None) 表示无需产出该文件（如没有任何工具级配置）。
    fn render_tool_config(
        &self,
        _current: Option<&str>,
        _servers: &[McpServer],
    ) -> Result<Option<String>> {
        Ok(None)
    }

    /// workspace 历史发现（v1.4）：各 Agent 用自己的机制发现历史工作目录。
    /// Claude 读 ~/.claude.json projects 键 / Codex 扫 sessions rollout cwd /
    /// ZCode 读 v2/setting.json recentProjects。新 Agent 按自家机制实现；默认空。
    fn recent_workspaces(&self) -> Vec<crate::model::workspace::WorkspaceRecord> {
        Vec::new()
    }

    /// 指示.md 规则文件（v1.5）：用户级规则文件路径与显示名。默认空。
    fn rule_files(&self) -> Vec<crate::model::rules::RuleFile> {
        Vec::new()
    }

    /// 项目级规则文件名（项目根目录下），如 AGENTS.md。默认 None。
    fn project_rule_name(&self) -> Option<String> {
        None
    }

    fn skills_dir(&self) -> Option<PathBuf> {
        None
    }

    /// 跨 Agent 会话共享 skill（v2.4，architecture.md §6.1）：渲染
    /// hamster-session-resolver 的 SKILL.md 全文（纯函数，目标文件由 上游 全权管理，
    /// `current` 仅用于 trait 对称；外部改动经引擎 Drift 流程处理）。
    /// Ok(None) = 该 Agent 不参与 skill 分发。
    fn render_session_skill(&self, _current: Option<&str>) -> Result<Option<String>> {
        Ok(None)
    }

    /// 启动命令（含项目目录注入）。
    fn launch_cmd(&self, project_dir: &Path) -> CommandSpec;

    /// PTY 内嵌对话的启动画像（v2.0，runtime-design.md §3.5）。
    /// 返回 None = 该 Agent 不支持内嵌对话（Chat 域隐藏入口）。
    fn runtime(&self) -> Option<crate::model::runtime::RuntimeSpec> {
        None
    }

    /// 历史会话摘要（v2.0 R2 前置切片）：从 Agent 官方落盘目录**只读**扫描，
    /// 按 mtime 倒序、数量有上限；只提取 resume 所需元数据，不解析消息全文。
    /// 默认空 = 该 Agent 会话机制未支持/未实测。
    fn recent_sessions(&self) -> Vec<crate::model::session::SessionSummary> {
        Vec::new()
    }

    /// 会话落盘扫描源（v2.0 R2 索引摄取输入）。默认空 = 不索引。
    fn session_sources(&self) -> Vec<crate::model::session::SessionSource> {
        Vec::new()
    }

    /// 把一段原始行解析为消息投影（纯函数：行内容由索引引擎读入传入，
    /// 与 render_mcp 同一纯度约定）。返回 None = 本段无法识别（索引跳过该文件）。
    fn parse_chunk(&self, _lines: &[String]) -> Option<crate::model::session::ChunkParse> {
        None
    }
}

/// 启动命令规格：引擎据此拉起终端，不感知具体 Agent。
#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
}
