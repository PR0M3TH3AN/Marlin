# Marlin — TUI + Query-DSL + Templates

**Integration Specification · v0.1 (2025-05-18)**

---

## 0 · Scope

This document turns the **concept sketch** (suck-less TUI) **plus** the new
*relationship-template* and *positive/negative filter* ideas into a concrete
development plan that extends the existing Marlin code-base without breaking
CLI workflows.

---

## 1 · Feature Set

| Epic     | Deliverable                  | Short Description                                                                                            |
| -------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------ |
| **T-1**  | **Marlin-TUI** binary        | One-screen, tiling, key-centred interface (sketch in §2).                                                    |
| **Q-1**  | **Query DSL v2**             | Lexer + parser + planner supporting `AND/OR`, `NOT`, parentheses, ranges, comparison ops on `size`, `mtime`… |
| **T-2**  | **Relationship templates**   | Named templates with typed fields (key + value-type) that auto-populate attributes when applied.             |
| **Glue** | Shared **`libmarlin`** crate | Re-export public APIs so both CLI and TUI call the same Rust functions.                                      |

These map cleanly onto the public roadmap:

* T-1  → *Phase 6: Search DSL v2 & Smart Views* (UI portion)
* Q-1  → *Phase 6* (parser)
* T-2  → *Phase 7: Structured Workflows* (templates subsection)

---

## 2 · TUI Reference Layout & Key-Map

```
┌─────────────────────────────────────────────────────────────┐
│ Marlin ▸ ~/demo  (q quit · ? help)                          │
├───────────────┬─────────────────────────────────────────────┤
│ Tags / Views  │ Path / Name            │ ⓘ Attributes       │
│───────────────│────────────────────────┼────────────────────│
│ project       │ api/design.md          │ status=pending     │
│ ├ alpha       │ roadmap.xlsx           │ reviewed=yes       │
│ │ ├ draft     │ ...                    │ tags=project/alpha │
│ │ └ final     │ (42/812 shown)         │ links: 2 outgoing  │
│ <Views>       │ grep: "(?i)todo"       │ ───── Preview ───── │
│ • urgent      │                        │ - [ ] TODO write…  │
└───────────────┴────────────────────────┴────────────────────┘
```

### Essential Keys

| Key             | Effect                    | CLI equivalence                  |
| --------------- | ------------------------- | -------------------------------- |
| `/expr⏎`        | Regex/FTS filter          | `marlin search`                  |
| `;key=val⏎`     | Attr filter               | `marlin search attr:key=val`     |
| `:`             | Command prompt            | raw CLI passthrough              |
| `Space`         | Mark/unmark               | adds to in-memory selection list |
| `:tag foo/bar⏎` | Tag marked files          | `marlin tag`                     |
| `Enter`         | open file in `$EDITOR`    | —                                |
| `Tab`           | toggle right preview pane | —                                |

Full table lives in **`docs/tui_keys.md`** (auto-generated from
`src/tui/keymap.rs`).

---

## 3 · Architecture Impact

### 3.1 Crate refactor (Week 1)

```
marlin/
├── cli-bin/          ← binary crate (keeps current main.rs)
├── tui-bin/          ← NEW binary crate (Marlin-TUI)
├── libmarlin/        ← NEW library: DB helpers, scan, query, templates
└── Cargo.toml
```

*Export surface*:

```rust
pub struct Marlin { /* connection, config … */ }
impl Marlin {
    pub fn search(&self, q: Query) -> Result<Vec<PathBuf>>;
    pub fn tag(&self, files: &[PathBuf], tag: &str) -> Result<()>;
    pub fn apply_template(&self, files: &[PathBuf], tpl: &Template) -> Result<()>;
    /* … */
}
```

CLI keeps 100 % of its flags; it just calls `libmarlin`.

TUI links against the same crate → zero business-logic duplication.

---

### 3.2 Query-DSL v2 (Week 2-3)

* **Lexer:** [`logos`](https://crates.io/crates/logos) (<500 LOC).
* **Parser:** `nom` (top-down precedence climbing).
* **AST → Plan:**

  ```rust
  struct Plan {
      fts_match: Option<String>,  // tags_text/path/attrs_text
      sql_where: Vec<String>,     // files.size > ?, files.mtime BETWEEN ? ?
      params:     Vec<SqlArg>,
  }
  ```
* **Planner** decides when to fall back to pure SQL (e.g. `size>1M AND NOT tag:archive` uses an FTS sub-query joined on rowid).

Backwards compatibility: old space-separated syntax is parsed first; if it fails,
DSL v2 is attempted, so no user breakage.

---

### 3.3 Templates (Week 4)

**Schema (migration 0005):**

```sql
CREATE TABLE templates (
  id          INTEGER PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  description TEXT
);
CREATE TABLE template_fields (
  id          INTEGER PRIMARY KEY,
  template_id INTEGER NOT NULL REFERENCES templates(id),
  key         TEXT NOT NULL,
  value_type  TEXT NOT NULL                -- 'text' | 'int' | 'date'
);
```

**CLI additions**

```
marlin template      list|show|add|rm
marlin template apply <name> <glob>
```

`Templater` helper validates type coercion before inserting into `attributes`.

TUI: `:apply <tpl>` applies to marked files and prompts inline for field values
(stenographic prompt à la `git commit --template`).

---

## 4 · Implementation Phases & Timeline

| Week | Milestone / Task                           | Owner    | Exit Criteria             |
| ---- | ------------------------------------------ | -------- | ------------------------- |
| 1    | Split repo into `libmarlin`, adapt CLI     | core dev | `cargo test --all` green  |
| 2    | Lexer + unit tests                         | core dev | 95 % coverage             |
| 3    | Parser + Planner; replace `run_search()`   | core dev | DSL green tests pass      |
| 4    | Template schema, CLI, tests                | DB dev   | `template apply` e2e test |
| 5    | **TUI MVP**<br>draw loop + panes + key nav | UI dev   | Can browse & open files   |
| 6    | TUI ↔ Marlin API glue<br>mark, tag, search | UI dev   | All key-map smoke tests   |
| 7    | Preview cache, file-watch refresh          | UI dev   | CPU < 5 % idle load       |
| 8    | Docs refresh, release notes                | all      | v0.2.0 tag                |

---

## 5 · Testing & CI

* **DSL golden-file tests** (`tests/dsl/*.txt` → expected SQL).
* **SQLite snapshot tests** for templates.
* **TUI headless tests** via `termwiz`’s `TerminalTest` harness.
* GitHub Actions matrix unchanged; new crate just builds in.

---

## 6 · Risks & Mitigations

| Risk                         | Impact         | Mitigation                                                   |
| ---------------------------- | -------------- | ------------------------------------------------------------ |
| Parser ambiguity             | broken queries | formal grammar + fuzzy tests                                 |
| TUI performance on huge dirs | lag            | incremental drawing + LRU file stat cache                    |
| Schema lock-in for templates | hard to evolve | keep `value_type` generic (text) until needs prove otherwise |

---

## 7 · What Stays Back-Compatible?

* **CLI syntax**: existing commands & search strings still work.
* **Database**: 0005 migration is additive.
* **Automation scripts**: unchanged; TUI is *optional* (`marlin-tui` binary).

---

## 8 · Why It’s Worth It

* **Differentiator**: Few local-first tools combine rich metadata, SQL-grade
  filtering, and both script-friendly CLI *and* keyboard-driven TUI.
* **Gateway to GUI**: The API we expose for TUI is the same one the
  future GTK/Qt desktop app can use.

---

**Let the hacking begin!**
