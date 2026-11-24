# Backups

Status: partial. Copies of a project's owned SQLite databases, taken by hand (IDE, command line, agent) and by clarity-migrate before a migration (the programs do not migrate, docs/ledger/migrations.md), listed, pruned and restored.

Progress: 70%

## Code
- Project file: `backups "data/backups"` read and written in `crates/clarity-project/src/file.rs` (`backups_problem`, the `"backups"` arm of `parse`, the line in `write`), `Settings.backups` in `crates/clarity-project/src/lib.rs`, checked again in `validate` (the IDE's saves).
- The one version of every operation: `crates/clarity-project/src/backups.rs` (`backed_up`: the database and its backups folder, or why not; `folder_of`: `<backups>/<source>`; `take`, `list`, `restore`, `delete`, the last two by a backup's file name only; `NO_BACKUPS`). The IDE, its agent and `clarity data backup | backups | restore` all call it.
- Core: `crates/clarity-db/src/backup.rs` (`take`, `list`, `prune_automatic`, `restore`, `delete`, each given the backups folder; `VACUUM INTO`, names `<db>-<UTC stamp>-<why>.db`, 10 automatic kept)
- Before a migration: `crates/clarity-migrate/src/lib.rs` (`Database.backups`, `migrate_open(.., backups)`, `backup_first`: a relative folder is in the folder of the database's file; none takes no backup); the names and pruning in `runtimes/rust/src/backup.rs` (`prune(folder, database)`). Filled by `crates/clarity-build/src/lib.rs` (`placed`: the project's folder in full, for Run and `clarity data migrate`) and by `clarity_gen::migrated_databases` (`crates/clarity-gen-core/src/lib.rs`, `MigratedDatabase.backups` = `<backups>/<source>`, for the installed app's `clarity-migrate.json`).
- The Go, C, C++ and C# runtimes carry no backup code: their programs neither migrate nor back up, and nothing generated called their helpers (removed 2026-11-23, with their `backups.json` tests and their `keptBackups` lines of the defaults tests; the C and C++ generators no longer copy `backup.c`, `backup.h`, `backup.cpp`, `backup.hpp`).
- IDE: `app/src/lib/BackupPanel.svelte` (Back up now, Restore, Delete, each confirmed with Clarity's dialogs; Restore stops the running app; shows the folder), `app/src-tauri/src/backups.rs`; agent tools `list_backups`, `backup_data`, `restore_backup` (the last two ask first) in `app/src-tauri/src/agent.rs`; not served by `clarity mcp`. **Backups folder** in `app/src/lib/ProjectSettings.svelte` writes the line.
- CLI: `clarity data backup | backups | restore <file name>` and `clarity data migrate` (`clarity_build::databases` + `clarity_build::migrate_databases`, as Run) in `crates/clarity-cli/src/main.rs` (`data`, `migrate_data`)
- Docs: manual `project.file.md` (`### backups`), `data.sources.md` (`### Backups`)

## Tests
- `conformance/backups.json`: run by R (`runtimes/rust/tests/backups.rs`) and core (`crates/clarity-db/tests/backup.rs`); `conformance/readers.json` names `rust` and `ui` only.
- `crates/clarity-project/tests/settings.rs` (`the_project_says_where_its_backups_go_under_its_own_folder`), `crates/clarity-project/tests/backups.rs` (which databases and folders, no backups without the line, take/list/restore/delete by file name)
- `crates/clarity-migrate/tests/migrate.rs` (backed up into the folder given, none without one, the installed app's json names the folder)
- `crates/clarity-cli/tests/data_migrate.rs` (`data migrate` as Run; `data backup | backups | restore` need the folder and restore by file name)
- `app/src-tauri/src/backups.rs` tests, `app/src-tauri/src/agent.rs` `backing_up_and_restoring_ask_the_person_first`; `crates/clarity-mcp/tests/server.rs` (backup tools not served)

## Open
- Server databases: NOT BUILT; see Design "Server databases". The IDE says a server database is backed up by its administrator.
- A backup from the program's own code: NOT BUILT; see Design "Later".
- Encrypted backups: NOT BUILT; see [encrypted-storage](encrypted-storage.md).
- Pruning written twice: `clarity_db::backup::prune_automatic` and `clarity_runtime::backup::prune` (clarity-migrate carries no clarity-db). One version for these is open. (The Go, C, C++ and C# copies are gone, 2026-11-23.)

## Design

### Backups are a project option (2026-11-23)

The user: "backups are a project option first of all!! and it path is set in the project file (self-contained)!!!", "I tell you that its a project option with the backup path stored (but must be UNDER the project folder otherwise OSes will not be able to figure it out!!!)", and for every operation: "only ONE version should exist!!!"

- The project file's line `backups "data/backups"`, beside `output` and `include migrate`, a folder relative to the project's folder (where `output` and the SQLite files are relative to). A path from the root of the disk (`/x`, `\\x`, `C:...`) or one that goes above the project's folder with `..` is refused, naming the line; `validate` refuses it too.
- Without the line the project takes no backups: taking, listing, restoring and deleting are refused with a message that says to set the folder (the line, or Project settings in the IDE), and clarity-migrate migrates without a backup first.
- With it, each data source's backups go in `<backups>/<source>/`.
- A restore takes only a backup of that folder, by its file name; a path is refused, in the IDE, the agent and the CLI alike.
- clarity-migrate's folder: Run and `clarity data migrate` give it the project's folder in full; the installed app's `clarity-migrate.json` names `<backups>/<source>` (`data/backups/main`), which clarity-migrate finds in the folder the database's file opens in (for a kept database, `<application data>/<program>/data/backups/main`).

### Server databases

- PostgreSQL, MySQL and SQL Server have no `VACUUM INTO`, and their dump tools are not on every machine.
- A backup of one is Clarity's own portable form: every owned table's rows, read through the runtime that already connects to it, written as one file per backup.
- It restores into the same database or into another kind, which also gives the copy tool between kinds of databases.

### Later

- The program backs up from its own code, as the modelling example's import should before it replaces domains: a statement such as `back up main "before-import"`.

### Open questions

- None. (Asked before: should the Go, C, C++ and C# runtimes' backup helpers take the folder too? Answered by removing them, 2026-11-23: no program called them.)

## Conflicts
- "Keep 10 automatic backups" is written in the Rust runtime and in clarity-db (equal today, both checked against `conformance/defaults.json` `keptBackups`).

## Decisions
- 2026-11-05: Clarity owns backups of the databases it owns; an `external` source is neither backed up nor restored by it. User: "back up the database... the ide should be able to do by itself"
- 2026-11-05: a backup is a copy beside the database, made with SQLite's `VACUUM INTO`; no tool is added to any stack. (Where it goes changed on 2026-11-23: the project's backups folder.)
- 2026-11-05: the name says when (UTC) and why: `manual`, `before-migration-NNNN`, `before-restore`.
- 2026-11-05: automatic backups are pruned to the 10 newest, manual ones never.
- 2026-11-05: a restore first backs up what it replaces, and refuses while the program runs on that database.
- 2026-11-05: a failed backup stops the migration it guards.
- 2026-11-05: the agent asks before taking or restoring; listing only reads.
- 2026-11-10: a project option to turn off the backup before a migration. User: "dont keep backups for the examples", "is it a project option? backup?", "yes, add it to the backup plan" (the `backups off` word on a data source, never built; replaced on 2026-11-23)
- 2026-11-23: backups are a project option: the `backups` line names a folder under the project's folder, each source's in `<backups>/<source>/`; without the line no backups are taken; one version of every operation (`clarity_project::backups`) for the IDE, the agent and the CLI; `clarity data migrate` migrates as Run does. User: "backups are a project option first of all!! and it path is set in the project file (self-contained)!!!", "must be UNDER the project folder otherwise OSes will not be able to figure it out!!!", "only ONE version should exist!!!"
- 2026-11-23: the Go, C, C++ and C# runtimes' backup helpers removed: no generated program called them; only the Rust runtime (clarity-migrate) and clarity-db read `conformance/backups.json`. User: "only ONE version should exist!!!"

## Was
- docs/BACKUP-PLAN.md "Decisions", steps 1, 2, step 3 (Design "Server databases"), "Next: backups off for a data source" (Design "Backups off"), "Later" (Design "Later")
- DESIGN 1.3 "Migrations", Backups bullet (DESIGN.md:157)
- MODULAR-PLAN.md order 2 (backups)
- IMPLEMENTED.md 6 "Row versions, undo, backups" (backups)
- UNIMPLEMENTED.md 2 "Migrations and backups" (backup items)
- memory: backup-plan.md
