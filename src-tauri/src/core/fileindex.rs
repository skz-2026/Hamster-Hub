//! 文件索引：配置目录扫描 → 分类/拼音列 → file_meta(FTS5) 重建 → 前缀检索。
//! 增量 watcher（notify）提供目录变更后的防抖重扫。

use std::path::PathBuf;
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher};
use rusqlite::Connection;
use tauri_specta::Event as _;
use walkdir::WalkDir;

use crate::core::pinyin::{build_match, pinyin_cols};
use crate::error::AppError;

/// 两次全量重扫的最小间隔（防常驻应用高频写入打满 DB）
const RESCAN_COOLDOWN: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct FileHit {
    pub path: String,
    pub name: String,
    pub ext: Option<String>,
    pub kind: String,
    pub size: u32,
    pub mtime: u32,
}

/// 扩展名 → 类型（SDD §3.3）
pub fn classify(ext: &str) -> &'static str {
    match ext {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" | "heic" => "图片",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" => "视频",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "音频",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "压缩包",
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "pdf" | "txt" | "md" | "csv" | "rtf" => {
            "文档"
        }
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "java" | "c" | "cpp" | "h" | "go" | "rb"
        | "php" | "sh" | "bat" | "ps1" | "json" | "xml" | "yaml" | "yml" | "toml" => "代码",
        "ttf" | "otf" | "woff" | "woff2" => "字体",
        _ => "",
    }
}

/// "~/Desktop" → 实际用户目录；非法/缺失根跳过
fn expand_root(root: &str) -> Option<PathBuf> {
    let p = if let Some(rest) = root.strip_prefix("~/") {
        let home = std::env::var("USERPROFILE").ok()?;
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(root)
    };
    p.is_dir().then_some(p)
}

pub fn roots_from(settings_roots: &[String]) -> Vec<PathBuf> {
    settings_roots
        .iter()
        .filter_map(|r| expand_root(r))
        .collect()
}

/// 全量重建索引（单事务批量 upsert），返回收录数
pub fn scan_and_rebuild(
    conn: &Connection,
    roots: &[PathBuf],
    max_files: u32,
) -> Result<usize, AppError> {
    conn.execute("BEGIN", [])?;
    let result = (|| -> Result<usize, rusqlite::Error> {
        conn.execute("DELETE FROM file_meta", [])?;
        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO file_meta(path, name, ext, dir, size, mtime, kind, pinyin_full, pinyin_initials)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut count = 0usize;
        for root in roots {
            for entry in WalkDir::new(root)
                .max_depth(8)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !is_skipped(e.file_name()))
            {
                let Ok(entry) = entry else { continue };
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if !meta.is_file() {
                    continue;
                }
                if count >= max_files as usize {
                    break;
                }
                let path = entry.path();
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => continue,
                };
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();
                let (full, initials) = pinyin_cols(name);
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as u32)
                    .unwrap_or(0);
                stmt.execute(rusqlite::params![
                    path.to_string_lossy(),
                    name,
                    ext,
                    path.parent()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    meta.len().min(u32::MAX as u64) as u32,
                    mtime,
                    classify(&ext),
                    full,
                    initials,
                ])?;
                count += 1;
            }
        }
        Ok(count)
    })();
    match result {
        Ok(n) => {
            conn.execute("COMMIT", []).map_err(AppError::from)?;
            // 全量重扫写入量大，主动 checkpoint 截断 WAL，防止长时间运行无限膨胀
            let _ = conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", []);
            Ok(n)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(AppError::db(e.to_string()))
        }
    }
}

fn is_skipped(name: &std::ffi::OsStr) -> bool {
    let n = name.to_string_lossy();
    matches!(
        n.as_ref(),
        "node_modules" | ".git" | ".svn" | "target" | "$RECYCLE.BIN" | "System Volume Information"
    ) || n.starts_with('.')
}

