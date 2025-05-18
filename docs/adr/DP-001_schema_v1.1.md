# DP-001: Schema v1.1 – Core Metadata Domains

**Status**: Proposed  
**Authors**: @carol  
**Date**: 2025-05-17

## 1. Context

We’ve landed a basic SQLite-backed `files` table and a contentless FTS5 index. Before we build out higher-level features, we need to lock down our **v1.1** metadata schema for:

- **Hierarchical tags** (`tags` + `file_tags`)
- **Custom attributes** (`attributes`)
- **File-to-file relationships** (`links`)
- **Named collections** (`collections` + `collection_files`)
- **Saved views** (`views`)

Locking this schema now lets downstream CLI & GUI work against a stable model and ensures our migrations stay easy to reason about.

## 2. Decision

1. **Bump to schema version 1.1** in our migration table.
2. Provide four migration scripts, applied in order:
   1. `0001_initial_schema.sql`   – create `files`, `tags`, `file_tags`, `attributes`, `files_fts`, core FTS triggers.
   2. `0002_update_fts_and_triggers.sql`  – replace old tag/attr FTS triggers with `INSERT OR REPLACE` semantics for full-row refresh.
   3. `0003_create_links_collections_views.sql`  – introduce `links`, `collections`, `collection_files`, `views` tables.
   4. `0004_fix_hierarchical_tags_fts.sql`  – refine FTS triggers to index full hierarchical tag-paths via a recursive CTE.
3. Expose this schema through our library (`libmarlin::db::open`) so any client sees a v1.1 store.

## 3. ER Diagram

Below is the updated entity-relationship diagram, expressed in PlantUML for clarity. It shows all of the core metadata domains and their relationships:

```plantuml
@startuml
entity files {
  * id : INTEGER <<PK>>
  --
    path  : TEXT
    size  : INTEGER
    mtime : INTEGER
    hash  : TEXT
}

entity tags {
  * id           : INTEGER <<PK>>
  --
    name         : TEXT
    parent_id    : INTEGER <<FK>>
    canonical_id : INTEGER <<FK>>
}

entity file_tags {
  * file_id : INTEGER <<FK>>
  * tag_id  : INTEGER <<FK>>
}

entity attributes {
  * id      : INTEGER <<PK>>
  --
    file_id : INTEGER <<FK>>
    key     : TEXT
    value   : TEXT
}

entity links {
  * id           : INTEGER <<PK>>
  --
    src_file_id : INTEGER <<FK>>
    dst_file_id : INTEGER <<FK>>
    type        : TEXT
}

entity collections {
  * id   : INTEGER <<PK>>
  --
    name : TEXT
}

entity collection_files {
  * collection_id : INTEGER <<FK>>
  * file_id       : INTEGER <<FK>>
}

entity views {
  * id    : INTEGER <<PK>>
  --
    name  : TEXT
    query : TEXT
}

files          ||--o{ file_tags
tags           ||--o{ file_tags

files          ||--o{ attributes

files          ||--o{ links : "src_file_id"
files          ||--o{ links : "dst_file_id"

collections    ||--o{ collection_files
files          ||--o{ collection_files

views           ||..|| files  : "smart queries (via FTS)"
@enduml
````

*(If you prefer a plain‐ASCII sketch, you can replace the above PlantUML block with:)*

```ascii
┌────────┐        ┌────────────┐        ┌───────┐
│ files  │1────*──│ file_tags  │*────1─│ tags  │
└────────┘        └────────────┘        └───────┘
     │                                    
     │1                                   
     *                                    
┌────────────┐                           
│ attributes │                           
└────────────┘                           

┌────────┐       ┌────────┐       ┌────────┐
│ files  │1──*──│ links  │*───1──│ files  │
└────────┘       └────────┘       └────────┘

┌─────────────┐     ┌──────────────────┐     ┌────────┐
│ collections │1──*─│ collection_files │*──1─│ files  │
└─────────────┘     └──────────────────┘     └────────┘

┌───────┐
│ views │
└───────┘
```

## 4. Migration Summary

| File                                            | Purpose                                                 |
| ----------------------------------------------- | ------------------------------------------------------- |
| **0001\_initial\_schema.sql**                   | Core tables + contentless FTS + path/triggers           |
| **0002\_update\_fts\_and\_triggers.sql**        | Full-row FTS refresh on tag/attr changes                |
| **0003\_create\_links\_collections\_views.sql** | Add `links`, `collections`, `collection_files`, `views` |
| **0004\_fix\_hierarchical\_tags\_fts.sql**      | Recursive CTE for full path tag indexing                |

## 5. Example CLI Session

```bash
$ marlin init
Database initialised at ~/.local/share/marlin/index_*.db
Initial scan complete – indexed/updated 42 files

$ marlin link add ./todo.md ./projects/plan.md
Linked './todo.md' → './projects/plan.md'

$ marlin coll create "MyDocs"
Created collection 'MyDocs'

$ marlin view save tasks "tag:project AND TODO"
Saved view 'tasks' = tag:project AND TODO

$ marlin view list
tasks: tag:project AND TODO
```

## 6. Consequences

* **Backward compatibility**: older v1.0 stores will be migrated on first open.
* **Stability**: downstream features (TUI, VS Code, web UI) can depend on a stable v1.1 schema.
* **Simplicity**: by consolidating metadata domains now, future migrations remain small and focused.

---

*End of DP-001*