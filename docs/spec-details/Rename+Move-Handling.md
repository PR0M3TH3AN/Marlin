# Marlin — Rename & Move Handling

**Integration Specification · v0.1 (2025-05-19)**

---

## 0 · Scope

This document outlines how Marlin should respond when files or folders are renamed or moved. It extends the watcher life‑cycle design (DP‑003) so that metadata remains consistent without requiring a full re‑scan.

## 1 · Background

The current watcher maps any `notify::EventKind::Modify(_)` – including renames – to the generic `EventPriority::Modify` and merely logs the event:

```
415  let prio = match event.kind {
416      EventKind::Create(_) => EventPriority::Create,
417      EventKind::Remove(_) => EventPriority::Delete,
418      EventKind::Modify(_) => EventPriority::Modify,
419      EventKind::Access(_) => EventPriority::Access,
420      _ => EventPriority::Modify,
421  };
...
455  for event_item in &evts_to_process {
456      info!("Processing event (DB available): {:?} for path {:?}",
457            event_item.kind, event_item.path);
458  }
```

No database update occurs, so renamed files keep their old `path` in the `files` table. The schema does have a trigger to propagate `path` updates to the FTS index:

```
72  -- When a file’s path changes
73  DROP TRIGGER IF EXISTS files_fts_au_file;
74  CREATE TRIGGER files_fts_au_file
75  AFTER UPDATE OF path ON files
76  BEGIN
77      UPDATE files_fts
78         SET path = NEW.path
79       WHERE rowid = NEW.id;
80  END;
```

## 2 · Requirements

1. **Detect old and new paths** from `Rename` events provided by the `notify` crate.
2. **Update the `files` table** with the new absolute path when the target remains inside a scanned root.
3. **Mark as removed** if the new location is outside all configured roots.
4. **Batch updates** to avoid excessive writes during large folder moves.
5. **Integration tests** exercising rename and move scenarios across platforms.

## 3 · Implementation Sketch

* Extend `ProcessedEvent` to carry `old_path` and `new_path` for `Rename` events.
* Upon flushing events, call `db::mark_dirty` for the affected row, then update the `files.path` column. The existing trigger keeps `files_fts` in sync.
* For directory renames, update child paths with a single SQL `UPDATE ... WHERE path LIKE 'old/%'` inside a transaction.
* Emit `Create` and `Remove` events for files crossing watch boundaries so `scan --dirty` can prune or index them accordingly.

## 4 · Edge Cases

* **Atomic cross-filesystem moves** may surface as `Remove` + `Create`; both should be handled.
* **Concurrent modifications** while moving should result in the newer metadata winning when `scan --dirty` runs.

## 5 · Future Work

Large scale refactors (e.g. moving an entire project) may benefit from a high‑level command that updates tags and links en masse. That is outside the scope of this spec but enabled by accurate rename tracking.

---

*End of document*

