# Queries

Status: built. `find v in Entity where ... order by ...` in code, with conditions through references and full-text `matches`, turned into SQL per dialect by every runtime.

Progress: 100%

## Code
- Spec: `find` in `crates/clarity-spec/src/code/parse.rs`; checks in `code/check.rs:4629` (the `find` expression), read-only sources never saved (:5124), web-API entities never found (:5227), conditions and order through references stay in one source (:5347); `search` modifier in `parser.rs:820`
- Generators: `crates/clarity-gen/src/query.rs` (paths through references, resolved once for all stacks); each stack's `logic.rs`
- Runtimes: R `runtimes/rust/src/query.rs`, `sql.rs`, `dialect.rs`; Go `runtimes/go/clarity/query.go`, `dialect.go`; C `runtimes/c/clarity/query.c`, `sql.c`, `dialect.c`; C++ `runtimes/cpp/clarity/query.cpp`, `sql.cpp`, `dialect.cpp`; C# `runtimes/csharp/Clarity.Runtime/Query.cs`, `Sql.cs`, `Dialect.cs`
- IDE: Search column in `app/src/lib/EntityEditor.svelte`
- Docs: manual `code.queries.md`

## Tests
- `conformance/queries.json`: run by R, Go, C, C++, C#
- `crates/clarity-spec/tests/`: queries.rs, search.rs, sources.rs
- `crates/clarity-gen/tests/`: query_id.rs, search.rs; servers.rs (ignored; needs `CLARITY_TEST_POSTGRES`, `_MYSQL`, `_MSSQL`)

## Open
- none. Server databases on C and C++ and server tests in CI: see [databases](databases.md).

## Conflicts
- Roadmap D1a says "open: conditions through references"; X2 records them built on 2026-10-18.
- `manual/code.queries.md:31-58` uses snake_case names; :53 a quoted text that must be a key.
- Empty values in order: see [language](language.md).

## Decisions
- 2026-10-18: a condition or order key may go through references, as a subquery per step; a query into another data source is an error.
- 2026-10-23: full-text `search` and `matches`; user chose every word, each as the start of a word, `matches` in the query's own record; SQLite FTS5 with triggers, PostgreSQL GIN on to_tsvector('simple'), MySQL FULLTEXT plus REGEXP for short words, SQL Server LIKE at word starts. `contains` stays the exact test.

## Was
- Roadmap D1a, U1h; X2 (queries through a reference); DESIGN 5.9 ("Later, with D1 and beyond: queries")
- IMPLEMENTED.md section 3 "Queries"
