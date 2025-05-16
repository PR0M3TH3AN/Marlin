# Marlin Demo 

Below is the **“hello-world” demo** that matches the current master branch (auto-scan on `marlin init`, no more forced-migration noise, and cleaner build).

---

## 0 Build & install Marlin

```bash
# inside the repo
cargo build --release            # build the new binary
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

*(`cargo install --path . --locked --force` works too if you prefer.)*

---

## 1 Create the demo tree

```bash
rm -rf ~/marlin_demo
mkdir -p ~/marlin_demo/{Projects/{Alpha,Beta,Gamma},Logs,Reports,Scripts,Media/Photos}

# Projects
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
- ACTION: follow up with design team
EOF

cat <<EOF > ~/marlin_demo/Projects/Beta/final.md
# Beta Final

All tasks complete. Ready to ship!
EOF

cat <<EOF > ~/marlin_demo/Projects/Gamma/TODO.txt
Gamma tasks:

TODO: refactor module Y
EOF

# Logs
echo "2025-05-15 12:00:00 INFO Starting app"   > ~/marlin_demo/Logs/app.log
echo "2025-05-15 12:01:00 ERROR Oops, crash"     >> ~/marlin_demo/Logs/app.log
echo "2025-05-15 00:00:00 INFO System check OK" > ~/marlin_demo/Logs/system.log

# Reports
printf "Q1 financials
" > ~/marlin_demo/Reports/Q1_report.pdf

# Scripts
cat <<'EOF' > ~/marlin_demo/Scripts/deploy.sh
#!/usr/bin/env bash
echo "Deploying version $1..."
EOF
chmod +x ~/marlin_demo/Scripts/deploy.sh

# Media
echo "JPEGDATA" > ~/marlin_demo/Media/Photos/event.jpg
```

*(copy the file-creation block from your original instructions — nothing about the files needs to change)*

---

## 2 Initialise **and** index (one step)

`marlin init` now performs a first-time scan of whatever directory you run it in.
So just:

```bash
cd ~/marlin_demo          # <-- important: run init from the folder you want indexed
marlin init
```

That will:

1. create/upgrade the DB,
2. run all migrations exactly once,
3. walk the current directory and ingest every file it finds.

Need to add more paths later? Use `marlin scan <dir>` exactly as before.

---

## 3 Tagging examples

```bash
# Tag all project markdown as “project/md”
marlin tag "~/marlin_demo/Projects/**/*.md" project/md

# Tag your logs
marlin tag "~/marlin_demo/Logs/**/*.log" logs/app

# Tag everything under Projects/Beta as “project/beta”
marlin tag "~/marlin_demo/Projects/Beta/**/*" project/beta
```

---

## 4 Set custom attributes

```bash
marlin attr set "~/marlin_demo/Projects/Beta/final.md"   status  complete
marlin attr set "~/marlin_demo/Reports/*.pdf"            reviewed yes
```

---

## 5 Play with search / exec hooks

```bash
marlin search TODO
marlin search tag:project/md
marlin search "tag:logs/app AND ERROR"
marlin search "attr:status=complete"
marlin search "attr:reviewed=yes AND pdf"
marlin search "attr:reviewed=yes" --exec 'xdg-open {}'
```

---

## 6 JSON output & verbose mode

```bash
marlin --format=json attr ls ~/marlin_demo/Projects/Beta/final.md
marlin --verbose      scan     ~/marlin_demo         # re-scan to see debug logs
```

---

## 7 Snapshot & restore

```bash
snap=$(marlin backup | awk '{print $NF}')
rm ~/.local/share/marlin/index.db           # simulate disaster
marlin restore "$snap"
marlin search TODO                          # should still work
```

---

## 8 Linking demo

```bash
touch ~/marlin_demo/foo.txt ~/marlin_demo/bar.txt
marlin scan ~/marlin_demo                   # index the new files

foo=~/marlin_demo/foo.txt
bar=~/marlin_demo/bar.txt

marlin link add        "$foo" "$bar"        # create link
marlin link list       "$foo"               # outgoing links from foo
marlin link backlinks  "$bar"               # incoming links to  bar
```

---

### Recap

* `cargo build --release` + `sudo install …` is still the build path.
* **`cd` to the folder you want indexed and run `marlin init`** — first scan happens automatically.
* Subsequent scans (`marlin scan …`) are only needed for *new* directories you add later.
* No more “forcing reapplication of migration 4” banner and the unused-import warnings are gone.

Happy organising!
