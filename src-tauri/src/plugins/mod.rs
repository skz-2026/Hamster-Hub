//! UI 插件域（M4 阶段四）：用户可扩展的主屏小组件。
//!
//! 形态 = 插件目录里一个文件夹（drop-in 安装，删目录即卸载）：
//! `%APPDATA%/com.hamsterhub.app/plugins/<id>/`
//! ├── plugin.json   清单：{ id, name, version, description, author, entry }
//! └── widget.js     入口：`export default (el, ctx) => cleanup?`（前端 blob 动态 import）
//!
//! 沙箱边界（v1）：插件运行在前端 WebView 里，可用 fetch 网络 + `ctx.storage`
//! 插件私有 KV（settings 表 `plugin.<id>.<key>` 命名空间），**不开放原生 IPC**；
//! 后续 manifest 权限声明再逐项放开。JS 由用户自己放置 = 用户脚本的信任模型
//! （同油猴脚本），启用与否由「放入/删除目录」表达。

use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::error::AppError;
use crate::store;

/// 插件清单（plugin.json）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// 入口 JS 文件名（相对插件目录，仅 .js）
    pub entry: String,
}

impl Default for PluginManifest {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            version: "0.0.0".into(),
            description: String::new(),
            author: String::new(),
            entry: "widget.js".into(),
        }
    }
}

/// 插件信息（命令返回；不含代码）
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub entry: String,
    /// 入口文件完整路径（前端据此请求代码）
    pub entry_path: String,
}

pub fn plugins_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("plugins")
}

/// id / 存储键的字符白名单（防路径逃逸；entry 另限 .js 后缀）
fn safe_slug(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// 首次启动自举：写入内置示例插件（用户可直接改这些文件立即看到效果）
pub fn ensure_examples(data_dir: &Path) -> std::io::Result<()> {
    let dir = plugins_dir(data_dir);
    std::fs::create_dir_all(dir.join("hello-hamster"))?;
    std::fs::create_dir_all(dir.join("hitokoto"))?;
    std::fs::write(
        dir.join("hello-hamster/plugin.json"),
        include_str!("examples/hello-hamster/plugin.json"),
    )?;
    std::fs::write(
        dir.join("hello-hamster/widget.js"),
        include_str!("examples/hello-hamster/widget.js"),
    )?;
    std::fs::write(
        dir.join("hitokoto/plugin.json"),
        include_str!("examples/hitokoto/plugin.json"),
    )?;
    std::fs::write(
        dir.join("hitokoto/widget.js"),
        include_str!("examples/hitokoto/widget.js"),
    )?;
    Ok(())
}

fn manifest_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join("plugin.json")
}

fn read_manifest(plugin_dir: &Path) -> Result<PluginManifest, AppError> {
    let raw = std::fs::read_to_string(manifest_path(plugin_dir))
        .map_err(|e| AppError::io(format!("读取 plugin.json 失败：{e}")))?;
    let m: PluginManifest = serde_json::from_str(&raw)
        .map_err(|e| AppError::new("PLUGIN_MANIFEST", format!("清单解析失败：{e}")))?;
    if !safe_slug(&m.id, 40) {
        return Err(AppError::new(
            "PLUGIN_MANIFEST",
            format!("插件 id「{}」非法（小写字母/数字/-/_，≤40 字符）", m.id),
        ));
    }
    let dir_id = plugin_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if dir_id != m.id {
        return Err(AppError::new(
            "PLUGIN_MANIFEST",
            format!("清单 id「{}」与目录名「{dir_id}」不一致", m.id),
        ));
    }
    if !m.entry.ends_with(".js")
        || m.entry.contains('/')
        || m.entry.contains('\\')
        || m.entry.contains("..")
    {
        return Err(AppError::new(
            "PLUGIN_MANIFEST",
            format!("entry「{}」非法（仅相对路径的 .js 文件）", m.entry),
        ));
    }
    Ok(m)
}

