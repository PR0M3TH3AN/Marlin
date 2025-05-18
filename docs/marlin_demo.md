# Marlin Demo 🚀

Below is a **“hello-world” walk-through** that matches the current `main`
branch (auto-scan on `marlin init`, no more forced-migration chatter, cleaner
build). Everything runs offline on a throw-away directory under `~/marlin_demo`.

---

## 0 Build & install Marlin

```bash
# inside the repo
export CARGO_TARGET_DIR=target      # <-- speeds up future builds (once)
cargo build --release               # build the new binary
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
#   (cargo install --path . --locked --force  works too)
````

---

## 1 Create the demo tree

```bash
rm -rf ~/marlin_demo
mkdir -p ~/marlin_demo/{Projects/{Alpha,Beta,Gamma},Logs,Reports,Scripts,Media/Photos}
# (zsh users: quote the pattern or enable braceexpand first)

# ── Projects ───────────────────────────────────────────────────
cat <<EOF > ~/marlin_demo/Projects/Alpha/draft1.md
# Alpha draft 1
- [ ] TODO: outline architecture
- [ ] TODO: write tests
EOF
cat <<EOF > ~/marlin_demo/Projects/Alpha/draft2.md
# Alpha draft 2
- [x] TODO: outline architecture
- [ ] TODO: implement feature X
EOF
cat <<EOF > ~/marlin_demo/Projects/Beta/notes.md
Beta meeting notes:

- decided on roadmap
- ACTION: follow-up with design team
EOF
cat <<EOF > ~/marlin_demo/Projects/Beta/final.md
# Beta Final
All tasks complete. Ready to ship!
EOF
cat <<EOF > ~/marlin_demo/Projects/Gamma/TODO.txt
Gamma tasks:
TODO: refactor module Y
EOF

# ── Logs & Reports ─────────────────────────────────────────────
echo "2025-05-15 12:00:00 INFO  Starting app"  >  ~/marlin_demo/Logs/app.log
echo "2025-05-15 12:01:00 ERROR Oops, crash"   >> ~/marlin_demo/Logs/app.log
echo "2025-05-15 00:00:00 INFO  System check OK" > ~/marlin_demo/Logs/system.log
printf "Q1 financials\n" > ~/marlin_demo/Reports/Q1_report.pdf

# ── Scripts & Media ────────────────────────────────────────────
cat <<'EOF' > ~/marlin_demo/Scripts/deploy.sh
#!/usr/bin/env bash
echo "Deploying version $1…"
EOF
chmod +x ~/marlin_demo/Scripts/deploy.sh
echo "JPEGDATA" > ~/marlin_demo/Media/Photos/event.jpg
```

---

## 2 Initialise **and** index (one step)

```bash
cd ~/marlin_demo          # run init from the folder you want indexed
marlin init               # • creates or migrates DB
                          # • runs *first* full scan of this directory
```

Add more directories later with `marlin scan <dir>`.

---

## 3 Tagging examples

```bash
# Tag all project markdown as ‘project/md’
marlin tag '~/marlin_demo/Projects/**/*.md' project/md

# Tag your logs
marlin tag '~/marlin_demo/Logs/**/*.log' logs/app

# Tag everything under Beta as ‘project/beta’
marlin tag '~/marlin_demo/Projects/Beta/**/*' project/beta
```

---

## 4 Set custom attributes

```bash
marlin attr set '~/marlin_demo/Projects/Beta/final.md' status  complete
marlin attr set '~/marlin_demo/Reports/*.pdf'          reviewed yes
```

---

## 5 Play with search / exec hooks

```bash
marlin search TODO
marlin search tag:project/md
marlin search 'tag:logs/app AND ERROR'
marlin search 'attr:status=complete'
marlin search 'attr:reviewed=yes AND pdf'
marlin search 'attr:reviewed=yes' --exec 'xdg-open {}'
marlin --format=json search 'attr:status=complete'     # machine-readable output
```

---

## 6 Verbose mode

```bash
marlin --verbose scan ~/marlin_demo     # watch debug logs stream by
```

---

## 7 Snapshot & restore

```bash
snap=$(marlin backup | awk '{print $NF}')
rm ~/.local/share/marlin/index.db       # simulate disaster
marlin restore "$snap"
marlin search TODO                      # still works
```

*(Reminder: Marlin also makes an **auto-backup** before every non-`init`
command, so manual snapshots are extra insurance.)*

---

## 8 Linking demo

```bash
touch ~/marlin_demo/foo.txt ~/marlin_demo/bar.txt
marlin scan ~/marlin_demo                        # index the new files

foo=~/marlin_demo/foo.txt
bar=~/marlin_demo/bar.txt

marlin link add "$foo" "$bar" --type references  # create typed link
marlin link list "$foo"                          # outgoing links from foo
marlin link backlinks "$bar"                     # incoming links to bar
```

---

## 9 Collections & smart views

```bash
# Collection
marlin coll create SetA
marlin coll add    SetA '~/marlin_demo/Projects/**/*.md'
marlin coll list   SetA

# Saved view (smart folder)
marlin view save tasks 'attr:status=complete OR TODO'
marlin view exec tasks
```

---

### Recap

* `cargo build --release` + `sudo install …` is still the build path.
* **`marlin init`** scans the **current working directory** on first run.
* Scan again only when you add *new* directories (`marlin scan …`).
* Auto-backups happen before every command; manual `marlin backup` gives you extra restore points.

Happy organising!

```