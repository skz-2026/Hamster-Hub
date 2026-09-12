//! 便签与倒数日自定义项 repo（表在 0001 迁移）

use rusqlite::{params, Connection};

use crate::error::AppError;

// ===== 便签 =====

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct Note {
    pub id: u32,
    pub content: String,
    pub pinned: bool,
    pub updated_at: u32,
}

fn row_to_note(r: &rusqlite::Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get::<_, i64>(0)? as u32,
        content: r.get(1)?,
        pinned: r.get::<_, i64>(2)? != 0,
        updated_at: r.get::<_, i64>(3)? as u32,
    })
}

pub fn note_list(conn: &Connection) -> Result<Vec<Note>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, content, pinned, updated_at FROM note
         ORDER BY pinned DESC, updated_at DESC",
    )?;
    let rows = stmt
        .query_map([], row_to_note)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn note_create(conn: &Connection, content: &str) -> Result<Note, AppError> {
    conn.execute("INSERT INTO note(content) VALUES(?1)", [content])?;
    let id = conn.last_insert_rowid();
    let (content, updated_at): (String, i64) = conn.query_row(
        "SELECT content, updated_at FROM note WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(Note {
        id: id as u32,
        content,
        pinned: false,
        updated_at: updated_at as u32,
    })
}

pub fn note_update(conn: &Connection, id: u32, content: &str) -> Result<(), AppError> {
    let n = conn.execute(
        "UPDATE note SET content = ?1, updated_at = unixepoch() WHERE id = ?2",
        params![content, id],
    )?;
    if n == 0 {
        return Err(AppError::validate(format!("便签不存在: {id}")));
    }
    Ok(())
}

pub fn note_toggle_pin(conn: &Connection, id: u32, pinned: bool) -> Result<(), AppError> {
    let n = conn.execute(
        "UPDATE note SET pinned = ?1 WHERE id = ?2",
        params![pinned as i64, id],
    )?;
    if n == 0 {
        return Err(AppError::validate(format!("便签不存在: {id}")));
    }
    Ok(())
}

pub fn note_delete(conn: &Connection, id: u32) -> Result<(), AppError> {
    let n = conn.execute("DELETE FROM note WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(AppError::validate(format!("便签不存在: {id}")));
    }
    Ok(())
}

// ===== 倒数日自定义项 =====

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CountdownCustom {
    pub id: u32,
    pub title: String,
    pub target_date: String,
    pub emoji: String,
}

pub fn countdown_list_custom(conn: &Connection) -> Result<Vec<CountdownCustom>, AppError> {
    let mut stmt = conn
        .prepare("SELECT id, title, target_date, emoji FROM countdown ORDER BY target_date ASC")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(CountdownCustom {
                id: r.get::<_, i64>(0)? as u32,
                title: r.get(1)?,
                target_date: r.get(2)?,
                emoji: r
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_else(|| "🎯".into()),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn countdown_create(
    conn: &Connection,
    title: &str,
    target_date: &str,
    emoji: Option<&str>,
) -> Result<CountdownCustom, AppError> {
    chrono::NaiveDate::parse_from_str(target_date, "%Y-%m-%d")
        .map_err(|_| AppError::validate(format!("日期格式需为 YYYY-MM-DD: {target_date}")))?;
    conn.execute(
        "INSERT INTO countdown(title, target_date, emoji) VALUES(?1, ?2, ?3)",
        params![title, target_date, emoji],
    )?;
    Ok(CountdownCustom {
        id: conn.last_insert_rowid() as u32,
        title: title.into(),
        target_date: target_date.into(),
        emoji: emoji.unwrap_or("🎯").into(),
    })
}

pub fn countdown_delete(conn: &Connection, id: u32) -> Result<(), AppError> {
    let n = conn.execute("DELETE FROM countdown WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(AppError::validate(format!("倒数日不存在: {id}")));
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
    fn note_crud() {
        let c = conn();
        let n = note_create(&c, "记得回张总消息").unwrap();
        assert_eq!(n.content, "记得回张总消息");
        assert!(!n.pinned);

        note_update(&c, n.id, "已回复").unwrap();
        note_toggle_pin(&c, n.id, true).unwrap();
        let all = note_list(&c).unwrap();
        assert_eq!(all[0].content, "已回复");
        assert!(all[0].pinned);

        note_delete(&c, n.id).unwrap();
        assert!(note_list(&c).unwrap().is_empty());
        assert!(note_delete(&c, n.id).is_err());
    }

    #[test]
    fn countdown_crud_and_validation() {
        let c = conn();
        let cd = countdown_create(&c, "项目上线", "2026-10-01", Some("🚀")).unwrap();
        assert_eq!(cd.emoji, "🚀");
        assert!(countdown_list_custom(&c).unwrap().len() == 1);

        // 非法日期拒绝
        assert!(countdown_create(&c, "bad", "2026/10/01", None).is_err());

        countdown_delete(&c, cd.id).unwrap();
        assert!(countdown_list_custom(&c).unwrap().is_empty());
    }
}
