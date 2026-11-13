# Build tools

Status: partial. Clarity finds each stack's tools, says what to install, lets the person pin a copy, builds with the stack's own tools only, and finds the Clarity.UI library.

Progress: 70%

## Code
- Spec: none
- Generators: `crates/clarity-gen/src/tools.rs` (`Probe`, `check` :351, `requirements` :333, `check_all` :463, `report` :484, `engine_tools` :447 for `wasm32-unknown-unknown`, `SETUP_TOPIC` :271; Windows folders, Visual Studio by folder and `vswhere`, MSYS2, Scoop, Chocolatey, WinGet :115-194; library checks through pkg-config on Linux only :402); `crates/clarity-gen/src/tools/environment.rs` (`TOOLS`, 9 tools people can pin :25; `~/.clarity/environment.json`); `crates/clarity-gen/src/lib.rs:1209` (`toolchain`), :1539 (`cli_toolchain`), :1291 (`go_cgo_env`), :1483-1515 (`needs_library_folder`, `uses_clarity_ui`, `with_clarity_ui`)
- Clarity.UI library: `crates/clarity-gen/src/clarity_ui.rs`: the folder `CLARITY_UI` names (`CLARITY_UI_BROWSER` for browser files), else the copy the installed Clarity carries, else a download of the `runtimes/rust-ui-ffi/v<ver>` release checked against SHA256SUMS into Clarity's cache; C# gets it in its NuGet
- Runtimes: none
- CLI: `clarity tools` (a line per stack), `clarity libraries` (rebuild the base libraries from source, `lib.rs:1463`)
- IDE: `app/src/lib/EnvironmentSettings.svelte`, `LibrariesSettings.svelte`, `libraries.ts`, `stackTools.ts`, `tools.svelte.ts`; `app/src-tauri/src/environment.rs`, `libraries.rs`; Build and Run `app/src-tauri/src/build.rs`; Build tools row in New project and the Project tab
- Docs: manual `setup.tools.md`, `project.run-build.md`

Per stack: R cargo; Go go (cgo to Clarity.UI); C and C++ a compiler, CMake, make or Ninja (C++ also nlohmann_json and SQLite3); C# a .NET SDK for `net10.0`. Builds put pinned tools first and pass chosen compilers as `CC` and `CXX`.

Paths for other tools go through `clarity_project::slashed` (plain, `/` between folders).

## Tests
- `crates/clarity-gen/tests/toolchain.rs`, `tools.rs`, `environment.rs`, `installed.rs`, `library_choices.rs`
- `app/src/lib/stackTools.test.ts`, `libraries.test.ts`
- No conformance file

## Open
- The tools check finds C and C++ libraries on Windows and macOS: NOT BUILT. It runs on Linux only (`tools.rs:402`), so C and C++ are called ready without their libraries (DESIGN EN1).
- Install hints for Fedora, Arch, Homebrew: NOT BUILT; only Debian's packages are named (DESIGN EN1).
- `CMAKE_PREFIX_PATH` from the chosen compiler: NOT BUILT (DESIGN 6.5 Windows point 11). C and C++ build on Windows from a terminal, not yet from Clarity.
- Visual Studio's build environment (vcvars) for `cl`: NOT BUILT (DESIGN 6.5 Windows point 12; two ways named, none chosen).
- Building without the internet: see [offline](offline.md).
- Choosing a folder for the Go module in Clarity settings > Libraries: NOT BUILT; the generator writes no `go.work` (the README says how by hand), so the row is read-only.

## Conflicts
- DESIGN.md:175 calls the module `clarity_gen::toolchain`; it is `clarity_gen::tools`.
- DESIGN.md:2439 describes `clarity-tools` as toolchain running; it holds the agents' design tools (see [mcp-tools](mcp-tools.md)).
- setup.tools.md:16, :64 say cargo downloads Skia; Rust links the ready-made library (`runtimes/rust/Cargo.toml:45`). README.md:74 and setup.tools.md:7 name the library for Go, C and C++ only.
- DESIGN P3 names WebKitGTK through pkg-config for Rust; webview desktops are gone.

## Decisions
- P3 (undated in DESIGN): Clarity is installed as a finished program; a missing tool only stops Build and Run, never designing, checking or generating.
- 2026-10-24: Environment settings: every copy a scan finds, the person picks one or types a path, read by the app and the `clarity` command (EN1).
- 2026-11-01: a generated project builds with its stack's own tools only; adding npm, Node or a registry is asked first. User: "why you always try to take shortcuts??? npm and not listening to my conventions"
- 2026-11-08: Rust links Clarity.UI's ready-made library instead of compiling Skia (DESIGN 5.39; see [stack-rust](stack-rust.md)).
- 2026-11-13: Clarity settings > Libraries: per library or package set, Clarity's own for this version or a chosen folder, and a Factory reset; kept in environment.json (`libraries`), handed to builds through `environment::use_libraries`; a chosen folder wins over CLARITY_UI, CLARITY_UI_BROWSER and CLARITY_RUNTIME. Go module and C/C++ runtime read-only. User: "I think settings should have a section that lets you point to the used clarity libraries/crates... with a 'factory reset' to take the libraries that belong to the version of the ide running!"

## Was
- roadmap P3, EN1; DESIGN 6.5 "Windows" points 11-13
- memory: no-new-build-tools.md
