Here’s the updated roadmap with each new feature slotted in where its dependencies are best met:

| Phase                      | Focus                                  | Why now?                                                                          | Key deliverables                                                             |
| -------------------------- | -------------------------------------- | --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| **1. 2025-Q2 – “Bedrock”** | Migrations + CI baseline + core schema | We’ve stabilized migrations; now add foundational tables for links, groups, views | • CI: `cargo test` + `cargo sqlx migrate run --dry-run`<br>• New migrations: |

* `links(src_file,dst_file,link_type)`
* `collections(name)` + `collection_files`
* `views(name,query)` <br>• CLI stubs for `marlin link` / `unlink` / `list-links` / `backlinks`, `marlin coll` and `marlin view`                                    |
  \| **2. 2025-Q2**               | Leaner FTS maintenance                                   | Per-row triggers don’t scale past \~100 k files                                                  | • Replace per-row triggers with a “dirty” flag + periodic rebuild<br>• Benchmark end-to-end on 100 k files                                                                                                                                                                                                  |
  \| **2.1 2025-Q2**              | Dirty-row FTS + CI                                       | Prep for both scale and live-watcher—avoid full rebuilds on every change                        | • `scan --dirty` reindexes only changed files<br>• CI coverage for dirty-scan edge cases                                                                                                                                                                                                                   |
  \| **2.2 2025-Q2**              | Live file watching                                       | Offer true “working-dir” mode—auto-scan on FS events                                           | • `marlin watch [dir]` via `notify` crate<br>• Incremental scan on create/modify/delete/rename                                                                                                                                                                                                             |
  \| **2.3 2025-Q2**              | Self-pruning backups                                     | Instant protection and bounded storage—no manual snapshot cleanup                               | • `marlin backup --prune <N>` flag<br>• Post-scan hook to prune to latest 10<br>• Daily prune automation (cron or CI)                                                                                                                                                                                      |
  \| **3. 2025-Q3**               | FTS5 content indexing & annotations                      | Full-text search over file bodies + per-file notes/highlights                                  | • Add `files.content` column + migration<br>• Extend `files_fts` to include `content`<br>• New `annotations` table + FTS triggers<br>• CLI: `marlin annotate add|list`                                                                                                                                       |
  \| **4. 2025-Q3**               | Content hashing, dedup & versioning                      | Detect duplicates, track history, enable diffs                                                 | • Populate `files.hash` with SHA-256<br>• `scan --rehash` option<br>• CLI: `marlin version diff <file>`                                                                                                                                                                                                    |
  \| **5. 2025-Q3**               | Tag aliases/canonicals & semantic/AI enhancements        | Control tag sprawl and lay groundwork for AI-driven suggestions                                | • Enforce `canonical_id` on `tags` + `tag alias add|ls|rm` CLI<br>• Create `embeddings` table<br>• `scan --embed` to generate vectors<br>• CLI: `marlin tag suggest`, `marlin summary <file>`, `marlin similarity scan`                                                                                       |
  \| **6. 2025-Q4**               | Search DSL v2 & Smart Views                              | More powerful query grammar + reusable “virtual folders”                                       | • Replace ad-hoc parser with a `nom`-based grammar<br>• CLI: `marlin view save|list|exec`                                                                                                                                                                                                                   |
  \| **7. 2025-Q4**               | Attribute templates, states, tasks & timeline            | Structured metadata unlocks workflows, reminders & temporal context                            | • `templates` + `template_fields` tables + validation<br>• CLI:
* `marlin state set|transitions add|state log`
* `marlin task scan|task list`
* `marlin remind set <file> <ts> "<msg>"`
* `marlin event add <file> <date> "<desc>"`, `marlin timeline`                                                                                            |
  \| **8. 2026-Q1**               | Dolphin read-only plugin                                 | Surface metadata, links, annotations in native file manager                                   | • Qt sidebar showing tags, attributes, links, annotations                                                                                                                                                                                                                                                 |
  \| **9. 2026-Q1**               | Full edit UI                                             | After proving read-only stable, add in-place editing                                          | • Tag editor, collection & view managers, state/task/event dialogs                                                                                                                                                                                                                                        |
  \| **10. 2026-Q2**              | Multi-device sync                                        | Final frontier: optional sync/replication layer                                                | • Choose sync backend (rqlite / Litestream / bespoke)<br>• Support read-only mounts for remote indexes                                                                                                                                                                                                    |

---

### Current sprint (ends **2025-06-01**)

1. FTS rebuild prototype (dirty-rows) – measure on 50 k files
2. `backup --prune` implementation + auto-prune hook
3. Integration tests for tag/attr workflows on Windows via GitHub Actions
4. **New:** basic `links`, `collections`, `views` migrations + CLI stubs

**Development principles remain**:

* Local-first, offline-capable
* Ship code = ship migrations
* Instrumentation first (trace spans & timings on all new commands)
