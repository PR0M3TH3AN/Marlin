![Marlin Logo](https://raw.githubusercontent.com/PR0M3TH3AN/Marlin/refs/heads/main/assets/png/marlin_logo.png)

# Marlin

**Marlin** is a lightweight, metadata-driven file indexer that runs **100 % on your computer**.  
It scans folders, stores paths and file stats in SQLite, lets you attach hierarchical **tags** and **custom attributes**, keeps timestamped **snapshots**, and offers instant full-text search via FTS5.  
_No cloud, no telemetry – your data never leaves the machine._

---

## Feature highlights

| Area                | What you get                                                                                         |
| ------------------- | ----------------------------------------------------------------------------------------------------- |
| **Safety**          | Timestamped backups (`marlin backup`) and one-command restore (`marlin restore`)                      |
| **Resilience**      | Versioned, idempotent schema migrations – zero-downtime upgrades                                      |
| **Indexing**        | Fast multi-path scanner with SQLite WAL concurrency                                                   |
| **Metadata**        | Hierarchical tags (`project/alpha`) & key-value attributes (`reviewed=yes`)                           |
| **Relations**       | Typed file ↔ file links (`marlin link`) with backlinks viewer                                         |
| **Collections / Views** | Named playlists (`marlin coll`) & saved searches (`marlin view`) for instant recall                   |
| **Search**          | Prefix-aware FTS5 across paths, tags, attrs & links; optional `--exec` per match <br>(grep-style context snippets coming Q3) |
| **DX / Logs**       | Structured tracing (`RUST_LOG=debug`) for every operation                                             |

---

## How it works

```text
┌──────────────┐  marlin scan          ┌─────────────┐
│  your files  │ ─────────────────────▶│   SQLite    │
│ (any folder) │                      │  files/tags │
└──────────────┘   tag / attr / link   │ attrs / FTS │
        ▲   search / exec             └──────┬──────┘
        └────────── backup / restore          ▼
                                     timestamped snapshots
````

---

## Prerequisites

| Requirement        | Why                           |
| ------------------ | ----------------------------- |
| **Rust ≥ 1.77**    | Build toolchain (`rustup.rs`) |
| C build essentials | Builds bundled SQLite (Linux) |

macOS & Windows users: let the Rust installer pull the matching build tools.

---

## Build & install

```bash
git clone https://github.com/PR0M3TH3AN/Marlin.git
cd Marlin
cargo build --release

# (Optional) install into your PATH
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

---

## Quick start

For a concise walkthrough—including **links, collections and views**—see
[**Quick start & Demo**](marlin_demo.md).

---

## Testing

Below is a **repeat-able 3-step flow** you can use **every time you pull fresh code**.

### 0 Prepare once

```bash
# Put build artefacts in one place (faster incremental builds)
export CARGO_TARGET_DIR=target
```

### 1 Build the new binary

```bash
git pull
cargo build --release
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

### 2 Run the smoke-test suite

```bash
cargo test --test e2e -- --nocapture
```

*Streams CLI output live; exit-code 0 = all good.*

### 3 (Optionally) run **all** tests

```bash
cargo test --all -- --nocapture
```

This now covers:

* unit tests in `src/**`
* positive & negative integration suites (`tests/pos.rs`, `tests/neg.rs`)
* doc-tests

#### One-liner helper

```bash
git pull && cargo build --release &&
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin &&
cargo test --test e2e -- --nocapture
```

Alias it as `marlin-ci` for a 5-second upgrade-and-verify loop.

---

### Database location

| OS          | Default path                                    |
| ----------- | ----------------------------------------------- |
| **Linux**   | `~/.local/share/marlin/index.db`                |
| **macOS**   | `~/Library/Application Support/marlin/index.db` |
| **Windows** | `%APPDATA%\marlin\index.db`                     |

Override:

```bash
export MARLIN_DB_PATH=/path/to/custom.db
```

---

## CLI reference

```text
marlin <COMMAND> [ARGS]

init                               create / migrate DB **and perform an initial scan of the cwd**
scan     <PATHS>...                walk directories & (re)index files
tag      "<glob>" <tag_path>       add hierarchical tag
attr     set <pattern> <key> <val> set or update custom attribute
attr     ls  <path>                list attributes
link     add|rm|list|backlinks     manage typed file-to-file relations
coll     create|add|list           manage named collections (“playlists”)
view     save|list|exec            save and run smart views (saved queries)
search   <query> [--exec CMD]      FTS5 query; optionally run CMD per hit
backup                             create timestamped snapshot in `backups/`
restore  <snapshot.db>             replace DB with snapshot
completions <shell>                generate shell completions
```

### Attribute sub-commands

| Command     | Example                                          |
| ----------- | ------------------------------------------------ |
| `attr set`  | `marlin attr set ~/Docs/**/*.pdf reviewed yes`   |
| `attr ls`   | `marlin attr ls ~/Docs/report.pdf`               |
| JSON output | `marlin --format=json attr ls ~/Docs/report.pdf` |

---

## Backups & restore

```bash
marlin backup
# → ~/.local/share/marlin/backups/backup_2025-05-14_22-15-30.db
```

```bash
marlin restore ~/.local/share/marlin/backups/backup_2025-05-14_22-15-30.db
```

> Marlin also creates an **automatic safety backup before every non-`init` command.**
> *Auto-prune (`backup --prune <N>`) lands in Q2.*

---

## Upgrading

```bash
cargo install --path . --force   # rebuild & replace installed binary
```

Versioned migrations preserve your data across upgrades.

---

## License

MIT – see [`LICENSE`](LICENSE).

