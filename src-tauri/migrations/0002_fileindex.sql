-- 0002_fileindex: 文件索引列与 FTS5 全文检索（SDD §3.3/§4）
-- 拼音列在 0001 建表时未预埋，此处 ALTER 补齐（SQLite ALTER ADD COLUMN 带 DEFAULT 安全）。

ALTER TABLE file_meta ADD COLUMN pinyin_full TEXT NOT NULL DEFAULT '';
ALTER TABLE file_meta ADD COLUMN pinyin_initials TEXT NOT NULL DEFAULT '';

CREATE VIRTUAL TABLE IF NOT EXISTS file_fts USING fts5(
  name,
  pinyin_full,
  pinyin_initials,
  path,
  content='file_meta',
  content_rowid='id'
);

CREATE TRIGGER IF NOT EXISTS file_fts_ai AFTER INSERT ON file_meta BEGIN
  INSERT INTO file_fts(rowid, name, pinyin_full, pinyin_initials, path)
  VALUES (new.id, new.name, new.pinyin_full, new.pinyin_initials, new.path);
END;

CREATE TRIGGER IF NOT EXISTS file_fts_ad AFTER DELETE ON file_meta BEGIN
  INSERT INTO file_fts(file_fts, rowid, name, pinyin_full, pinyin_initials, path)
  VALUES ('delete', old.id, old.name, old.pinyin_full, old.pinyin_initials, old.path);
END;

CREATE TRIGGER IF NOT EXISTS file_fts_au AFTER UPDATE ON file_meta BEGIN
  INSERT INTO file_fts(file_fts, rowid, name, pinyin_full, pinyin_initials, path)
  VALUES ('delete', old.id, old.name, old.pinyin_full, old.pinyin_initials, old.path);
  INSERT INTO file_fts(rowid, name, pinyin_full, pinyin_initials, path)
  VALUES (new.id, new.name, new.pinyin_full, new.pinyin_initials, new.path);
END;
