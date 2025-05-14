[Marlin Logo](https://raw.githubusercontent.com/PR0M3TH3AN/Marlin/refs/heads/main/assets/png/marlin_logo.png?token=GHSAT0AAAAAADDJQCM7EIFN3NMAIUGVOUQO2BE7YQA)

# Marlin

**Marlin** is a lightweight, metadata-driven file indexer you run on your own
machine. It scans folders, stores paths and basic stats in a local SQLite
database, and lets you tag files from the command line.

Nothing leaves your computer.

This repo contains the **Sprint-0 foundation**:

* XDG-aware config — no hard-coded paths  
* Embedded SQLite migrations (WAL mode)  
* Fast directory scanner (now accepts *multiple* paths in one call)  
* Simple tagging tool  
* Human-readable logging via `tracing`

---

## How it works

```text
┌──────────────┐  scan dir(s)    ┌─────────────┐
│  your files  │ ───────────────▶│   SQLite    │
└──────────────┘                 │  index.db   │
        ▲  tag <glob> <tag>      │ files tags  │
        └────────────────────────┴─────────────┘
````

1. `marlin scan <PATHS>...` walks each directory tree, gathers size and
   modification time, then upserts rows into **`files`**.
2. `marlin tag "<glob>" <tag>` looks up each matching file row and inserts
   junction rows into **`file_tags`**. New tag names are created on the fly.
3. You can open the DB yourself
   (`sqlite3 ~/.local/share/marlin/index.db`) while search and GUI features
   are still under construction.

---

## Prerequisites

| What             | Why                                                 |
| ---------------- | --------------------------------------------------- |
| **Rust** ≥ 1.77  | Build toolchain (`rustup.rs`)                       |
| Build essentials | `gcc`, `make`, etc. for `rusqlite`’s bundled SQLite |

<details><summary>Platform notes</summary>

### Windows

`rustup-init.exe` installs MSVC build tools automatically.

### macOS

```bash
xcode-select --install        # command-line tools
```

### Linux (Debian / Ubuntu)

```bash
sudo apt install build-essential
```

or on Fedora / RHEL

```bash
sudo dnf groupinstall "Development Tools"
```

</details>

---

## Build & install

```bash
git clone https://github.com/yourname/marlin.git
cd marlin
cargo build --release            # produces target/release/marlin
```

Copy the release binary somewhere on your `PATH` (optional):

```bash
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

---

## Quick start

```bash
# 1 – create or upgrade the database (idempotent)
marlin init

# 2 – index all common folders in one shot
marlin scan ~/Pictures ~/Documents ~/Downloads ~/Music ~/Videos

# 3 – add a tag to matching files
marlin tag "~/Pictures/**/*.jpg" vacation
```

The database path defaults to:

```
~/.local/share/marlin/index.db         # Linux
~/Library/Application Support/marlin   # macOS
%APPDATA%\marlin\index.db              # Windows
```

Override with:

```bash
export MARLIN_DB_PATH=/path/to/custom.db
```

---

## CLI reference

```text
USAGE:
    marlin <COMMAND> [ARGS]

COMMANDS:
    init                              Create (or upgrade) the SQLite database
    scan <PATHS>...                   Walk one or more directories recursively
    tag  "<glob>" <tag>               Apply <tag> to all files matched

FLAGS:
    -h, --help                        Show this help
    -V, --version                     Show version info
```

| Command                     | Notes                                                                                                 |
| --------------------------- | ----------------------------------------------------------------------------------------------------- |
| `marlin init`               | Safe to run repeatedly; applies pending migrations.                                                   |
| `marlin scan <PATHS>...`    | Accepts any number of absolute/relative paths. Directories you can’t read are skipped with a warning. |
| `marlin tag "<glob>" <tag>` | Quote the glob so your shell doesn’t expand it. Uses `glob` crate rules (`**` for recursive matches). |

---

## Upgrading to a new build

During development you’ll be editing source files frequently. Two common ways
to run the updated program:

### 1. Run straight from the project directory

```bash
cargo run --release -- scan ~/Pictures
```

*Cargo recompiles what changed and runs the fresh binary located in
`target/release/marlin`.*

### 2. Replace the global copy

If you previously installed Marlin (e.g. into `~/.cargo/bin/` or `/usr/local/bin/`),
overwrite it:

```bash
cargo install --path . --force
```

Now `which marlin` should print the new location, and multi-path scan works:

```bash
marlin scan ~/Pictures ~/Documents …
```

If the CLI still shows the old single-path usage (`Usage: marlin scan <PATH>`),
you’re invoking an outdated executable—check your `PATH` and reinstall.

---

## Development tips

* Tight loop: `cargo watch -x 'run -- scan ~/Pictures'`
* Debug logs: `RUST_LOG=debug marlin scan ~/Pictures`
* Lint: `cargo clippy --all-targets --all-features -D warnings`
* Tests: `cargo test`

---

## Roadmap

| Milestone | Coming soon                                                     |
| --------- | --------------------------------------------------------------- |
| **M1**    | Hierarchical tags • attributes table • `tags://` virtual folder |
| **M2**    | Sync service • change log • diff viewer                         |
| **M3**    | Natural-language search • visual query builder                  |
| **M4**    | Plug-in marketplace • mobile companion (view-only)              |

---

## License

Released under the **MIT License** – see `LICENSE` for full text.


