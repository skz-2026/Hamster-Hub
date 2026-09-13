-- 0004_todo_due_remind: 待办截止时间、提醒与循环（阶段化功能一次加列，全部可空向后兼容）
-- 时间均为 unixepoch 秒。due_at 为精确截止时刻（日期任务取当日 00:00 本地）；due_date(TEXT) 旧列弃用保留。

ALTER TABLE todo ADD COLUMN due_at INTEGER;
ALTER TABLE todo ADD COLUMN remind_at INTEGER;
ALTER TABLE todo ADD COLUMN reminded_at INTEGER;
ALTER TABLE todo ADD COLUMN recur TEXT;

CREATE INDEX IF NOT EXISTS idx_todo_due ON todo(done, due_at);
CREATE INDEX IF NOT EXISTS idx_todo_pending_remind ON todo(remind_at) WHERE done = 0 AND reminded_at IS NULL AND remind_at IS NOT NULL;
