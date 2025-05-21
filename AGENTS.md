# Marlin – Contributor Guidelines

This project follows a lightweight “spec first” workflow with strict CI gates.
Follow the instructions below so your PRs can merge cleanly.

## Workflow

- **Branching** – trunk‑based. Work in a feature branch, open a PR, obtain two
  reviews, then squash‑merge.
- **Design Proposals** – any major feature or epic starts with a DP‑xxx document
  in `docs/adr/` describing schema changes, example CLI output and performance
  targets.
- **Coverage gate** – Tarpaulin must report ≥ 85 % coverage on lines touched in a
  sprint. CI fails otherwise.
- **Performance gate** – cold start P95 ≤ 3 s on a 100 k file corpus (unless the
  relevant DP states a different budget). CI benchmarks enforce this.
- **Documentation** – update `README.md` and the auto‑generated CLI cheatsheet in
  the same PR that adds or changes functionality.
- **Demo** – closing an epic requires a ≤ 2‑min asciinema or GIF committed under
  `docs/demos/`.

## Coding standards

- Run `cargo fmt --all -- --check` and `cargo clippy -- -D warnings`
  before committing.
- Internal logging uses `tracing` (`info!`, `warn!` etc.); avoid `println!`
  except in CLI output.
- Handle mutex poisoning and other errors with `anyhow::Result` rather than
  panicking.
- Ensure every text file ends with a single newline.
- Generated coverage reports (`cobertura.xml`, `tarpaulin-report.html`) and
  other artifacts listed in `.gitignore` must not be checked in.

## Testing

- Execute `./run_all_tests.sh` locally before pushing. It builds the CLI,
  runs unit and integration tests across crates, performs benchmarks and
  exercises demo flows.
- CI replicates these steps and uploads benchmark and coverage artifacts.

## Commit and PR etiquette

- Use concise, imperative commit messages (e.g. “Add file watcher debouncer”).
  Reference the relevant DP or issue in the body if applicable.
- PRs should link to the associated DP or issue, include documentation updates
  and—when closing an epic—a short asciinema/GIF demo.
