# Roadmap

| Phase                      | Focus                    | Why now?                                                                                    | Key deliverables                                                                       |
| -------------------------- | ------------------------ | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| **1. 2025‑Q2 – "Bedrock"** | Migrations + CI baseline | We’ve landed versioned migrations and removed runtime column hacks – ensure it stays solid. | • CI job runs `cargo test` + `cargo sqlx migrate run --dry-run`                        |
| **2. 2025‑Q2**             | Leaner FTS maintenance   | Per‑row triggers don’t scale past \~100 k files.                                            | • Replace triggers with “dirty” flag + periodic rebuild <br>• Benchmark on 100 k files |
| **3. 2025‑Q3**             | Content hashing & dedup  | Detect duplicates, enable future integrity checks.                                          | • SHA‑256 in `files.hash` <br>• `scan --rehash` option                                 |
| **4. 2025‑Q3**             | Tag aliases / canonicals | Control tag sprawl before users accumulate thousands.                                       | • `canonical_id` enforcement <br>• `tag alias add/ls/rm` CLI                           |
| **5. 2025‑Q4**             | Search DSL v2            | Power users want grouping, boolean ops, quoted phrases.                                     | • Replace ad‑hoc parser with `nom` grammar <br>• Unit‑tested examples                  |
| **6. 2025‑Q4**             | Attribute templates      | Structured metadata unlocks real workflows.                                                 | • `templates` + `template_fields` tables <br>• Validation on `attr set`                |
| **7. 2026‑Q1**             | Dolphin read‑only plugin | Browse tags/attrs inside the default file manager.                                          | • Qt sidebar showing metadata                                                          |
| **8. 2026‑Q1**             | Full edit UI             | After read‑only proves stable, add editing.                                                 | • Tag editor widget, attribute dialog                                                  |
| **9. 2026‑Q2**             | Multi‑device sync        | Final frontier: optional sync/replication layer.                                            | • Choose between rqlite / Litestream / bespoke <br>• Read‑only mode for network mounts |

---

### Current sprint (ends **2025‑06‑01**)

1. **FTS rebuild prototype** – dirtied‑rows approach, measure on 50 k files.
2. `backup --prune` to keep only N most recent snapshots.
3. Integration tests for tag/attr workflows on Windows via GitHub Actions.

---

### Development principles

* **Local‑first** – every feature must work offline.
* **Zero manual migrations** – shipping code *is* the migration.
* **Instrumentation first** – every new command logs trace spans and timings.
