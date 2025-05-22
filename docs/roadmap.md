# Marlin ― Delivery Road-map **v3.2**

*Engineering-ready version — updated 2025-05-18*

> **Legend**
> **△** = engineering artefact (spec / ADR / perf target)  **✦** = user-visible deliverable

---

## 0 · Methodology primer  (what “Done” means)

| Theme          | Project rule-of-thumb                                                                                                            |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **Branching**  | Trunk-based. Feature branches → PR → 2 reviews → squash-merge.                                                                   |
| **Spec first** | Every epic starts with a **Design Proposal (DP-xxx)** in `/docs/adr/`.   Include schema diffs, example CLI session, perf budget. |
| **Tests**      | Unit + integration coverage ≥ 85 % on lines **touched in the sprint** (checked by Tarpaulin).                                    |
| **Perf gate**  | Cold start P95 ≤ 3 s on 100 k files **unless overridden in DP**. Regressions fail CI.                                            |
| **Docs**       | CLI flags & examples land in `README.md` **same PR** that ships the code.                                                        |
| **Demo**       | Closing each epic produces a 2-min asciinema or gif in `docs/demos/`.                                                            |

---

## 1 · Bird’s-eye table (now includes engineering columns)

| Phase / Sprint                                | Timeline | Focus & Rationale                        | ✦ Key UX Deliverables                                                                  | △ Engineering artefacts / tasks                                                                                                    | Definition of Done                                                                                       |
| --------------------------------------------- | -------- | ---------------------------------------- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| ~~**Epic 1 — Scale & Reliability**~~          | ~~2025-Q2~~  | ~~Stay fast @ 100 k files~~                  | ~~• `scan --dirty` (re-index touched rows only)~~                                          | ~~• DP-002 Dirty-flag design + FTS rebuild cadence<br>• Hyperfine benchmark script committed~~                                         | ~~Dirty scan vs full ≤ 15 % runtime on 100 k corpus; benchmark job passes~~                                  |
| **Epic 2 — Live Mode & Self-Pruning Backups** | 2025-Q2  | “Just works” indexing, DB never explodes | • `marlin watch <dir>` (notify/FSEvents)<br>• `backup --prune N` & auto-prune<br>• rename/move tracking keeps paths current | • DP-003 file-watcher life-cycle & debouncing<br>• Integration test with inotify-sim<br>• Rename/Move handling spec & tests<br>• Cron-style GitHub job for nightly prune | 8 h stress-watch alters 10 k files < 1 % misses; backup dir ≤ N                                          |
| **Phase 3 — Content FTS + Annotations**       | 2025-Q3  | Search inside files, leave notes         | • Grep-style snippet output (`-C3`)<br>• `marlin annotate add/list`                    | • DP-004 content-blob strategy (inline vs ext-table)<br>• Syntax-highlight via `syntect` PoC<br>• New FTS triggers unit-tested     | Indexes 1 GB corpus in ≤ 30 min; snippet CLI passes golden-file tests                                    |
| **Phase 4 — Versioning & Deduplication**      | 2025-Q3  | Historic diffs, detect dupes             | • `scan --rehash` (SHA-256)<br>• `version diff <file>`                                 | • DP-005 hash column + Bloom-de-dupe<br>• Binary diff adapter research                                                             | Diff on 10 MB file ≤ 500 ms; dupes listed via CLI                                                        |
| **Phase 5 — Tag Aliases & Semantic Booster**  | 2025-Q3  | Tame tag sprawl, start AI hints          | • `tag alias add/ls/rm`<br>• `tag suggest`, `summary`                                  | • DP-006 embeddings size & model choice<br>• Vector store schema + k-NN index bench                                                | 95 % of “foo/bar~foo” alias look-ups resolve in one hop; suggest CLI returns ≤ 150 ms                   |
| **Phase 6 — Search DSL v2 & Smart Views**     | 2025-Q4  | Pro-grade query language                 | • New `nom` grammar: AND/OR, parentheses, ranges                                       | • DP-007 BNF + 30 acceptance strings<br>• Lexer fuzz-tests with `cargo-fuzz`                                                       | Old queries keep working (migration shim); 0 crashes in fuzz run ≥ 1 M cases                             |
| **Phase 7 — Structured Workflows**            | 2025-Q4  | Tasks, state, reminders, templates       | • `state set/transitions add/log`<br>• `task scan/list`<br>• **NEW:** `template apply` | • DP-008 Workflow tables & validation<br>• Sample YAML template spec + CLI expansion tests                                         | Create template, apply to 20 files → all attrs/link rows present; state graph denies illegal transitions |
| **Phase 8 — Lightweight Integrations**        | 2026-Q1  | First “shell” GUIs                       | • VS Code side-bar (read-only)<br>• **TUI v1** (tag tree ▸ file list ▸ preview)        | • DP-009 TUI key-map & redraw budget<br>• Crate split `marlin_core`, `marlin_tui`                                                  | TUI binary ≤ 2.0 MB; 10 k row scroll ≤ 4 ms redraw                                                       |
| **Phase 9 — Dolphin Sidebar (MVP)**           | 2026-Q1  | Peek metadata in KDE file-manager        | • Qt-plugin showing tags, attrs, links                                                 | • DP-010 DB/IP bridge (D-Bus vs UNIX socket)<br>• CMake packaging script                                                           | Sidebar opens ≤ 150 ms; passes KDE lint                                                               |
| **Phase 10 — Full GUI & Multi-device Sync**   | 2026-Q2  | Edit metadata visually, sync option      | • Electron/Qt hybrid explorer UI<br>• Pick & integrate sync backend                    | • DP-011 sync back-end trade-study<br>• UI e2e tests in Playwright                                                                 | Round-trip CRUD between two nodes in < 2 s; 25 GUI tests green                                           |

