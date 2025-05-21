# Testing

Below is a **repeat-able 3-step flow** you can use **every time you pull fresh code**.

---

## 0  Prepare once

```bash
# Run once (or add to ~/.bashrc) so debug + release artefacts land
# in the same predictable place.  Speeds-up future builds.
export CARGO_TARGET_DIR=target
```

---

## 1  Build the new binary

```bash
git pull             # grab the latest commit
cargo build --release
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

* `cargo build --release` – builds the optimised binary.
* `install …` – copies it into your `$PATH` so `marlin` on the CLI is the fresh one.

---

## 2  Run the smoke-test suite

```bash
# Runs the end-to-end test we added in tests/e2e.rs
cargo test --test e2e -- --nocapture
```

* `--test e2e` – compiles and runs **only** `tests/e2e.rs`; other unit-tests are skipped (add them later if you like).
* `--nocapture` – streams stdout/stderr so you can watch each CLI step in real time.
* Exit-code **0** ➜ everything passed.
  Any non-zero exit or a red ✗ line means a step failed; the assert’s diff will show the command and its output.

---

## 3  (Optionally) run all tests

```bash
cargo test --all -- --nocapture
```

This will execute:

* unit tests in `src/**`
* every file in `tests/`
* doc-tests

If you wire **“cargo test --all”** into CI (GitHub Actions, GitLab, etc.), pushes that break a workflow will be rejected automatically.

---

### One-liner helper (copy/paste)

```bash
cargo build --release &&
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin &&
cargo test --all -- --nocapture
```

or

```bash
./run_all_tests.sh
```

to see test coverage run:

```bash
cargo tarpaulin --out Html
```
