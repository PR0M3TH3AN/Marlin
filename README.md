# Marlin

**Marlin** is a lightweight, metadata-driven file indexer you run on your own machine.
It scans folders, stores paths and basic stats in a local SQLite database, and lets you tag
files from the command line. Nothing leaves your computer.

The goal is to build up toward a full “smart” file explorer (hierarchical tags, custom
attributes, search, sync, etc.). This repo contains the **Sprint 0** foundation:

* XDG-aware config — no hard-coded paths
* Embedded SQLite migrations (WAL mode)
* Fast directory scanner
* Simple tagging tool
* Human-readable logging via `tracing`

---

## How it works

```text
┌──────────────┐  scan           ┌─────────────┐
│  your files  │ ───────────────▶│   SQLite    │
└──────────────┘                 │  index.db   │
        ▲  tag <pattern> <tag>   │ files tags  │
        └────────────────────────┴─────────────┘
```

1. `marlin scan <dir>` walks the directory tree with `walkdir`, gathers size and
   modification time, then upserts rows into `files`.
2. `marlin tag "<glob>" <tag>` resolves the glob, looks up each file row, and inserts
   junction rows into `file_tags`. New tag names are created on the fly.
3. You can query the database yourself (e.g. with `sqlite3 ~/.local/share/marlin/index.db`)
   while higher-level search commands are being built.

---

## Prerequisites

| What             | Why                                                 |
| ---------------- | --------------------------------------------------- |
| **Rust** ≥ 1.77  | Build toolchain (`rustup.rs`)                       |
| Build essentials | `gcc`, `make`, etc. for `rusqlite`’s bundled SQLite |

### Windows

Rust installs MSVC build tools automatically.
SQLite is compiled from source; nothing else to set up.

### macOS

Install Xcode Command-Line Tools:

```bash
xcode-select --install
```

### Linux

Deb-/RPM-based distros:

```bash
sudo apt install build-essential            # or
sudo dnf groupinstall 'Development Tools'
```

---

## Build & install

Clone then build in release mode:

```bash
git clone https://github.com/yourname/marlin.git
cd marlin
cargo build --release
```

The binary is placed at `target/release/marlin`.
Feel free to copy it into a directory on your `PATH`, e.g.:

```bash
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

---

## Quick start

```bash
# 1 - create the database (idempotent)
marlin init

# 2 - index a folder
marlin scan ~/Pictures

# 3 - add a tag to matching files
marlin tag "~/Pictures/**/*.jpg" vacation
```

The database defaults to:

```
~/.local/share/marlin/index.db         # Linux
~/Library/Application Support/marlin   # macOS
%APPDATA%\\marlin\\index.db            # Windows
```

Override with an environment variable:

```bash
export MARLIN_DB_PATH=/path/to/custom.db
```

---

## CLI reference

```text
USAGE:
    marlin <COMMAND> [ARGS]

COMMANDS:
    init                   Create the SQLite database and run migrations
    scan <path>            Walk a directory recursively and index all files found
    tag  <glob> <tag>      Apply <tag> to files matched by <glob>

FLAGS:
    -h, --help             Show this help
    -V, --version          Show version info
```

### Details

| Command                     | Arguments / Notes                                                                                                   |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `marlin init`               | Safe to run more than once; upgrades the DB in place if schema changes.                                             |
| `marlin scan <path>`        | Accepts absolute or relative paths. Ignores directories it can’t read.                                              |
| `marlin tag "<glob>" <tag>` | Use quotes if your shell would otherwise expand the glob. Wildcards follow `glob` crate rules (`**` for recursive). |

---

## Development tips

* `RUST_LOG=debug marlin scan /some/dir` shows per-file indexing messages.
* The integration database for tests lives in `/tmp` and is wiped automatically.
* Run `cargo clippy --all-targets --all-features -D warnings` before opening a PR.

---

## Roadmap

| Milestone | What’s coming next                                         |
| --------- | ---------------------------------------------------------- |
| **M1**    | Hierarchical tags, attributes table, virtual `tags://` URI |
| **M2**    | Sync service, change log, diff viewer                      |
| **M3**    | Natural-language search, visual query builder              |
| **M4**    | Plug-in marketplace, mobile companion (view-only)          |

---

## License

This project is licensed under the **MIT License**. See `LICENSE` for details.
