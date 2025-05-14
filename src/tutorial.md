# Marlin Usage Tutorial

Below is a hands-on lab you can run in a throw-away directory.
It shows how Marlin’s **tags** give you cross-folder “links” that a plain Bash
workflow can’t match without resorting to symlinks or scratch scripts.

Everything uses *only the functionality that exists today* (`init / scan / tag`)
plus one `sqlite3` query for discovery.

---

## 0 .  Prep

```bash
# make a playground so we don't touch real files
mkdir -p ~/marlin_demo/{Projects/{Alpha,Beta},Media/Photos,Docs}
cd ~/marlin_demo
```

### Create a handful of files

```bash
echo "Alpha draft"            > Projects/Alpha/draft.txt
echo "Alpha final"            > Projects/Alpha/final.txt
echo "Beta summary"           > Projects/Beta/summary.md
echo "Budget spreadsheet"     > Docs/budget.ods
echo "Scan of receipt"        > Docs/receipt.pdf
echo "fake JPEG header"       > Media/Photos/vacation001.jpg
echo "fake JPEG header"       > Media/Photos/vacation002.jpg
```

---

## 1 .  Initialise & scan

```bash
marlin init
marlin scan ~/marlin_demo
```

*What happened?*
Marlin walked every file under `~/marlin_demo` and upserted rows into `files`.

---

## 2 .  Tagging – adding cross-folder metadata

### Tag Alpha project files

```bash
marlin tag "~/marlin_demo/Projects/Alpha/**/*.txt" project-alpha
```

### Tag everything Markdown or ODS as **docs**

```bash
marlin tag "~/marlin_demo/**/*.md" docs
marlin tag "~/marlin_demo/**/*.ods" docs
```

### Tag photos

```bash
marlin tag "~/marlin_demo/Media/Photos/**/*.jpg" photos
```

You can layer tags—`vacation001.jpg` now has both `photos` and (later) `trip-2024`
if you choose to add that.

---

## 3 .  Discovering files with plain SQL

There’s no `marlin search` command *yet*, but the DB is just SQLite:

```bash
sqlite3 ~/.local/share/marlin/index.db <<'SQL'
.headers on
.mode column

-- show all files tagged 'docs'
SELECT path
FROM   files f
JOIN   file_tags ft ON ft.file_id = f.id
JOIN   tags t       ON t.id       = ft.tag_id
WHERE  t.name = 'docs';
SQL
```

Expected output:

```
path
--------------------------------------------------------------
/home/user/marlin_demo/Projects/Beta/summary.md
/home/user/marlin_demo/Docs/budget.ods
```

Do the same for `project-alpha`:

```bash
sqlite3 ~/.local/share/marlin/index.db "
SELECT path FROM files
JOIN file_tags USING(file_id)
JOIN tags      USING(tag_id)
WHERE tags.name = 'project-alpha';
"
```

---

## 4 .  Why this beats a pure Bash approach

| Task                                                                 | With Bash alone                                                                 | With Marlin tags                                                              |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Gather every Alpha file (any extension) scattered across sub-folders | `find ~/Projects -path '*Alpha*'` (works) but blows up if naming scheme changes | One-time glob + `marlin tag ... project-alpha`, then just query by tag.       |
| Re-classify files later                                              | Mass-rename or new `find`/`grep` pipeline                                       | `marlin tag` new glob or manual ad-hoc files; DB keeps history (future).      |
| Combine orthogonal facets e.g. “docs AND project-alpha”              | Complex `find` piped to `grep -F -f list.txt` or symlink forest                 | Future `marlin search docs AND project-alpha` (for now SQL query).            |
| Persist metadata when files move                                     | Must update symlinks / scripts                                                  | Scanner sees the move (once watcher lands); tags stay attached by inode/hash. |

Think of tags as **Git branches for files**—cheap, many-to-many, roam across
directories, and live in one place.

---

## 5 .  Cleaning up

```bash
rm -rf ~/marlin_demo
sqlite3 ~/.local/share/marlin/index.db "DELETE FROM files; DELETE FROM tags; DELETE FROM file_tags;"
```

*(or simply delete the DB file to start fresh).*

---

### Recap

1. **Scan** every folder once.
2. **Tag** by glob to create semantic “links.”
3. **Query** the DB (today) or use future built-in search (soon).

Even with just these three commands, you get an index that answers questions
plain Bash would need an ever-growing tangle of `find`, `grep`, and symlinks to solve.