/// 最近文件（按修改时间倒序）
pub fn recent(conn: &Connection, limit: u32) -> Result<Vec<FileHit>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT path, name, ext, kind, size, mtime FROM file_meta
         ORDER BY mtime DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![limit], |r| {
            Ok(FileHit {
                path: r.get(0)?,
                name: r.get(1)?,
                ext: r.get(2)?,
                kind: r.get(3)?,
                size: r.get(4)?,
                mtime: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// FTS 检索（bm25 相关度排序）
pub fn search(conn: &Connection, query: &str, limit: u32) -> Result<Vec<FileHit>, AppError> {
    let Some(match_expr) = build_match(query) else {
        return Ok(vec![]);
    };
    let mut stmt = conn.prepare(
        "SELECT f.path, f.name, f.ext, f.kind, f.size, f.mtime
         FROM file_fts JOIN file_meta f ON f.id = file_fts.rowid
         WHERE file_fts MATCH ?1
         ORDER BY bm25(file_fts), f.mtime DESC
         LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![match_expr, limit], |r| {
            Ok(FileHit {
                path: r.get(0)?,
                name: r.get(1)?,
                ext: r.get(2)?,
                kind: r.get(3)?,
                size: r.get(4)?,
                mtime: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// 后台扫描线程：完成后发事件刷新前端
pub fn spawn_scan(
    app: tauri::AppHandle,
    db_path: std::path::PathBuf,
    roots: Vec<PathBuf>,
    max_files: u32,
) {
    std::thread::spawn(move || {
        let result = Connection::open(&db_path)
            .map_err(AppError::from)
            .and_then(|c| scan_and_rebuild(&c, &roots, max_files));
        match result {
            Ok(count) => {
                let _ = crate::events::FileIndexUpdated {
                    count: count as u32,
                }
                .emit(&app);
            }
            Err(e) => eprintln!("[fileindex] 扫描失败: {e}"),
        }
    });
}

/// 目录变更 watcher：防抖 5s 后全量重扫（量级 10 万内秒级，简单可靠；
/// 单根局部重扫在量级增长后优化）。随主进程存活。
pub fn spawn_watcher(
    app: tauri::AppHandle,
    db_path: std::path::PathBuf,
    roots: Vec<PathBuf>,
    max_files: u32,
) {
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let Ok(mut watcher) = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        }) else {
            eprintln!("[fileindex] watcher 创建失败");
            return;
        };
        for root in &roots {
            if let Err(e) = watcher.watch(root, RecursiveMode::Recursive) {
                eprintln!("[fileindex] watch {} 失败: {e}", root.display());
            }
        }
        eprintln!("[fileindex] watching {} roots", roots.len());

        loop {
            match rx.recv() {
                Ok(_) => {
                    // 防抖：合并 5s 窗口内的后续事件
                    let deadline = Instant::now() + Duration::from_secs(5);
                    loop {
                        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                            Ok(_) => continue,
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                        }
                    }
                    eprintln!("[fileindex] 变更防抖结束，重扫");
                    let result = Connection::open(&db_path)
                        .map_err(AppError::from)
                        .and_then(|c| scan_and_rebuild(&c, &roots, max_files));
                    match result {
                        Ok(count) => {
                            let _ = crate::events::FileIndexUpdated {
                                count: count as u32,
                            }
                            .emit(&app);
                        }
                        Err(e) => eprintln!("[fileindex] 重扫失败: {e}"),
                    }

                    // 冷却：微信等常驻应用会持续写 watched 目录（xwechat_files 每几秒
                    // 刷 apm 指标），防抖后立刻重扫会陷入「重扫完→又触发」循环，
                    // DB 长期高压曾致 AppHang（2026-09-12 真机）。冷却期内的变更静默丢弃。
                    let cooldown_end = Instant::now() + RESCAN_COOLDOWN;
                    loop {
                        match rx
                            .recv_timeout(cooldown_end.saturating_duration_since(Instant::now()))
                        {
                            Ok(_) => continue,
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                        }
                    }
                }
                Err(_) => return, // 通道关闭 = 退出
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(concat!(
            include_str!("../../migrations/0001_init.sql"),
            "\n",
            include_str!("../../migrations/0002_fileindex.sql"),
        ))
        .expect("迁移失败（bundled SQLite 是否缺 FTS5？）");
        c
    }

    #[test]
    fn pinyin_columns() {
        let (full, init) = pinyin_cols("微信截图.png");
        assert!(full.contains("wei xin jie tu"), "分音节形态: {full}");
        assert!(full.contains("weixinjietu"), "连续形态: {full}");
        assert!(init.starts_with("wxjt"), "首字母: {init}");
        let (f2, i2) = pinyin_cols("Annual Report 2026");
        assert!(f2.contains("annual report 2026"), "连续形态: {f2}");
        assert!(i2.starts_with("an"), "首字母: {i2}");
    }

    #[test]
    fn classify_kinds() {
        assert_eq!(classify("png"), "图片");
        assert_eq!(classify("docx"), "文档");
        assert_eq!(classify("rar"), "压缩包");
        assert_eq!(classify("xyz"), "");
    }

    #[test]
    fn search_chinese_and_initials() {
        let c = conn();
        let insert = |path: &str, name: &str| {
            let (full, init) = pinyin_cols(name);
            let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
            c.execute(
                "INSERT INTO file_meta(path, name, ext, dir, size, mtime, kind, pinyin_full, pinyin_initials)
                 VALUES(?1, ?2, ?3, '', 0, 0, '', ?4, ?5)",
                rusqlite::params![path, name, ext, full, init],
            )
            .unwrap();
        };
        insert(
            r"C:\Users\t\Pictures\微信截图_0901.png",
            "微信截图_0901.png",
        );
        insert(r"C:\Users\t\Docs\年度总结.docx", "年度总结.docx");
        insert(r"C:\Users\t\Downloads\setup.exe", "setup.exe");

        // 全拼
        let hits = search(&c, "weixin", 10).unwrap();
        assert!(hits.iter().any(|h| h.name.contains("微信截图")));
        // 首字母
        let hits = search(&c, "ndzj", 10).unwrap();
        assert!(hits.iter().any(|h| h.name.contains("年度总结")));
        // 中文
        let hits = search(&c, "截图", 10).unwrap();
        assert!(hits.iter().any(|h| h.name.contains("微信截图")));
        // 英文前缀
        let hits = search(&c, "set", 10).unwrap();
        assert!(hits.iter().any(|h| h.name == "setup.exe"));
        // 空查询
        assert!(search(&c, "  ", 10).unwrap().is_empty());
    }

    #[test]
    fn scan_fixture_dir() {
        let dir = std::env::temp_dir().join("hamster-fileindex-test");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("旅游计划.xlsx"), b"x").unwrap();
        std::fs::write(dir.join("sub").join("readme.md"), b"y").unwrap();
        let c = conn();
        let n = scan_and_rebuild(&c, std::slice::from_ref(&dir), 1000).unwrap();
        assert_eq!(n, 2);
        let hits = search(&c, "lyjh", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].name.contains("旅游计划"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
