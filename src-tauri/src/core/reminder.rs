//! 待办提醒调度：后台线程扫描到点的提醒 → 系统通知 + 前端事件。
//! 独立数据库连接（WAL 并发），20s 一拍；漏扫补发（reminded_at 未标记下次仍会命中）。

use rusqlite::{params, Connection};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use tauri_specta::Event;

use crate::error::AppError;
use crate::events::TodoReminder;

/// 查询此刻应提醒的待办（与 tauri 解耦，可测）
fn due_reminds(conn: &Connection, now: i64) -> Result<Vec<(u32, String, Option<i64>)>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, content, due_at FROM todo
         WHERE done = 0 AND remind_at IS NOT NULL AND reminded_at IS NULL AND remind_at <= ?1
         ORDER BY remind_at ASC
         LIMIT 20",
    )?;
    let rows = stmt
        .query_map([now], |r| {
            Ok((
                r.get::<_, i64>(0)? as u32,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// 常驻调度循环（setup 时拉起）
pub fn spawn(app: AppHandle, db_path: std::path::PathBuf) {
    std::thread::spawn(move || {
        let Ok(conn) = Connection::open(&db_path) else {
            eprintln!("[reminder] 提醒线程打不开主库，调度停用");
            return;
        };
        let _ = conn.busy_timeout(std::time::Duration::from_secs(2));
        loop {
            let now = chrono::Utc::now().timestamp();
            match due_reminds(&conn, now) {
                Ok(rows) => {
                    for (id, content, due_at) in rows {
                        if let Err(e) = app
                            .notification()
                            .builder()
                            .title("待办提醒")
                            .body(&content)
                            .show()
                        {
                            eprintln!("[reminder] 系统通知失败（不影响应用内提醒）: {e}");
                        }
                        let _ = TodoReminder {
                            id,
                            content: content.clone(),
                            due_at,
                        }
                        .emit(&app);
                        if let Err(e) = conn.execute(
                            "UPDATE todo SET reminded_at = ?2 WHERE id = ?1",
                            params![id, now],
                        ) {
                            eprintln!("[reminder] 标记已提醒失败（下次会重复提醒）: {e}");
                        }
                    }
                }
                Err(e) => eprintln!("[reminder] 扫描失败: {e}"),
            }
            std::thread::sleep(std::time::Duration::from_secs(20));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .unwrap();
        c.execute_batch(include_str!("../../migrations/0004_todo_due_remind.sql"))
            .unwrap();
        c
    }

    #[test]
    fn picks_only_due_undone_unreminded() {
        let c = conn();
        let now = 1_800_000_000;
        // 命中：到点未提醒
        c.execute(
            "INSERT INTO todo(content, done, remind_at) VALUES('到点', 0, ?1)",
            [now - 10],
        )
        .unwrap();
        // 不命中：未到点
        c.execute(
            "INSERT INTO todo(content, done, remind_at) VALUES('未到点', 0, ?1)",
            [now + 100],
        )
        .unwrap();
        // 不命中：已完成
        c.execute(
            "INSERT INTO todo(content, done, remind_at) VALUES('已完成', 1, ?1)",
            [now - 10],
        )
        .unwrap();
        // 不命中：已提醒
        c.execute(
            "INSERT INTO todo(content, done, remind_at, reminded_at) VALUES('已提醒', 0, ?1, ?1)",
            [now - 10],
        )
        .unwrap();
        // 不命中：无提醒时间
        c.execute("INSERT INTO todo(content, done) VALUES('无提醒', 0)", [])
            .unwrap();

        let hits = due_reminds(&c, now).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "到点");
    }

    #[test]
    fn orders_earliest_first_and_limits() {
        let c = conn();
        let now = 1_800_000_000;
        for i in 0..25 {
            c.execute(
                "INSERT INTO todo(content, done, remind_at) VALUES(?1, 0, ?2)",
                params![format!("t{i}"), now - 100 + i],
            )
            .unwrap();
        }
        let hits = due_reminds(&c, now).unwrap();
        assert_eq!(hits.len(), 20, "单拍最多 20 条防通知风暴");
        assert_eq!(hits[0].1, "t0", "最早到点的先发");
    }
}
