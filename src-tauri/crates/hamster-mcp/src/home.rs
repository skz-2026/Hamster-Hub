//! home 工具组：iOS 主屏布局的读写（M4 第一个 agent 真实功能「自动整理主屏图标」）。
//!
//! 布局单一事实源 = 主库 settings 表 KV `home.layout`（前端 features/home/layout.ts
//! 定义模型并在读取时 normalizeLayout 清错）。本模块给 agent 三个原语：
//! 读布局 / 读应用清单（含启动次数，供放置决策）/ 写布局（结构+容量校验）。
//! 整理的智能（怎么分类、怎么建文件夹）由 agent 出——LLM 比关键词规则更懂分类。
//! 写入成功后由承载层（mcp_server.rs）广播 `hamster:layout-updated`，主屏即时刷新。
//!
//! 容量常量与前端 layout.ts 对齐（GRID 7×5 / DOCK 6）；此处做守门校验，
//! 前端 normalizeLayout 做最后兜底（剔除已卸载应用、空文件夹、重分页）。

use hamster_core::HamsterError;
use rusqlite::{params, Connection};

/// 与前端 layout.ts GRID_COLS*GRID_ROWS 对齐
pub const PAGE_CAPACITY: usize = 35;
/// 与前端 layout.ts DOCK_CAPACITY 对齐
pub const DOCK_CAPACITY: usize = 6;
/// KV 键（前端 hooks.ts LAYOUT_KEY 同源）
pub const LAYOUT_KV_KEY: &str = "home.layout";

/// 一枚应用（清单输出：供 agent 决定归类与摆放）
pub struct HomeAppInfo {
    pub app_key: String,
    pub display_name: String,
    /// lnk / uwp 等（索引来源类型）
    pub kind: String,
    /// 历史启动次数（0 = 从未启动；高频应用建议靠前）
    pub use_count: i64,
}

/// 当前布局原始 JSON（未配置返回 None）
pub fn layout_get(conn: &Connection) -> Result<Option<String>, HamsterError> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [LAYOUT_KV_KEY],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(HamsterError::Other(format!("读取布局失败：{other}"))),
        })?;
    Ok(raw)
}

