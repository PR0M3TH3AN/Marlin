PRAGMA foreign_keys = ON;

-- ─── core tables ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS files (
    id    INTEGER PRIMARY KEY,
    path  TEXT NOT NULL UNIQUE,
    size  INTEGER,
    mtime INTEGER
);

CREATE TABLE IF NOT EXISTS tags (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    parent_id     INTEGER REFERENCES tags(id),
    canonical_id  INTEGER REFERENCES tags(id)
);

CREATE TABLE IF NOT EXISTS file_tags (
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
    PRIMARY KEY (file_id, tag_id)
);

CREATE TABLE IF NOT EXISTS attributes (
    id      INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    key     TEXT NOT NULL,
    value   TEXT
);

-- optional free-form JSON metadata
CREATE TABLE IF NOT EXISTS json_meta (
    file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    data    TEXT                -- arbitrary JSON blob
);

-- ─── full-text search ──────────────────────────────────────────────────
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts
USING fts5(
    path,
    content='files', content_rowid='id',
    prefix='2 3 4 5 6 7 8 9 10'
);

CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN
  INSERT INTO files_fts(rowid, path) VALUES (new.id, new.path);
END;
CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files BEGIN
  UPDATE files_fts SET path = new.path WHERE rowid = new.id;
END;
CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN
  DELETE FROM files_fts WHERE rowid = old.id;
END;

-- ─── version table for incremental migrations ─────────────────────────
CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);

-- ─── useful indexes ────────────────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_files_path        ON files(path);
CREATE INDEX IF NOT EXISTS idx_file_tags_tag_id  ON file_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_attr_file_key     ON attributes(file_id, key);
