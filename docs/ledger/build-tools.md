# Build tools

Status: partial. Clarity finds each stack's tools, says what to install, lets the person pin a copy, builds with the stack's own tools only, and finds the Clarity.UI library.

Progress: 70%

## Code
- Generate, Build and Run: `crates/clarity-build` (`generate`, the one way to generate: the IDE's Generate, `clarity gen` and the agents' generate tool hook into it, `clarity-tools/src/generate.rs` removed; user 2026-11-23: "align the generate paths!! the must be only one way and 'ide/agents OR WHATEVER' can hook into it!!!", "that accounts for all tools... only ONE version should exist!!!"; `generate_out`, `ready`, `toolchain`, `compile`, `migrate`, `engines::build_changed`; lines to a `Say`), used by the IDE (`app/src-tauri/src/build.rs`, `engines.rs`: the Output panel, events, the running program) and by `clarity build` / `clarity run` (`crates/clarity-cli/src/main.rs` `build_or_run`); manual project.run-build.md "From the terminal". BUILT 2026-11-23.
- Spec: none
- Generators: `crates/clarity-gen/src/tools.rs` (`Probe`, `check` :357, `requirements` :339, `check_all` :471, `report` :492, `engine_tools` :455 for `wasm32-unknown-unknown`, `SETUP_TOPIC` :277; Windows folders, Visual Studio by folder and `vswhere`, MSYS2, Scoop, Chocolatey, WinGet :99-200; library checks through pkg-config on Linux only :407-418); `crates/clarity-gen/src/tools/environment.rs` (`TOOLS`, 9 tools people can pin :25; `~/.clarity/environment.json`; `use_libraries` :189, also called by `crates/clarity-cli/src/main.rs:245`); `crates/clarity-gen/src/lib.rs:1248` (`toolchain`), :1579 (`cli_toolchain`), :1330 (`go_cgo_env`), :1523-1543 (`needs_library_folder`, `uses_clarity_ui`, `with_clarity_ui`)
- Clarity.UI library: `crates/clarity-gen/src/clarity_ui.rs`: the folder `CLARITY_UI` names (`CLARITY_UI_BROWSER` for browser files), else the copy the installed Clarity carries, else a download of the `runtimes/rust-ui-ffi/v<ver>` release checked against SHA256SUMS into Clarity's cache; C# gets it in its NuGet
- Runtimes: none
- CLI: `clarity tools` (a line per stack), `clarity libraries` (rebuild the base libraries from source, `lib.rs:1503`)
- IDE: `app/src/lib/EnvironmentSettings.svelte`, `stackTools.ts`, `tools.svelte.ts`; `app/src-tauri/src/environment.rs`; Build and Run `app/src-tauri/src/build.rs`; Build tools row in New project and the Project tab
- Clarity's own libraries in a dev IDE (Clarity settings > DevTools): see [devtools](devtools.md)
- Docs: manual `setup.tools.md`, `project.run-build.md`

Per stack: R cargo; Go go (cgo to Clarity.UI); C and C++ a compiler, CMake, make or Ninja (C++ also nlohmann_json and SQLite3); C# a .NET SDK for `net10.0`. Builds put pinned tools first and pass chosen compilers as `CC` and `CXX`.

Paths for other tools go through `clarity_project::slashed` (plain, `/` between folders).

## Tests
- `crates/clarity-build/tests/build.rs` (Generate's checks, the tools message, the port, C#'s source; fetching the library, ignored); `crates/clarity-cli/tests/build_run.rs`: `build_and_run_refuse_a_project_not_generated_yet`, and `a_generated_project_builds_and_runs_from_the_terminal` (ignored: Go and the internet)
- `crates/clarity-gen/tests/gen/toolchain.rs`, `tools.rs`, `environment.rs`, `installed.rs`, `library_choices.rs`
- `app/src/lib/stackTools.test.ts`, `libraries.test.ts`
- No conformance file

