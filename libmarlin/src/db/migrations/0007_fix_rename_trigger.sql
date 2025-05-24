-- src/db/migrations/0007_fix_rename_trigger.sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- Recreate files_fts_au_file trigger using INSERT OR REPLACE
DROP TRIGGER IF EXISTS files_fts_au_file;
CREATE TRIGGER files_fts_au_file
AFTER UPDATE OF path ON files
BEGIN
    INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
        SELECT NEW.id,
               NEW.path,
               (SELECT IFNULL(GROUP_CONCAT(t.name, ' '), '')
                  FROM file_tags ft
                  JOIN tags t ON ft.tag_id = t.id
                 WHERE ft.file_id = NEW.id),
               (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
                  FROM attributes a
                 WHERE a.file_id = NEW.id);
END;