---

### 2 · Feature cross-matrix (quick look-ups)


| Capability                 | Sprint / Phase | CLI / GUI element    | Linked DP |
| -------------------------- | -------------- | -------------------- | --------- |
| Crate split & docs autogen | S0             | —                    | –         |
| Tarpaulin coverage gate    | S0             | —                    | –         |
| Watch mode (FS events)     | Epic 1         | `marlin watch .`     | DP‑002    |
| Backup auto‑prune          | Epic 1         | `backup --prune N`   | –         |
| Rename/move tracking     | Epic 2         | automatic path update | Spec‑RMH |
| Dirty‑scan                 | Epic 2         | `scan --dirty`       | DP‑002    |
| Grep snippets              | Phase 3        | `search -C3 …`       | DP‑004    |
| Hash / dedupe              | Phase 4        | `scan --rehash`      | DP‑005    |
| Tag aliases                | Phase 5        | `tag alias` commands | DP‑006    |
| Search DSL v2              | Phase 6        | new grammar, `--legacy-search` flag | DP‑007    |
| Relationship templates     | Phase 7        | `template new/apply` | DP‑008    |
| TUI v1                     | Phase 8        | `marlin‑tui`         | DP‑009    |
---

## 3 · Milestone acceptance checklist

Before a milestone is declared “shipped”:

* [ ] **Spec** merged (DP-xxx) with schema diff & example ASCII-cast
* [ ] **Unit & integration tests** ≥ 85 % coverage on changed lines
* [ ] **Perf guard-rail** script passes on CI matrix (Ubuntu 22, macOS 14)
* [ ] **Docs** — CLI man-page, README table row, roadmap ticked
* [ ] **Demo** uploaded to `docs/demos/` and linked in release notes
* [ ] **Release tag** pushed; Cargo binary on GitHub Releases

---

### 4 · Next immediate actions

| # | Task                           | Owner  | Due           |
| - | ------------------------------ | ------ | ------------- |
| ~~1~~ | ~~Crate split + CI baseline~~      | @alice | ~~26 May 25~~ |
| ~~2~~ | ~~Tarpaulin + Hyperfine jobs~~     | @bob   | ~~26 May 25~~ |
| 3 | **DP‑001 Schema v1.1** draft   | @carol | **30 May 25** |
| ~~4~~ | ~~backup prune CLI + nightly job~~ | @dave  | ~~05 Jun 25~~ |

> *This roadmap now contains both product-level “what” and engineering-level “how/when/prove it”.  It should allow a new contributor to jump in, pick the matching DP, and know exactly the bar they must clear for their code to merge.*  
