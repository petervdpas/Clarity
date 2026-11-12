# C stack

Status: built. `stack c ...`: a CMake project with the C runtime copied in, drawn by Clarity.UI's library.

## Code
- Generators: `crates/clarity-gen/src/c/` (logic.rs 2625 lines, mod.rs 1279, model.rs 503, cli.rs 402, parts.rs 340, names.rs 207). Output: `src/records.h/.c` (every module's records), `src/modules/<module>.c`, `src/app.h`, `CMakePresets.json` (`cmake --workflow --preset debug`) (`c/mod.rs:1-11`)
- Runtimes: C `runtimes/c`, no version of its own, copied into each program as `clarity-runtime/`:
  - `clarity/`: core, db, sqlite, sql, dialect, query, value(s), decimal, text, code, edits, backup, settings, store, files, picture, record_file, people, json, ui, views, watch, splash, cli and part rules (tables, trees, ranges, steps, charts, dropdown_options, reorder, canvas)
  - `desktop/desktop.c` (the window on Clarity.UI), `service/` (service.c, wire.c, serve.c, watches.c: the commands and the web server)
  - `auth/`, `formats/` (formats, file_source), `http/` (libcurl), `pictures/`, `secrets/`, `engine/` (wasm3, screen engines)
  - `third_party/`: cjson, wasm3 (vendored); `clarity.cmake`, `clarity_ui.cmake`
- Desktop program: links Clarity.UI from the folder CLARITY_UI names; web target carries the browser files (`c/mod.rs:6-9`); see [web-target](web-target.md)
- Docs: manual/stacks.c.md

## Tests
- Runtime tests: `runtimes/c/tests/*_test.c` (main.c, scenario_test.c, service_test.c, desktop_test.c, ...)
- Conformance run by C: every runtime file except `shortcut-keys`, `secret-store` (C writes the keychains itself), `formats` (display formats), `picture-functions` (CONFLICTING 3)
- Generator: `crates/clarity-gen/tests/c.rs`, versions_c.rs, signin_c.rs; build: c_build.rs (ignored)
- Examples on this stack: kitchen (`c desktop`); no example uses c web

## Open
- PostgreSQL, MySQL, SQL Server: NOT BUILT. "the C backend stores records in SQLite for now" (`crates/clarity-gen/src/c/mod.rs:246-248`); see [databases](databases.md)
- Generator gaps, C only (PARTIAL): list literals other than records or plain values (`c/logic.rs:1902`); a position in such a list (:1926); window data of `list<bool>` or `list<Enum>` (:437); a list of values given to a window (:1621); `json of` a `list<bool>` (:2027)
- `list<Entity>` fields: NOT BUILT (`c/logic.rs:1740`)
- Picture functions in a web-only or cli C program: refused (`crates/clarity-gen/src/lib.rs:971`)

## Conflicts
- Refusal wording differs: "is a PostgreSQL/MySQL/SQL Server database" in the desktop generator (`c/mod.rs:248`), "is a server database" in the cli one (`c/cli.rs:142`)
- Decimals keep 16 places (`runtimes/c/clarity/decimal.h:13`) and hold about 38 digits (`__int128`); R and C# keep 28. Dividing by zero gives a value error (`decimal.c:114`); R, Go, C# panic or throw
- Integer overflow in `+ - *` is undefined behaviour; only `div` has a wrap rule (`runtimes/c/clarity/code.c:60`)
- Uncommitted sign-in changes in `runtimes/c/auth` (CONFLICTING 5)
- DESIGN 1.2 line :60 gives C the webview; every desktop draws with Clarity.UI

## Decisions
- 2026-11-03: the webview desktop (webview/webview) dropped; C links the one Clarity.UI library through CMake; see [clarity-ui](clarity-ui.md)

## Was
- G4 (C part), DESIGN 7.1 (C row)
