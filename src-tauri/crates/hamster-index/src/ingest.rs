//! 摄取编排：扫描各 Adapter 的会话源 → 增量读新行 → 解析（纯函数）→ 落库。
//!
//! 游标语义（runtime-design.md §4.3）：按 (source_size, source_mtime, byte_offset,
//! line_count) 增量；size 收缩 = 文件被重写，先 reset 再全量重解析；解析失败只
//! 跳过该文件，不阻塞整体（fail-safe）。

use std::path::PathBuf;

use hamster_core::error::Result;
use hamster_core::Registry;

use crate::store::{ApplyChunk, IndexStore};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IngestReport {
    pub scanned: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: usize,
}

/// 单文件单块读取上限（追加式会话文件 tail 很大时分期摄取）。
const MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// 全量增量摄取（未变更文件近乎零成本：仅 stat + 游标比对）。
pub fn ingest_all(registry: &Registry, store: &IndexStore) -> Result<IngestReport> {
    let mut report = IngestReport::default();
    for adapter in registry.list() {
        for source in adapter.session_sources() {
            let agent = adapter.id();
            let files = walk_jsonl_recent_first(&source.dir, source.max_files);
            for path in files {
                report.scanned += 1;
                let Some((size, mtime)) = stat_ms(&path) else {
                    report.skipped += 1;
                    continue;
                };
                let path_str = path.display().to_string();
                let prev = store.source_state(agent, &path_str)?;
                let (offset_bytes, line_base) = match prev {
                    Some((ps, pm, _, _)) if ps == size && pm == mtime => {
                        report.skipped += 1;
                        continue;
                    }
                    Some((ps, _pm, po, pl)) => {
                        if size < ps {
                            // 文件被重写/轮转：清派生数据后全量重解析
                            store.reset_file(agent, &path_str)?;
                            (0, 0)
                        } else {
                            (po, pl)
                        }
                    }
                    None => (0, 0),
                };
                if size <= offset_bytes {
                    report.skipped += 1;
                    continue;
                }

                let lines = read_lines_from(&path, offset_bytes as u64, MAX_CHUNK_BYTES);
                if lines.is_empty() {
                    report.skipped += 1;
                    continue;
                }

                let Some(parse) = adapter.parse_chunk(&lines) else {
                    report.errors += 1;
                    continue;
                };

                let chunk = ApplyChunk {
                    agent,
                    source_path: &path_str,
                    source_size: size,
                    source_mtime: mtime,
                    new_byte_offset: offset_bytes + bytes_consumed(&lines),
                    new_line_base: line_base + lines.len() as i64,
                    seq_line_base: line_base,
                    parse: &parse,
                };
                store.apply_chunk(&chunk)?;
                report.updated += 1;
            }
        }
    }
    Ok(report)
}

fn stat_ms(path: &PathBuf) -> Option<(i64, i64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as i64;
    Some((meta.len() as i64, mtime))
}

/// 递归收集 *.jsonl：目录按名倒序（sessions 按日期组织，名近 = 时间近），
/// 文件按 mtime 倒序，截断至上限。
fn walk_jsonl_recent_first(dir: &PathBuf, max_files: usize) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    walk_inner(dir, &mut files, 0);
    files.truncate(max_files);
    files
}

fn walk_inner(dir: &PathBuf, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            walk_inner(&p, out, depth + 1);
        } else if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            out.push(p);
        }
    }
}

/// 从字节偏移读取最多 max_bytes 的原始行（保留行尾符；最后不完整的行不计入，
/// 其字节不计入游标——追加后自然续读）。解析器自行 trim。
fn read_lines_from(path: &PathBuf, offset: u64, max_bytes: u64) -> Vec<String> {
    use std::io::{BufRead, Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return Vec::new();
    }
    let limited = file.take(max_bytes);
    let mut reader = std::io::BufReader::new(limited);
    let mut lines = Vec::new();
    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        lines.push(std::mem::take(&mut buf));
    }
    lines
}

/// 精确字节数（含行尾符）：保证游标与文件字节严格对齐。
fn bytes_consumed(lines: &[String]) -> i64 {
    lines.iter().map(|l| l.len() as i64).sum()
}
