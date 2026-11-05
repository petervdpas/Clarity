# Backing up a project's data

A plan, agreed 2026-11-05. Steps 1 and 2 are built (2026-11-05); step 3, server databases, is not.

## Goal

A project's data is never lost to a step that went wrong: a migration, an import that replaces records, a restore of the wrong file. Clarity takes a backup itself before anything that changes a database wholesale, and the person (or Clarity's agent, with their permission) can take one and restore one from the IDE and the command line. Today none of this exists: a developer copies the database file by hand, and the agent has no way to do it at all.

## Decisions

- **Clarity owns backups of the databases it owns.** An `external` data source belongs to someone else; Clarity neither backs it up nor restores it.
- **A backup is a copy of the database, next to it.** For SQLite: `data/backups/<name>-<when>-<why>.db` beside `data/<name>.db`, made with SQLite's own `VACUUM INTO`, which writes a consistent copy while the database is open. No tool is added to any stack: every runtime already has SQLite.
- **Its name says when and why**: `modelling-20261105T213012Z-before-migration-0006.db`. The time is UTC, so names sort and compare on every machine; the IDE shows it in the person's own time. The reasons are `manual`, `before-migration-NNNN` (the first step it applies) and `before-restore`.
- **Automatic backups are pruned, manual ones never.** Each database keeps its 10 newest automatic backups; a manual one stays until the person deletes it.
- **A restore first backs up what it replaces** (`before-restore`), so a restore can itself be undone. It refuses while the application is running on that database (the IDE stops its preview first).
- **A failed backup stops what it guards.** A runtime that cannot back up before migrating (the disk is full, the folder cannot be written) does not migrate, and says why.
- **The agent asks first.** Its tools `backup_data` and `restore_backup` ask the person before they run, as its build tool does; `list_backups` only reads.
- **Every stack alike**: the Rust, Go, C#, C and C++ runtimes back up before migrating in the same way, with the same names, tested in each.
- **Every system alike**: the paths are the project's own, so Linux, Windows and macOS keep backups in the same place in the project.

## Steps

### 1. The core, the command line, the IDE and the agent

- **The core** (`clarity-db`, module `backup`): find a project's SQLite databases from its data sources, take a backup, list them, restore one, prune the automatic ones.
- **The command line**:
  - `clarity data backup <project> [--source main]`
  - `clarity data backups <project> [--source main]`
  - `clarity data restore <project> <backup> [--source main]`
- **The IDE**: in the Database section of the Project tab, per data source, **Back up now**, and the list of backups with **Restore** and **Delete**, each confirmed through Clarity's own dialogs.
- **The agent**: `list_backups`, `backup_data` and `restore_backup` (the last two ask first).

### 2. Before a migration, in every runtime

When a generated application starts and has steps to apply to an existing SQLite database that already holds a schema, it backs up first, then migrates. A new, empty database needs no backup. Each runtime gets a test that a migration leaves a backup with the data as it was before.

### 3. Server databases

PostgreSQL, MySQL and SQL Server have no `VACUUM INTO`, and their own dump tools are not every machine's. A backup of one becomes Clarity's own portable form instead: every owned table's rows, read through the runtime that already connects to it, written as one file per backup, and restored into the same database or into another kind (which is also the copy tool section 1.3 of DESIGN.md promises). Until then the IDE says that a server database is backed up by its administrator.

## Later

- **An application backs up from its own code**, as the modelling example's import should before it replaces domains: a statement such as `back up main "before-import"`.
- **Backups kept elsewhere**: a setting that puts them in a folder of the person's choosing instead of beside the database.
