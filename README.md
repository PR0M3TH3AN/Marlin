![Marlin Logo](https://raw.githubusercontent.com/PR0M3TH3AN/Marlin/refs/heads/main/assets/png/marlin_logo.png)

# Marlin

**Marlin** is a lightweight, metadata-driven file indexer that runs entirely on your computer.  It scans folders, stores paths and file stats in SQLite, lets you add hierarchical **tags** and **custom attributes**, takes automatic snapshots, and offers instant full-text search with FTS5.  Nothing ever leaves your machine.

---

## Feature highlights

| Area           | What you get                                                                    |
|----------------|---------------------------------------------------------------------------------|
| **Safety**     | Timestamped backups&nbsp;`marlin backup` and one-command restore&nbsp;`marlin restore` |
| **Upgrades**   | Automatic schema migrations + dynamic column adds                               |
| **Indexing**   | Fast multi-path scanner (WAL mode)                                              |
| **Metadata**   | Hierarchical tags (`project/alpha`) & key-value attributes (`reviewed=yes`)     |
| **Search**     | Prefix-aware FTS5, optional `--exec` action per hit                              |
| **DX / Logs**  | Readable tracing (`RUST_LOG=debug …`)                                           |

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
````

---

## Prerequisites

| Requirement        | Why                                    |
| ------------------ | -------------------------------------- |
| **Rust** ≥ 1.77    | Build toolchain (`rustup.rs`)          |
| C build essentials | `gcc`, `make`, etc. for bundled SQLite |

*(Windows/macOS: let the Rust installer pull the matching build tools.)*

---

## Build & install

```bash
git clone https://github.com/yourname/marlin.git
cd marlin
cargo build --release
# optional: add to PATH
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

---

## Quick start

```bash
marlin init                                        # create DB
marlin scan ~/Pictures ~/Documents                 # index files
marlin tag  "~/Pictures/**/*.jpg" photos/trip-2024 # add tag
marlin attr set "~/Documents/**/*.pdf" reviewed yes
marlin search reviewed --exec "xdg-open {}"        # open hits
marlin backup                                      # snapshot DB
```

### Database location

* **Linux**  `~/.local/share/marlin/index.db`
* **macOS** `~/Library/Application Support/marlin/index.db`
* **Windows** `%APPDATA%\marlin\index.db`

Override:

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

* **Create snapshot**

  ```bash
  marlin backup
  # → ~/.local/share/marlin/backups/backup_2025-05-14_22-15-30.db
  ```

* **Restore snapshot**

  ```bash
  marlin restore ~/.local/share/marlin/backups/backup_2025-05-14_22-15-30.db
  ```

Marlin automatically takes a safety backup before any schema migration.

---

## Upgrading to a new build

```bash
cargo install --path . --force    # rebuild & overwrite installed binary
```

Backups + dynamic migrations mean your data is preserved across upgrades.

---

## Roadmap

| Milestone | Focus                                              |
| --------- | -------------------------------------------------- |
| **M1**    | `tags://` virtual folder • attribute search DSL    |
| **M2**    | Real-time sync service • change-log diff viewer    |
| **M3**    | Natural-language query builder                     |
| **M4**    | Plug-in marketplace • mobile (read-only) companion |

---

## Five-minute tutorial

```bash
# 0. Playground
mkdir -p ~/marlin_demo/{Projects/{Alpha,Beta},Media/Photos,Docs}
echo "Alpha draft"  > ~/marlin_demo/Projects/Alpha/draft.txt
echo "Receipt PDF"  > ~/marlin_demo/Docs/receipt.pdf
echo "fake jpg"     > ~/marlin_demo/Media/Photos/vacation.jpg

# 1. Init & scan
marlin init
marlin scan ~/marlin_demo

# 2. Tags & attributes
marlin tag  "~/marlin_demo/Projects/Alpha/**/*"  project/alpha
marlin attr set "~/marlin_demo/**/*.pdf" reviewed yes

# 3. Search
marlin search alpha
marlin search reviewed --exec "echo Found: {}"

# 4. Snapshot & restore
marlin backup
marlin restore ~/.local/share/marlin/backups/backup_YYYY-MM-DD_HH-MM-SS.db
```

---

## License

MIT – see `LICENSE`


