//! codex app-server v2 协议归一化回归测试（纯函数，无进程）。
//! 报文形状取自本机实测（codex-cli 0.144.5，2026-08-31）。

use molto_core::ProtocolDialect::CodexAppServer;
use molto_core::StreamEventKind::*;
use serde_json::json;

use crate::appserver::normalize_event;

fn map(method: &str, params: serde_json::Value) -> crate::appserver::MappedEvents {
    normalize_event("s1", CodexAppServer, method, &params)
}

#[test]
fn turn_lifecycle_maps_to_turn_events_and_side_effects() {
    let started = map(
        "turn/started",
        json!({ "threadId": "t", "turn": { "id": "turn-9", "status": "inProgress" } }),
    );
    assert_eq!(started.events.len(), 1);
    assert_eq!(started.events[0].kind, TurnStarted);
    assert_eq!(started.turn_started.as_deref(), Some("turn-9"));
    assert!(started.touch);

    let done = map(
        "turn/completed",
        json!({ "threadId": "t", "turn": { "id": "turn-9", "status": "completed" } }),
    );
    assert_eq!(done.events[0].kind, TurnCompleted);
    assert!(done.turn_completed);
}

#[test]
fn deltas_carry_item_id_for_assembly() {
    let d = map(
        "item/agentMessage/delta",
        json!({ "threadId": "t", "turnId": "u", "itemId": "m1", "delta": "你" }),
    );
    assert_eq!(d.events[0].kind, AgentDelta);
    assert_eq!(d.events[0].item_id, "m1");
    assert_eq!(d.events[0].text, "你");

    let r = map(
        "item/reasoning/summaryTextDelta",
        json!({ "threadId": "t", "itemId": "r1", "delta": "思考…" }),
    );
    assert_eq!(r.events[0].kind, ReasoningDelta);
}

#[test]
fn item_completed_variants_map_correctly() {
    let user = map(
        "item/completed",
        json!({ "threadId": "t", "turnId": "u", "item": {
            "id": "i1", "type": "userMessage",
            "content": [{ "type": "text", "text": "修登录超时" }]
        }}),
    );
    assert_eq!(user.events[0].kind, UserEcho);
    assert_eq!(user.events[0].text, "修登录超时");

    let agent = map(
        "item/completed",
        json!({ "item": { "id": "i2", "type": "agentMessage", "text": "已修复", "phase": "final_answer" } }),
    );
    assert_eq!(agent.events[0].kind, AgentDone);
    assert_eq!(agent.events[0].text, "已修复");

    let reasoning = map(
        "item/completed",
        json!({ "item": { "id": "i3", "type": "reasoning", "content": ["先看", "日志"], "summary": [] } }),
    );
    assert_eq!(reasoning.events[0].kind, ReasoningDone);
    assert_eq!(reasoning.events[0].text, "先看日志");

    let cmd = map(
        "item/completed",
        json!({ "item": {
            "id": "i4", "type": "commandExecution", "command": "cargo test -p molto",
            "status": "completed", "exitCode": 0, "aggregatedOutput": ""
        }}),
    );
    assert_eq!(cmd.events[0].kind, ToolItem);
    assert_eq!(cmd.events[0].tool_name.as_deref(), Some("cargo"));
    assert!(cmd.events[0].text.contains("cargo test"));
    assert_eq!(cmd.events[0].status.as_deref(), Some("completed"));

    let mcp = map(
        "item/completed",
        json!({ "item": {
            "id": "i5", "type": "mcpToolCall", "server": "zai", "tool": "search",
            "arguments": {}, "status": "completed"
        }}),
    );
    assert_eq!(mcp.events[0].tool_name.as_deref(), Some("zai.search"));

    let err = map(
        "item/completed",
        json!({ "item": { "id": "i6", "type": "errorItem", "message": "配额超限" } }),
    );
    assert_eq!(err.events[0].kind, Error);
    assert_eq!(err.events[0].text, "配额超限");
}

