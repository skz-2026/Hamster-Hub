//! 应用索引：扫描开始菜单 .lnk → 解析目标 → 提取图标缓存 → upsert app_meta
//! （UWP 枚举与 frecency 排序在 M2 fileindex 阶段一并接入）

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use tauri_specta::Event as _;
use walkdir::WalkDir;

use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct AppEntry {
    pub app_key: String,
    pub display_name: String,
    /// 启动目标：.lnk 完整路径（ShellExecute 直接打开）
    pub exec_target: String,
    pub kind: String,
    /// 图标 PNG 缓存路径（None = 前端用字母占位）
    pub icon_path: Option<String>,
}

/// 明显不是「应用」的入口名（卸载器/帮助/自述）
pub fn is_skippable(display_name: &str) -> bool {
    const TOKENS: &[&str] = &["uninstall", "unins", "卸载", "help", "readme", "说明"];
    let lower = display_name.to_lowercase();
    TOKENS.iter().any(|t| lower.contains(t))
}

fn start_menu_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(pd) = std::env::var_os("ProgramData") {
        dirs.push(PathBuf::from(pd).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(ad) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(ad).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    dirs
}

/// 扫描并重建 app_meta（含图标缓存提取）
pub fn scan_and_rebuild(conn: &Connection, icons_dir: &Path) -> Result<usize, AppError> {
    let mut entries: Vec<AppEntry> = Vec::new();
    for dir in start_menu_dirs() {
        for item in WalkDir::new(&dir)
            .max_depth(6)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = item.path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| !e.eq_ignore_ascii_case("lnk"))
                .unwrap_or(true)
            {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if is_skippable(stem) {
                continue;
            }
            if let Some(entry) = build_entry(path, stem, icons_dir) {
                // 同名去重：保留 ProgramData（系统级）优先出现的
                if !entries.iter().any(|e| e.app_key == entry.app_key) {
                    entries.push(entry);
                }
            }
        }
    }
    upsert_all(conn, &entries)?;
    Ok(entries.len())
}

fn build_entry(lnk_path: &Path, stem: &str, icons_dir: &Path) -> Option<AppEntry> {
    let app_key = lnk_path.to_string_lossy().to_lowercase();

    let parsed = lnk::ShellLink::open(lnk_path).ok();
    let target: Option<PathBuf> = parsed
        .as_ref()
        .and_then(|l| l.relative_path().clone())
        .map(PathBuf::from)
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
                && p.exists()
        });

    // 图标：目标 exe 提取；失败回退 lnk 声明的 icon_location；再失败对 .lnk 本身
    // SHGetFileInfo（自动解析 shell32.dll,-23 这类带资源索引的声明）；仍失败交前端占位
    let mut icon_path: Option<String> = None;
    let mut hasher = DefaultHasher::new();
    app_key.hash(&mut hasher);
    let png = icons_dir.join(format!("{:016x}.png", hasher.finish()));
    if !png.exists() {
        std::fs::create_dir_all(icons_dir).ok()?;
        let source = target.clone().or_else(|| {
            parsed
                .as_ref()
                .and_then(|l| l.icon_location().clone())
                .map(PathBuf::from)
                .filter(|p| p.exists())
        });
        if let Some(src) = source {
            let _ = hamster_platform::icons::extract_icon_png(&src, &png);
        }
        if !png.exists() {
            let _ = hamster_platform::icons::extract_icon_png(lnk_path, &png);
        }
    }
    if png.exists() {
        icon_path = Some(png.to_string_lossy().into_owned());
    }

    Some(AppEntry {
        app_key,
        display_name: stem.to_string(),
        exec_target: lnk_path.to_string_lossy().into_owned(),
        kind: "lnk".into(),
        icon_path,
    })
}

