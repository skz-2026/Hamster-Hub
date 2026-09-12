-- 0003_apps_pinyin: app_meta 拼音列 + app_fts（应用拼音/首字母检索）

ALTER TABLE app_meta ADD COLUMN pinyin_full TEXT NOT NULL DEFAULT '';
ALTER TABLE app_meta ADD COLUMN pinyin_initials TEXT NOT NULL DEFAULT '';

CREATE VIRTUAL TABLE IF NOT EXISTS app_fts USING fts5(
  name,
  pinyin_full,
  pinyin_initials,
  content='app_meta',
  content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS app_fts_ai AFTER INSERT ON app_meta BEGIN
  INSERT INTO app_fts(rowid, name, pinyin_full, pinyin_initials)
  VALUES (new.rowid, new.display_name, new.pinyin_full, new.pinyin_initials);
END;

CREATE TRIGGER IF NOT EXISTS app_fts_ad AFTER DELETE ON app_meta BEGIN
  INSERT INTO app_fts(app_fts, rowid, name, pinyin_full, pinyin_initials)
  VALUES ('delete', old.rowid, old.display_name, old.pinyin_full, old.pinyin_initials);
END;

CREATE TRIGGER IF NOT EXISTS app_fts_au AFTER UPDATE ON app_meta BEGIN
  INSERT INTO app_fts(app_fts, rowid, name, pinyin_full, pinyin_initials)
  VALUES ('delete', old.rowid, old.display_name, old.pinyin_full, old.pinyin_initials);
  INSERT INTO app_fts(rowid, name, pinyin_full, pinyin_initials)
  VALUES (new.rowid, new.display_name, new.pinyin_full, new.pinyin_initials);
END;