- 2026-11-23, from a Windows run of 1.9.0 (Settings > Environment showed every stack green): Rust's MSVC toolchain (`rustc -vV` host `-pc-windows-msvc`) needs Visual Studio's own linker (`Probe::msvc_linker`, link.exe in a Visual Studio installation's compiler folder, found by Program Files or vswhere); without it Rust is missing "Visual Studio's C++ build tools", where before it was called ready and cargo took MSYS2's coreutils `link.exe` (`tools::rust_with_msvc_needs_visual_studio_s_build_tools`). The Clarity.UI folder reaches every build plain (`clarity_project::plain_in`, `with_clarity_ui_on`; `set_bundled`, `set_bundled_browser`, `set_carried`, the custom part build): Tauri's `resource_dir()` names the installed folder `\\?\C:\Program Files\Clarity IDE`, which gcc took as `//?/C:/...` and found no `clarity_ui.h` (`toolchain::clarity_ui_goes_to_the_build_without_a_verbatim_prefix`, `settings::a_verbatim_path_is_plain_by_windows_rule_on_every_system`).

## Open
- Self-contained projects: NOT BUILT, no design beyond below; see Design "Self-contained projects".
- The tools check finds C and C++ libraries on Windows and macOS: NOT BUILT. It runs on Linux only (`tools.rs:407-418`), so C and C++ are called ready without their libraries (DESIGN EN1).
- Install hints for Fedora, Arch, Homebrew: NOT BUILT; only Debian's packages are named (DESIGN EN1).
- `CMAKE_PREFIX_PATH` from the chosen compiler: NOT BUILT (DESIGN 6.5 Windows point 11). C and C++ build on Windows from a terminal, not yet from Clarity.
- Visual Studio's build environment (vcvars) for `cl`: NOT BUILT (DESIGN 6.5 Windows point 12; two ways named, none chosen).
- Building without the internet: see [offline](offline.md).
- Environment guiding the whole toolbelt: NOT DESIGNED. User (2026-11-17): "the environment in settings should guide a dev better in setting up its toolbelt". Engines' Install is the first piece; the rest waits for an agreed design.

## Design

### Self-contained projects

Asked 2026-11-23, after a Windows run of 1.9.0 where builds reached outside the project (an example's engine took `clarity-engine` from the repository, the Clarity.UI folder came from where Clarity is installed, C and C++ libraries from the system). User: "My view is that projects are not self-contained enough!!", "a project should enclose as much as possible to scan the right paths?", and on "a project carries what its build needs inside its own folder, and the build looks there first, not wherever the machine has it": "yes that's self-contained".

- A project copied anywhere builds the same: what its build needs is in its own folder, and the build looks there first.
- What it is, in the user's words: "see the self-contained as a registry gathered for tool paths". A project keeps a registry of the paths its build takes its tools and libraries from (cargo, the C and C++ compilers, CMake, Visual Studio's build tools, Clarity.UI's library, ...), gathered once and read by every build, so the build scans the right paths instead of whatever the machine's PATH holds.
- Open questions on the registry (to ask): its file and place in the project; who gathers it (Generate, the Environment settings, a command); whether git keeps it (paths differ per machine) or each machine gathers its own; what happens when a path in it is gone.
- Open questions (to ask, none decided): which pieces a project carries (the engine library, a pinned Clarity.UI library, the C and C++ libraries, the stack's packages), in which folder of the project, whether git keeps them, and how a project gets them (copied in by Generate, or on a person's choice).

### `clarity build` and `clarity run`

User 2026-11-23: "all commands like generate, build and run should be able to be done from the terminal (as in clarity generate <project.clarity>, clarity build <pr... etc)", then "yes, add clarity build and clarity run", "yes, clarity-build is fine". The IDE's Build and Run move into `crates/clarity-build`, which the IDE and the command-line tool share; the IDE keeps its Output panel and its process handling. `clarity build <project.clarity> [--release]` builds what Generate wrote; `clarity run <project.clarity>` builds when needed, migrates the databases and starts the program in the terminal, a command-line program with the words after `--`. Both refuse a project that changed since it was generated, as the IDE does.

## Conflicts
- none. (Fixed 2026-11-16: manual setup.tools.md said cargo downloads Skia for Rust; Rust links Clarity.UI's ready-made library, `runtimes/rust/Cargo.toml:41-45`, `crates/clarity-gen/src/rust/mod.rs:107-109`; nothing downloads or builds Skia. README.md:75 is in [docs](docs.md).)

## Decisions
- 2026-11-23: `clarity build` and `clarity run` in a crate `clarity-build` shared with the IDE. User: "yes, add clarity build and clarity run", "yes, clarity-build is fine".
- P3 (undated in DESIGN): Clarity is installed as a finished program; a missing tool only stops Build and Run, never designing, checking or generating.
- 2026-10-24: Environment settings: every copy a scan finds, the person picks one or types a path, read by the app and the `clarity` command (EN1).
- 2026-11-01: a generated project builds with its stack's own tools only; adding npm, Node or a registry is asked first. User: "why you always try to take shortcuts??? npm and not listening to my conventions"
- 2026-11-08: Rust links Clarity.UI's ready-made library instead of compiling Skia (DESIGN 5.39; see [stack-rust](stack-rust.md)).
- 2026-11-13: Clarity settings > Libraries: per library or package set, Clarity's own for this version or a chosen folder, and a Factory reset; kept in environment.json (`libraries`), handed to builds through `environment::use_libraries`; a chosen folder wins over CLARITY_UI, CLARITY_UI_BROWSER and CLARITY_RUNTIME. Go module and C/C++ runtime read-only. User: "I think settings should have a section that lets you point to the used clarity libraries/crates... with a 'factory reset' to take the libraries that belong to the version of the ide running!"
- 2026-11-17: on a fresh Windows VM, calculator (Go), modelling (Rust) and novel (C++) could not find DLLs. Clarity's Windows libraries need Microsoft's C++ runtime (vcruntime140.dll, msvcp140.dll). The library archives and the installer ship vcruntime140.dll, vcruntime140_1.dll and msvcp140.dll beside them (`.github/scripts/vc_runtime.py`), checked by `.github/scripts/check_dll_imports.py`; C and C++ (CMake), Rust (build.rs) and Go (`libraries_beside`, after `go build` in the IDE, which put nothing beside the program before) copy them beside the program; guarded by `tests/windows_runtime.rs`. Linking the runtime in (+crt-static) needs a static prebuilt Skia, which rust-skia does not publish. C#'s NuGets do not carry the runtime yet (each package has its own native folder; one file in several packages breaks publish). MSYS2's own DLLs for C and C++ programs are documented in manual/setup.tools.md, not shipped. User: "yes, go ahead with 1 to 4... and maybe better instructions for the c++ redistributable?"
- 2026-11-17: on Windows, Clarity settings > Environment and Run flashed console windows open and shut: every program Clarity started (tool probes, builds, the built program) got a console of its own. All of them start through `clarity_project::background_command` (CREATE_NO_WINDOW; the ACP agent's tokio command sets the flag itself), guarded by `crates/clarity-project/tests/background.rs`, which fails on any other `Command::new` in Clarity's code. User: "should be background processes?". A desktop program alone (stack desktop, no web; `window_program_only`) is now built as a Windows GUI program: Rust `#![windows_subsystem = "windows"]`, Go `-ldflags=-H=windowsgui` (`go_ldflags`), C and C++ `clarity_window_program` (WIN32_EXECUTABLE, MSVC `/ENTRY:mainCRTStartup`), C# was WinExe already; one that also serves keeps its console for `--serve`. C#'s packages carry the C++ runtime once, in Clarity.Runtime (every native package stands on it), C# runtime 0.100.0. The manual window opens visible on its dark ground (WebView2 did not style a page loaded in a hidden window). User: "Not in it??"
- 2026-11-17: Clarity settings > Environment showed every stack Ready while an engine's build failed for want of `wasm32-unknown-unknown`. It now has an Engines block (`tools::engine_check`: what is missing, the setup page's part, and `ENGINE_INSTALL` when rustup is there to add the target; `EnginesView`), with Install, which asks first and runs only `rustup target add wasm32-unknown-unknown` (`install_engine_target`); never Rust itself. Test `the_webassembly_target_is_installed_with_rustup`. User: "Yes, line + Install"; "the environment in settings should guide a dev better in setting up its toolbelt" (open below).

## Was
- roadmap P3, EN1; DESIGN 6.5 "Windows" points 11-13
- memory: no-new-build-tools.md
