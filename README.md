![Marlin Logo](https://raw.githubusercontent.com/PR0M3TH3AN/Marlin/refs/heads/main/assets/png/marlin_logo.png)

# Marlin

**Marlin** is a lightweight, metadata-driven file indexer that runs 100 % on your computer. It scans folders, stores paths and file stats in SQLite, lets you attach hierarchical **tags** and **custom attributes**, takes automatic snapshots, and offers instant full-text search via FTS5.  
_No cloud, no telemetry – your data never leaves the machine._

---

## Feature highlights

| Area           | What you get                                                                      |
| -------------- | --------------------------------------------------------------------------------- |
| **Safety**     | Timestamped backups (`marlin backup`) and one-command restore (`marlin restore`)  |
| **Resilience** | Versioned, idempotent schema migrations – zero-downtime upgrades                  |
| **Indexing**   | Fast multi-path scanner with SQLite WAL concurrency                               |
| **Metadata**   | Hierarchical tags (`project/alpha`) & key-value attributes (`reviewed=yes`)       |
| **Search**     | Prefix-aware FTS5 across paths, tags, and attributes; optional `--exec` per match |
| **DX / Logs**  | Structured tracing (`RUST_LOG=debug`) for every operation                         |

---

## How it works

```text
┌──────────────┐  marlin scan          ┌─────────────┐
│  your files  │ ─────────────────────▶│   SQLite    │
│ (any folder) │                      │  files/tags │
└──────────────┘   tag / attr          │ attrs / FTS │
        ▲  search / exec              └──────┬──────┘
        └────────── backup / restore          ▼
                                     timestamped snapshots
```

---

## Prerequisites

| Requirement        | Why                           |
| ------------------ | ----------------------------- |
| **Rust** ≥ 1.77    | Build toolchain (`rustup.rs`) |
| C build essentials | Builds bundled SQLite (Linux) |

macOS & Windows users: let the Rust installer pull the matching build tools.

---

## Build & install

```bash
git clone https://github.com/yourname/marlin.git
cd marlin
cargo build --release

# (Optional) Install the binary into your PATH:
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

## Quick start

For a concise walkthrough, see [Quick start & Demo](marlin_demo.md).

## Testing 

Below is a **repeat-able 3-step flow** you can use **every time you pull fresh code**.

---

### 0  Prepare once

```bash
# Run once (or add to ~/.bashrc) so debug + release artefacts land
# in the same predictable place.  Speeds-up future builds.
export CARGO_TARGET_DIR=target
```

---

### 1  Build the new binary

```bash
git pull             # grab the latest commit
cargo build --release
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

* `cargo build --release` – builds the optimised binary.
* `install …` – copies it into your `$PATH` so `marlin` on the CLI is the fresh one.

---

### 2  Run the smoke-test suite

```bash
# Runs the end-to-end test we added in tests/e2e.rs
cargo test --test e2e -- --nocapture
```

* `--test e2e` – compiles and runs **only** `tests/e2e.rs`; other unit-tests are skipped (add them later if you like).
* `--nocapture` – streams stdout/stderr so you can watch each CLI step in real time.
* Exit-code **0** ➜ everything passed.
  Any non-zero exit or a red ✗ line means a step failed; the assert’s diff will show the command and its output.

---

### 3  (Optionally) run all tests

```bash
cargo test --all -- --nocapture
```

This will execute:

* unit tests in `src/**`
* every file in `tests/`
* doc-tests

If you wire **“cargo test --all”** into CI (GitHub Actions, GitLab, etc.), pushes that break a workflow will be rejected automatically.

---

#### One-liner helper (copy/paste)

```bash
git pull && cargo build --release &&
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin &&
cargo test --test e2e -- --nocapture
```

Stick that in a shell alias (`alias marlin-ci='…'`) and you’ve got a 5-second upgrade-and-verify loop.

### Database location

* **Linux**  `~/.local/share/marlin/index.db`
* **macOS**  `~/Library/Application Support/marlin/index.db`
* **Windows** `%APPDATA%\marlin\index.db`

Override with:

```bash
export MARLIN_DB_PATH=/path/to/custom.db
```

---

## CLI reference

```text
marlin <COMMAND> [ARGS]

init                             create / migrate database
scan   <PATHS>...                walk directories & index files
tag    "<glob>" <tag_path>       add hierarchical tag
attr   set <pattern> <key> <value>  manage custom attributes
attr   ls <path>
search <query> [--exec CMD]      FTS5 query, optionally run CMD on each hit
backup                           create timestamped snapshot in backups/
restore <snapshot.db>            replace DB with snapshot
completions <shell>              generate shell completions
```

### Attribute subcommands

| Command    | Example                                        |
| ---------- | ---------------------------------------------- |
| `attr set` | `marlin attr set ~/Docs/**/*.pdf reviewed yes` |
| `attr ls`  | `marlin attr ls ~/Docs/report.pdf`             |

---

## Backups & restore

**Create snapshot**

```bash
marlin backup
# → ~/.local/share/marlin/backups/backup_2025-05-14_22-15-30.db
```

**Restore snapshot**

```bash
marlin restore ~/.local/share/marlin/backups/backup_2025-05-14_22-15-30.db
```

Marlin also takes an **automatic safety backup before every non-init command**.

---

## Upgrading

```bash
cargo install --path . --force    # rebuild & replace installed binary
```

The versioned migration system preserves your data across upgrades.

---

## License

MIT – see `LICENSE`
