//! MCP 中间表示（IRM）。同步引擎只认识这里的类型，
//! JSON / TOML / JSONC 等目标格式的转换全部封死在 Adapter 内。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{MoltoError, Result};

/// MCP Server 中间表示。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct McpServer {
    pub name: String,
    pub transport: Transport,
    /// false = 不参与分发
    pub enabled: bool,
    /// 工具颗粒度（v1.2）：该 Server 内禁用的工具名；空 = 全部启用。
    /// 仅分发到 tool_granularity 能力的 Agent（如 Claude Code）。
    pub disabled_tools: Vec<String>,
}

/// 传输方式。序列化带 `type` 判别字段（stdio / http）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum Transport {
    #[serde(rename_all = "camelCase")]
    Stdio {
        command: String,
        args: Vec<String>,
        #[ts(type = "Record<string, string>")]
        env: BTreeMap<String, String>,
    },
    #[serde(rename_all = "camelCase")]
    Http {
        url: String,
        #[ts(type = "Record<string, string>")]
        headers: BTreeMap<String, String>,
    },
}

/// 配置作用域。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum Scope {
    User,
    #[serde(rename_all = "camelCase")]
    Project {
        dir: String,
    },
}

impl Scope {
    /// 稳定的短标识，用于 state.json 键与备份目录名。
    pub fn key(&self) -> String {
        match self {
            Scope::User => "user".to_string(),
            Scope::Project { dir } => {
                // 路径归一：统一正斜杠，避免分隔符差异产生两个键
                format!("project:{}", dir.replace('\\', "/"))
            }
        }
    }
}

impl McpServer {
    /// 保存源配置前的校验：fail-fast，坏数据不进 store。
    pub fn validate(&self) -> Result<()> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(MoltoError::config_invalid("Server 名称不能为空"));
        }
        if name != self.name {
            return Err(MoltoError::config_invalid(format!(
                "Server 名称「{}」首尾不能有空格",
                self.name
            )));
        }
        match &self.transport {
            Transport::Stdio { command, .. } => {
                if command.trim().is_empty() {
                    return Err(MoltoError::config_invalid(format!(
                        "Server「{}」的 command 不能为空",
                        name
                    )));
                }
            }
            Transport::Http { url, .. } => {
                let u = url.trim();
                if !(u.starts_with("http://") || u.starts_with("https://")) {
                    return Err(MoltoError::config_invalid(format!(
                        "Server「{}」的 URL 必须以 http:// 或 https:// 开头",
                        name
                    )));
                }
            }
        }
        Ok(())
    }
}
