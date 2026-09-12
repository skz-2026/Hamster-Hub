//! unified diff 生成（分发预览的文本基础）。

use similar::TextDiff;

pub fn unified_diff(path: &str, old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let header_a = format!("a/{}", path);
    let header_b = format!("b/{}", path);
    diff.unified_diff()
        .context_radius(3)
        .header(header_a.as_str(), header_b.as_str())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_marks_added_and_removed_lines() {
        let d = unified_diff("f.json", "{\"a\":1}\n", "{\"a\":2}\n");
        assert!(d.contains("-{\"a\":1}"));
        assert!(d.contains("+{\"a\":2}"));
        assert!(d.contains("--- a/f.json"));
        assert!(d.contains("+++ b/f.json"));
    }

    #[test]
    fn identical_content_empty_diff_body() {
        let d = unified_diff("f.json", "x\n", "x\n");
        // 无变化：没有 +/- 行（只有 header）
        assert!(!d.lines().any(|l| l.starts_with('+') || l.starts_with('-')));
    }
}
