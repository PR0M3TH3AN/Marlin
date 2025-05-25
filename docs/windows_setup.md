# Marlin on Windows

This short guide covers a few tasks when running Marlin on Windows:

1. **Running `marlin init`.**
2. **Moving and renaming files.**
3. **Verifying that tags and attributes stay linked.**
4. **Checking watcher performance under heavy activity.**

---

## 1 Run `marlin init`

1. Open *PowerShell* or *Command Prompt*.
2. Navigate to the directory you want indexed, e.g.
   ```powershell
   cd C:\Users\You\Documents\Project
   ```
3. Run `marlin init` from that folder. The command creates the database and performs the first scan.

---

## 2 Move and rename files

Windows Explorer renames and moves are tracked automatically when the watcher is running.

1. Start the watcher in a terminal:
   ```powershell
   marlin watch start C:\Users\You\Documents\Project
   ```
2. Move or rename files/directories through Explorer or the `move` command.
3. The watcher logs the operations and updates the database.

---

## 3 Verify tags and attributes

After moving or renaming files, confirm that metadata stayed linked:

```powershell
marlin search "tag:mytag"         # paths should reflect new locations
marlin attr get path/to/file.txt   # attributes move with the file
```

If anything is missing, run a manual dirty scan:
```powershell
marlin scan --dirty C:\Users\You\Documents\Project
```

---

## 4 Check watcher performance

To stress-test the watcher under many events:

1. Open another terminal window and create a burst of file operations:
   ```powershell
   1..1000 | % { New-Item -Path test$_ -ItemType File }
   1..1000 | % { Remove-Item test$_ }
   ```
2. Watch the original terminal for log messages and ensure the watcher keeps up without large delays.
3. For a longer test, let the watcher run overnight while copying or deleting large trees.

---

*End of guide*

