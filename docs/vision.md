# Marlin – Metadata‑Driven File Explorer

*Version 2 – 12 May 2025*

---

## 1  Key Features & Functionality

| Feature Area                        | Capabilities                                                                                                                                                                                                                                                                                                                             |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Tagging System**                  | • Unlimited, hierarchical or flat tags.<br>• Alias/synonym support with precedence rules (admin‑defined canonical name).<br>• **Bulk tag editing** via multi‑select context menu.<br>• Folder‑to‑Tag import with optional *watch & sync* mode so new sub‑folders inherit tags automatically.                                             |
| **Custom Metadata Attributes**      | • User‑defined fields (text, number, date, enum, boolean).<br>• Per‑template **Custom Metadata Schemas** (e.g. *Photo* → *Date, Location*).                                                                                                                                                                                              |
| **File Relationships**              | • Typed, directional or bidirectional links (*related to*, *duplicate of*, *cites*…).<br>• Plugin API can register new relationship sets.                                                                                                                                                                                                |
| **Version Control for Metadata**    | • Every change logged; unlimited roll‑back.<br>• Side‑by‑side diff viewer and *blame* panel showing *who/when/what*.<br>• Offline edits stored locally and merged (Git‑style optimistic merge with conflict prompts).                                                                                                                    |
| **Advanced Search & Smart Folders** | • Structured query syntax: `tag:ProjectX AND author:Alice`.<br>• Natural‑language search (*"files Alice edited last month"*) with toggle to exact mode.<br>• Visual Query Builder showing live query string.<br>• Saved queries appear as virtual “smart folders” that update in real‑time.                                              |
| **User Interface**                  | • Sidebar: tags, attributes, relationships.<br>• Drag‑and‑drop tagging; inline metadata editor.<br>• Search bar with auto‑complete (Bloom filter backed).<br>• **Dual View Mode** – metadata vs traditional folder; remembers preference per location.<br>• **Interactive 60‑second tour** on first launch plus contextual tooltip help. |
| **Collaboration**                   | • Real‑time metadata sync across devices via cloud or self‑hosted relay.<br>• Conflict handling as per Version Control.<br>• Role‑based permissions (read / write / admin) on tags & attributes.                                                                                                                                         |
| **Performance & Scale**             | • Sharded/distributed index optional for >1 M files.<br>• Query cache with LRU eviction.<br>• Target metrics (100 k files): cold start ≤ 3 s, complex query ≤ 150 ms (stretch 50 ms).                                                                                                                                                    |
| **Backup & Restore**                | • Scheduled encrypted backups; export to JSON / XML.<br>• One‑click restore from any point‑in‑time snapshot.                                                                                                                                                                                                                             |
| **Extensibility**                   | • Plug‑in system (TypeScript/JS) – see §2.4.<br>• Python scripting hook for automation and batch tasks.<br>• REST/IPC API for external tools.                                                                                                                                                                                            |

---

## 2  Technical Implementation

### 2.1  Core Stack

| Component      | Primary Choice                                                                                     | Notes                                                                       |
| -------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| File Manager   | **Dolphin (KDE)** KIO‑based plug‑ins                                                               | GTK users can install a Nautilus extension (feature‑parity subset).         |
| Metadata Store | **SQLite + FTS5** (single‑user) → optional **LiteFS/Postgres** for replication & multi‑user scale. | Per‑row AES‑GCM encryption for sensitive fields; keys stored in OS keyring. |
| Indexer Daemon | Rust service using `notify` (inotify on Linux, FSEvents on macOS).                                 | 100 ms debounce batches, async SQLite writes.                               |
| Cache          | In‑memory LRU + Bloom filter for auto‑complete.                                                    |                                                                             |

### 2.2  Database Schema (simplified)

