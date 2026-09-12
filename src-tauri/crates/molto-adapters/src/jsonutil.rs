//! JSON 家族（Claude / ZCode）共享的 MCP 结构转换。
//!
//! 目标格式均为「name → server 对象」的映射，server 对象内：
//! stdio = { command, args?, env? }，http = { url, headers? }。
//! 渲染时在现有条目上合并已知字段，**保留条目内未知字段**。

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value};

use molto_core::error::{MoltoError, Result};
use molto_core::model::mcp::{McpServer, Transport};

/// 从 JSON 文本读取 servers 映射。`key_path` 是到映射的键路径（如 ["mcpServers"] 或 ["mcp","servers"]）。
pub fn read_servers(raw: &str, key_path: &[&str], path: &Path) -> Result<Vec<McpServer>> {
    if raw.trim().is_empty() {
        return Ok(vec![]);
    }
    let root: Value =
        serde_json::from_str(raw).map_err(|e| MoltoError::json_parse(path, e.to_string()))?;
    let Some(node) = dig(&root, key_path) else {
        return Ok(vec![]);
    };
    let Some(map) = node.as_object() else {
        return Ok(vec![]);
    };
    let mut out = vec![];
    for (name, entry) in map {
        let Some(transport) = entry_to_transport(entry) else {
            // 识别不了的形态：跳过但保留在文件里（渲染走合并，不会丢）
            continue;
        };
        out.push(McpServer {
            name: name.clone(),
            transport,
            enabled: true,
            disabled_tools: Vec::new(),
        });
    }
    Ok(out)
}

/// 渲染：在 `current` 基础上把 servers 映射写到 `key_path` 位置。
/// 策略为严格单源：映射内容 = 源 servers；外部多出的 server 走漂移流程保护，
/// 只有用户明确选择「以源为准」时才会被移除。
pub fn render_servers(
    current: Option<&str>,
    servers: &[McpServer],
    key_path: &[&str],
    path: &Path,
) -> Result<String> {
    let mut root: Value = match current {
        Some(raw) if !raw.trim().is_empty() => {
            serde_json::from_str(raw).map_err(|e| MoltoError::json_parse(path, e.to_string()))?
        }
        _ => Value::Object(Map::new()),
    };
    let map = ensure_object(&mut root, key_path).ok_or_else(|| {
        MoltoError::config_invalid(format!(
            "「{}」中 {} 不是对象，无法写入 MCP 配置",
            path.display(),
            key_path.join(".")
        ))
    })?;

    let mut new_map = Map::new();
    for server in servers.iter().filter(|s| s.enabled) {
        let existing = map.get(&server.name).cloned();
        new_map.insert(
            server.name.clone(),
            transport_to_entry(&server.transport, existing),
        );
    }
    *map = new_map;

    let mut out = serde_json::to_string_pretty(&root)
        .map_err(|e| MoltoError::Other(format!("序列化 JSON 失败：{}", e)))?;
    out.push('\n');
    Ok(out)
}

fn dig<'a>(root: &'a Value, key_path: &[&str]) -> Option<&'a Value> {
    let mut node = root;
    for key in key_path {
        node = node.get(key)?;
    }
    Some(node)
}

/// 沿键路径逐层确保对象存在，返回最内层映射的可变引用。
fn ensure_object<'a>(root: &'a mut Value, key_path: &[&str]) -> Option<&'a mut Map<String, Value>> {
    let mut node = root;
    for key in key_path {
        if !node.is_object() {
            *node = Value::Object(Map::new());
        }
        let obj = node.as_object_mut()?;
        node = obj
            .entry(key.to_string())
            .or_insert(Value::Object(Map::new()));
    }
    node.as_object_mut()
}

