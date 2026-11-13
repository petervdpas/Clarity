# Generation

Status: partial. `clarity gen` (or Generate in the IDE) turns a checked project into one program per target in `out/`, in the stack's language, built by the stack's own tools; round tripping is not built.

Progress: 70%

## Code
- Spec: `crates/clarity-project/src/file.rs:303` (`version`), :310 (`icon`), :319-336 (`output`, `output keeps`), :401-473 (`stack`); `clarity_spec::translations::with_app_slots` (`{app.version}`)
- Generators: `crates/clarity-gen/src/lib.rs`: `generate` :860-932 (refuse a project with errors, fill app slots, check stores, web secrets, parts in the browser, custom parts' language, place views, compile engines, the stack's generator, codec libraries, `versioned`, README.md, THIRD_PARTY_NOTICES.md for Lucide icons); `programs` :947 (per target; C# desktop + web gives two); `generate_cli` :990; `write_programs_listed` :1031 (`.clarity-out` marker, refuses a folder holding files it did not make :1038); `write_out` :1088, `remove_stale` :1128; `.clarity-generated` and `fingerprint` :1153-1171; `versioned` :1658; `build_output` :1580 (each stack's build folders kept)
- Per stack: `crates/clarity-gen/src/{rust,go,c,cpp,csharp}/mod.rs`, `cli.rs`, `logic.rs`, `model.rs`; windows as `.cxaml` `web_windows.rs`; README `readme.rs`
- Windows resources: `crates/clarity-gen/src/windows_resources.rs` (icon PNG, JPEG, WebP, SVG to `app.ico`; COFF `.rsrc` with icon and VERSIONINFO for amd64, arm64, 386; no resource compiler); R links it from build.rs, Go as `resources_windows_<arch>.syso`, C and C++ from CMakeLists.txt, C# `ApplicationIcon` and `<Version>`
- CLI: `crates/clarity-cli/src/main.rs:253` (`clarity gen <project> [--out]`)
- IDE: `app/src-tauri/src/build.rs:243` (`generate_source`), :292 (`generate_out`)
- Docs: manual `project.run-build.md`; per-stack output in [stack-rust](stack-rust.md), [stack-go](stack-go.md), [stack-c](stack-c.md), [stack-cpp](stack-cpp.md), [stack-csharp](stack-csharp.md); targets in [web-target](web-target.md), [cli-target](cli-target.md)

## Tests
- `crates/clarity-gen/tests/out_layout.rs`, `windows_resources.rs`, `icons.rs`, `readme.rs`, `versions_{rust,go,c,cpp,csharp}.rs`, the per-stack `rust.rs`, `go.rs`, `c.rs`, `cpp.rs`, `csharp.rs`, `everything.rs`
- Ignored (real tools): `examples_build.rs` (one per example, each on its own stack), `everything_build.rs`
- No conformance file (generation is per stack)

## Open
- `clarity gen --verify` for CI: NOT BUILT (DESIGN 6.6, M2).
- Round tripping (RT1): per-file hashes, finding hand-edited files, a three-way merge and its view, hand-written items in the spec: PARTIAL. Only one hash of the whole program exists, used to say "press Generate first" (`lib.rs:1153-1171`).
- `AGENTS.md` and `CLAUDE.md` in generated projects: NOT BUILT (DESIGN 6.5, RT1).
- The generated README carries the project's README: NOT BUILT (DESIGN 5.35; `crates/clarity-gen/src/readme.rs:41`).
- macOS app bundle (Info.plist, .icns): NOT BUILT; macOS gets rpath and Keychain linking only.
- Generator refusals (by design or not yet): `list<X>` fields of stored entities read in code, on all five (`rust/logic.rs:903`, `csharp/logic.rs:1017`, `go/logic.rs:1543`, `c/logic.rs:1740`, `cpp/logic.rs:1101`); C: list literals of non-record, non-value types (`c/logic.rs:1902`), a position in a list of records (:1926), window data of `list<bool>` or `list<Enum>` (:437), `json of` some value types (:2027); Go: a `files ... where` that calls something that can fail (`go/logic.rs:701`); custom part or library in another stack's language (`lib.rs:871`); picture functions in a Go, C or C++ web-only or cli program (`lib.rs:971`); custom part in a web-only program (`lib.rs:1628`); non-SQLite database on C or C++ (`c/mod.rs:246`, `cpp/mod.rs:251`); `keychain:` secrets (`lib.rs:67`); entities without a data source (`lib.rs:53`).

## Conflicts
- DESIGN 6.2-6.4 describe a shell and frontend generator per framework, `generated/` folders and action stubs; the code writes one whole program per target with no owned files.
- DESIGN 6.6 says `clarity gen` writes only inside `generated/`; it writes the whole program folder.
- `crates/clarity-project/src/lib.rs:876-878` says web targets serve the shared web screen; they serve Clarity.UI.
- The C and C++ non-SQLite refusal is worded differently in the desktop and cli generators (`c/mod.rs:248` vs `c/cli.rs:142`; `cpp/mod.rs:253` vs `cpp/cli.rs:123`).

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
