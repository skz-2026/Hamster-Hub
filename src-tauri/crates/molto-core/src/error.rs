//! 统一错误类型。core/adapters 一律返回 `Result<T, MoltoError>`。

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MoltoError {
    #[error("文件读写失败（{path}）：{source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("JSON 解析失败（{path}）：{message}")]
    JsonParse { path: PathBuf, message: String },

    #[error("TOML 解析失败（{path}）: {message}")]
    TomlParse { path: PathBuf, message: String },

    #[error("配置无效：{message}")]
    ConfigInvalid { message: String },

    #[error("未找到：{what}")]
    NotFound { what: String },

    #[error("目标文件在 Molto 之外被修改过（{path}），需要先决策处理方式")]
    Drift { path: PathBuf },

    #[error("未注册的 Agent：{0}")]
    UnknownAdapter(String),

    #[error("该 Agent 不支持此操作：{0}")]
    Unsupported(String),

    #[error("启动失败（{program}）：{message}")]
    Launch { program: String, message: String },

    #[error("{0}")]
    Other(String),
}

impl MoltoError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn json_parse(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::JsonParse {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn toml_parse(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::TomlParse {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn config_invalid(message: impl Into<String>) -> Self {
        Self::ConfigInvalid {
            message: message.into(),
        }
    }

    pub fn not_found(what: impl Into<String>) -> Self {
        Self::NotFound { what: what.into() }
    }

    /// 错误码：前端据此做分支处理（见 molto-app 的 ApiError）。
    pub fn code(&self) -> &'static str {
        match self {
            MoltoError::Io { .. } => "io",
            MoltoError::JsonParse { .. } => "json_parse",
            MoltoError::TomlParse { .. } => "toml_parse",
            MoltoError::ConfigInvalid { .. } => "config_invalid",
            MoltoError::NotFound { .. } => "not_found",
            MoltoError::Drift { .. } => "drift",
            MoltoError::UnknownAdapter(_) => "unknown_adapter",
            MoltoError::Unsupported(_) => "unsupported",
            MoltoError::Launch { .. } => "launch",
            MoltoError::Other(_) => "other",
        }
    }

    /// 主消息（不含路径上下文时也可读）。
    pub fn message(&self) -> String {
        match self {
            MoltoError::Io { path, source } => {
                format!("文件读写失败（{}）：{}", path.display(), source)
            }
            MoltoError::JsonParse { path, message } => {
                format!("JSON 解析失败（{}）：{}", path.display(), message)
            }
            MoltoError::TomlParse { path, message } => {
                format!("TOML 解析失败（{}）：{}", path.display(), message)
            }
            MoltoError::ConfigInvalid { message } => format!("配置无效：{}", message),
            MoltoError::NotFound { what } => format!("未找到：{}", what),
            MoltoError::Drift { path } => format!(
                "目标文件在 Molto 之外被修改过（{}），需要先决策处理方式",
                path.display()
            ),
            MoltoError::UnknownAdapter(id) => format!("未注册的 Agent：{}", id),
            MoltoError::Unsupported(what) => format!("该 Agent 不支持此操作：{}", what),
            MoltoError::Launch { program, message } => {
                format!("启动失败（{}）：{}", program, message)
            }
            MoltoError::Other(message) => message.clone(),
        }
    }

    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            MoltoError::Io { path, .. }
            | MoltoError::JsonParse { path, .. }
            | MoltoError::TomlParse { path, .. }
            | MoltoError::Drift { path } => Some(path),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, MoltoError>;