```text
files(id PK, path, inode, size, mtime, ctime, hash)
tags(id PK, name, parent_id, canonical_id)
file_tags(file_id FK, tag_id FK)
attributes(id PK, file_id FK, key, value, value_type)
relationships(id PK, src_file_id FK, dst_file_id FK, rel_type, direction)
change_log(change_id PK, object_table, object_id, op, actor, ts, payload_json)
```

### 2.3  Sync & Conflict Resolution

1. Each client appends to **change\_log** (CRDT‑compatible delta).
2. Delta sync via WebSocket; server merges and re‑broadcasts.
3. Conflicts → *Conflict Queue* UI (choose theirs / mine / merge).

### 2.4  Plugin API (TypeScript)

```ts
export interface MarlinPlugin {
  onInit(ctx: CoreContext): void;
  extendSchema?(db: Database): void;    // e.g. add new relationship table
  addCommands?(ui: UIContext): void;    // register menus, actions
}
```

Plugins run in a sandboxed process with whitelisted IPC calls.

---

## 3  UX & Accessibility

* **Keyboard‑only workflow** audit (Tab / Shift‑Tab / Space toggles).
* High‑contrast theme; adheres to WCAG 2.1 AA.
* `Ctrl+Alt+V` toggles Dual View.
* Generated query string shown live under Visual Builder – educates power users.

---

## 4  Performance Budget

| Metric                   | MVP       | Stretch    |
| ------------------------ | --------- | ---------- |
| Cold start (100 k files) | ≤ 3 s     | 1 s        |
| Complex AND/OR query     | ≤ 150 ms  | 50 ms      |
| Sustained inserts        | 5 k ops/s | 20 k ops/s |

Benchmarks run nightly; regressions block merge.

---

## 5  Security & Privacy

* **Role‑based ACL** on tags/attributes.
* Per‑change audit trail; logs rotated to cold storage (≥ 90 days online).
* Plugins confined by seccomp/AppArmor; no direct disk/network unless declared.

---

## 6  Packaging & Distribution

* **Flatpak** (GNOME/KDE) and **AppImage** for portable builds.
* Background service runs as a systemd user unit: `--user marlin-indexerd.service`.
* CLI (`marlin-cli`) packaged for headless servers & CI.

---

## 7  Roadmap

| Milestone | Scope                                                                         | Timeline |
| --------- | ----------------------------------------------------------------------------- | -------- |
| **M1**    | Tagging, attributes, virtual folders, SQLite, Dolphin plug‑in                 | 6 weeks  |
| **M2**    | Sync service, version control, CLI                                            | +6 weeks |
| **M3**    | NLP search, Visual Builder, distributed index prototype                       | +6 weeks |
| **M4**    | Plugin marketplace, enterprise auth (LDAP/OIDC), mobile companion (view‑only) | +8 weeks |

---

## 8  Branding

* **Name**: **Marlin** – fast, precise.
* Icon: stylised sailfish fin forming a folder corner.
* Tagline: *“Cut through clutter.”*
* Domain: `marlin‑explorer.io` (availability checked 2025‑05‑12).

---

## 9  Quick‑Win Checklist (Sprint 0)

* [ ] Implement bulk metadata editor UI
* [ ] Write conflict‑resolution spec & unit tests
* [ ] Build diff viewer prototype
* [ ] Keyboard‑only navigation audit
* [ ] Establish performance CI with sample 100 k file corpus

---

---

## 10  Development Plan (Outline)

### 10.1  Process & Methodology

* **Framework** – 2‑week Scrum sprints with Jira backlog, GitHub Projects mirror for public issues.
* **Branching** – Trunk‑based: feature branches → PR → required CI & code‑review approvals (2).*Main* auto‑deploys nightly Flatpak.
* **Definition of Done** – Code + unit tests + docs + passing CI + demo video (for UI work).
* **CI/CD** – GitHub Actions matrix (Ubuntu 22.04, KDE Neon, Fedora 39) → Flatpak / AppImage artefacts, `cargo clippy`, coverage gate ≥ 85 %.

### 10.2  Team & Roles (FTE‑equivalent)

