PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;  -- Use WAL for better concurrency

-- Version 1: Initial Schema (with FTS5-backed search over paths, tags & attrs)

-- Core tables

CREATE TABLE IF NOT EXISTS files (
    id    INTEGER PRIMARY KEY,
    path  TEXT    NOT NULL UNIQUE,
    size  INTEGER,
    mtime INTEGER,
    hash  TEXT    -- file content hash (e.g. SHA256)
);

CREATE TABLE IF NOT EXISTS tags (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL,           -- tag segment
    parent_id    INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    canonical_id INTEGER REFERENCES tags(id) ON DELETE SET NULL,
    UNIQUE(name, parent_id)
);

CREATE TABLE IF NOT EXISTS file_tags (
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY(file_id, tag_id)
);

CREATE TABLE IF NOT EXISTS attributes (
    id      INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    key     TEXT    NOT NULL,
    value   TEXT,
    UNIQUE(file_id, key)
);

-- Full-text search

-- Drop any old FTS table, then recreate it as a contentless standalone table
DROP TABLE IF EXISTS files_fts;
CREATE VIRTUAL TABLE files_fts
USING fts5(
    path,                     -- Remove UNINDEXED to enable path searching
    tags_text,                -- concat of all tag names for this file
    attrs_text,               -- concat of all key=value attrs
    content='',               -- Explicitly mark as contentless
    tokenize="unicode61 remove_diacritics 2"
);

-- FTS-sync triggers

-- When a file is added
DROP TRIGGER IF EXISTS files_fts_ai_file;
CREATE TRIGGER files_fts_ai_file
AFTER INSERT ON files
BEGIN
    INSERT INTO files_fts(rowid, path, tags_text, attrs_text)
    VALUES (
        NEW.id, -- Sets files_fts.rowid to files.id
        NEW.path,
        (SELECT IFNULL(GROUP_CONCAT(t.name, ' '), '')
           FROM file_tags ft
           JOIN tags t ON ft.tag_id = t.id
          WHERE ft.file_id = NEW.id),
        (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
           FROM attributes a
          WHERE a.file_id = NEW.id)
    );
END;

-- When a file’s path changes
DROP TRIGGER IF EXISTS files_fts_au_file;
CREATE TRIGGER files_fts_au_file
AFTER UPDATE OF path ON files
BEGIN
    UPDATE files_fts
       SET path = NEW.path
     WHERE rowid = NEW.id; -- rowid refers to files_fts.rowid which matches files.id
END;

-- When a file is removed
DROP TRIGGER IF EXISTS files_fts_ad_file;
CREATE TRIGGER files_fts_ad_file
AFTER DELETE ON files
BEGIN
    DELETE FROM files_fts WHERE rowid = OLD.id; -- OLD.id from files table
END;

-- When tags are added, replace the entire FTS row
DROP TRIGGER IF EXISTS file_tags_fts_ai;
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

-- When tags are removed, replace the entire FTS row
DROP TRIGGER IF EXISTS file_tags_fts_ad;
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

-- When attributes are added, replace the entire FTS row
DROP TRIGGER IF EXISTS attributes_fts_ai;
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

-- When attribute values change, replace the entire FTS row
DROP TRIGGER IF EXISTS attributes_fts_au;
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

-- When attributes are removed, replace the entire FTS row
DROP TRIGGER IF EXISTS attributes_fts_ad;
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

-- Versioning & helpful indexes

CREATE TABLE IF NOT EXISTS schema_version (
    version    INTEGER PRIMARY KEY,
    applied_on TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_files_path       ON files(path);
CREATE INDEX IF NOT EXISTS idx_files_hash       ON files(hash);
CREATE INDEX IF NOT EXISTS idx_tags_name_parent ON tags(name, parent_id);
CREATE INDEX IF NOT EXISTS idx_file_tags_tag_id ON file_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_attr_file_key    ON attributes(file_id, key);
