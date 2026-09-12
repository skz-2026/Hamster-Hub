//! Agent 相关模型：能力声明、安装信息、扫描结果。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Agent 能力声明。Adapter 必须如实声明，不夸大。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Caps {
    /// 支持用户级 MCP 分发
    pub mcp: bool,
    /// 支持项目级 MCP（scope = Project）
    pub project_mcp: bool,
    /// 规则文件分发（M1）
    pub rules: bool,
    /// Skills 目录管理（M1）
    pub skills: bool,
    /// 工具颗粒度（v1.2）：支持把 disabledTools 分发为工具级权限规则
    pub tool_granularity: bool,
}

/// 已安装信息（detect 的产物，内部使用）。
#[derive(Debug, Clone)]
pub struct InstallInfo {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub config_root: String,
}

/// 扫描结果（前端展示形态）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    /// 配置根路径（未安装为 null）
    pub config_root: Option<String>,
    pub capabilities: Caps,
    /// 支持内嵌对话（v2.0：adapter 提供 runtime 画像）
    pub chat: bool,
    /// 首条 prompt 注入方式；None = 不支持自动注入（消息不会随启动送达）
    pub prompt_inject: Option<crate::model::runtime::PromptInject>,
    /// 启动可选参数（模型/推理强度，官方 CLI 参数）；None = 不支持内嵌对话
    pub launch_options: Option<crate::model::runtime::LaunchOptions>,
    /// 支持官方结构化流式通道（v2.0 R3：GUI 会话实时渲染）
    pub streaming: bool,
}