| Role                          | Core Skills                      | Allocation |
| ----------------------------- | -------------------------------- | ---------- |
| Lead Engineer                 | Rust, Qt/Kirigami, KIO           | 1.0        |
| Backend Engineer              | Rust, LiteFS/Postgres, WebSocket | 1.0        |
| Full‑stack / Plug‑in Engineer | TypeScript, Node, IPC            | 0.8        |
| UX / QA                       | Figma, accessibility, Playwright | 0.5        |
| DevOps (fractional)           | CI, Flatpak, security hardening  | 0.2        |

### 10.3  Roadmap → Sprint‑level Tasks

| Sprint                 | Goal                                   | Key Tasks                                                                                                                                                        | Exit Criteria                                                                     |
| ---------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| **S0 (2 wks)**         | Project bootstrap                      | • Repo + CI skeleton<br>• SQLite schema + migrations<br>• `marlin-cli init` & basic scan<br>• Hyperfine perf baseline                                            | CLI scans dir; tests pass; artefact builds                                        |
| **S1–3 (M1, 6 wks)**   | Tagging + virtual folders MVP          | • Indexer daemon in Rust<br>• CRUD tags/attributes via CLI & DB<br>• Dolphin plug‑in: sidebar + tag view<br>• KIO `tags://` virtual folder<br>• Bulk‑edit dialog | 100 k‑file corpus cold‑start ≤ 3 s; user can tag files & navigate `tags://Urgent` |
| **S4–6 (M2, 6 wks)**   | Sync & version control                 | • Change‑log table + diff viewer<br>• LiteFS replication PoC<br>• WebSocket delta sync<br>• Conflict queue UI + last‑write‑wins fallback                         | Two devices sync metadata in <1 s round‑trip; rollback works                      |
| **S7–9 (M3, 6 wks)**   | NLP search & Visual Builder            | • Integrate Tantivy FTS + ONNX intent model<br>• Toggle exact vs natural search<br>• QML Visual Builder with live query string                                   | NL query "docs Alice edited last week" returns expected set in ≤ 300 ms           |
| **S10–13 (M4, 8 wks)** | Plug‑in marketplace & mobile companion | • IPC sandbox + manifest spec<br>• Sample plug‑ins (image EXIF auto‑tagger)<br>• Flutter read‑only client<br>• LDAP/OIDC enterprise auth                         | First external plug‑in published; mobile app lists smart folders                  |

### 10.4  Tooling & Infrastructure

* **Issue tracking** – Jira → labels `component/indexer`, `component/ui`.
* **Docs** – mkdocs‑material hosted on GitHub Pages; automatic diagram generation via `cargo doc` + Mermaid.
* **Nightly Perf Benchmarks** – Run in CI against 10 k, 100 k, 1 M synthetic corpora; fail build if P95 query > target.
* **Security** – Dependabot, Trivy scans, optional SLSA level 2 provenance for releases.

### 10.5  Risks & Mitigations

| Risk                           | Impact           | Mitigation                                                                  |
| ------------------------------ | ---------------- | --------------------------------------------------------------------------- |
| CRDT complexity                | Delays M2        | Ship LWW first; schedule CRDT refactor post‑launch                          |
| File system event overflow     | Index corruption | Debounce & auto‑fallback to full rescan; alert user                         |
| Cross‑distro packaging pain    | Adoption drops   | Stick to Flatpak; AppImage only for power users; collect telemetry (opt‑in) |
| Scaling >1 M files on slow HDD | Perf complaints  | Offer "index on SSD" wizard; tune FTS page cache                            |

### 10.6  Budget & Timeline Snapshot

* **Total dev time** ≈ 30 weeks.
* **Buffer** +10 % (3 weeks) for holidays & unknowns → **33 weeks** (\~8 months).
* **Rough budget** (3 FTE avg × 33 wks × \$150 k/yr) ≈ **\$285 k** payroll + \$15 k ops / tooling.

---
