-- 密码箱（字段级加密）：title/username/url 明文供 SQL 搜索；
-- secret = 信封加密（0x01|nonce|AES-256-GCM 密文）的 JSON {password, notes}。
-- 主密钥由主密码经 Argon2id 派生，仅存内存（vault_meta 只存 KDF 参数与校验密文）。
CREATE TABLE vault_meta (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  kdf_algo TEXT NOT NULL,
  kdf_params TEXT NOT NULL,
  verifier BLOB NOT NULL,
  hint TEXT,
  auto_lock_secs INTEGER NOT NULL DEFAULT 300
);

CREATE TABLE vault_item (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  username TEXT NOT NULL DEFAULT '',
  url TEXT NOT NULL DEFAULT '',
  secret BLOB NOT NULL,
  favorite INTEGER NOT NULL DEFAULT 0,
  -- 预留 M2（标签筛选/强度面板），M1 不读写
  tags TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  password_updated_at INTEGER NOT NULL DEFAULT 0,
  -- 软删除（回收站 M2 展示；数据保留可恢复）
  deleted_at INTEGER
);

CREATE INDEX idx_vault_item_sort ON vault_item(deleted_at, favorite DESC, updated_at DESC);
