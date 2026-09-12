//! 跨 Agent 会话共享 skill（v2.4，architecture.md §6.1）。
//!
//! 各 Agent 的 skills 目录下由 SyncEngine 分发同名 `hamster-session-resolver` skill，
//! 内容（本模块）对全部 Agent 一致：指引 agent 用只读 `上游 session` 命令解析
//! `hamster://session/<agent>/<sessionKey>` 引用。渲染是纯函数；文件为 上游 全权
//! 管理文件（不做局部合并），外部改动由引擎按 Drift 三选一处理。

use hamster_core::error::Result;

/// skill 目录名（与 core::sync::plan::SESSION_SKILL_NAME 保持一致的字面量，
/// 由引擎在分发时引用 core 常量）。
pub const SESSION_SKILL_NAME: &str = "hamster-session-resolver";

pub const SESSION_SKILL_BODY: &str = r#"---
name: hamster-session-resolver
description: Resolve hamster://session/<agent>/<sessionKey> references — read another AI agent's conversation from the local 上游 index. Use whenever a message contains a hamster://session link and the referenced conversation matters as context.
---

# 上游 Session Resolver

`hamster://session/<agent>/<sessionKey>` is a 上游 workspace reference to a conversation of another AI coding agent (for example Claude Code or Codex). It is a pointer only: resolve it on demand with the command below. Never guess the referenced content from the link label.

## Resolution

1. Run the read-only command:

   ```
   上游 session get --agent <agent> --id <sessionKey> --json
   ```

   Optional: `--last <n>` caps the returned messages (default 20, max 200). `上游 session list [--agent <id>] --json` discovers which sessions exist.

2. The JSON result is `{"session": {...}, "messages": [...]}`. Messages are chronological projections of the referenced conversation; `role` is one of `user` / `assistant` / `thinking` / `tool` / `system`. `user` and `assistant` messages carry the conversation; skip `thinking` / `tool` rows unless the task needs that detail.

3. Treat the result as read-only context from another agent's work: build on it or cite it, but never claim it as your own and never restate it wholesale unless asked.

## Boundaries

- Reference-only: a `hamster://session/...` link does not authorize resuming, deleting, or writing to that session, its project, or any agent configuration.
- If the command fails (unknown session, index unavailable), say so plainly and continue with what you have — do not fabricate the missing conversation.
- The index is a local projection and may trail the newest turns. When the freshest state matters, ask the user or the owning agent instead of assuming.
"#;

/// 渲染 SKILL.md 全文（纯函数；`current` 不参与合并，见模块注释）。
pub fn render(_current: Option<&str>) -> Result<String> {
    Ok(SESSION_SKILL_BODY.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 渲染稳定性：同一输入输出一致，且包含 agent 侧解析所需的关键要素。
    #[test]
    fn render_is_stable_and_complete() {
        let a = render(None).unwrap();
        let b = render(Some("旧的任意内容")).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with("---\nname: hamster-session-resolver\n"));
        assert!(a.contains("hamster://session/<agent>/<sessionKey>"));
        assert!(a.contains("上游 session get --agent <agent> --id <sessionKey> --json"));
        assert!(a.contains("description:"));
        // skill 元数据块闭合
        assert!(a.contains("\n---\n\n# 上游 Session Resolver"));
    }
}
