# Generation

Status: partial. `clarity gen` (or Generate in the IDE) turns a checked project into one program per target in `out/`, in the stack's language, built by the stack's own tools; round tripping is not built.

Progress: 70%

## Code
- Spec: `crates/clarity-project/src/file.rs:305` (`version`), :312 (`icon`), :321-335 (`output keeps`, `output`), :431 (`stack`); `clarity_spec::translations::with_app_slots` (`{app.version}`)
- Generators: `crates/clarity-gen/src/lib.rs`: `generate` :881-956 (refuse a project with errors, fill app slots and the agent settings defaults (`agent_tools::with_agent_defaults` :886), check stores, web secrets, parts in the browser, custom parts' language, place views, compile engines, the stack's generator, codec libraries, `versioned`, README.md, THIRD_PARTY_NOTICES.md for Lucide icons); `programs` :968 (per target; C# desktop + web gives two); `generate_cli` :1011; `write_programs_listed` :1053 (`.clarity-out` marker `OUT_MARKER` :1637, refuses a folder holding files it did not make :1060-1061); `write_out` :1110, `remove_stale` :1167; `.clarity-generated` and `fingerprint` :1192-1210; `versioned` :1698; `build_output` :1620 (each stack's build folders kept)
- Agents: every program carries the agent tool list (`crates/clarity-gen/src/agent_tools.rs`; see [app-mcp](app-mcp.md))
- Per stack: `crates/clarity-gen/src/{rust,go,c,cpp,csharp}/mod.rs`, `cli.rs`, `logic.rs`, `model.rs`; windows as `.cxaml` `web_windows.rs`; README `readme.rs`
- Windows resources: `crates/clarity-gen/src/windows_resources.rs` (icon PNG, JPEG, WebP, SVG to `app.ico`; COFF `.rsrc` with icon and VERSIONINFO for amd64, arm64, 386; no resource compiler); R links it from build.rs, Go as `resources_windows_<arch>.syso`, C and C++ from CMakeLists.txt, C# `ApplicationIcon` and `<Version>`
- CLI: `crates/clarity-cli/src/main.rs:255` (`clarity gen <project> [--out]`)
- IDE: `app/src-tauri/src/build.rs:243` (`generate_source`), :292 (`generate_out`)
- Docs: manual `project.run-build.md`; per-stack output in [stack-rust](stack-rust.md), [stack-go](stack-go.md), [stack-c](stack-c.md), [stack-cpp](stack-cpp.md), [stack-csharp](stack-csharp.md); targets in [web-target](web-target.md), [cli-target](cli-target.md)

## Tests
- `crates/clarity-gen/tests/out_layout.rs`, `windows_resources.rs`, `icons.rs`, `readme.rs`, `versions_{rust,go,c,cpp,csharp}.rs`, the per-stack `rust.rs`, `go.rs`, `c.rs`, `cpp.rs`, `csharp.rs`, `everything.rs`
- Ignored (real tools): `examples_build.rs` (one per example, each on its own stack), `everything_build.rs`
- No conformance file (generation is per stack)

## Open
- `clarity gen --verify` for CI: NOT BUILT (DESIGN 6.6, M2).
- Round tripping (RT1): per-file hashes, finding hand-edited files, a three-way merge and its view, hand-written items in the spec: PARTIAL. Only one hash of the whole program exists, used to say "press Generate first" (`lib.rs:1192-1210`).
- `AGENTS.md` and `CLAUDE.md` in generated projects: NOT BUILT (DESIGN 6.5, RT1).
- The generated README carries the project's README: NOT BUILT (DESIGN 5.35; `crates/clarity-gen/src/readme.rs:41`).
- macOS app bundle (Info.plist, .icns): NOT BUILT; macOS gets rpath and Keychain linking only.
- Generator refusals (by design or not yet): `list<X>` fields of stored entities read in code, on all five (`rust/logic.rs:974`, `csharp/logic.rs:1017`, `go/logic.rs:1546`, `c/logic.rs:1743`, `cpp/logic.rs:1104`); C: list literals of non-record, non-value types (`c/logic.rs:1905`), a position in a list of records (:1929), window data of `list<bool>` or `list<Enum>` (:437), `json of` some value types (near :2030); Go: a `files ... where` that calls something that can fail (`go/logic.rs:701`); custom part or library in another stack's language (`lib.rs:892`); picture functions in a Go, C or C++ web-only or cli program (`pictures_without_library`, `lib.rs:993`); custom part in a web-only program (`lib.rs:1668`); non-SQLite database on C or C++ (`c/mod.rs:251-259`, `cpp/mod.rs:262`); `keychain:` secrets (`lib.rs:68`); entities without a data source (`lib.rs:55`).

## Conflicts
- `crates/clarity-project/src/lib.rs:967-969` says web targets serve the shared web screen; they serve Clarity.UI.
- The C and C++ non-SQLite refusal is worded differently in the desktop and cli generators (`c/mod.rs:252-253` vs `c/cli.rs:142`; `cpp/mod.rs:262` vs `cpp/cli.rs:124`).

## Decisions
- 2026-10-16: generation is deterministic: the same project and generator version give byte-identical output.
- 2026-10-23: one folder per target, always: `out/desktop`, `out/web`, `out/cli` (C1).
- 2026-10-23: round tripping is planned as seams per file, hand-written items, then a three-way merge; reading hand edits back is not planned (raised by the user, RT1).
- 2026-11-01: a project transpiles to a source tree in its stack's language, built and run in or out of the IDE; not a screen per stack. User: "I never ever said that every stack should have 'it's own screen'"
- 2026-11-01: windows are transpiled too, as `.cxaml` markup. User: "clarity code should direct everything as a transpiler language"
- 2026-11-06: the app version is optional; each stack carries it its own way (Cargo.toml, `<Version>`, CMake `project(VERSION)`, Go through the Windows resource); no new build tool.
- 2026-11-06: Clarity writes the Windows icon and version resource itself and hands it to each stack's build.

## Was
- DESIGN 6, 6.1, 6.2, 6.3, 6.4, 6.6, 6.7; roadmap RT1, M2 (`clarity gen --verify`)
- VERSION-PLAN.md steps 2-4 (step 1, the setting: see [project-file](project-file.md))
- APP-ICON-SPLASH-PLAN.md step 2 (step 1: see [project-file](project-file.md); step 3, the splash: see [windows-dialogs](windows-dialogs.md))
- memory: version-plan.md, standalone-executable.md, transpiler-everything.md, generated-code-per-stack.md