fn plugin_dir_of(data_dir: &Path, plugin_id: &str) -> Result<PathBuf, AppError> {
    if !safe_slug(plugin_id, 40) {
        return Err(AppError::validate("插件 id 非法"));
    }
    Ok(plugins_dir(data_dir).join(plugin_id))
}

/// 扫描插件目录：返回全部合法插件（非法目录跳过并忽略，不阻塞其它插件）
pub fn scan(data_dir: &Path) -> Vec<PluginInfo> {
    let dir = plugins_dir(data_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut out = vec![];
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let pdir = entry.path();
        let Ok(m) = read_manifest(&pdir) else {
            continue;
        };
        let entry_path = pdir.join(&m.entry);
        if !entry_path.is_file() {
            continue;
        }
        out.push(PluginInfo {
            entry_path: entry_path.display().to_string(),
            entry: m.entry,
            id: m.id,
            name: m.name,
            version: m.version,
            description: m.description,
            author: m.author,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn storage_key(plugin_id: &str, key: &str) -> Result<String, AppError> {
    if !safe_slug(key, 64) {
        return Err(AppError::validate("存储键非法（小写字母/数字/-/_，≤64）"));
    }
    Ok(format!("plugin.{plugin_id}.{key}"))
}

// ===== IPC 命令 =====

/// 扫描插件列表
#[tauri::command]
#[specta::specta]
pub fn plugin_list(app: tauri::AppHandle) -> Result<Vec<PluginInfo>, AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::io(e.to_string()))?;
    Ok(scan(&data_dir))
}

/// 读取插件入口代码（前端 blob 动态 import 用）
#[tauri::command]
#[specta::specta]
pub fn plugin_read_code(
    app: tauri::AppHandle,
    plugin_id: String,
    entry: String,
) -> Result<String, AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::io(e.to_string()))?;
    let dir = plugin_dir_of(&data_dir, &plugin_id)?;
    if entry != "widget.js" || !safe_slug(entry.trim_end_matches(".js"), 40) {
        return Err(AppError::validate("入口仅支持 widget.js"));
    }
    let path = dir.join(&entry);
    // 双保险：canonicalize 后必须仍在插件目录内
    let canon = path
        .canonicalize()
        .map_err(|e| AppError::io(e.to_string()))?;
    let base = dir
        .canonicalize()
        .map_err(|e| AppError::io(e.to_string()))?;
    if !canon.starts_with(&base) {
        return Err(AppError::validate("路径逃逸"));
    }
    std::fs::read_to_string(canon).map_err(|e| AppError::io(e.to_string()))
}

/// 插件私有存储读（settings 表 `plugin.<id>.<key>` 命名空间）
#[tauri::command]
#[specta::specta]
pub fn plugin_storage_get(
    state: tauri::State<'_, crate::AppState>,
    plugin_id: String,
    key: String,
) -> Result<Option<String>, AppError> {
    let dir_key = storage_key(&plugin_id, &key)?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::settings::kv_load(&conn, &dir_key)
}

/// 插件私有存储写
#[tauri::command]
#[specta::specta]
pub fn plugin_storage_set(
    state: tauri::State<'_, crate::AppState>,
    plugin_id: String,
    key: String,
    value: String,
) -> Result<(), AppError> {
    let dir_key = storage_key(&plugin_id, &key)?;
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    store::settings::kv_save(&conn, &dir_key, &value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_slug_rules() {
        assert!(safe_slug("hello-hamster", 40));
        assert!(safe_slug("a1_b", 40));
        assert!(!safe_slug("", 40));
        assert!(!safe_slug("../evil", 40));
        assert!(!safe_slug("大写", 40));
        assert!(!safe_slug(&"x".repeat(41), 40));
    }

    #[test]
    fn storage_key_namespaces_by_plugin() {
        assert_eq!(storage_key("hello", "count").unwrap(), "plugin.hello.count");
        assert!(storage_key("hello", "../x").is_err());
    }
}
