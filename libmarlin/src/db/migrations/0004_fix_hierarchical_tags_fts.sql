-- src/db/migrations/0004_fix_hierarchical_tags_fts.sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- Force drop all FTS triggers to ensure they're recreated even if migration is already recorded
DROP TRIGGER IF EXISTS files_fts_ai_file;
DROP TRIGGER IF EXISTS files_fts_au_file;
DROP TRIGGER IF EXISTS files_fts_ad_file;
DROP TRIGGER IF EXISTS file_tags_fts_ai;
DROP TRIGGER IF EXISTS file_tags_fts_ad;
DROP TRIGGER IF EXISTS attributes_fts_ai;
DROP TRIGGER IF EXISTS attributes_fts_au;
DROP TRIGGER IF EXISTS attributes_fts_ad;

-- Create a new trigger for file insertion that uses recursive CTE for full tag paths
CREATE TRIGGER files_fts_ai_file
AFTER INSERT ON files
BEGIN
    INSERT INTO files_fts(rowid, path, tags_text, attrs_text)
    VALUES (
        NEW.id,
        NEW.path,
        (SELECT IFNULL(GROUP_CONCAT(tag_path, ' '), '')
         FROM (
           WITH RECURSIVE tag_tree(id, name, parent_id, path) AS (
             SELECT t.id, t.name, t.parent_id, t.name
             FROM tags t
             WHERE t.parent_id IS NULL
             
             UNION ALL
             
             SELECT t.id, t.name, t.parent_id, tt.path || '/' || t.name
             FROM tags t
             JOIN tag_tree tt ON t.parent_id = tt.id
           )
           SELECT DISTINCT tag_tree.path AS tag_path
           FROM file_tags ft
           JOIN tag_tree ON ft.tag_id = tag_tree.id
           WHERE ft.file_id = NEW.id
           
           UNION
           
           SELECT t.name AS tag_path
           FROM file_tags ft
           JOIN tags t ON ft.tag_id = t.id
           WHERE ft.file_id = NEW.id AND t.parent_id IS NULL
         )),
        (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
           FROM attributes a
          WHERE a.file_id = NEW.id)
    );
END;

-- Recreate the file path update trigger
CREATE TRIGGER files_fts_au_file
AFTER UPDATE OF path ON files
BEGIN
    UPDATE files_fts
       SET path = NEW.path
     WHERE rowid = NEW.id;
END;

-- Recreate the file deletion trigger
CREATE TRIGGER files_fts_ad_file
AFTER DELETE ON files
BEGIN
    DELETE FROM files_fts WHERE rowid = OLD.id;
END;

-- Create new trigger for tag insertion that uses recursive CTE for full tag paths
CREATE TRIGGER file_tags_fts_ai
AFTER INSERT ON file_tags
BEGIN
  INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
    SELECT f.id, f.path,
      (SELECT IFNULL(GROUP_CONCAT(tag_path, ' '), '')
       FROM (
         WITH RECURSIVE tag_tree(id, name, parent_id, path) AS (
           SELECT t.id, t.name, t.parent_id, t.name
           FROM tags t
           WHERE t.parent_id IS NULL
           
           UNION ALL
           
           SELECT t.id, t.name, t.parent_id, tt.path || '/' || t.name
           FROM tags t
           JOIN tag_tree tt ON t.parent_id = tt.id
         )
         SELECT DISTINCT tag_tree.path AS tag_path
         FROM file_tags ft
         JOIN tag_tree ON ft.tag_id = tag_tree.id
         WHERE ft.file_id = f.id
         
         UNION
         
         SELECT t.name AS tag_path
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
         WHERE ft.file_id = f.id AND t.parent_id IS NULL
       )),
      (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
         FROM attributes a
        WHERE a.file_id = f.id)
    FROM files f
   WHERE f.id = NEW.file_id;
END;

-- Create new trigger for tag deletion that uses recursive CTE for full tag paths
CREATE TRIGGER file_tags_fts_ad
AFTER DELETE ON file_tags
BEGIN
  INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
    SELECT f.id, f.path,
      (SELECT IFNULL(GROUP_CONCAT(tag_path, ' '), '')
       FROM (
         WITH RECURSIVE tag_tree(id, name, parent_id, path) AS (
           SELECT t.id, t.name, t.parent_id, t.name
           FROM tags t
           WHERE t.parent_id IS NULL
           
           UNION ALL
           
           SELECT t.id, t.name, t.parent_id, tt.path || '/' || t.name
           FROM tags t
           JOIN tag_tree tt ON t.parent_id = tt.id
         )
         SELECT DISTINCT tag_tree.path AS tag_path
         FROM file_tags ft
         JOIN tag_tree ON ft.tag_id = tag_tree.id
         WHERE ft.file_id = f.id
         
         UNION
         
         SELECT t.name AS tag_path
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
         WHERE ft.file_id = f.id AND t.parent_id IS NULL
       )),
      (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
         FROM attributes a
        WHERE a.file_id = f.id)
    FROM files f
   WHERE f.id = OLD.file_id;
