//! 待办 repo（0001 建表；0004 加 due_at/remind_at/recur，时间均为 unixepoch 秒）

use rusqlite::{params, Connection};

use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct Todo {
    pub id: u32,
    pub content: String,
    pub done: bool,
    pub created_at: u32,
    /// 截止时刻（NULL = 无；日期任务取当日 00:00）
    pub due_at: Option<i64>,
    /// 提醒时刻（NULL = 不提醒）
    pub remind_at: Option<i64>,
    /// 已提醒时刻（NULL = 未提醒）
    pub reminded_at: Option<i64>,
    /// 循环规则（core::recur::RULES；NULL = 不循环）
    pub recur: Option<String>,
}

const COLS: &str = "id, content, done, created_at, due_at, remind_at, reminded_at, recur";

fn row_to_todo(r: &rusqlite::Row) -> rusqlite::Result<Todo> {
    Ok(Todo {
        id: r.get::<_, i64>(0)? as u32,
        content: r.get(1)?,
        done: r.get::<_, i64>(2)? != 0,
        created_at: r.get::<_, i64>(3)? as u32,
        due_at: r.get(4)?,
        remind_at: r.get(5)?,
        reminded_at: r.get(6)?,
        recur: r.get(7)?,
    })
}

/// remind=true 且有截止时间时，提醒时刻 = 截止时刻
fn resolve_remind(due_at: Option<i64>, remind: bool) -> Option<i64> {
    remind.then_some(due_at).flatten()
}

fn get(conn: &Connection, id: u32) -> Result<Todo, AppError> {
    let sql = format!("SELECT {COLS} FROM todo WHERE id = ?1");
    conn.query_row(&sql, [id], row_to_todo)
        .map_err(|_| AppError::validate(format!("待办不存在: {id}")))
}

pub fn list(conn: &Connection) -> Result<Vec<Todo>, AppError> {
    let sql = format!(
        "SELECT {COLS} FROM todo
         ORDER BY done ASC,
                  (due_at IS NULL) ASC, due_at ASC,
                  id DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], row_to_todo)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn create(
    conn: &Connection,
    content: &str,
    due_at: Option<i64>,
    remind: bool,
) -> Result<Todo, AppError> {
    conn.execute(
        "INSERT INTO todo(content, due_at, remind_at) VALUES(?1, ?2, ?3)",
        params![content, due_at, resolve_remind(due_at, remind)],
    )?;
    get(conn, conn.last_insert_rowid() as u32)
}

/// 设置/清除截止时间；重置提醒状态（改动后重新提醒一次）
pub fn set_due(
    conn: &Connection,
    id: u32,
    due_at: Option<i64>,
    remind: bool,
) -> Result<Todo, AppError> {
    let n = conn.execute(
        "UPDATE todo SET due_at = ?2, remind_at = ?3, reminded_at = NULL WHERE id = ?1",
        params![id, due_at, resolve_remind(due_at, remind)],
    )?;
    if n == 0 {
        return Err(AppError::validate(format!("待办不存在: {id}")));
    }
    get(conn, id)
}

pub fn set_recur(conn: &Connection, id: u32, recur: Option<&str>) -> Result<Todo, AppError> {
    let n = conn.execute(
        "UPDATE todo SET recur = ?2 WHERE id = ?1",
        params![id, recur],
    )?;
    if n == 0 {
        return Err(AppError::validate(format!("待办不存在: {id}")));
    }
    get(conn, id)
}

