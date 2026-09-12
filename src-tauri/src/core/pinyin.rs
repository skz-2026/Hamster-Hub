//! 拼音列生成与 FTS5 MATCH 构造（appindex / fileindex 共享）。
//! 列约定：两表 FTS 虚表列均为 (name, pinyin_full, pinyin_initials)。

use pinyin::ToPinyin;

fn is_cjk(c: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&c)
}

/// 文件/应用名 → (全拼列, 首字母列)。
/// - 中文：全拼列双形态（`wei xin jie tu` + `weixinjietu`），首字母列 = 每字一母（wxjt）
/// - ASCII：全拼列 = 原词小写连续串；首字母列额外拼上**词首字母缩写**（Visual Studio Code →
///   `visualstudiocode` + `vsc`），支持 vsc 类缩写检索
pub fn pinyin_cols(name: &str) -> (String, String) {
    let mut syllables: Vec<&'static str> = Vec::new();
    let mut concat = String::new();
    let mut initials = String::new();
    let mut ascii_word_initials = String::new();
    let mut prev_ascii = false;
    for (ch, py) in name.chars().zip(name.to_pinyin()) {
        match py {
            Some(p) => {
                syllables.push(p.plain());
                concat.push_str(p.plain());
                initials.push_str(p.first_letter());
                prev_ascii = false;
            }
            None => {
                concat.push(ch.to_ascii_lowercase());
                if ch.is_ascii_alphanumeric() {
                    initials.push(ch.to_ascii_lowercase());
                    // ASCII 词首（前一个字符不是字母数字时）
                    if !prev_ascii {
                        ascii_word_initials.push(ch.to_ascii_lowercase());
                    }
                    prev_ascii = true;
                } else {
                    prev_ascii = false;
                }
            }
        }
    }
    // 分隔符：中文音节形态与 ASCII 缩写之间
    let mut full = syllables.join(" ");
    if !full.is_empty() {
        full.push(' ');
    }
    full.push_str(&concat);
    if !ascii_word_initials.is_empty() {
        full.push(' ');
        full.push_str(&ascii_word_initials);
    }
    initials.push_str(&ascii_word_initials);
    (full, initials)
}

/// 检索词 → FTS5 前缀查询。
/// - ASCII 词保持整词（weixin → weixin*，命中连续拼音串）
/// - 中文词转拼音音节（截图 → jie* AND tu*，命中分音节形态）
/// - 首字母缩写（wxjt）直接命中首字母列
pub fn build_match(query: &str) -> Option<String> {
    fn split_word(w: &str) -> Vec<String> {
        if !w.chars().any(is_cjk) {
            return vec![w.to_string()];
        }
        w.chars()
            .zip(w.to_pinyin())
            .filter_map(|(_, py)| py.map(|p| p.plain().to_string()))
            .collect()
    }

    let toks: Vec<String> = query
        .split_whitespace()
        .flat_map(split_word)
        .filter(|t| !t.is_empty())
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|t: &String| !t.is_empty())
        .map(|t| format!("{t}*"))
        .collect();
    if toks.is_empty() {
        return None;
    }
    Some(format!(
        "{{name pinyin_full pinyin_initials}} : ({})",
        toks.join(" AND ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinyin_columns() {
        let (full, init) = pinyin_cols("微信截图.png");
        assert!(full.contains("wei xin jie tu"), "分音节形态: {full}");
        assert!(full.contains("weixinjietu"), "连续形态: {full}");
        assert!(init.starts_with("wxjt"), "首字母: {init}");
        let (f2, i2) = pinyin_cols("Visual Studio Code");
        assert!(f2.contains("visual studio code"), "连续形态: {f2}");
        assert!(f2.contains(" vsc"), "词首缩写进全拼列: {f2}");
        assert!(i2.contains("vsc"), "词首缩写进首字母列: {i2}");
        let (f3, i3) = pinyin_cols("Annual Report 2026");
        assert!(f3.contains("annual report 2026"), "连续形态: {f3}");
        assert!(i3.ends_with("ar2"), "词首缩写追加: {i3}");
    }

    #[test]
    fn match_building() {
        assert_eq!(
            build_match("weixin").as_deref(),
            Some("{name pinyin_full pinyin_initials} : (weixin*)")
        );
        assert_eq!(
            build_match("截图").as_deref(),
            Some("{name pinyin_full pinyin_initials} : (jie* AND tu*)")
        );
        assert!(build_match("  ").is_none());
    }
}
