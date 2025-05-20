# Marlin ― Delivery Road‑map **v3.2**

*Engineering‑ready – revised 2025‑05‑18*

> **Legend** △ engineering artefact ✦ user‑visible deliverable

---

## 0 · Methodology primer  (what “Done” means)

| Theme          | Project rule‑of‑thumb                                                                                                                 |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| **Branching**  | Trunk‑based. Feature branch → PR → 2 reviews → squash‑merge.                                                                          |
| **Spec first** | Each epic begins with a **Design Proposal (DP‑xxx)** in `/docs/adr/` containing schema diffs, example CLI session, perf targets.      |
| **Coverage**   | Tarpaulin gate ≥ 85 % **on lines touched this sprint** (checked in CI).                                                               |
| **Perf gate**  | Cold‑start P95 ≤ 3 s on 100 k files **unless overridden in DP**. Regressions fail CI.                                                 |
| **Docs**       | CLI flags & examples land in `README.md` **same PR**.  Docs tables (CLI cheatsheet, TUI key‑map) are auto‑generated during the build. |
| **Demo**       | Closing each epic yields a ≤ 2‑min asciinema or GIF in `docs/demos/`.                                                                 |

---

## 1 · Bird’s‑eye table (engineering details + deliverables)

| Phase / Sprint                                  | Timeline                      | Focus & Rationale                                        | ✦ Key UX Deliverables                                                                                   | △ Engineering artefacts / tasks                                                                                                                                                                                                                    | Definition of Done                                                                                   |
| ----------------------------------------------- | ----------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| **Sprint 0 — Bootstrap & CI Baseline**          | **2025‑Q2<br>(now → 30 May)** | CI scaffolding, coverage, crate split                    | —                                                                                                       | • Split repo into **`libmarlin` (core)** + **`cli-bin`** + **`tui-bin`**  <br>• Tarpaulin coverage + Hyperfine perf jobs wired <br>• `build.rs` renders CLI cheatsheet from `commands.yaml` <br>• Docs / cheatsheet autogen step in GitHub Actions | `cargo test --all` passes with coverage gate ≥ 85 %; docs artefacts appear in build; crates compile. |
| **Sprint α — Bedrock & Metadata Domains**       | **31 May → 13 Jun 2025**      | Lock schema v1.1, first metadata objects                 | • CLI stubs: `marlin link / coll / view` <br>• `marlin demo` interactive tour                           | • **DP‑001 Schema v1.1** (ER + migration scripts) <br>• Unit tests (`escape_fts`, `determine_scan_root`) <br>• GitHub Action for SQL dry‑run                                                                                                       | 100 % migrations green; demo prints ✅; logo badge shows schema version.                              |
| **Epic 1 — Live‑Watch Mode & Backup Prune**     | **2025‑Q2**                   | Continuous indexing via FS events; backups never explode | • `marlin watch <dir>` (inotify / FSEvents) <br>• `backup --prune N` (auto‑prune pre‑ and post‑command) | • **DP‑002** file‑watch life‑cycle & debounce strategy <br>• Change‑table schema storing dirty file IDs <br>• Nightly prune CI job                                                                                                                 | 8 h stress‑watch alters 10 k files → < 1 % missed; backup dir size ≤ N; watch CPU idle < 3 %.        |
| **Epic 2 — Dirty‑scan optimisation**            | **2025‑Q2**                   | Re‑index only paths marked dirty by watch table          | • `scan --dirty`                                                                                        | • Reuse change‑table from watch; Hyperfine benchmark script committed                                                                                                                                                                              | Dirty‑scan runtime ≤ 15 % full scan on 100 k corpus; bench job passes.                               |
| **Phase 3 — Content FTS + Annotations**         | 2025‑Q3                       | Grep snippets, inline notes                              | • `search -C3` grep‑style context <br>• `annotate add/list`                                             | • **DP‑004** content‑blob strategy (inline vs ext‑table) <br>• `syntect` highlight PoC                                                                                                                                                             | Indexes 1 GB corpus ≤ 30 min; snippet CLI golden tests pass.                                         |
| **Phase 4 — Versioning & De‑duplication**       | 2025‑Q3                       | Historic diffs, SHA‑256 dedupe                           | • `scan --rehash` <br>• `version diff <file>`                                                           | • **DP‑005** hash column + Bloom‑de‑dupe research                                                                                                                                                                                                  | Diff on 10 MB file ≤ 500 ms; duplicate sets emitted by CLI.                                          |
| **Phase 5 — Tag Aliases & Semantic Booster**    | 2025‑Q3                       | Tame tag sprawl; start AI hints                          | • `tag alias add/ls/rm` <br>• `tag suggest`, `summary`                                                  | • **DP‑006** embeddings size & k‑NN search bench                                                                                                                                                                                                   | 95 % alias look‑ups resolved in one hop; suggest query ≤ 150 ms.                                     |
| **Phase 6 — Search DSL v2 & Smart Views**       | 2025‑Q4                       | AND/OR, ranges, structured grammar; smart folders        | • New `nom` grammar <br>• Legacy parser behind **`--legacy-search`** (warn on use)                      | • **DP‑007** BNF + 30 acceptance strings <br>• Lexer fuzz tests (`cargo‑fuzz`)                                                                                                                                                                     | Old queries keep working; 0 panics in fuzz run ≥ 1 M cases.                                          |
| **Phase 7 — Structured Workflows & Templates**  | 2025‑Q4                       | State graph, relationship templates                      | • `state set/log` <br>• `template apply`                                                                | • **DP‑008** workflow tables & YAML template spec <br>• Sample template e2e tests                                                                                                                                                                  | Create template, apply to 20 files → all attrs/link rows present; illegal transitions blocked.       |
| **Phase 8 — TUI v1 + Lightweight Integrations** | 2026‑Q1                       | Keyboard UI, VS Code sidebar                             | • **`marlin‑tui`** binary (tiling panes, key‑map) <br>• Read‑only VS Code sidebar                       | • **DP‑009** TUI redraw budget & key‑map <br>• Crate split fully consumed                                                                                                                                                                          | TUI binary ≤ 2 MB; scroll redraw ≤ 4 ms; VS Code extension loads index.                              |
| **Phase 9 — Dolphin Sidebar (MVP)**             | 2026‑Q1                       | Peek metadata inline in KDE Dolphin                      | • Qt/KIO sidebar                                                                                        | • **DP‑010** DB/IP bridge (D‑Bus vs UNIX socket) <br>• CMake packaging script                                                                                                                                                                      | Sidebar opens ≤ 150 ms; passes KDE lint.                                                             |
| **Phase 10 — Full GUI & Multi‑device Sync**     | 2026‑Q2                       | Visual editor + optional sync backend                    | • Electron/Qt hybrid explorer UI <br>• Select & integrate sync (LiteFS / Postgres)                      | • **DP‑011** sync back‑end trade‑study <br>• Busy‑timeout/retry strategy for multi‑writer mode                                                                                                                                                     | CRUD round‑trip < 2 s between two nodes; 25 GUI e2e tests green.                                     |

