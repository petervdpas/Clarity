# Migrations

Status: partial. Owned SQL sources follow the dictionary through numbered steps written in Clarity, replayed to give the schema and applied by every runtime at start.

Progress: 60%

## Code
- Spec: `crates/clarity-spec/src/migrate.rs` (`Op` :160-191, `parse` :363, `schema` :545, `pending` :594, `proposed_renames` :714); `field_rename.rs`, `rename.rs`
- Generators: steps to SQL per dialect `crates/clarity-gen/src/schema.rs` (baseline SQLite only :426), `ddl.rs`; the generator refuses to write while changes are pending
- Core: `crates/clarity-db/src/lib.rs` (applied steps, `clarity_schema`)
- Runtimes: R `runtimes/rust/src/store.rs`, `dialect.rs`, `lib.rs`; Go `runtimes/go/clarity/store.go`, `dialect.go`, `db.go`; C `runtimes/c/clarity/store.c`, `dialect.c`, `db.c`; C++ `runtimes/cpp/clarity/store.cpp`, `dialect.cpp`, `db.cpp`; C# `Clarity.Runtime/Store.cs`, `Dialect.cs`, `Db.cs`. Each applies missing steps and records them in `clarity_schema`.
- IDE: `app/src/lib/MigrationPanel.svelte`, `app/src-tauri/src/migrations.rs`; agent tool `migrate` (`crates/clarity-tools/src/lib.rs`)
- CLI: `clarity migrate <project> [--check] [--allow-data-loss]`, `clarity own`, `clarity data migrate` (`crates/clarity-cli/src/main.rs`)
- Docs: manual `data.migrations.md`; LANGUAGE.md section 4 "Migrations" (:499)

## Tests
- `conformance/migrations.json` (written by `CLARITY_WRITE_CONFORMANCE=1 cargo test -p clarity-gen --test servers`): run by Go (`server_test.go`), C (`runtimes/c/tests/scenario_test.c`), C++ (`scenario_test.cpp`); C# only on server databases (`ServerTests.cs`), which CI lacks; skipped by R
- `crates/clarity-spec/tests/migrations.rs`, `rename_field.rs`, `ownership.rs`
- `crates/clarity-db/tests/applied_steps.rs`; `crates/clarity-cli/tests/data_migrate.rs`; `crates/clarity-gen/tests/servers.rs` (ignored, needs `CLARITY_TEST_*`)

## Open
- A default or fill expression when a field becomes required: NOT BUILT (DESIGN.md:151-152).
- Counting the rows a narrowing type change would not fit: NOT BUILT (DESIGN.md:153). The data-loss confirmation is built.
- Data migrations (Clarity code that changes rows), down steps, one-way steps, renaming enum values: NOT BUILT (DESIGN.md:155-156; `Op` holds schema operations only, `migrate.rs:160-191`).
- A migrate switch in the generated program for server deployments: NOT BUILT (DESIGN.md:158). `clarity data migrate` works on the IDE's SQLite only.
- Baseline steps for server databases: NOT BUILT (`crates/clarity-gen/src/schema.rs:426`); see [databases](databases.md).
- Shared runtime core for migrations: NOT BUILT; each runtime applies steps itself (MODULAR-PLAN.md order 2).

## Conflicts
- `conformance/migrations.json` is not run by R, and C# runs it only against server databases (CONFLICTING.md 3).
- LANGUAGE.md grammar summary (:3028-3149) lacks the migration lines search, version, table and key.

## Decisions
- 2026-10-16: migrations built in for every owned source; a rename is a designer edit with a known intent, so no data is lost by guessing (DESIGN 1.3).
- 2026-10-17: numbered steps and no snapshot; replaying them gives the schema, no `schema.lock` (D1b).
- 2026-10-17: steps do not depend on a database; each runtime writes its own dialect, so one project migrates SQLite and PostgreSQL alike (DESIGN 1.3).
- Renames are proposed when a field goes and one of the same type comes, and confirmed or declined by the person, never applied by guess (DESIGN 1.3; LANGUAGE.md section 4).
- 2026-10-29: an integer field that becomes a duration is proposed as a rename (X2).
- 2026-11-12: `add version Entity` and `version` in `add entity`; see [row-versions](row-versions.md).
- Before applying steps to a SQLite database with a schema, every runtime backs up first; see [backups](backups.md).

## Was
- DESIGN 1.3 "Migrations" (:144-162, except the Backups bullet), roadmap D1b
- LANGUAGE.md section 4 "Migrations"
- IMPLEMENTED.md 6 "Databases and migrations" (migrations)
- UNIMPLEMENTED.md 2 "Migrations and backups" (migrations items)
- MODULAR-PLAN.md order 2 (migrations as the runtimes run them)
- CONFLICTING.md 3 (migrations row), 6 (grammar: migrations)
