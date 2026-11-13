# Backups

Status: partial. Copies of a project's owned SQLite databases, taken by hand (IDE, command line, agent) and by every runtime before a migration, listed, pruned and restored.

Progress: 60%

## Code
- Core: `crates/clarity-db/src/backup.rs` (`take` :64, `list` :115, `prune_automatic` :130, `restore` :148, `delete` :178; `VACUUM INTO`, names `<db>-<UTC stamp>-<why>.db` in `backups/` beside the database, 10 automatic kept)
- Project: `crates/clarity-project/src/lib.rs:1078` (`database_to_back_up`: owned SQLite only; external and server sources say why)
- Runtimes (back up before applying steps to a database that has a schema, refuse to migrate when they cannot): R `runtimes/rust/src/backup.rs`; Go `runtimes/go/clarity/backup.go`; C `runtimes/c/clarity/backup.c`; C++ `runtimes/cpp/clarity/backup.cpp`; C# `runtimes/csharp/Clarity.Runtime/Backup.cs`
- IDE: `app/src/lib/BackupPanel.svelte` (Back up now, Restore, Delete, each confirmed with Clarity's dialogs; Restore stops the running app), `app/src-tauri/src/backups.rs`; agent tools `list_backups`, `backup_data`, `restore_backup` (the last two ask first) in `app/src-tauri/src/agent.rs`; not served by `clarity mcp`
- CLI: `clarity data backup | backups | restore` (`crates/clarity-cli/src/main.rs:97-182`)
- Docs: manual `data.migrations.md`; LANGUAGE.md section 4 "Migrations" ("A backup first")

## Tests
- `conformance/backups.json`: run by R (`runtimes/rust/tests/backups.rs`), Go (`backup_test.go`), C (`backup_test.c`), C++ (`backup_test.cpp`), C# (`BackupTests.cs`), core (`crates/clarity-db/tests/backup.rs`)
- `crates/clarity-project/tests/backups.rs`; `crates/clarity-mcp/tests/server.rs` (backup tools not served)

## Open
- Server databases (PostgreSQL, MySQL, SQL Server) in Clarity's own portable form, one file per backup, restorable into another kind: NOT BUILT (BACKUP-PLAN.md step 3). The IDE says a server database is backed up by its administrator.
- `backups off` on a data source, a "Back up before a migration" checkbox in project settings, every example's SQLite source with it: NOT BUILT (BACKUP-PLAN.md "Next: backups off for a data source").
- `back up main "name"` from app code: NOT BUILT (BACKUP-PLAN.md "Later").
- Backups kept in a folder the person picks: NOT BUILT (BACKUP-PLAN.md "Later").
- Encrypted backups: NOT BUILT; see [encrypted-storage](encrypted-storage.md).
- Shared Rust core for backups: NOT BUILT; each runtime has its own (MODULAR-PLAN.md order 2).

## Conflicts
- "Keep 10 automatic backups" is written in each runtime and in clarity-db (CONFLICTING.md 4; equal today).

## Decisions
- 2026-11-05: Clarity owns backups of the databases it owns; an `external` source is neither backed up nor restored by it. User: "back up the database... the ide should be able to do by itself"
- 2026-11-05: a backup is a copy beside the database, made with SQLite's `VACUUM INTO`; no tool is added to any stack.
- 2026-11-05: the name says when (UTC) and why: `manual`, `before-migration-NNNN`, `before-restore`.
- 2026-11-05: automatic backups are pruned to the 10 newest, manual ones never.
- 2026-11-05: a restore first backs up what it replaces, and refuses while the program runs on that database.
- 2026-11-05: a failed backup stops the migration it guards.
- 2026-11-05: the agent asks before taking or restoring; listing only reads.
- 2026-11-10: a project option to turn off the backup before a migration. User: "dont keep backups for the examples", "is it a project option? backup?", "yes, add it to the backup plan"

## Was
- BACKUP-PLAN.md "Decisions", steps 1, 2, 3, "Next: backups off for a data source", "Later"
- DESIGN 1.3 "Migrations", Backups bullet (DESIGN.md:157)
- MODULAR-PLAN.md order 2 (backups)
- IMPLEMENTED.md 6 "Row versions, undo, backups" (backups)
- UNIMPLEMENTED.md 2 "Migrations and backups" (backup items)
- memory: backup-plan.md
