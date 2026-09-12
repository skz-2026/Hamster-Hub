//! 天气：Open-Meteo 免 key（地理编码 + 实况/今日温度），weather_cache 表缓存 30 分钟。
//! 设置里 weather.city_id 存城市名（如「北京」），经 geocoding API 换坐标。
//! 缓存存原始响应 JSON，读取时统一 parse_forecast 还原（天气码→文案）。

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct WeatherNow {
    pub city: String,
    pub temp: i32,
    pub kind: String,
    pub emoji: String,
    pub temp_min: i32,
    pub temp_max: i32,
    /// 缓存时间（unix 秒）；0 = 实时
    pub fetched_at: u32,
}

/// WMO weather code → (文案, emoji)
pub fn describe_code(code: i64) -> (&'static str, &'static str) {
    match code {
        0 => ("晴", "☀️"),
        1 => ("晴间多云", "🌤️"),
        2 => ("多云", "⛅"),
        3 => ("阴", "☁️"),
        45 | 48 => ("雾", "🌫️"),
        51..=57 => ("毛毛雨", "🌦️"),
        61..=67 | 80..=82 => ("雨", "🌧️"),
        71..=77 | 85 | 86 => ("雪", "🌨️"),
        95..=99 => ("雷阵雨", "⛈️"),
        _ => ("未知", "🌡️"),
    }
}

pub const CACHE_TTL: u32 = 30 * 60;

pub fn now_secs() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

/// 解析 Open-Meteo forecast 原始响应（纯函数，可测）
pub fn parse_forecast(city: &str, v: &Value, fetched_at: u32) -> Option<WeatherNow> {
    let current = v.get("current")?;
    let daily = v.get("daily")?;
    let temp = current.get("temperature_2m")?.as_f64()? as i32;
    let code = current.get("weather_code")?.as_i64()?;
    let (kind, emoji) = describe_code(code);
    let temp_min = daily.get("temperature_2m_min")?.get(0)?.as_f64()? as i32;
    let temp_max = daily.get("temperature_2m_max")?.get(0)?.as_f64()? as i32;
    Some(WeatherNow {
        city: city.to_string(),
        temp,
        kind: kind.into(),
        emoji: emoji.into(),
        temp_min,
        temp_max,
        fetched_at,
    })
}

fn cache_read(conn: &Connection, city: &str) -> Option<(Value, u32)> {
    let (payload, at): (String, i64) = conn
        .query_row(
            "SELECT payload, fetched_at FROM weather_cache WHERE city_id = ?1",
            [city],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .ok()??;
    let raw: Value = serde_json::from_str(&payload).ok()?;
    Some((raw, at as u32))
}

/// 新鲜缓存（TTL 内）；供 async 命令在 await 前调用
pub fn cache_fresh(conn: &Connection, city: &str, now: u32) -> Option<WeatherNow> {
    let (raw, at) = cache_read(conn, city)?;
    (now.saturating_sub(at) < CACHE_TTL).then_some(())?;
    parse_forecast(city, &raw, at)
}

/// 过期缓存兜底；供 async 命令在 await 后调用
pub fn cache_stale(conn: &Connection, city: &str) -> Option<WeatherNow> {
    let (raw, at) = cache_read(conn, city)?;
    parse_forecast(city, &raw, at)
}

pub fn cache_write(conn: &Connection, city: &str, raw: &Value) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO weather_cache(city_id, payload, fetched_at) VALUES(?1, ?2, unixepoch())
         ON CONFLICT(city_id) DO UPDATE SET payload = excluded.payload, fetched_at = excluded.fetched_at",
        params![city, serde_json::to_string(raw)?],
    )?;
    Ok(())
}

/// 实时拉取原始响应（网络 + 解析无关数据库，可跨 await）
pub async fn fetch_live(city: &str) -> Option<Value> {
    let client = reqwest::Client::new();
    let geo: Value = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[("name", city), ("count", "1"), ("language", "zh")])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let lat = geo.pointer("/results/0/latitude")?.as_f64()?;
    let lon = geo.pointer("/results/0/longitude")?.as_f64()?;

    client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", lat.to_string().as_str()),
            ("longitude", lon.to_string().as_str()),
            ("current", "temperature_2m,weather_code"),
            ("daily", "temperature_2m_max,temperature_2m_min"),
            ("timezone", "auto"),
            ("forecast_days", "1"),
        ])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()
}