#[test]
fn unhandled_notifications_are_silently_ignored() {
    for m in [
        "thread/started",
        "mcpServer/startupStatus/updated",
        "thread/tokenUsage/updated",
    ] {
        let out = map(m, json!({ "threadId": "t" }));
        assert!(out.events.is_empty(), "{m} 不应产生事件");
        assert!(!out.touch);
    }
}

/// claude stream-json 归一化回归（报文形状取自本机实测 2.1.251）。
#[cfg(test)]
mod claude_tests {
    use crate::appserver::{normalize_claude, ClaudeMapped};
    use molto_core::StreamEventKind::*;

    fn map(v: serde_json::Value) -> ClaudeMapped {
        normalize_claude("s1", &v)
    }

    #[test]
    fn init_fills_thread_id_flag() {
        let m = map(serde_json::json!({
            "type": "system", "subtype": "init",
            "session_id": "bfdc980e-1111", "model": "glm-5.3"
        }));
        assert!(m.thread_id_filled);
        assert!(m.events.is_empty());
    }

    #[test]
    fn partial_deltas_map_to_agent_and_reasoning() {
        let text = map(serde_json::json!({
            "type": "stream_event",
            "session_id": "s1",
            "event": { "type": "content_block_delta", "index": 1,
                "delta": { "type": "text_delta", "text": "你" } }
        }));
        assert_eq!(text.events.len(), 1);
        assert_eq!(text.events[0].kind, AgentDelta);
        assert_eq!(text.events[0].text, "你");
        assert!(text.events[0].item_id.starts_with("blk-"));

        let think = map(serde_json::json!({
            "type": "stream_event",
            "event": { "type": "content_block_delta", "index": 0,
                "delta": { "type": "thinking_delta", "thinking": "想" } }
        }));
        assert_eq!(think.events[0].kind, ReasoningDelta);
    }

    #[test]
    fn assistant_blocks_map_to_done_and_tool() {
        let m = map(serde_json::json!({
            "type": "assistant",
            "message": { "id": "msg_1", "content": [
                { "type": "text", "text": "收到" },
                { "type": "tool_use", "name": "Bash", "input": { "command": "ls" } }
            ]}
        }));
        assert_eq!(m.events.len(), 2);
        assert_eq!(m.events[0].kind, AgentDone);
        assert_eq!(m.events[0].text, "收到");
        assert_eq!(m.events[1].kind, ToolItem);
        assert_eq!(m.events[1].tool_name.as_deref(), Some("Bash"));
        assert!(m.events[1].text.contains("ls"));
    }

    #[test]
    fn result_completes_turn() {
        let m = map(serde_json::json!({
            "type": "result", "subtype": "success",
            "result": "ok", "total_cost_usd": 0.09, "session_id": "s1"
        }));
        assert!(m.turn_completed);
        assert!(m
            .events
            .iter()
            .any(|e| e.kind == AgentDone && e.text == "ok"));
    }

    #[test]
    fn status_and_tool_results_are_ignored() {
        for v in [
            serde_json::json!({ "type": "system", "subtype": "status", "status": "compacting" }),
            serde_json::json!({ "type": "user", "message": { "content": [
                { "type": "tool_result", "content": "out" }] } }),
        ] {
            let m = map(v);
            assert!(m.events.is_empty());
            assert!(!m.turn_completed);
        }
    }
}

/// ACP 方言归一化（session/update 通知；runtime-design §11.5）。
mod acp_tests {
    use molto_core::ProtocolDialect::Acp;
    use molto_core::StreamEventKind::*;
    use serde_json::json;

    use crate::appserver::normalize_event;

    fn map(update: serde_json::Value) -> crate::appserver::MappedEvents {
        normalize_event("s1", Acp, "session/update", &json!({ "update": update }))
    }

