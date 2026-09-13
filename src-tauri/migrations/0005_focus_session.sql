-- 0005_focus_session: 番茄钟/专注计时历史（完成或手动停止时落一条）

CREATE TABLE IF NOT EXISTS focus_session (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  todo_id    INTEGER,
  kind       TEXT NOT NULL DEFAULT 'focus',
  seconds    INTEGER NOT NULL,
  started_at INTEGER NOT NULL,
  ended_at   INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_focus_started ON focus_session(started_at);
