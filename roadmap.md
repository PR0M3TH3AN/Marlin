# Roadmap

| Phase                           | Functional focus         | Why do it now?                                                                                 | Key deliverables                                                                                                                           |
| ------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **1. Lock down the foundation** | *Migrations + tests*     | Schema churn and silent breakage are the biggest hidden costs. Catch them early.               | • Split `migrations.sql` into versioned files<br>• Remove runtime “ensure\_column” path<br>• Add CI job that runs `cargo test` on every PR |
| **2. Trim the FTS triggers**    | *Efficient index upkeep* | The current triggers will bog down as soon as users bulk-tag thousands of files.               | • Replace per-row GROUP\_CONCAT triggers with a “dirty” flag or app-side refresh<br>• Benchmark a full scan + mass tag on ≥100 k files     |
| **3. Hashing & dedup logic**    | *Content integrity*      | Once the index is stable and fast, add SHA-256 so the DB can detect duplicates/corruption.     | • `files.hash` column populated on first scan<br>• `marlin scan --rehash` to force refresh                                                 |
| **4. Alias / canonical tags**   | *Usable taxonomy*        | Without this, tag sprawl happens quickly. Better to solve before users have thousands of tags. | • `tags.aliases` table or `canonical_id` enforcement<br>• CLI subcommands: `tag alias add`, `tag alias ls`                                 |
| **5. Search parser upgrade**    | *Power queries*          | After the data model is solid, richer search is the next visible win.                          | • Swap ad-hoc parser for `nom`-based grammar<br>• Support grouping `(...)`, boolean ops, quoted phrases                                    |
| **6. Attribute schemas**        | *Structured metadata*    | Custom field templates let you build real workflows (e.g. Photo > Aperture).                   | • `templates` + `template_fields` tables<br>• Validation on `attr set`                                                                     |
| **7. Dolphin extension MVP**    | *Desktop integration*    | No point shipping a GUI until the backend is rock-solid.                                       | • Read-only sidebar showing tags/attrs<br>• Double-click tag to filter view                                                                |
| **8. Write / edit UI**          | *End-user adoption*      | Once people can browse metadata inside Dolphin, they’ll want to edit it too.                   | • In-place tag editor widget<br>• Attribute form dialog tied to templates                                                                  |
| **9. Sync & sharing**           | *Multi-device story*     | Last—most complex. Only tackle when single-machine use is boring.                              | • Lite RPC layer (SQLite WAL + notify?)<br>• Optional read-only mode for network mounts                                                    |

---

#### How to tackle each phase

1. **Do one migration PR that just moves existing DDL into `0001.sql`**. Merge, tag a release.
2. **Prototype trigger-less FTS maintenance** in a branch; measure with `--timings` tracing.
3. **Hashing:** gate expensive work behind `mtime/size` check you already coded.
4. **Alias logic:** start simple—single-level `canonical_id`; later add synonym sets if needed.
5. **Parser:** write unit tests for every example query first, then swap implementation—same public API.
6. **Templates:** store JSON schema in DB, validate with `serde_json::Value` + compiled regexes.
7. **Dolphin plugin:** expose DBus calls from Rust core, C++/Qt side just calls them.
8. **Write UI:** reuse the same DBus interface; no extra DB code.
9. **Sync:** decide early if you aim for local-first replication (Litestream, rqlite) or a bespoke solution.

---

### Practical next sprint (2 weeks)

1. **Finish phase 1** (migrations + CI) ⇒ release `v0.2.0`.
2. **Start phase 2:** rip out FTS triggers, implement dirtied-rows rebuild, test at 50 k files.
3. **If time remains:** add `--rehash` flag and wire in SHA-256 function (phase 3 seed).

This path keeps user-visible features arriving every couple of weeks without accumulating technical debt.