/// 条目 → Transport。command 优先（stdio），其次 url（http）。
fn entry_to_transport(entry: &Value) -> Option<Transport> {
    let obj = entry.as_object()?;
    if let Some(command) = obj.get("command").and_then(|c| c.as_str()) {
        let args = obj
            .get("args")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let env = string_map(obj.get("env"));
        return Some(Transport::Stdio {
            command: command.to_string(),
            args,
            env,
        });
    }
    if let Some(url) = obj.get("url").and_then(|u| u.as_str()) {
        let headers = string_map(obj.get("headers"));
        return Some(Transport::Http {
            url: url.to_string(),
            headers,
        });
    }
    None
}

fn string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// Transport → 条目对象。在 `existing` 基础上合并已知字段，保留未知字段。
fn transport_to_entry(transport: &Transport, existing: Option<Value>) -> Value {
    let mut obj = match existing {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };
    match transport {
        Transport::Stdio { command, args, env } => {
            // 换传输方式时清掉另一类的字段
            obj.remove("url");
            obj.remove("headers");
            obj.insert("command".into(), Value::String(command.clone()));
            if args.is_empty() {
                obj.remove("args");
            } else {
                obj.insert(
                    "args".into(),
                    Value::Array(args.iter().map(|a| Value::String(a.clone())).collect()),
                );
            }
            if env.is_empty() {
                obj.remove("env");
            } else {
                obj.insert("env".into(), to_string_value(env));
            }
        }
        Transport::Http { url, headers } => {
            obj.remove("command");
            obj.remove("args");
            obj.remove("env");
            obj.insert("url".into(), Value::String(url.clone()));
            if headers.is_empty() {
                obj.remove("headers");
            } else {
                obj.insert("headers".into(), to_string_value(headers));
            }
        }
    }
    Value::Object(obj)
}

fn to_string_value(map: &BTreeMap<String, String>) -> Value {
    Value::Object(
        map.iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect(),
    )
}

/// 只读读取 JSONL 文件头部（≤256KB、最多 `max_lines` 行），逐行解析为 JSON，
/// 坏行跳过。用于会话摘要提取——绝不动文件本身（红线①）。
pub(crate) fn read_jsonl_head(path: &std::path::Path, max_lines: usize) -> Vec<Value> {
    use std::io::{BufRead, Read};

    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut reader = std::io::BufReader::new(file.take(256 * 1024));
    let mut out = Vec::new();
    let mut line = String::new();
    for _ in 0..max_lines {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if let Ok(v) = serde_json::from_str::<Value>(line.trim()) {
            out.push(v);
        }
    }
    out
}

/// RFC3339 时间戳 → Unix 毫秒（会话消息时间；解析失败返回 None）。
pub(crate) fn iso_to_ms(ts: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|t| t.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_object_creates_nested() {
        let mut root = Value::Object(Map::new());
        {
            let inner = ensure_object(&mut root, &["mcp", "servers"]).unwrap();
            inner.insert("x".into(), Value::String("y".into()));
        }
        assert_eq!(root["mcp"]["servers"]["x"], Value::String("y".into()));
    }

    #[test]
    fn entry_to_transport_prefers_command_over_url() {
        let entry: Value = serde_json::json!({"command": "npx", "url": "http://x"});
        match entry_to_transport(&entry) {
            Some(Transport::Stdio { command, .. }) => assert_eq!(command, "npx"),
            _ => panic!("expected stdio"),
        }
    }

    #[test]
    fn render_preserves_unknown_entry_fields() {
        let current = r#"{"mcpServers":{"a":{"command":"c1","custom":{"keep":true}}}}"#;
        let servers = vec![McpServer {
            name: "a".into(),
            transport: Transport::Stdio {
                command: "c2".into(),
                args: vec![],
                env: BTreeMap::new(),
            },
            enabled: true,
            disabled_tools: Vec::new(),
        }];
        let out = render_servers(
            Some(current),
            &servers,
            &["mcpServers"],
            Path::new("x.json"),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["mcpServers"]["a"]["command"], "c2");
        assert_eq!(v["mcpServers"]["a"]["custom"]["keep"], true);
    }
}
