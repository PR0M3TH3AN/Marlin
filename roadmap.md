# Marlin Roadmap 2025 → 2026  📜

This document outlines the **official delivery plan** for Marlin over the next four quarters.
Every work-item below is *time-boxed, testable,* and traceable back to an end-user benefit.

> **Legend**
> ✅  = item added/clarified in the latest planning round
> Δ  = new sub-deliverable (wasn’t in the previous version)

---

## 1 Bird’s-eye Table

| Phase / Sprint                                  | Timeline                  | Focus & Rationale                                                        | Key Deliverables (Δ = new)                                                                                                                                                                                                                                                                                  |                 |                                                                                                                    |
| ----------------------------------------------- | ------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Sprint α – Bedrock & Metadata Domains**       | **2025-Q2 (now → 6 Jun)** | Stabilise schema & CI; land first metadata domains with discoverability. | Δ CI: `cargo test` + SQL dry-run<br>Δ Unit tests (`determine_scan_root`, `escape_fts`)<br>Δ Coverage: e2e `attr --format=json`<br>Δ Refactor: move `naive_substring_search` to shared util<br>Migrations: `links`, `collections`, `views`<br>CLI stubs: `link`, `coll`, `view`<br>`marlin demo` walkthrough |                 |                                                                                                                    |
| **Epic 1 – Scale & Reliability**                | 2025-Q2                   | Keep scans fast; bullet-proof CI at 100 k files.                         | Δ Dirty-flag column + `scan --dirty`<br>Benchmarks: full vs dirty scan (100 k)<br>Replace per-row triggers with periodic rebuild<br>CI edge-case tests                                                                                                                                                      |                 |                                                                                                                    |
| **Epic 2 – Live Mode & Self-Pruning Backups**   | 2025-Q2                   | Continuous indexing & hygiene—Marlin “just works”.                       | Δ `marlin watch [dir]` (notify/FSEvents)<br>Δ `backup --prune <N>` + auto-prune post-scan<br>Daily / PR-merge prune in CI                                                                                                                                                                                   |                 |                                                                                                                    |
| **Phase 3 – Content FTS & Annotations**         | 2025-Q3                   | Index file bodies, grep-style context, inline notes.                     | `files.content` + migration<br>Extend `files_fts` (context snippets `-C`)<br>`annotations` table + triggers<br>CLI \`annotate add                                                                                                                                                                           | list\`          |                                                                                                                    |
| **Phase 4 – Versioning & Deduplication**        | 2025-Q3                   | History, diffs & duplicate detection.                                    | `files.hash` (SHA-256)<br>`scan --rehash` refresh<br>CLI `version diff <file>`                                                                                                                                                                                                                              |                 |                                                                                                                    |
| **Phase 5 – Tag Aliases & Semantic Booster**    | 2025-Q3                   | Tame tag sprawl; seed AI-powered suggestions.                            | `canonical_id` on `tags`; CLI `tag alias …`<br>`embeddings` table + `scan --embed`<br>CLI `tag suggest`, `similarity scan`, `summary <file>`                                                                                                                                                                |                 |                                                                                                                    |
| **Phase 6 – Search DSL v2 & Smart Views**       | 2025-Q4                   | Robust grammar + virtual folders.                                        | Replace parser with **`nom`** grammar (`AND`, `OR`, `()` …)<br>CLI \`view save                                                                                                                                                                                                                              | list            | exec\` with aliases & paging                                                                                       |
| **Phase 7 – Structured Workflows**              | 2025-Q4                   | First-class task / state / reminder / event life-cycles.                 | ✅ State engine (`files.state`, `state_changes`)<br>CLI \`state set                                                                                                                                                                                                                                          | transitions add | log`<br>✅ Task extractor (`tasks` table) + CLI<br>`templates`+ validation<br>CLI`remind …`, `event …`, `timeline\` |
| **Phase 8 – Lightweight Integrations**          | 2026-Q1                   | Surface Marlin in editors / terminal.                                    | VS Code & TUI extension (tags / attrs / links / notes)                                                                                                                                                                                                                                                      |                 |                                                                                                                    |
| **Phase 9 – Dolphin Sidebar Plugin (MVP)**      | 2026-Q1                   | Read-only Qt sidebar for Linux file managers.                            | Qt plug-in: tags, attrs, links, annotations                                                                                                                                                                                                                                                                 |                 |                                                                                                                    |
| **Phase 10 – Full Edit UI & Multi-Device Sync** | 2026-Q2                   | In-place metadata editor & optional sync layer.                          | GUI editors (tags, views, tasks, reminders, events)<br>Pick/implement sync backend (rqlite, Litestream, …)                                                                                                                                                                                                  |                 |                                                                                                                    |

---

## 2 Narrative & Dependencies

1. **Lock down core schema & demo** *(Sprint α).*
   Developers get immediate feedback via the `marlin demo` command while CI ensures migrations never regress.

2. **Scale & Live Mode** *(Epics 1-2).*
   Dirty scanning, file-watching and auto-pruned backups guarantee snappy, hands-off operation even on six-figure corpora.

3. **Richer Search** *(Phases 3-6).*
   Body-content FTS + grep-style snippets lay the groundwork; `nom` grammar then elevates power-user queries and smart views.

4. **Workflow Layers** *(Phase 7).*
   State transitions, tasks and reminders turn Marlin from a passive index into an active workflow engine.

5. **UX Expansions** *(Phases 8-10).*
   Start lightweight (VS Code / TUI), graduate to a read-only Dolphin plug-in, then ship full editing & sync for multi-device teams.

Every outer milestone depends only on the completion of the rows above it, **so shipping discipline in early sprints de-risks the headline features down the line.**

---

## 3 Next Steps

* **Sprint α kickoff:** break deliverables into stories, estimate, assign.
* **Add roadmap as `docs/ROADMAP.md`** (this file).
* Wire a **Checklist issue** on GitHub: one task per Δ bullet for instant tracking.

---

*Last updated · 2025-05-16*
