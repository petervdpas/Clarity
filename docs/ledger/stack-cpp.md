# C++ stack

Status: built. `stack cpp ...`: a C++20 CMake project with the C++ runtime copied in, drawn by Clarity.UI's library.

Progress: 90%

## Code
- Generators: `crates/clarity-gen/src/cpp/` (logic.rs 1777 lines, mod.rs 981, model.rs 435, cli.rs 332, parts.rs 304, names.rs 137). Output: `src/records.hpp/.cpp`, `src/modules/<module>.cpp`, `src/app.hpp`, `ui/` (windows, app frames, themes, resources), `CMakePresets.json` (`cpp/mod.rs:1-11`)
- Runtimes: C++ `runtimes/cpp`, no version of its own, copied into each program as `clarity-runtime/`:
  - `clarity/`: db, sqlite, sql, dialect, query, values, decimal, text, texts, code, edits, backup, settings, store, files, picture, record_file, people, paths, ui, views, watch, splash, cli and part rules (tables, trees, ranges, steps, charts, dropdown_options, reorder, canvas)
  - `shell/` (desktop.cpp, service.cpp, serve.cpp), `ui/ui.cpp`
  - `auth/`, `formats/`, `http/` (libcurl), `pictures/`, `secrets/`, `engine/` (wasm3, vendored in `runtimes/c/third_party/wasm3`; `runtimes/cpp/clarity.cmake:381-386`)
  - Needs nlohmann_json and SQLite3 from the system; `clarity.cmake`, `clarity_ui.cmake`
- Desktop program: links Clarity.UI where CLARITY_UI says; web serves the browser files and answers over HTTP (`cpp/mod.rs:6-10`); see [web-target](web-target.md)
- Docs: manual/stacks.cpp.md

## Tests
- Runtime tests: `runtimes/cpp/tests/*_test.cpp` (main.cpp, scenario_test.cpp, shell_test.cpp, serve_test.cpp, ...)
- Conformance run by C++: every runtime file except `shortcut-keys`, `secret-store`, `formats` (display formats), `picture-functions` (CONFLICTING 3)
- Generator: `crates/clarity-gen/tests/cpp.rs`, versions_cpp.rs, signin_cpp.rs; build: cpp_build.rs (ignored)
- Examples on this stack: novel, orbs (`cpp desktop`); no example uses cpp web

## Open
- PostgreSQL, MySQL, SQL Server: NOT BUILT. Refused (`crates/clarity-gen/src/cpp/mod.rs:251-253`); see [databases](databases.md)
- `list<Entity>` fields: NOT BUILT (`cpp/logic.rs:1101`)
- Picture functions in a web-only or cli C++ program: refused (`crates/clarity-gen/src/lib.rs:971`)

## Conflicts
- Refusal wording differs: desktop generator `cpp/mod.rs:253` vs cli `cpp/cli.rs:123`
- Decimals keep 16 places (`runtimes/cpp/clarity/decimal.hpp:20`), about 38 digits; dividing by zero gives a value error; integer overflow undefined (CONFLICTING 2)
- Uncommitted sign-in changes in `runtimes/cpp/auth` (CONFLICTING 5)
- DESIGN 7.7 and G5 describe wxWidgets as the C++ plan with no replaced note; no wx code exists

## Decisions
- 2026-10-29: wxWidgets for C++ decided (G5); replaced when Clarity.UI became the one UI (2026-11-03); nothing was built
- 2026-11-03: the webview desktop dropped; C++ links the one Clarity.UI library; see [clarity-ui](clarity-ui.md)

## Was
- G4 (G4a, C++), G5, DESIGN 7.7, DESIGN 7.1 (C++ row)