    #[test]
    fn agent_and_thought_chunks_map_to_deltas() {
        let a = map(json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "增量" }
        }));
        assert_eq!(a.events.len(), 1);
        assert_eq!(a.events[0].kind, AgentDelta);
        assert_eq!(a.events[0].text, "增量");
        assert!(a.touch);

        let t = map(json!({
            "sessionUpdate": "agent_thought_chunk",
            "content": { "type": "text", "text": "思考" }
        }));
        assert_eq!(t.events[0].kind, ReasoningDelta);
    }

    #[test]
    fn tool_call_variants_carry_title_and_status() {
        let start = map(json!({
            "sessionUpdate": "tool_call", "toolCallId": "tc1",
            "title": "读取 main.rs", "kind": "read", "status": "pending"
        }));
        assert_eq!(start.events[0].kind, ToolItem);
        assert_eq!(start.events[0].item_id, "tc1");
        assert_eq!(start.events[0].tool_name.as_deref(), Some("读取 main.rs"));
        assert_eq!(start.events[0].status.as_deref(), Some("pending"));

        let done = map(json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "tc1",
            "status": "completed"
        }));
        assert_eq!(done.events[0].status.as_deref(), Some("completed"));
        // 无 title 时以 kind 兜底命名
        assert_eq!(done.events[0].tool_name.as_deref(), Some("tool"));
    }

    #[test]
    fn message_chunks_carry_message_id_as_item_key() {
        // 实测（opencode 1.18.25）：一条助手消息的思考部分与文本部分共用同一个
        // messageId——必须按角色加命名空间前缀，否则助手文本会被前端
        // findRow 追加进思考行（表现为回复整体消失在「思考」折叠块里）
        let a = map(json!({
            "sessionUpdate": "agent_message_chunk",
            "messageId": "msg_1",
            "content": { "type": "text", "text": "你好" }
        }));
        assert_eq!(a.events[0].item_id, "msg:msg_1");

        let t = map(json!({
            "sessionUpdate": "agent_thought_chunk",
            "messageId": "msg_1",
            "content": { "type": "text", "text": "想" }
        }));
        assert_eq!(t.events[0].item_id, "think:msg_1");

        // toolCallId 优先级不变（tool_call 系列不受影响）
        let tc = map(json!({
            "sessionUpdate": "tool_call", "toolCallId": "tc9", "messageId": "msg_1",
            "title": "写文件", "kind": "edit"
        }));
        assert_eq!(tc.events[0].item_id, "tc9");

        // 两者皆缺：空 id（前端回退「并入最后一条流式行」）
        let bare = map(json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "x" }
        }));
        assert_eq!(bare.events[0].item_id, "");
    }

    #[test]
    fn user_chunks_and_plan_updates_are_not_forwarded() {
        // user_message_chunk 不透传（前端乐观回显，避免双显）
        let user = map(json!({
            "sessionUpdate": "user_message_chunk",
            "content": { "type": "text", "text": "我说的" }
        }));
        assert!(user.events.is_empty());

        let plan = map(json!({ "sessionUpdate": "plan", "entries": [] }));
        assert!(plan.events.is_empty());
    }
}

#[test]
fn acp_config_options_parse_marks_mode_and_model_settable() {
    // 报文形状取自本机实测（opencode 1.18.x session/new，2026-09-06）；
    // mode 走 set_mode、model 走 set_config_option（§11.6 勘误，均实测生效）
    let resp = json!({
        "sessionId": "ses_x",
        "configOptions": [
            {
                "id": "model", "name": "Model", "category": "model", "type": "select",
                "currentValue": "zhipuai-coding-plan/glm-5.3",
                "options": [
                    { "value": "zhipuai-coding-plan/glm-5.3", "name": "Zhipu AI Coding Plan/GLM-5.3" },
                    { "value": "opencode/big-pickle", "name": "OpenCode Zen/Big Pickle" }
                ]
            },
            {
                "id": "mode", "name": "Session Mode", "category": "mode", "type": "select",
                "currentValue": "build",
                "options": [
                    { "value": "build", "name": "build" },
                    { "value": "plan", "name": "plan" }
                ]
            },
            { "id": "effort", "name": "Effort", "currentValue": "low" }
        ]
    });
    let opts = crate::appserver::parse_acp_config_options(&resp);
    assert_eq!(opts.len(), 3);
    let model = opts.iter().find(|o| o.id == "model").expect("model option");
    assert!(model.settable, "model 类配置项经 set_config_option 可写");
    assert_eq!(model.current_value, "zhipuai-coding-plan/glm-5.3");
    assert_eq!(model.choices.len(), 2);
    assert_eq!(model.choices[0].name, "Zhipu AI Coding Plan/GLM-5.3");
    let mode = opts.iter().find(|o| o.id == "mode").expect("mode option");
    assert!(mode.settable);
    // 无 options 数组的选择项：choices 为空但不炸
    let effort = opts
        .iter()
        .find(|o| o.id == "effort")
        .expect("effort option");
    assert!(effort.choices.is_empty() && !effort.settable);
}