---

### 2 · Feature cross‑matrix (quick look‑ups)

| Capability                 | Sprint / Phase | CLI / GUI element                   | Linked DP |
| -------------------------- | -------------- | ----------------------------------- | --------- |
| Crate split & docs autogen | S0             | —                                   | –         |
| Tarpaulin coverage gate    | S0             | —                                   | –         |
| Watch mode (FS events)     | Epic 1         | `marlin watch .`                    | DP‑002    |
| Backup auto‑prune          | Epic 1         | `backup --prune N`                  | –         |
| Dirty‑scan                 | Epic 2         | `scan --dirty`                      | DP‑002    |
| Grep snippets              | Phase 3        | `search -C3 …`                      | DP‑004    |
| Hash / dedupe              | Phase 4        | `scan --rehash`                     | DP‑005    |
| Tag aliases                | Phase 5        | `tag alias` commands                | DP‑006    |
| Search DSL v2              | Phase 6        | new grammar, `--legacy-search` flag | DP‑007    |
| Relationship templates     | Phase 7        | `template new/apply`                | DP‑008    |
| TUI v1                     | Phase 8        | `marlin‑tui`                        | DP‑009    |

---

## 3 · Milestone acceptance checklist

Before a milestone is declared **shipped**:

* [ ] **Spec** DP‑xxx merged with schema diff, ASCII‑cast demo
* [ ] **Tests** Tarpaulin ≥ 85 % on changed lines; all suites green
* [ ] **Perf guard** script passes on CI matrix (Ubuntu 22, macOS 14)
* [ ] **Docs** auto‑regenerated; README & cheatsheet updated
* [ ] **Demo** asciinema/GIF committed and linked in release notes
* [ ] **Release tag** pushed; Cargo binary uploaded to GitHub Releases

---

## 4 · Next immediate actions

| # | Task                           | Owner  | Due           |
| - | ------------------------------ | ------ | ------------- |
| 1 | Crate split + CI baseline      | @alice | **26 May 25** |
| 2 | Tarpaulin + Hyperfine jobs     | @bob   | **26 May 25** |
| 3 | **DP‑001 Schema v1.1** draft   | @carol | **30 May 25** |
| 4 | backup prune CLI + nightly job | @dave  | **05 Jun 25** |

## CLI Cheatsheet

The full command reference is generated during the build of the CLI. See
[docs/cli_cheatsheet.md](docs/cli_cheatsheet.md).
