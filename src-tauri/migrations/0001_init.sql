-- 0001_init: 核心业务表（SDD §4 数据设计）
-- 注意：FTS5 虚拟表随 M1 fileindex 模块在 0002 中追加，避免捆绑 SQLite 无 FTS5 时整体迁移失败。

CREATE TABLE IF NOT EXISTS settings (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS todo (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  content      TEXT NOT NULL,
  done         INTEGER NOT NULL DEFAULT 0,
  due_date     TEXT,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL DEFAULT (unixepoch()),
  completed_at INTEGER
);

CREATE TABLE IF NOT EXISTS note (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  content    TEXT NOT NULL,
  color      TEXT,
  pinned     INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS countdown (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  title          TEXT NOT NULL,
  target_date    TEXT NOT NULL,
  repeat_yearly  INTEGER NOT NULL DEFAULT 0,
  emoji          TEXT,
  sort_order     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS schedule_event (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  title          TEXT NOT NULL,
  start_at       TEXT NOT NULL,
  end_at         TEXT,
  all_day        INTEGER NOT NULL DEFAULT 0,
  location       TEXT,
  color          TEXT,
  remind_minutes INTEGER,
  recurrence     TEXT
);

CREATE TABLE IF NOT EXISTS app_group (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL UNIQUE,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS app_pin (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id     INTEGER NOT NULL REFERENCES app_group(id) ON DELETE CASCADE,
  app_key      TEXT NOT NULL,
  display_name TEXT NOT NULL,
  icon_path    TEXT,
  exec_target  TEXT NOT NULL,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  UNIQUE(group_id, app_key)
);

CREATE TABLE IF NOT EXISTS app_meta (
  app_key      TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  exec_target  TEXT NOT NULL,
  kind         TEXT NOT NULL,
  icon_path    TEXT,
  indexed_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS file_meta (
  id     INTEGER PRIMARY KEY,
  path   TEXT NOT NULL UNIQUE,
  name   TEXT NOT NULL,
  ext    TEXT,
  dir    TEXT NOT NULL,
  size   INTEGER NOT NULL,
  mtime  INTEGER NOT NULL,
  kind   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS usage_log (
  target_key  TEXT NOT NULL,
  target_kind TEXT NOT NULL,
  at          INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_target ON usage_log(target_kind, target_key);

CREATE TABLE IF NOT EXISTS weather_cache (
  city_id    TEXT PRIMARY KEY,
  payload    TEXT NOT NULL,
  fetched_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS hotlist_cache (
  source     TEXT PRIMARY KEY,
  payload    TEXT NOT NULL,
  fetched_at INTEGER NOT NULL
);
