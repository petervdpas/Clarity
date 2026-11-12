# Databases

Status: partial. SQL data sources (SQLite, PostgreSQL, MySQL, SQL Server), owned or external, one database per source, where the file opens, and what a program does when a database does not open.

## Code
- Project file: `datasource name kind "where"` with external, readonly, beside (`crates/clarity-project/src/file.rs:474-540`); `SourceKind` (`crates/clarity-project/src/lib.rs:66-75`); `keychain:` refused (lib.rs:607); import and take ownership (lib.rs:1399, `file_import.rs`)
- Spec: `source erp` per module (`crates/clarity-spec/src/parser.rs:332`, `check.rs:2636`); external mapping `table`, `key`, `column`
- Core: `crates/clarity-db` (live-table checks, keys `src/lib.rs:42-49`)
- Generators: SQL per dialect `crates/clarity-gen/src/ddl.rs` (`Sql::of`, `feature`); entities without a source refused (`crates/clarity-gen/src/lib.rs:53`); C and C++ refuse non-SQLite (`c/mod.rs:248`, `cpp/mod.rs:257`, cli `c/cli.rs:142`, `cpp/cli.rs:123`)
- Runtimes: R `runtimes/rust/src/{sqlite,postgres,mysql,mssql,dialect,store,data_folder}.rs` (Cargo features); Go `runtimes/go/clarity/{sqlite,server,dialect,db,data_folder}.go`, `runtimes/go/{postgresql,mysql,sqlserver}`; C `runtimes/c/clarity/{sqlite.c,db.c,store.c,dialect.c}`; C++ `runtimes/cpp/clarity/{sqlite,db,store,dialect,paths}.cpp`; C# `Clarity.Runtime/{Db,Store,Dialect,Locations,ServerDriver,Unconnected}.cs`, NuGets `Clarity.Runtime.Sqlite`, `.PostgreSql`, `.MySql`, `.SqlServer`
- IDE: data sources in `app/src/lib/ProjectSettings.svelte`; `app/src/lib/ImportPanel.svelte`; `app/src-tauri/src/external.rs` (source_tables, import_tables, take_ownership)
- CLI: `clarity import`, `clarity own` (`crates/clarity-cli/src/main.rs`)
- Docs: manual `data.sources.md`, `project.file.md`; LANGUAGE.md section 4 "Databases" (:542), "External databases" (:587)

## Tests
- `conformance/data-folder.json`: run by R, Go, C, C++, C#
- `conformance/unconnected.json`: run by R, Go, C, C++, C#
- `crates/clarity-spec/tests/sources.rs`, `external.rs`; `crates/clarity-project/tests/external.rs`; `crates/clarity-db/tests/sqlite.rs`
- `crates/clarity-gen/tests/servers.rs`, Go `server_test.go`, C# `ServerTests.cs` need `CLARITY_TEST_POSTGRES`, `_MYSQL`, `_MSSQL`; CI sets none, so server databases are not tested in CI
- `crates/clarity-gen/tests/no_datasource.rs`, `unconnected_csharp.rs`

## Open
- PostgreSQL, MySQL and SQL Server on C and C++: NOT BUILT ("the C backend stores records in SQLite for now", `crates/clarity-gen/src/c/mod.rs:248`).
- Import, live-table checks and baselines for server databases: NOT BUILT ("importing is for SQLite and files only for now", `crates/clarity-project/src/lib.rs:1399`; `crates/clarity-gen/src/schema.rs:426`).
- Keys that are not one integer column: NOT BUILT (`crates/clarity-db/src/lib.rs:42-49`).
- TLS to PostgreSQL and MySQL: PARTIAL. Rust connects without TLS (`runtimes/rust/src/postgres.rs:144`, `runtimes/rust/Cargo.toml:57`); SQL Server uses rustls; Go and C# not checked.
- Money shown with the four decimals the servers store: open (roadmap D1d).
- Testing a source's connection from the Project tab: NOT BUILT (DESIGN.md:131).
- Copying data when a source changes kind (SQLite to PostgreSQL): NOT BUILT (DESIGN.md:160, roadmap D3); BACKUP-PLAN step 3 would give it, see [backups](backups.md).
- The shop's windows against a server database: UNTRIED (roadmap G2d).

## Conflicts
- DESIGN.md:138 says C# uses EF Core or Dapper; it is plain ADO.NET (`runtimes/csharp/Clarity.Runtime.Sqlite/SqliteDriver.cs:5`).
- DESIGN.md:121 lists sqlite, postgres, mysql, mssql, rest; LANGUAGE.md:91 and manual `project.file.md:50` add the file formats ([formats](formats.md)).
- C and C++ refusal wording differs: "is a PostgreSQL/MySQL/SQL Server database" in `c/mod.rs:248`, `cpp/mod.rs:257`; "is a server database" in `c/cli.rs:142`, `cpp/cli.rs:123`.
- LANGUAGE.md:87 and the grammar leave out `beside` and the `file` kind with `using Mapping` (`file.rs:484-519`).

## Decisions
- 2026-10-16: the first data source is the default; a new project gets one SQLite source `main`; passwords never in the project file, only `env:` or `setting:` (DESIGN 1.3).
- 2026-10-17: one database per owned source; a reference into another database is a plain id without a foreign key (D1c).
- 2026-10-17: a runtime driver per database, a Cargo feature a program turns on only for its kinds; SQL per dialect in one place (D1d).
- 2026-10-17: `external` databases are mapped with `table`, `key`, `column` and never migrated; `readonly` refuses save and delete (D1e).
- 2026-10-25: when the databases do not open, the program starts on an empty database in memory and runs `on unconnected` or shows the window with the secret setting (DESIGN 5.27); see [settings-secrets](settings-secrets.md).
- 2026-11-02: `CLARITY_DATA` names the folder each SQLite file opens in (DESIGN 7.9, G7d).

## Was
- DESIGN 1.3 (sources, owned and external, REST paragraph shared with [web-apis](web-apis.md)), 7.4, 5.27 ("When the databases do not open"), roadmap D1c, D1d, D1e, D3, G2d, G3d; DESIGN 7.9 G7d (`CLARITY_DATA`)
- LANGUAGE.md section 4 "Databases", "External databases"
- IMPLEMENTED.md 6 "Databases and migrations" (drivers, kinds)
- UNIMPLEMENTED.md 2 "Databases"
- CONFLICTING.md 2 (refusal wording), 4 (non-SQLite refusal), 7 (EF Core), 9 (data source kinds)
