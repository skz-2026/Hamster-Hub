//! 统一错误模型（SDD §5：code 稳定枚举 + message）
#![allow(dead_code)]

#[derive(Debug, Clone, thiserror::Error, serde::Serialize, specta::Type)]
#[error("{code}: {message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn io(e: impl Into<String>) -> Self {
        Self::new("IO", e)
    }
    pub fn db(e: impl Into<String>) -> Self {
        Self::new("DB", e)
    }
    pub fn validate(e: impl Into<String>) -> Self {
        Self::new("VALIDATE", e)
    }
    pub fn poison(e: impl Into<String>) -> Self {
        Self::new("POISON", e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::io(e.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::db(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new("SERDE", e.to_string())
    }
}
