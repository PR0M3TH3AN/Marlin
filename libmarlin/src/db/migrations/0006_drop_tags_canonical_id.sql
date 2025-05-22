PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- Remove canonical_id column from tags table
ALTER TABLE tags DROP COLUMN canonical_id;

