//! 待办 repo（表在 0001 迁移，时间戳由 DEFAULT unixepoch() 填充）

use rusqlite::{params, Connection};

use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct Todo {
    pub id: u32,
    pub content: String,
    pub done: bool,
    pub created_at: u32,
}

fn row_to_todo(r: &rusqlite::Row) -> rusqlite::Result<Todo> {
    Ok(Todo {
        id: r.get::<_, i64>(0)? as u32,
        content: r.get(1)?,
        done: r.get::<_, i64>(2)? != 0,
        created_at: r.get::<_, i64>(3)? as u32,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<Todo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, content, done, created_at FROM todo
         ORDER BY done ASC, id DESC",
    )?;
    let rows = stmt
        .query_map([], row_to_todo)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn create(conn: &Connection, content: &str) -> Result<Todo, AppError> {
    conn.execute("INSERT INTO todo(content) VALUES(?1)", [content])?;
    let id = conn.last_insert_rowid();
    let (content, created_at): (String, i64) = conn.query_row(
        "SELECT content, created_at FROM todo WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(Todo {
        id: id as u32,
        content,
        done: false,
        created_at: created_at as u32,
    })
}

pub fn toggle(conn: &Connection, id: u32, done: bool) -> Result<(), AppError> {
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
        c
    }

    #[test]
    fn crud_roundtrip() {
        let c = conn();
        let t = create(&c, "给仓鼠喂粮").unwrap();
        assert_eq!(t.content, "给仓鼠喂粮");
        assert!(!t.done);

        toggle(&c, t.id, true).unwrap();
        let all = list(&c).unwrap();
        assert!(all[0].done, "已完成的排后面但 done 生效");

        toggle(&c, t.id, false).unwrap();
        let all = list(&c).unwrap();
        assert!(!all[0].done);

        delete(&c, t.id).unwrap();
        assert!(list(&c).unwrap().is_empty());
    }

    #[test]
    fn toggle_missing_errors() {
        let c = conn();
        assert!(toggle(&c, 999, true).is_err());
        assert!(delete(&c, 999).is_err());
    }
}
