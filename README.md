![Marlin Logo](https://raw.githubusercontent.com/PR0M3TH3AN/Marlin/refs/heads/main/assets/png/marlin_logo.png)

# Marlin

**Marlin** is a lightweight, metadata-driven file indexer that runs 100 % on your computer. It scans folders, stores paths and file stats in SQLite, lets you attach hierarchical **tags** and **custom attributes**, takes automatic snapshots, and offers instant full-text search via FTS5.
*No cloud, no telemetry – your data never leaves the machine.*

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
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin  # optional
```

---

## Quick start

```bash
marlin init                                        # create DB (idempotent)
marlin scan ~/Pictures ~/Documents                 # index files
marlin tag  "~/Pictures/**/*.jpg" photos/trip-2024 # add tag
marlin attr set "~/Documents/**/*.pdf" reviewed yes
marlin search reviewed --exec "xdg-open {}"        # open matches
marlin backup                                      # snapshot DB
```

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
attr   set|ls …                  manage custom attributes
search <query> [--exec CMD]      FTS query, optionally run CMD on each hit
backup                           create timestamped snapshot in backups/
restore <snapshot.db>            replace DB with snapshot
```

### Attribute subcommands

| Command    | Example                                          |
| ---------- | ------------------------------------------------ |
| `attr set` | `marlin attr set "~/Docs/**/*.pdf" reviewed yes` |
| `attr ls`  | `marlin attr ls ~/Docs/report.pdf`               |

---

## Backups & restore

*Create snapshot*

```bash
marlin backup
# → ~/.local/share/marlin/backups/backup_2025-05-14_22-15-30.db
```

*Restore snapshot*

```bash
marlin restore ~/.local/share/marlin/backups/backup_2025-05-14_22-15-30.db
```

Marlin also takes an **automatic safety backup before every schema migration**.

---

## Upgrading

```bash
cargo install --path . --force    # rebuild & replace installed binary
```

The versioned migration system preserves your data across upgrades.

---

## Roadmap

See [`ROADMAP.md`](./ROADMAP.md) for the full development plan.

---

## Five-Minute Quickstart

Paste & run each block in your terminal.

---

### 0 Prepare & build

```bash
# Clone or cd into your Marlin repo
cd ~/Documents/GitHub/Marlin

# Build the release binary
cargo build --release
```

---

### 1 Install on your PATH

```bash
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

> Now `marlin` is available everywhere.

---

### 2 Prepare a clean demo directory

```bash
rm -rf ~/marlin_demo
mkdir -p ~/marlin_demo/{Projects/{Alpha,Beta},Media/Photos,Docs}

printf "Alpha draft\n"  > ~/marlin_demo/Projects/Alpha/draft.txt
printf "Beta notes\n"   > ~/marlin_demo/Projects/Beta/notes.md
printf "Receipt PDF\n"  > ~/marlin_demo/Docs/receipt.pdf
printf "fake jpg\n"     > ~/marlin_demo/Media/Photos/vacation.jpg
```

---

### 3 Initialize & index files

```bash
# Use --verbose if you want full debug traces:
marlin init
marlin scan ~/marlin_demo

# or, to see every path tested:
marlin --verbose init
marlin --verbose scan ~/marlin_demo
```

> **Tip:** Rerun `marlin scan` after you add/remove/modify files; only changed files get re-indexed.

---

### 4 Attach tags & attributes

```bash
# Tag everything under “Alpha”
marlin tag "~/marlin_demo/Projects/Alpha/**/*" project/alpha

# Mark all PDFs as reviewed
marlin attr set "~/marlin_demo/**/*.pdf" reviewed yes

# (or with debug)
marlin --verbose tag "~/marlin_demo/Projects/Alpha/**/*" project/alpha
marlin --verbose attr set "~/marlin_demo/**/*.pdf" reviewed yes
```

---

### 5 Search your index

```bash
# By tag or filename
marlin search alpha

# Combined terms (AND across path+attrs)
marlin search "reviewed AND pdf"

# Run a command on each hit
marlin search reviewed --exec "echo HIT → {}"

# If things aren’t matching, add --verbose to see the underlying FTS query:
marlin --verbose search "reviewed AND pdf"
```

> `{}` in `--exec` is replaced with each file’s path.

---

### 6 Backup & restore

```bash
# Snapshot and store its name
snap=$(marlin backup | awk '{print $NF}')

# Simulate data loss
rm ~/.local/share/marlin/index.db

# Restore instantly
marlin restore "$snap"

# Verify your files still show up
marlin search reviewed
```

> Backups live under `~/.local/share/marlin/backups` by default.

##### What you just exercised

| Command           | Purpose                                   |
| ----------------- | ----------------------------------------- |
| `marlin init`     | Create / upgrade the SQLite database      |
| `marlin scan`     | Walk directories and (re)index files      |
| `marlin tag`      | Attach hierarchical tags                  |
| `marlin attr set` | Add/overwrite custom key-value attributes |
| `marlin search`   | FTS5 search across path / tags / attrs    |
| `--exec`          | Pipe hits into any shell command          |
| `marlin backup`   | Timestamped snapshot of the DB            |
| `marlin restore`  | Replace live DB with a chosen snapshot    |

That’s the complete surface area of Marlin today—feel free to play around or
point the scanner at real folders.


---

## License

MIT – see `LICENSE`
