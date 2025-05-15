-- src/db/migrations/0002_update_fts_and_triggers.sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;  -- Use WAL for better concurrency

-- Drop old FTS5 triggers so we can fully replace the row on tag/attr changes
DROP TRIGGER IF EXISTS file_tags_fts_ai;
DROP TRIGGER IF EXISTS file_tags_fts_ad;
DROP TRIGGER IF EXISTS attributes_fts_ai;
DROP TRIGGER IF EXISTS attributes_fts_au;
DROP TRIGGER IF EXISTS attributes_fts_ad;

-- Recreate triggers with INSERT OR REPLACE to ensure full reindex:

CREATE TRIGGER file_tags_fts_ai
AFTER INSERT ON file_tags
BEGIN
  INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
    SELECT f.id, f.path,
      (SELECT IFNULL(GROUP_CONCAT(t.name, ' '), '')
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
        WHERE ft.file_id = f.id),
      (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
         FROM attributes a
        WHERE a.file_id = f.id)
    FROM files f
   WHERE f.id = NEW.file_id;
END;

CREATE TRIGGER file_tags_fts_ad
AFTER DELETE ON file_tags
BEGIN
  INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
    SELECT f.id, f.path,
      (SELECT IFNULL(GROUP_CONCAT(t.name, ' '), '')
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
        WHERE ft.file_id = f.id),
      (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
         FROM attributes a
        WHERE a.file_id = f.id)
    FROM files f
   WHERE f.id = OLD.file_id;
END;

CREATE TRIGGER attributes_fts_ai
AFTER INSERT ON attributes
BEGIN
  INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
    SELECT f.id, f.path,
      (SELECT IFNULL(GROUP_CONCAT(t.name, ' '), '')
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
        WHERE ft.file_id = f.id),
      (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
         FROM attributes a
        WHERE a.file_id = f.id)
    FROM files f
   WHERE f.id = NEW.file_id;
END;

CREATE TRIGGER attributes_fts_au
AFTER UPDATE OF value ON attributes
BEGIN
  INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
    SELECT f.id, f.path,
      (SELECT IFNULL(GROUP_CONCAT(t.name, ' '), '')
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
        WHERE ft.file_id = f.id),
      (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
         FROM attributes a
        WHERE a.file_id = f.id)
    FROM files f
   WHERE f.id = NEW.file_id;
END;

CREATE TRIGGER attributes_fts_ad
AFTER DELETE ON attributes
BEGIN
  INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
    SELECT f.id, f.path,
      (SELECT IFNULL(GROUP_CONCAT(t.name, ' '), '')
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
        WHERE ft.file_id = f.id),
      (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
         FROM attributes a
        WHERE a.file_id = f.id)
    FROM files f
   WHERE f.id = OLD.file_id;
END;
