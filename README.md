# Marlin

This repository hosts the Marlin indexing tool.
See [docs/roadmap.md](docs/roadmap.md) and
[docs/adr/DP-001_schema_v1.1.md](docs/adr/DP-001_schema_v1.1.md)
for the current delivery roadmap and schema.

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

## License

Licensed under the [MIT License](LICENSE).
