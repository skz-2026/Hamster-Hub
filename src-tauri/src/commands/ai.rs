//! AI 助手命令：OpenAI 兼容 /chat/completions（Agent 接入点）。
//! 配置来自设置页（ai.base_url / ai.model / ai.api_key），未配置时返回
//! 引导性错误（前端展示配置入口）。当前非流式，流式 SSE 后续扩展。

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::AppError;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// 会话补全：messages 全量传入（前端管上下文），返回助手回复文本。
/// 三段式（不持 DB 锁跨 await）：阻塞读配置 → await 网络 → 返回。
#[tauri::command]
#[specta::specta]
pub async fn ai_chat(
    state: State<'_, AppState>,
    messages: Vec<ChatMessage>,
) -> Result<String, AppError> {
    if messages.is_empty() || messages.len() > 40 {
        return Err(AppError::validate("消息数为空或超出上下文上限"));
    }
    for m in &messages {
        if m.content.chars().count() > 8000 {
            return Err(AppError::validate("单条消息过长"));
        }
    }

    // 阻塞段：读 AI 配置
    let (base_url, model, api_key) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        let s = crate::store::settings::load(&conn)?;
        (s.ai.base_url, s.ai.model, s.ai.api_key)
    };
    let api_key = api_key
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| AppError::new("AI_UNCONFIGURED", "未配置 API Key，请先在设置页填写"))?;
    if model.trim().is_empty() {
        return Err(AppError::new(
            "AI_UNCONFIGURED",
            "未配置模型，请先在设置页填写",
        ));
    }
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    // await 段：OpenAI 兼容请求
    #[derive(Serialize)]
    struct Body<'a> {
        model: &'a str,
        messages: &'a [ChatMessage],
    }
    #[derive(Deserialize)]
    struct Resp {
        choices: Vec<Choice>,
    }
    #[derive(Deserialize)]
    struct Choice {
        message: RespMessage,
    }
    #[derive(Deserialize)]
    struct RespMessage {
        content: Option<String>,
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&Body {
            model: model.trim(),
            messages: &messages,
        })
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| AppError::new("NET", format!("AI 请求失败: {e}")))?;

    let status = resp.status();
    let parsed: Resp = resp
        .json()
        .await
        .map_err(|e| AppError::new("NET", format!("AI 响应解析失败（HTTP {status}）: {e}")))?;

    parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .filter(|c| !c.trim().is_empty())
        .ok_or_else(|| AppError::new("NET", "AI 返回空回复"))
}
