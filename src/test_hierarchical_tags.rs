// Test script to validate hierarchical tag FTS fix
// This script demonstrates how the fix works with a simple test case

use rusqlite::{Connection, params};
use std::path::Path;
use std::fs;
use anyhow::Result;

fn main() -> Result<()> {
    // Create a test database in a temporary location
    let db_path = Path::new("/tmp/marlin_test.db");
    if db_path.exists() {
        fs::remove_file(db_path)?;
    }
    
    println!("Creating test database at {:?}", db_path);
    
    // Initialize database with our schema and migrations
    let conn = Connection::open(db_path)?;
    
    // Apply schema (simplified version of what's in the migrations)
    println!("Applying schema...");
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         
         CREATE TABLE files (
             id    INTEGER PRIMARY KEY,
             path  TEXT    NOT NULL UNIQUE,
             size  INTEGER,
             mtime INTEGER,
             hash  TEXT
         );
         
         CREATE TABLE tags (
             id           INTEGER PRIMARY KEY,
             name         TEXT    NOT NULL,
             parent_id    INTEGER REFERENCES tags(id) ON DELETE CASCADE,
             canonical_id INTEGER REFERENCES tags(id) ON DELETE SET NULL,
             UNIQUE(name, parent_id)
         );
         
         CREATE TABLE file_tags (
             file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
             tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
             PRIMARY KEY(file_id, tag_id)
         );
         
         CREATE TABLE attributes (
             id      INTEGER PRIMARY KEY,
             file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
             key     TEXT    NOT NULL,
             value   TEXT,
             UNIQUE(file_id, key)
         );
         
         CREATE VIRTUAL TABLE files_fts
         USING fts5(
             path,
             tags_text,
             attrs_text,
             content='',
             tokenize=\"unicode61 remove_diacritics 2\"
         );"
    )?;
    
    // Apply our fixed triggers
    println!("Applying fixed FTS triggers...");
    conn.execute_batch(
        "CREATE TRIGGER files_fts_ai_file
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
                    SELECT DISTINCT tag_tree.path as tag_path
                    FROM file_tags ft
                    JOIN tag_tree ON ft.tag_id = tag_tree.id
                    WHERE ft.file_id = NEW.id
                    
                    UNION
                    
                    SELECT t.name as tag_path
                    FROM file_tags ft
                    JOIN tags t ON ft.tag_id = t.id
                    WHERE ft.file_id = NEW.id AND t.parent_id IS NULL
                  )),
                 (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
                    FROM attributes a
                   WHERE a.file_id = NEW.id)
             );
         END;
         
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
                  SELECT DISTINCT tag_tree.path as tag_path
                  FROM file_tags ft
                  JOIN tag_tree ON ft.tag_id = tag_tree.id
                  WHERE ft.file_id = f.id
                  
                  UNION
                  
                  SELECT t.name as tag_path
                  FROM file_tags ft
                  JOIN tags t ON ft.tag_id = t.id
                  WHERE ft.file_id = f.id AND t.parent_id IS NULL
                )),
               (SELECT IFNULL(GROUP_CONCAT(a.key || '=' || a.value, ' '), '')
                  FROM attributes a
                 WHERE a.file_id = f.id)
             FROM files f
            WHERE f.id = NEW.file_id;
         END;"
    )?;
    
    // Insert test data
    println!("Inserting test data...");
    
    // Insert a test file
    conn.execute(
        "INSERT INTO files (id, path) VALUES (1, '/test/document.md')",
        [],
    )?;
    
    // Create hierarchical tags: project/md
    println!("Creating hierarchical tags: project/md");
    
    // Insert parent tag 'project'
    conn.execute(
        "INSERT INTO tags (id, name, parent_id) VALUES (1, 'project', NULL)",
        [],
    )?;
    
    // Insert child tag 'md' under 'project'
    conn.execute(
        "INSERT INTO tags (id, name, parent_id) VALUES (2, 'md', 1)",
        [],
    )?;
    
    // Tag the file with the 'md' tag (which is under 'project')
    conn.execute(
        "INSERT INTO file_tags (file_id, tag_id) VALUES (1, 2)",
        [],
    )?;
    
    // Check what's in the FTS index
    println!("\nChecking FTS index content:");
    let mut stmt = conn.prepare("SELECT rowid, path, tags_text, attrs_text FROM files_fts")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    
    for row in rows {
        let (id, path, tags, attrs) = row?;
        println!("ID: {}, Path: {}, Tags: '{}', Attrs: '{}'", id, path, tags, attrs);
    }
    
    // Test searching for the full hierarchical tag path
    println!("\nTesting search for 'project/md':");
    let mut stmt = conn.prepare("SELECT f.path FROM files_fts JOIN files f ON f.id = files_fts.rowid WHERE files_fts MATCH 'project/md'")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    
    let mut found = false;
    for row in rows {
        found = true;
        println!("Found file: {}", row?);
    }
    
    if !found {
        println!("No files found with tag 'project/md'");
    }
    
    // Test searching for just the parent tag
    println!("\nTesting search for just 'project':");
    let mut stmt = conn.prepare("SELECT f.path FROM files_fts JOIN files f ON f.id = files_fts.rowid WHERE files_fts MATCH 'project'")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    
    let mut found = false;
    for row in rows {
        found = true;
        println!("Found file: {}", row?);
    }
    
    if !found {
        println!("No files found with tag 'project'");
    }
    
    // Test searching for just the child tag
    println!("\nTesting search for just 'md':");
    let mut stmt = conn.prepare("SELECT f.path FROM files_fts JOIN files f ON f.id = files_fts.rowid WHERE files_fts MATCH 'md'")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    
    let mut found = false;
    for row in rows {
        found = true;
        println!("Found file: {}", row?);
    }
    
    if !found {
        println!("No files found with tag 'md'");
    }
    
    println!("\nTest completed successfully!");
    Ok(())
}
