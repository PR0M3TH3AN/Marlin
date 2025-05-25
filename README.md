# Marlin

This repository hosts the Marlin indexing tool.
See [docs/roadmap.md](docs/roadmap.md) and
[docs/adr/DP-001_schema_v1.1.md](docs/adr/DP-001_schema_v1.1.md)
for the current delivery roadmap and schema.

## Quick Start

Follow the short walkthrough in
[docs/marlin_demo.md](docs/marlin_demo.md) to build the
binary and test Marlin on a sample project. Paths in the
database are always stored with forward slashes (`/`), even
on Windows.

```powershell
# PowerShell build example
$env:CARGO_TARGET_DIR = "target"
cargo build --release
Copy-Item target\release\marlin.exe C:\Tools\marlin.exe
```

## CLI Cheatsheet

The full command reference is generated during the build of the CLI. See
[cli-bin/docs/cli_cheatsheet.md](cli-bin/docs/cli_cheatsheet.md).

## Collections and Views

Named **collections** act like playlists of files. Create one with
`marlin coll create <name>`, add files via
`marlin coll add <name> <pattern>` and list contents using
`marlin coll list <name>`.

**Views** save search queries for quick reuse. Save a query with
`marlin view save <view> "tag:todo"`, list all views using
`marlin view list` and execute one with `marlin view exec <view>`.

Other handy commands include:

- `marlin watch <dir>` to keep the index updated in real time.
- `marlin backup run` to create or prune database backups.
- `marlin link add` to relate files with typed edges.
- `marlin annotate add` to attach notes or highlights.

## License

Licensed under the [MIT License](LICENSE).