/// 应用清单（含启动次数）：高频在前、名称次序兜底
pub fn apps_list(conn: &Connection) -> Result<Vec<HomeAppInfo>, HamsterError> {
    let mut stmt = conn
        .prepare(
            "SELECT m.app_key, m.display_name, m.kind,
                    COALESCE((SELECT COUNT(*) FROM usage_log u
                              WHERE u.target_kind = 'app' AND u.target_key = m.app_key), 0) AS uses
             FROM app_meta m
             ORDER BY uses DESC, m.display_name COLLATE NOCASE",
        )
        .map_err(|e| HamsterError::Other(format!("应用清单查询失败：{e}")))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(HomeAppInfo {
                app_key: r.get(0)?,
                display_name: r.get(1)?,
                kind: r.get(2)?,
                use_count: r.get(3)?,
            })
        })
        .map_err(|e| HamsterError::Other(format!("应用清单查询失败：{e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| HamsterError::Other(format!("应用清单查询失败：{e}")))?;
    Ok(rows)
}

/// 校验并写入布局。校验规则（错误信息可直接指导 agent 自修正）：
/// - 根对象 + version=1 + wallpaper 字符串；
/// - pages：数组的数组；每槽 `app:` / `folder:` / `widget:` 前缀；单页 ≤35；
/// - dock：字符串数组（原始 appKey，无前缀）；≤6；
/// - folders：{ id: { name, apps: [原始 appKey] } }。
pub fn layout_set(conn: &Connection, layout_json: &str) -> Result<(), HamsterError> {
    let v: serde_json::Value = serde_json::from_str(layout_json)
        .map_err(|e| HamsterError::config_invalid(format!("布局不是合法 JSON：{e}")))?;
    let obj = v
        .as_object()
        .ok_or_else(|| HamsterError::config_invalid("布局必须是 JSON 对象"))?;
    if obj.get("version").and_then(serde_json::Value::as_i64) != Some(1) {
        return Err(HamsterError::config_invalid(
            "version 必须为 1（当前布局模型版本）",
        ));
    }
    if !obj.get("wallpaper").map(|w| w.is_string()).unwrap_or(false) {
        return Err(HamsterError::config_invalid(
            "wallpaper 必须是字符串（如 midnight / aurora / dune / hamster）",
        ));
    }
    let pages = obj
        .get("pages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| HamsterError::config_invalid("pages 必须是数组（每页一个数组）"))?;
    for (pi, page) in pages.iter().enumerate() {
        let slots = page
            .as_array()
            .ok_or_else(|| HamsterError::config_invalid(format!("pages[{pi}] 必须是数组")))?;
        if slots.len() > PAGE_CAPACITY {
            return Err(HamsterError::config_invalid(format!(
                "pages[{pi}] 有 {} 个图标，超过单页容量 {PAGE_CAPACITY}（7×5）；请分页",
                slots.len()
            )));
        }
        for (si, slot) in slots.iter().enumerate() {
            let s = slot.as_str().ok_or_else(|| {
                HamsterError::config_invalid(format!("pages[{pi}][{si}] 必须是字符串"))
            })?;
            if !(s.starts_with("app:") || s.starts_with("folder:") || s.starts_with("widget:")) {
                return Err(HamsterError::config_invalid(format!(
                    "pages[{pi}][{si}] = \"{s}\" 前缀非法：槽位必须是 app:应用key / folder:文件夹id / widget:组件类型"
                )));
            }
        }
    }
    let dock = obj
        .get("dock")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| HamsterError::config_invalid("dock 必须是数组（原始 appKey，无前缀）"))?;
    if dock.len() > DOCK_CAPACITY {
        return Err(HamsterError::config_invalid(format!(
            "dock 有 {} 个应用，超过容量 {DOCK_CAPACITY}",
            dock.len()
        )));
    }
    for (di, d) in dock.iter().enumerate() {
        if !d.is_string() {
            return Err(HamsterError::config_invalid(format!(
                "dock[{di}] 必须是字符串（原始 appKey）"
            )));
        }
    }
    let folders = obj
        .get("folders")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            HamsterError::config_invalid(
                "folders 必须是对象：{ 文件夹id: { name, apps: [appKey] } }",
            )
        })?;
    for (fid, f) in folders {
        let Some(fobj) = f.as_object() else {
            return Err(HamsterError::config_invalid(format!(
                "folders[{fid}] 必须是对象 {{ name, apps }}"
            )));
        };
        if !fobj.get("name").map(|n| n.is_string()).unwrap_or(false) {
            return Err(HamsterError::config_invalid(format!(
                "folders[{fid}].name 必须是字符串"
            )));
        }
        let Some(apps) = fobj.get("apps").and_then(serde_json::Value::as_array) else {
            return Err(HamsterError::config_invalid(format!(
                "folders[{fid}].apps 必须是字符串数组（原始 appKey）"
            )));
        };
        if apps.iter().any(|a| !a.is_string()) {
            return Err(HamsterError::config_invalid(format!(
                "folders[{fid}].apps 含非字符串项"
            )));
        }
    }

    let canonical = v.to_string();
    conn.execute(
        "INSERT INTO settings(key, value, updated_at) VALUES(?1, ?2, unixepoch())
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![LAYOUT_KV_KEY, canonical],
    )
    .map_err(|e| HamsterError::Other(format!("写入布局失败：{e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../../../migrations/0001_init.sql"))
            .unwrap();
        c.execute_batch(include_str!("../../../migrations/0003_apps_pinyin.sql"))
            .unwrap();
        c
    }

    fn seed_app(c: &Connection, key: &str, name: &str, uses: i64) {
        c.execute(
            "INSERT INTO app_meta(app_key, display_name, exec_target, kind, indexed_at, pinyin_full, pinyin_initials)
             VALUES (?1, ?2, 'x', 'lnk', 0, '', '')",
            params![key, name],
        )
        .unwrap();
        for _ in 0..uses {
            c.execute(
                "INSERT INTO usage_log(target_key, target_kind, at) VALUES (?1, 'app', 0)",
                params![key],
            )
            .unwrap();
        }
    }

    const GOOD_LAYOUT: &str = r#"{
        "version": 1,
        "wallpaper": "midnight",
        "pages": [["app:C:\\lnk\\wechat.lnk", "widget:clock"], ["folder:f1"]],
        "dock": ["C:\\lnk\\code.lnk"],
        "folders": { "f1": { "name": "社交", "apps": ["C:\\lnk\\qq.lnk"] } }
    }"#;

    #[test]
    fn layout_set_accepts_valid_and_persists() {
        let c = conn();
        layout_set(&c, GOOD_LAYOUT).unwrap();
        let stored = layout_get(&c).unwrap().unwrap();
        assert!(stored.contains("midnight"));
        // 幂等覆盖
        layout_set(&c, GOOD_LAYOUT).unwrap();
        assert_eq!(layout_get(&c).unwrap().unwrap(), stored);
    }

    #[test]
    fn layout_set_rejects_structural_errors_with_guidance() {
        let c = conn();
        for (bad, hint) in [
            ("[]", "JSON 对象"),
            (r#"{"version": 2}"#, "version 必须为 1"),
            (r#"{"version": 1}"#, "wallpaper"),
            (
                r#"{"version":1,"wallpaper":"m","pages":[["wechat"]],"dock":[],"folders":{}}"#,
                "前缀非法",
            ),
            (
                r#"{"version":1,"wallpaper":"m","pages":[["app:x"]],"dock":[1],"folders":{}}"#,
                "dock[0]",
            ),
        ] {
            let err = layout_set(&c, bad).unwrap_err();
            assert!(err.message().contains(hint), "{bad} → {}", err.message());
        }
    }

    #[test]
    fn layout_set_rejects_page_overflow() {
        let c = conn();
        let slots: Vec<String> = (0..36).map(|i| format!(r#""app:k{i}""#)).collect();
        let bad = format!(
            r#"{{"version":1,"wallpaper":"m","pages":[[{}]],"dock":[],"folders":{{}}}}"#,
            slots.join(",")
        );
        let err = layout_set(&c, &bad).unwrap_err();
        assert!(err.message().contains("35"), "{}", err.message());
    }

    #[test]
    fn apps_list_orders_by_usage_then_name() {
        let c = conn();
        seed_app(&c, "b", "哔哩", 1);
        seed_app(&c, "a", "微信", 9);
        seed_app(&c, "c", "Alpha", 0);
        let list = apps_list(&c).unwrap();
        let names: Vec<_> = list.iter().map(|a| a.display_name.as_str()).collect();
        assert_eq!(names, vec!["微信", "哔哩", "Alpha"]);
        assert_eq!(list[0].use_count, 9);
    }

    #[test]
    fn layout_get_none_when_unconfigured() {
        let c = conn();
        assert!(layout_get(&c).unwrap().is_none());
    }
}