#[test]
fn acp_config_options_absent_or_malformed_yields_empty() {
    assert!(
        crate::appserver::parse_acp_config_options(&json!({ "sessionId": "ses_x" })).is_empty()
    );
    assert!(
        crate::appserver::parse_acp_config_options(&json!({
            "configOptions": [ { "no_id": 1 }, { "id": "model" } ]
        }))
        .len()
            == 1,
        "缺 id 的项跳过，缺 currentValue 兜底空串"
    );
}

#[test]
fn acp_tool_call_diff_content_is_extracted() {
    // 实测形状：tool_call 的 content[].type=diff 携带 path/oldText/newText
    let upd = json!({
        "sessionUpdate": "tool_call", "toolCallId": "tc1",
        "title": "编辑 main.rs", "kind": "edit", "status": "completed",
        "content": [
            { "type": "diff", "path": "src/main.rs",
              "oldText": "fn main() {}", "newText": "fn main() {\n    println!(\"hi\");\n}" }
        ]
    });
    let mapped = crate::appserver::normalize_event(
        "s1",
        molto_core::ProtocolDialect::Acp,
        "session/update",
        &json!({ "update": upd }),
    );
    assert_eq!(mapped.events[0].kind, molto_core::StreamEventKind::ToolItem);
    let diff = mapped.events[0].diff.as_ref().expect("diff extracted");
    assert_eq!(diff.path, "src/main.rs");
    assert_eq!(diff.old_text.as_deref(), Some("fn main() {}"));
    assert!(diff.new_text.contains("println!"));

    // 无 diff 块 → None
    let plain = crate::appserver::normalize_event(
        "s1",
        molto_core::ProtocolDialect::Acp,
        "session/update",
        &json!({ "update": {
            "sessionUpdate": "tool_call", "toolCallId": "tc2",
            "title": "ls", "kind": "execute",
            "content": [{ "type": "content", "content": { "type": "text", "text": "file.txt" } }]
        } }),
    );
    assert!(plain.events[0].diff.is_none());
}

#[test]
fn claude_resume_flags_expand_resume_and_fork() {
    use crate::appserver::claude_resume_flags;

    // 续聊：--resume <key>（复用原会话 id）
    assert_eq!(
        claude_resume_flags(Some("abc-123"), false),
        vec!["--resume", "abc-123"]
    );
    // fork：--resume + --fork-session（派生新会话 id，原会话不动）
    assert_eq!(
        claude_resume_flags(Some("abc-123"), true),
        vec!["--resume", "abc-123", "--fork-session"]
    );
}

#[test]
fn claude_resume_flags_without_key_start_fresh() {
    use crate::appserver::claude_resume_flags;

    // 无 key（含空白）= 新会话；fork 无从派生同样不加 flag（与 codex None→thread/start 一致）
    assert!(claude_resume_flags(None, false).is_empty());
    assert!(claude_resume_flags(None, true).is_empty());
    assert!(claude_resume_flags(Some("   "), true).is_empty());
}