fn upsert_all(conn: &Connection, entries: &[AppEntry]) -> Result<(), AppError> {
    conn.execute("BEGIN", [])?;
    let result = (|| -> Result<(), rusqlite::Error> {
        conn.execute("DELETE FROM app_meta", [])?;
        let mut stmt = conn.prepare(
            "INSERT INTO app_meta(app_key, display_name, exec_target, kind, icon_path, pinyin_full, pinyin_initials, indexed_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())",
        )?;
        for e in entries {
            let (full, initials) = crate::core::pinyin::pinyin_cols(&e.display_name);
            stmt.execute(rusqlite::params![
                e.app_key,
                e.display_name,
                e.exec_target,
                e.kind,
                e.icon_path,
                full,
                initials
            ])?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            conn.execute("COMMIT", []).map_err(AppError::from)?;
            let _ = conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", []);
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(AppError::db(e.to_string()))
        }
    }
}

pub fn list(conn: &Connection) -> Result<Vec<AppEntry>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT app_key, display_name, exec_target, kind, icon_path FROM app_meta
         ORDER BY display_name COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(AppEntry {
                app_key: r.get(0)?,
                display_name: r.get(1)?,
                exec_target: r.get(2)?,
                kind: r.get(3)?,
                icon_path: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// FTS 拼音/首字母/名称检索（bm25 排序 + 使用频次加权）
pub fn search(conn: &Connection, query: &str, limit: u32) -> Result<Vec<AppEntry>, AppError> {
    let Some(match_expr) = crate::core::pinyin::build_match(query) else {
        return Ok(vec![]);
    };
    let mut stmt = conn.prepare(
        "SELECT a.app_key, a.display_name, a.exec_target, a.kind, a.icon_path,
                (SELECT count(*) FROM usage_log u WHERE u.target_kind='app' AND u.target_key = a.app_key) AS uses
         FROM app_fts JOIN app_meta a ON a.rowid = app_fts.rowid
         WHERE app_fts MATCH ?1
         ORDER BY uses DESC, bm25(app_fts)
         LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![match_expr, limit.clamp(1, 20)], |r| {
            Ok(AppEntry {
                app_key: r.get(0)?,
                display_name: r.get(1)?,
                exec_target: r.get(2)?,
                kind: r.get(3)?,
                icon_path: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn log_usage(conn: &Connection, app_key: &str) {
    let _ = conn.execute(
        "INSERT INTO usage_log(target_key, target_kind, at) VALUES(?1, 'app', unixepoch())",
        [app_key],
    );
}

/// 后台扫描线程：完成后发事件刷新前端
pub fn spawn_scan(app: tauri::AppHandle, db_path: PathBuf, icons_dir: PathBuf) {
    std::thread::spawn(move || {
        // 与主连接/文件索引扫描并发时可能撞 WAL 写锁：busy_timeout + 重试
        let mut attempt = 0;
        let result = loop {
            attempt += 1;
            match Connection::open(&db_path)
                .map_err(AppError::from)
                .and_then(|c| {
                    c.busy_timeout(std::time::Duration::from_secs(3))
                        .map_err(AppError::from)?;
                    scan_and_rebuild(&c, &icons_dir)
                }) {
                Ok(n) => break Ok(n),
                Err(e) if attempt < 3 && e.to_string().contains("locked") => {
                    std::thread::sleep(std::time::Duration::from_millis(500 * attempt as u64));
                }
                Err(e) => break Err(e),
            }
        };
        match result {
            Ok(count) => {
                let _ = crate::events::AppIndexUpdated {
                    count: count as u32,
                }
                .emit(&app);
            }
            Err(e) => eprintln!("[appindex] 扫描失败: {e}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skippable_names() {
        assert!(is_skippable("Uninstall WeChat"));
        assert!(is_skippable("微信卸载程序"));
        assert!(!is_skippable("微信"));
        assert!(!is_skippable("Visual Studio Code"));
    }

    #[test]
    fn search_by_pinyin_and_initials() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(concat!(
            include_str!("../../migrations/0001_init.sql"),
            "\n",
            include_str!("../../migrations/0003_apps_pinyin.sql"),
        ))
        .unwrap();
        let entries = vec![
            AppEntry {
                app_key: "k1".into(),
                display_name: "微信".into(),
                exec_target: "C:\\w.lnk".into(),
                kind: "lnk".into(),
                icon_path: None,
            },
            AppEntry {
                app_key: "k2".into(),
                display_name: "Visual Studio Code".into(),
                exec_target: "C:\\v.lnk".into(),
                kind: "lnk".into(),
                icon_path: None,
            },
        ];
        upsert_all(&c, &entries).unwrap();

        // 中文
        let hits = search(&c, "微信", 10).unwrap();
        assert!(hits.iter().any(|h| h.display_name == "微信"));
        // 全拼
        assert!(search(&c, "weixin", 10)
            .unwrap()
            .iter()
            .any(|h| h.display_name == "微信"));
        // 首字母
        assert!(search(&c, "vsc", 10)
            .unwrap()
            .iter()
            .any(|h| h.display_name.contains("Visual")));
        // 使用频次优先
        log_usage(&c, "k1");
        log_usage(&c, "k1");
        let hits = search(&c, "wx", 10).unwrap();
        assert_eq!(hits.first().map(|h| h.display_name.as_str()), Some("微信"));
    }
}