END;

-- Create new triggers for attribute operations that use recursive CTE for full tag paths
CREATE TRIGGER attributes_fts_ai
AFTER INSERT ON attributes
BEGIN
  INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
    SELECT f.id, f.path,
      (SELECT IFNULL(GROUP_CONCAT(tag_path, ' '), '')
       FROM (
         WITH RECURSIVE tag_tree(id, name, parent_id, path) AS (
           SELECT t.id, t.name, t.parent_id, t.name
           FROM tags t
           WHERE t.parent_id IS NULL
           
           UNION ALL
           
           SELECT t.id, t.name, t.parent_id, tt.path || '/' || t.name
           FROM tags t
           JOIN tag_tree tt ON t.parent_id = tt.id
         )
         SELECT DISTINCT tag_tree.path AS tag_path
         FROM file_tags ft
         JOIN tag_tree ON ft.tag_id = tag_tree.id
         WHERE ft.file_id = f.id
         
         UNION
         
         SELECT t.name AS tag_path
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
         WHERE ft.file_id = f.id AND t.parent_id IS NULL
       )),
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
      (SELECT IFNULL(GROUP_CONCAT(tag_path, ' '), '')
       FROM (
         WITH RECURSIVE tag_tree(id, name, parent_id, path) AS (
           SELECT t.id, t.name, t.parent_id, t.name
           FROM tags t
           WHERE t.parent_id IS NULL
           
           UNION ALL
           
           SELECT t.id, t.name, t.parent_id, tt.path || '/' || t.name
           FROM tags t
           JOIN tag_tree tt ON t.parent_id = tt.id
         )
         SELECT DISTINCT tag_tree.path AS tag_path
         FROM file_tags ft
         JOIN tag_tree ON ft.tag_id = tag_tree.id
         WHERE ft.file_id = f.id
         
         UNION
         
         SELECT t.name AS tag_path
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
         WHERE ft.file_id = f.id AND t.parent_id IS NULL
       )),
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
      (SELECT IFNULL(GROUP_CONCAT(tag_path, ' '), '')
       FROM (
         WITH RECURSIVE tag_tree(id, name, parent_id, path) AS (
           SELECT t.id, t.name, t.parent_id, t.name
           FROM tags t
           WHERE t.parent_id IS NULL
           
           UNION ALL
           
           SELECT t.id, t.name, t.parent_id, tt.path || '/' || t.name
           FROM tags t
           JOIN tag_tree tt ON t.parent_id = tt.id
         )
         SELECT DISTINCT tag_tree.path AS tag_path
         FROM file_tags ft
         JOIN tag_tree ON ft.tag_id = tag_tree.id
         WHERE ft.file_id = f.id
         
         UNION
         
         SELECT t.name AS tag_path
         FROM file_tags ft
         JOIN tags t ON ft.tag_id = t.id
         WHERE ft.file_id = f.id AND t.parent_id IS NULL
       )),
      (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
         FROM attributes a
        WHERE a.file_id = f.id)
    FROM files f
   WHERE f.id = OLD.file_id;
END;

-- Update all existing FTS entries with the new tag-path format
INSERT OR REPLACE INTO files_fts(rowid, path, tags_text, attrs_text)
SELECT f.id, f.path,
  (SELECT IFNULL(GROUP_CONCAT(tag_path, ' '), '')
   FROM (
     WITH RECURSIVE tag_tree(id, name, parent_id, path) AS (
       SELECT t.id, t.name, t.parent_id, t.name
       FROM tags t
       WHERE t.parent_id IS NULL
       
       UNION ALL
       
       SELECT t.id, t.name, t.parent_id, tt.path || '/' || t.name
       FROM tags t
       JOIN tag_tree tt ON t.parent_id = tt.id
     )
     SELECT DISTINCT tag_tree.path AS tag_path
     FROM file_tags ft
     JOIN tag_tree ON ft.tag_id = tag_tree.id
     WHERE ft.file_id = f.id
     
     UNION
     
     SELECT t.name AS tag_path
     FROM file_tags ft
     JOIN tags t ON ft.tag_id = t.id
     WHERE ft.file_id = f.id AND t.parent_id IS NULL
   )),
  (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
     FROM attributes a
    WHERE a.file_id = f.id)
FROM files f;
