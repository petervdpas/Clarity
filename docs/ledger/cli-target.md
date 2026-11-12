# Command-line target

Status: built. `stack <backend> cli`: a command-line program over the same records, actions and settings, with `cli` and `command` declarations, in `out/cli/`, on every stack.

## Code
- Spec: `crates/clarity-spec/src/code/parse.rs:785-849` (`cli`, `command [group] name(args)`, `option` lines, `help @key`); statements `print`, `print list columns`, `ask` as an expression, `exit n`; `crates/clarity-project/src/file.rs:401-473` (`cli` target)
- Generators: `crates/clarity-gen/src/lib.rs:990` (`generate_cli`), :1539 (`cli_toolchain`); `crates/clarity-gen/src/commands.rs` (`CommandSpec`, `ParamSpec`); per stack `crates/clarity-gen/src/{rust,go,c,cpp,csharp}/cli.rs`
- Runtimes: R `runtimes/rust/src/cli.rs` (crate without Clarity.UI); Go `runtimes/go/clarity/cli.go` (module without Clarity.UI), `signin_cli.go`; C `runtimes/c/clarity/cli.c`; C++ `runtimes/cpp/clarity/cli.cpp`; C# `runtimes/csharp/Clarity.Runtime/Cli.cs` (console project on Clarity.Runtime only)
- IDE: Commands in the project tree; Run asks which program and the command line, output and `ask` answers in the Output panel with `CLARITY_TERMINAL=1` (`app/src-tauri/src/build.rs:514`)
- Docs: LANGUAGE.md "Commands" (:1792, :2569)

What each stack writes in `out/cli/`:

| Stack | Program | Build |
|---|---|---|
| R | crate on `clarity-runtime`, no Clarity.UI | `cargo build` |
| Go | module on the runtime module, no Clarity.UI | `go build` |
| C | CMake project, runtime copied in, no desktop | `cmake --workflow --preset` |
| C++ | CMake project, runtime copied in | `cmake --workflow --preset` |
| C# | console project on Clarity.Runtime | `dotnet build` |

- Exit codes: 0 done, 1 refused or failed, 2 wrong command line, `exit n` otherwise.
- Help and error words are Clarity's own keys; language from `settings.language`, else the system locale, else the project's `language`.
- Without a terminal, `ask` with a default gives it; without one, exit 2.
- Sign-in strategy `account` (the OS user) works in cli programs: see [signin](signin.md).

## Tests
- `conformance/command-line.json`: run by R (`runtimes/rust/tests/cli.rs`), Go (`cli_test.go`), C (`cli_test.c`), C++ (`cli_test.cpp`), C# (`CliCaseTests.cs`); skipped by none
- `crates/clarity-spec/tests/commands.rs`; `crates/clarity-gen/tests/commands.rs`
- Examples: `ledger` (csharp cli), `stock` (go desktop cli)

## Open
- Picture functions in a Go, C or C++ cli program: refused (see [picture-functions](picture-functions.md)).
- A record as a command argument: NOT BUILT ("for now", DESIGN 5.20).

## Conflicts
- The cli duration readers say "as formats.json reads them" (`runtimes/rust/src/cli.rs:308`, `runtimes/go/clarity/cli.go:387`), but the R and Go runtime tests do not load `conformance/formats.json`.
- The C and C++ non-SQLite refusal says "is a server database" in the cli generator and "is a PostgreSQL/MySQL/SQL Server database" in the desktop one (`c/cli.rs:142` vs `c/mod.rs:248`; `cpp/cli.rs:123` vs `cpp/mod.rs:253`).
- DESIGN 5.20 still writes `stack go webview desktop cli` and "the UI word"; naming a UI is refused now.

## Decisions
- 2026-10-23: `cli` is a target beside `desktop` and `web`; one folder per target in `out/` (C1, decided with the user).
- 2026-10-23: the parser, help, `ask` and tables are written once per runtime, checked by one shared file; no outside parser library.
- 2026-10-23: everything a cli program writes is a key, as in windows.

## Was
- DESIGN 5.20, roadmap C1
