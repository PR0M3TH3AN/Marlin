PRAGMA foreign_keys = ON;

-- File-to-file links
CREATE TABLE IF NOT EXISTS links (
  id            INTEGER PRIMARY KEY,
  src_file_id   INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  dst_file_id   INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  type          TEXT,
  UNIQUE(src_file_id, dst_file_id, type)
);

-- Named collections
CREATE TABLE IF NOT EXISTS collections (
  id   INTEGER PRIMARY KEY,
  name TEXT    NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS collection_files (
  collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  file_id       INTEGER NOT NULL REFERENCES files(id)       ON DELETE CASCADE,
  PRIMARY KEY(collection_id, file_id)
);

-- Saved views
CREATE TABLE IF NOT EXISTS views (
  id    INTEGER PRIMARY KEY,
  name  TEXT    NOT NULL UNIQUE,
  query TEXT    NOT NULL
);
