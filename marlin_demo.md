# Marlin Demo

Here’s a little demo you can spin up to exercise tags, attributes, FTS queries, `--exec` hooks, backups & restores, and linking. Just copy–paste each block into your terminal:

---

### 0 Create the demo folder and some files

```bash
cargo build --release
```

```bash
sudo install -Dm755 target/release/marlin /usr/local/bin/marlin
```

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

---

### 1 Initialize & index

```bash
marlin init
marlin scan ~/marlin_demo
```

---

### 2 Attach hierarchical tags

```bash
# Tag all project markdown as “project/md”
marlin tag "~/marlin_demo/Projects/**/*.md" project/md

# Tag your logs
marlin tag "~/marlin_demo/Logs/**/*.log" logs/app

# Tag everything under Projects/Beta as “project/beta”
marlin tag "~/marlin_demo/Projects/Beta/**/*" project/beta
```

---

### 3 Set custom attributes

```bash
# Mark only the “final.md” as complete
marlin attr set "~/marlin_demo/Projects/Beta/final.md" status complete

# Mark PDF as reviewed
marlin attr set "~/marlin_demo/Reports/*.pdf" reviewed yes
```

---

### 4 Play with search

```bash
# Find all TODOs (in any file)
marlin search TODO

# All markdown under your “project/md” tag
marlin search tag:project/md

# All files tagged “logs/app” containing ERROR
marlin search "tag:logs/app AND ERROR"

# Only your completed Beta deliverable
marlin search "attr:status=complete"

# Only reviewed PDFs
marlin search "attr:reviewed=yes AND pdf"

# Open every reviewed report
marlin search "attr:reviewed=yes" --exec 'xdg-open {}'
```

---

### 5 Try JSON output & verbose mode

```bash
marlin --format=json attr ls ~/marlin_demo/Projects/Beta/final.md
marlin --verbose scan ~/marlin_demo
```

---

### 6 Snapshot & restore

```bash
# Snapshot
snap=$(marlin backup | awk '{print $NF}')

# Delete your DB to simulate data loss
rm ~/.local/share/marlin/index.db

# Bring it back
marlin restore "$snap"

# Confirm you still see “TODO”
marlin search TODO
```

---

### 7 Test linking functionality

```bash
# Create two demo files
touch ~/marlin_demo/foo.txt ~/marlin_demo/bar.txt

# Re-scan to index new files
marlin scan ~/marlin_demo

# Link foo.txt → bar.txt
foo=~/marlin_demo/foo.txt
bar=~/marlin_demo/bar.txt
marlin link add "$foo" "$bar"

# List outgoing links for foo.txt
marlin link list "$foo"

# List incoming links (backlinks) to bar.txt
marlin link backlinks "$bar"
```

---

That gives you:

* **wide folder structures** (Projects, Logs, Reports, Scripts, Media)
* **hierarchical tags** you can mix and match
* **key-value attributes** to flag state & review
* **FTS5 queries** with AND/OR/NOT
* **`--exec` hooks** to trigger external commands
* **JSON output** for programmatic gluing
* **backups & restores** to guard your data
* **file-to-file links** for graph relationships

Have fun playing around!
