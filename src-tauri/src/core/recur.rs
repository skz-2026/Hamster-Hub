//! 循环规则：待办/日程的滚动日期计算（纯函数，无 SQL；规则串存 todo.recur 列）

use chrono::{DateTime, Datelike, Local, Months, TimeZone, Weekday};

/// 支持的规则（存库原样保留；非法值按不循环处理）
pub const RULES: &[&str] = &["daily", "weekly", "monthly", "weekdays"];

pub fn is_valid(rule: &str) -> bool {
    RULES.contains(&rule)
}

/// 从当前时刻（unixepoch 秒）滚动到下一次发生。规则未知返回 None。
///
/// 月末钳制：1月31日 monthly → 2月28日；weekdays 跳过周六周日。
pub fn next_occurrence(rule: &str, from_ts: i64) -> Option<i64> {
    let from = Local.timestamp_opt(from_ts, 0).single()?;
    let next: DateTime<Local> = match rule {
        "daily" => from + chrono::Duration::days(1),
        "weekly" => from + chrono::Duration::weeks(1),
        "monthly" => from.checked_add_months(Months::new(1))?,
        "weekdays" => {
            let mut d = from + chrono::Duration::days(1);
            while !matches!(
                d.weekday(),
                Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
            ) {
                d += chrono::Duration::days(1);
            }
            d
        }
        _ => return None,
    };
    Some(next.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, h: u32) -> i64 {
        Local
            .with_ymd_and_hms(y, m, d, h, 0, 0)
            .single()
            .unwrap()
            .timestamp()
    }

    fn expect_at(ts: i64, y: i32, m: u32, d: u32, h: u32) {
        let got = Local.timestamp_opt(ts, 0).single().unwrap();
        let want = Local.with_ymd_and_hms(y, m, d, h, 0, 0).single().unwrap();
        assert_eq!(got, want, "滚动结果不符");
    }

    #[test]
    fn daily_weekly_add_fixed_span() {
        let from = at(2026, 9, 13, 10);
        expect_at(next_occurrence("daily", from).unwrap(), 2026, 9, 14, 10);
        expect_at(next_occurrence("weekly", from).unwrap(), 2026, 9, 20, 10);
    }

    #[test]
    fn monthly_clamps_to_month_end() {
        // 1月31日 monthly → 2月28日（2026 非闰年），时刻不变
        let from = at(2026, 1, 31, 9);
        expect_at(next_occurrence("monthly", from).unwrap(), 2026, 2, 28, 9);
    }

    #[test]
    fn weekdays_skips_weekend() {
        // 2026-09-11 是周五：weekdays → 下周一 09-14
        let friday = at(2026, 9, 11, 15);
        expect_at(
            next_occurrence("weekdays", friday).unwrap(),
            2026,
            9,
            14,
            15,
        );
        // 周六 → 周一
        let saturday = at(2026, 9, 12, 8);
        expect_at(
            next_occurrence("weekdays", saturday).unwrap(),
            2026,
            9,
            14,
            8,
        );
    }

    #[test]
    fn unknown_rule_and_garbage_ts() {
        assert_eq!(next_occurrence("never", at(2026, 9, 13, 10)), None);
        assert_eq!(next_occurrence("daily", i64::MAX), None);
    }
}
