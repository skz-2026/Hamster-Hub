//! 专注历史 repo（0005 迁移）：番茄钟完成/停止时落一条，按日聚合查询

use rusqlite::{params, Connection};

use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct FocusDayStat {
    /// 本地日期 "2026-09-13"
    pub day: String,
    /// 当日专注分钟（向下取整）
    pub minutes: u32,
}

/// 落一条专注记录（kind: focus/break；seconds 为净专注秒数，不含暂停）
pub fn record(
    conn: &Connection,
    todo_id: Option<u32>,
    kind: &str,
    seconds: i64,
    started_at: i64,
    ended_at: i64,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO focus_session(todo_id, kind, seconds, started_at, ended_at) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![todo_id.map(|v| v as i64), kind, seconds, started_at, ended_at],
    )?;
    Ok(())
}

/// 最近 n 天按日聚合（本地时区），旧的在前
pub fn daily_stats(conn: &Connection, days: u32) -> Result<Vec<FocusDayStat>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT date(started_at, 'unixepoch', 'localtime') AS day, SUM(seconds)
         FROM focus_session WHERE kind = 'focus'
         GROUP BY day ORDER BY day DESC LIMIT ?1",
    )?;
    let mut rows = stmt
        .query_map([days as i64], |r| {
            Ok(FocusDayStat {
                day: r.get::<_, String>(0)?,
                minutes: (r.get::<_, i64>(1)? as f64 / 60.0).floor() as u32,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.reverse();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .unwrap();
        c.execute_batch(include_str!("../../migrations/0005_focus_session.sql"))
            .unwrap();
        c
    }

    #[test]
    fn record_and_daily_stats() {
        let c = conn();
        // 2026-09-13 10:00 与 09-12 各一条（本地时区构造，机器无关）
        let mk = |d: u32| {
            Local
                .with_ymd_and_hms(2026, 9, d, 10, 0, 0)
                .single()
                .unwrap()
                .timestamp()
        };
        record(&c, Some(1), "focus", 1500, mk(13), mk(13) + 1500).unwrap();
        record(&c, None, "focus", 600, mk(12), mk(12) + 600).unwrap();
        record(&c, None, "break", 300, mk(13), mk(13) + 300).unwrap(); // break 不计入

        let stats = daily_stats(&c, 7).unwrap();
        assert_eq!(stats.len(), 2, "break 不计入聚合");
        assert!(
            stats[1].day.ends_with("09-13"),
            "按日聚合且旧的在前: {:?}",
            stats
        );
        assert_eq!(stats[1].minutes, 25);
        assert_eq!(stats[0].minutes, 10);
    }
}
