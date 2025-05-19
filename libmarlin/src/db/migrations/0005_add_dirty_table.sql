PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- Track which files need re-indexing
CREATE TABLE IF NOT EXISTS file_changes (
  file_id   INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
  marked_at INTEGER NOT NULL             -- UNIX timestamp
);
