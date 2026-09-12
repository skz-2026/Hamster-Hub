//! 倒数日：数据库自定义事件 + 自动计算项（距离周末 / 跨年）

use chrono::Datelike;
use rusqlite::Connection;

use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CountdownItem {
    pub title: String,
    pub days: u32,
    /// auto = 内置计算项；custom = 用户自建
    pub kind: String,
    pub emoji: String,
}

fn item(title: &str, days: i64, kind: &str, emoji: &str) -> Option<CountdownItem> {
    (days >= 0).then(|| CountdownItem {
        title: title.into(),
        days: days as u32,
        kind: kind.into(),
        emoji: emoji.into(),
    })
}

/// 自动项：距离周末（下一个周六；周六当天=0）、距离新年（1月1日）
pub fn auto_items(today: chrono::NaiveDate) -> Vec<CountdownItem> {
    let mut out = Vec::new();
    let wd = today.weekday().num_days_from_sunday(); // 周日=0 … 周六=6
    let to_saturday = (6 + 7 - wd) % 7;
    if let Some(it) = item("距离周末", to_saturday as i64, "auto", "🛋️") {
        out.push(it);
    }
    let next_new_year = chrono::NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap();
    if let Some(it) = item("新年", (next_new_year - today).num_days(), "auto", "🎊") {
        out.push(it);
    }
    out
}

/// 全量：自动项 + 数据库自定义项（未来日期）
pub fn list(conn: &Connection, today: chrono::NaiveDate) -> Result<Vec<CountdownItem>, AppError> {
    let mut out = auto_items(today);
    let mut stmt =
        conn.prepare("SELECT title, target_date, emoji FROM countdown ORDER BY target_date ASC")?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (title, date, emoji) in rows {
        if let Ok(target) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
            let days = (target - today).num_days();
            if days >= 0 {
                out.push(CountdownItem {
                    title,
                    days: days as u32,
                    kind: "custom".into(),
                    emoji: emoji.unwrap_or_else(|| "🎯".into()),
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_items_friday() {
        // 2026-09-11 是周五：距离周末 1 天，新年 112 天
        let d = chrono::NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        let items = auto_items(d);
        assert_eq!(items[0].title, "距离周末");
        assert_eq!(items[0].days, 1);
        assert_eq!(items[1].title, "新年");
        assert_eq!(items[1].days, 112);
    }

    #[test]
    fn auto_items_saturday_is_zero() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let items = auto_items(d);
        assert_eq!(items[0].days, 0, "周六当天周末已到");
    }
}
