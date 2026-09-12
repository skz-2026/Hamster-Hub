//! 仪表盘数据命令：天气 / 倒数日 / 最近文件 / 常用应用

use tauri::State;

use crate::core::appindex::AppEntry;
use crate::core::{countdown, weather};
use crate::error::AppError;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub async fn weather_get(state: State<'_, AppState>) -> Result<weather::WeatherNow, AppError> {
    let now = weather::now_secs();
    let (city, db_path) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::poison(e.to_string()))?;
        let s = crate::store::settings::load(&conn)?;
        let city = if s.weather.city_id.is_empty() {
            "北京".to_string()
        } else {
            s.weather.city_id.clone()
        };
        (city, state.db_path.clone())
    };
    // 阻塞段：TTL 内的新鲜缓存直接返回
    let fresh = {
        let conn = crate::store::db::open_at(&db_path)?;
        weather::cache_fresh(&conn, &city, now)
    };
    if let Some(w) = fresh {
        return Ok(w);
    }
    // 异步段：网络拉取（不持数据库连接跨 await）
    let raw = weather::fetch_live(&city).await;
    // 阻塞段：写缓存 / 过期缓存兜底
    let conn = crate::store::db::open_at(&db_path)?;
    match raw {
        Some(raw) => {
            let w = weather::parse_forecast(&city, &raw, weather::now_secs())
                .ok_or_else(|| AppError::new("NET", "天气响应解析失败"))?;
            weather::cache_write(&conn, &city, &raw)?;
            Ok(w)
        }
        None => weather::cache_stale(&conn, &city)
            .ok_or_else(|| AppError::new("NET", format!("天气获取失败且无缓存: {city}"))),
    }
}

#[tauri::command]
#[specta::specta]
pub fn weather_set_city(state: State<'_, AppState>, city: String) -> Result<(), AppError> {
    let city = city.trim();
    if city.is_empty() || city.chars().count() > 30 {
        return Err(AppError::validate("城市名不合法"));
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let mut s = crate::store::settings::load(&conn)?;
    s.weather.city_id = city.to_string();
    crate::store::settings::save(&conn, &s)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn countdown_list(
    state: State<'_, AppState>,
) -> Result<Vec<countdown::CountdownItem>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    countdown::list(&conn, chrono::Local::now().date_naive())
}

/// 最近文件（来自文件索引，按修改时间）
#[tauri::command]
#[specta::specta]
pub fn recent_files(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<crate::core::fileindex::FileHit>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    crate::core::fileindex::recent(&conn, limit.unwrap_or(6).clamp(1, 20))
}

/// 常用应用：使用频次 Top N；无使用记录时返回空（前端显示引导态）
#[tauri::command]
#[specta::specta]
pub fn top_apps(state: State<'_, AppState>, limit: Option<u32>) -> Result<Vec<AppEntry>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::poison(e.to_string()))?;
    let mut stmt = conn.prepare(
        "SELECT a.app_key, a.display_name, a.exec_target, a.kind, a.icon_path
         FROM app_meta a
         JOIN usage_log u ON u.target_kind = 'app' AND u.target_key = a.app_key
         GROUP BY a.app_key, a.display_name, a.exec_target, a.kind, a.icon_path
         ORDER BY count(*) DESC, max(u.at) DESC
         LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![limit.unwrap_or(8).clamp(1, 12)], |r| {
            Ok(AppEntry {
                app_key: r.get(0)?,
                display_name: r.get(1)?,
                exec_target: r.get(2)?,
                kind: r.get(3)?,
                icon_path: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
