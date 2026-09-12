//! 原子写工具：temp + rename，坏配置不落盘。
//!
//! 红线（AGENTS.md §0.6）：所有对外部配置文件的写入都必须经过这里。
//! Windows 上 `std::fs::rename` 使用 MOVEFILE_REPLACE_EXISTING，可原子覆盖已存在文件。

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::error::{HamsterError, Result};

/// 读文本文件（UTF-8）。文件不存在返回 Ok(None)。
pub fn read_text(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(HamsterError::io(path, e)),
    }
}

/// 原子写文本：先创建父目录 → 同目录临时文件 → rename 覆盖。
pub fn write_text_atomic(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        HamsterError::config_invalid(format!("路径没有父目录：{}", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|e| HamsterError::io(parent, e))?;

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "hamster-atomic".to_string());
    let tmp = tempfile::Builder::new()
        .prefix(&format!(".{}.上游-tmp-", file_name))
        .rand_bytes(6)
        .tempfile_in(parent)
        .map_err(|e| HamsterError::io(parent, e))?;

    {
        let mut handle = tmp
            .as_file()
            .try_clone()
            .map_err(|e| HamsterError::io(path, e))?;
        handle
            .write_all(content.as_bytes())
            .map_err(|e| HamsterError::io(path, e))?;
        handle.sync_all().map_err(|e| HamsterError::io(path, e))?;
    }
    // persist = rename 到目标路径；Windows 下等价于覆盖式移动
    tmp.persist(path)
        .map_err(|e| HamsterError::io(path, e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_creates_parents_and_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/config.json");
        write_text_atomic(&path, "{\"x\":1}").unwrap();
        assert_eq!(read_text(&path).unwrap().unwrap(), "{\"x\":1}");
    }

    #[test]
    fn write_overwrites_atomically_no_tmp_left() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.toml");
        write_text_atomic(&path, "v1").unwrap();
        write_text_atomic(&path, "v2").unwrap();
        assert_eq!(read_text(&path).unwrap().unwrap(), "v2");
        // 目录里只剩目标文件本身
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn read_missing_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_text(&dir.path().join("nope.json")).unwrap().is_none());
    }
}