/// 完成或撤销完成。循环待办完成时滚动到下一次（保持未完成、重置提醒），而非终结。
pub fn set_done(conn: &Connection, id: u32, done: bool, now: i64) -> Result<(), AppError> {
    if done {
        let (recur, due_at, remind_at): (Option<String>, Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT recur, due_at, remind_at FROM todo WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| AppError::validate(format!("待办不存在: {id}")))?;
        if let Some(rule) = recur.as_deref() {
            let base = due_at.map_or(now, |d| d.max(now));
            if let Some(next) = crate::core::recur::next_occurrence(rule, base) {
                let remind_at = remind_at.map(|_| next);
                conn.execute(
                    "UPDATE todo SET done = 0, completed_at = NULL,
                     due_at = ?2, remind_at = ?3, reminded_at = NULL WHERE id = ?1",
                    params![id, next, remind_at],
                )?;
                return Ok(());
            }
        }
    }
    let n = conn.execute(
        "UPDATE todo SET done = ?1, completed_at = CASE WHEN ?1 THEN unixepoch() ELSE NULL END WHERE id = ?2",
        params![done as i64, id],
    )?;
    if n == 0 {
        return Err(AppError::validate(format!("待办不存在: {id}")));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: u32) -> Result<(), AppError> {
    let n = conn.execute("DELETE FROM todo WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(AppError::validate(format!("待办不存在: {id}")));
    }
    Ok(())
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
    fn crud_roundtrip() {
        let c = conn();
        let t = create(&c, "给仓鼠喂粮", None, false).unwrap();
        assert_eq!(t.content, "给仓鼠喂粮");
        assert!(!t.done);
        assert_eq!(t.due_at, None);

        set_done(&c, t.id, true, 0).unwrap();
        let all = list(&c).unwrap();
        assert!(all[0].done, "已完成的排后面但 done 生效");

        set_done(&c, t.id, false, 0).unwrap();
        let all = list(&c).unwrap();
        assert!(!all[0].done);

        delete(&c, t.id).unwrap();
        assert!(list(&c).unwrap().is_empty());
    }

    #[test]
    fn create_with_due_and_remind() {
        let c = conn();
        let t = create(&c, "交房租", Some(1_800_000_000), true).unwrap();
        assert_eq!(t.due_at, Some(1_800_000_000));
        assert_eq!(
            t.remind_at,
            Some(1_800_000_000),
            "开提醒时 remind_at = due_at"
        );
        let t2 = create(&c, "不开提醒", Some(1_800_000_000), false).unwrap();
        assert_eq!(t2.remind_at, None);

        let t3 = set_due(&c, t.id, Some(1_800_100_000), true).unwrap();
        assert_eq!(t3.remind_at, Some(1_800_100_000));
        assert_eq!(t3.reminded_at, None, "改截止时间重置提醒状态");
    }

    #[test]
    fn set_due_missing_errors() {
        let c = conn();
        assert!(set_due(&c, 999, None, false).is_err());
        assert!(set_recur(&c, 999, None).is_err());
    }

    #[test]
    fn toggle_missing_errors() {
        let c = conn();
        assert!(set_done(&c, 999, true, 0).is_err());
        assert!(delete(&c, 999).is_err());
    }

    #[test]
    fn recurring_todo_rolls_forward_on_complete() {
        let c = conn();
        let now = 1_800_000_000;
        let t = create(&c, "喂仓鼠", Some(now), true).unwrap();
        set_recur(&c, t.id, Some("daily")).unwrap();

        set_done(&c, t.id, true, now).unwrap();
        let after = get(&c, t.id).unwrap();
        assert!(!after.done, "循环待办完成 = 滚到下一次，不终结");
        assert_eq!(after.due_at, Some(now + 86_400));
        assert_eq!(after.remind_at, Some(now + 86_400), "提醒随下一次滚动");
        assert_eq!(after.reminded_at, None, "下一轮重新提醒");

        // 非循环待办照常终结
        let t2 = create(&c, "一次性", None, false).unwrap();
        set_done(&c, t2.id, true, now).unwrap();
        assert!(get(&c, t2.id).unwrap().done);
    }

    #[test]
    fn list_orders_undone_by_due_first() {
        let c = conn();
        let later = create(&c, "后", Some(1_800_000_000), false).unwrap();
        let earlier = create(&c, "前", Some(1_700_000_000), false).unwrap();
        create(&c, "无截止", None, false).unwrap();
        let all = list(&c).unwrap();
        assert_eq!(all[0].id, earlier.id);
        assert_eq!(all[1].id, later.id);
        assert_eq!(all[2].content, "无截止");
    }
}
