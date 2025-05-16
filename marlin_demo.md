# Quick start & Demo

## Quick start

```bash
# initialize the demo database
marlin init

# index only your demo folder
marlin scan ~/marlin_demo_complex

# tag all markdown in your demo Projects as “project/md”
marlin tag "~/marlin_demo_complex/Projects/**/*.md" project/md

# mark your demo reports as reviewed
marlin attr set "~/marlin_demo_complex/Reports/*.pdf" reviewed yes

# search for any reviewed files
marlin search "attr:reviewed=yes"

# snapshot the demo database
marlin backup

# test linking within your demo
touch ~/marlin_demo_complex/foo.txt ~/marlin_demo_complex/bar.txt
marlin scan ~/marlin_demo_complex
foo=~/marlin_demo_complex/foo.txt
bar=~/marlin_demo_complex/bar.txt
marlin link add "$foo" "$bar"
marlin link list "$foo"
marlin link backlinks "$bar"
````

---

# Marlin Demo

Here’s a little “complex‐demo” you can spin up to exercise tags, attributes, FTS queries, `--exec` hooks, backups & restores. Just copy–paste each block into your terminal:

### 0 Create the demo folder and some files

```bash
rm -rf ~/marlin_demo_complex
mkdir -p ~/marlin_demo_complex/{Projects/{Alpha,Beta,Gamma},Logs,Reports,Scripts,Media/Photos}

# Projects
cat <<EOF > ~/marlin_demo_complex/Projects/Alpha/draft1.md
# Alpha draft 1

- [ ] TODO: outline architecture
- [ ] TODO: write tests
EOF

cat <<EOF > ~/marlin_demo_complex/Projects/Alpha/draft2.md
# Alpha draft 2

- [x] TODO: outline architecture
- [ ] TODO: implement feature X
EOF

cat <<EOF > ~/marlin_demo_complex/Projects/Beta/notes.md
Beta meeting notes:

- decided on roadmap
- ACTION: follow up with design team
EOF

cat <<EOF > ~/marlin_demo_complex/Projects/Beta/final.md
# Beta Final

All tasks complete. Ready to ship!
EOF

cat <<EOF > ~/marlin_demo_complex/Projects/Gamma/TODO.txt
Gamma tasks:

TODO: refactor module Y
EOF

# Logs
echo "2025-05-15 12:00:00 INFO Starting app"   > ~/marlin_demo_complex/Logs/app.log
echo "2025-05-15 12:01:00 ERROR Oops, crash"     >> ~/marlin_demo_complex/Logs/app.log
echo "2025-05-15 00:00:00 INFO System check OK" > ~/marlin_demo_complex/Logs/system.log

# Reports
printf "Q1 financials\n" > ~/marlin_demo_complex/Reports/Q1_report.pdf

# Scripts
cat <<'EOF' > ~/marlin_demo_complex/Scripts/deploy.sh
#!/usr/bin/env bash
echo "Deploying version $1..."
EOF
chmod +x ~/marlin_demo_complex/Scripts/deploy.sh

# Media
echo "JPEGDATA" > ~/marlin_demo_complex/Media/Photos/event.jpg
```

---

### 1 Initialize & index

```bash
marlin init
marlin scan ~/marlin_demo_complex
```

---

### 2 Attach hierarchical tags

```bash
# Tag all project markdown as “project/md”
marlin tag "~/marlin_demo_complex/Projects/**/*.md" project/md

# Tag your logs
marlin tag "~/marlin_demo_complex/Logs/**/*.log" logs/app

# Tag everything under Projects/Beta as “project/beta”
marlin tag "~/marlin_demo_complex/Projects/Beta/**/*" project/beta
```

---

### 3 Set custom attributes

```bash
# Mark only the “final.md” as complete
marlin attr set "~/marlin_demo_complex/Projects/Beta/final.md" status complete

# Mark PDF as reviewed
marlin attr set "~/marlin_demo_complex/Reports/*.pdf" reviewed yes
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
marlin --format=json attr ls ~/marlin_demo_complex/Projects/Beta/final.md
marlin --verbose scan ~/marlin_demo_complex
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