# Clarity.UI

Status: built. One Rust library that draws every window of every generated program (parts, layout, look, input) on the desktop and in the browser, used by all five stacks.

Progress: 90%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (catalog, 70 kinds), `layout.rs` (window 1100 x 760, row 36, gap 8), `cxaml.rs` (markup read and written)
- Generators: `crates/clarity-gen/src/web_windows.rs` (window markup a program carries), `web_browser.rs` (`browser/` folder, `files.json`), `clarity_ui.rs` (where the library comes from: `CLARITY_UI`, the installed copy, else the `runtimes/rust-ui-ffi/v<ver>` release checked against SHA256SUMS), `preview.rs` (the IDE's preview server)
- Runtimes: R `runtimes/rust/src/native_ui.rs`, `program.rs`, `wire.rs`, `serve.rs`; Go `runtimes/go/ui/` (cgo), `runtimes/go/shell/program.go`; C `runtimes/c/desktop/desktop.c`, `runtimes/c/service/`; C++ `runtimes/cpp/ui/ui.hpp`, `runtimes/cpp/shell/desktop.cpp`; C# `runtimes/csharp/Clarity.Ui/` (P/Invoke layer, `DesktopWindow.cs`, `Custom/`), `Clarity.Runtime/Program/ProgramWire.cs`
- Clarity.UI: `runtimes/rust-ui/src/` (151 files, about 53,000 lines): `screen.rs`, `element.rs`, `grid.rs`, `painter.rs`, `fonts/`, `markup.rs`, `xml.rs`, `parts/mod.rs` (makers: `DRAWN` :585 plus the `KINDS` of `inputs.rs`, `containers.rs`, `data.rs`, `signin_view.rs`; `register` :613; `MissingPart` shows a missing or failed kind in its place :661-699), `desktop/` (winit, glutin, files, sound, icon), `browser/` (Web Worker, WebGL2), `access/` (AccessKit and the browser mirror), `program_wire/`, `headless.rs` (`CLARITY_HEADLESS`), `catalog/*.json`, `themes/*.json`
- C interface: `runtimes/rust-ui-ffi/src/`, header `include/clarity_ui.h`; browser build `runtimes/rust-ui-browser/` (`build.sh`, Emscripten)
- IDE: `app/src/lib/ClarityWindow.svelte` (preview and canvas draw with the browser build)
- Docs: manual `design.parts.md`, `design.windows.md`; LANGUAGE.md sections 5 and 6

## Tests
- `conformance/grid-layout.json`: run by rust-ui (`tests/screen/grid_layout.rs`), `crates/clarity-spec/tests/layout.rs`, the IDE
- `conformance/windows.json`, `forms.json`, `tables.json`, `small-parts.json`, `overlays.json`, `display.json`: run by rust-ui only
- `runtimes/rust-ui/tests/` (1248 tests: parts/, inputs/, data/, windows/, screen/, hosts/, language/); `tests/screen/markup.rs:180` (a kind without a maker); `tests/windows/access.rs`; `tests/hosts/browser_host.rs`, `browser_page.rs` (headless Chromium), `wire_round_trip.rs`
- `runtimes/rust-ui-ffi/src/tests/`; Go `runtimes/go/ui/ui_test.go`
- `crates/clarity-gen/tests/everything_live.rs` (driven as a person: go, rust, c, cpp; ignored), `everything_build.rs`, `web_windows.rs`, `rust_ui_library.rs`, `browser_fresh.rs`
- Guards: `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-gen --test web_windows` writes `runtimes/rust-ui/catalog/*.json`

## Open
- Reflow of the grid on narrow windows and the web: NOT BUILT (DESIGN.md:487, 2462).
- A check that a container's contents fit its height: NOT BUILT (DESIGN.md:2462).
- Files dropped from the system onto a part: NOT BUILT (DESIGN.md:1921).
- A test that every catalog kind has a maker: NOT BUILT (kinds and makers equal today, 70 and 70).
- The C# program driven live in tests: NOT BUILT (`everything_live.rs` has go, rust, c, cpp).
- Splitting the large files by job (MODULAR-PLAN step 4): NOT BUILT. `parts/data/table.rs` 2287, `parts/data/canvas.rs` 1956, `canvas/modes.rs` 1077, `desktop/mod.rs` 1428, `window/mod.rs` 1251, `screen.rs` 1228, `parts/text_editor.rs` 1198.
- Screen readers (Orca, NVDA, VoiceOver, TalkBack) with AccessKit and the browser mirror: UNTRIED (DESIGN.md:2032-2034).
- Go, C, C++ and C# programs on Clarity.UI on Windows and macOS; the web target in Edge: UNTRIED (WINDOWS-TESTS.md:96, 104-110, 144). See [os-checks](os-checks.md).
- System file and folder pickers: see [parts-inputs](parts-inputs.md).

## Conflicts
- Part kinds and their makers: `crates/clarity-spec/src/widgets.rs` and `runtimes/rust-ui/src/parts/mod.rs:585` with the `KINDS` lists; equal, no test.
- Part properties code can read (34): each generator's logic.rs (`rust/logic.rs:674`, `go/logic.rs:1568`, `cpp/logic.rs:1128`, `csharp/logic.rs:199`, c); equal, no test.
- Layout constants: `crates/clarity-spec/src/layout.rs` against `runtimes/rust-ui/src/grid.rs:16-42`, `window/view.rs:28`, `window/messages.rs:20`, `parts/containers/popover.rs:39`, `runtimes/web/src/screen/grid.ts:9-55`; `crates/clarity-spec/tests/layout.rs` checks some, not GAP, CELL_HEIGHTS or the popover column.
- Two versions: `runtimes/rust-ui/Cargo.toml` 0.21.0, `runtimes/rust-ui-ffi/Cargo.toml` 0.20.0; downloads follow the ffi version.
- DESIGN.md:1904 says each stack has its own version; :1937-1943 names SkiaSharp, Silk.NET, CanvasKit. One Rust library behind `runtimes/rust-ui-ffi` (DESIGN.md:1992 agrees with the code).
- DESIGN.md 1.6 (:269-270) names `runtimes/csharp/Clarity.Ui/Parts` and `Clarity.Ui.Tests`; both gone, tests are in `runtimes/rust-ui/tests`. :272 says `drive.mjs` drives the web renderer; it drives the Clarity.UI browser build.
- DESIGN.md 7.5 (:1762-1767) and 7.8 (:1880-1896, G6) describe a frontend renderer and `@clarity/ui` in apps; `runtimes/web` serves only the IDE.
- Conformance "about" texts name Avalonia: `forms.json`, `tables.json`, `display.json`, `custom-parts.json`, `table-order.json`, `themes.json`.

## Decisions
- 2026-10-19: a part behaves alike on every stack, pinned by `conformance/`. User: "you did realize the 'widgets' need to work (very much) the same on every language/framework right?"
- 2026-10-30: a part that fails shows an error in its own place; the rest goes on. User: "I meant to make guards that guard any such issue... make code fail safe!!!!!!!!!"
- 2026-10-31: a runtime decides, a screen shows and tells (DESIGN 1.7). User: "I highly doubt whether the language stacks follow the rule of 'backend drives frontend' ... and then I'm not talking about undo alone"
- 2026-11-01: a thin host plus one module per part kind. User: "this isnt modular for a bit, otherwise you could split it up and do stuff in parallel... this architecture you made from it is wrong..."
- 2026-11-01: Clarity.UI draws itself, desktop and web, with Skia from prebuilt binaries. User: "SHOULD Clarity.UI DRAW YOU THINK?", "and the framework should be able to do both desktop and web!!!!!"
- 2026-11-02: Avalonia removed; its code is a reference in git (3d8baa0). User: "I NO LONGER WANT TO SEE AVALONIA AT ALL"
- 2026-11-02: a desktop program never uses a webview. User: "to me desktop is nothing web, get it?"
- 2026-11-03: one Clarity.UI in Rust for all five languages, reached through a C interface; the program boundary (`PartHost`, `Program`) is plain data.
- 2026-11-04: screen readers through AccessKit (the user's choice), one core for every language.
- 2026-11-08: Rust desktops link the ready-made library too; `ui library` / `ui crate` is read and dropped since v0.30.0 (`crates/clarity-project/src/file.rs:366`).
- 2026-11-12: the seven files over 1,000 lines go into the modular plan. User: "yes, add them to the plan"

## Was
- DESIGN 1.6, 1.7, 5.6 (catalog, snap grid, containers), 5.39, 7.5, 7.8 (G6, G6a-G6d), 7.9 (G7, G7a-G7e, C1, R1 and R2 of 7.9, L, B1, B2, G8, G8a-G8d, A1 screen readers)
- Roadmap: G6, G7
- MODULAR-PLAN.md step 4 (Clarity.UI's large files)
- WINDOWS-TESTS.md section 5c; :139, :142, :144
- memory: clarity-ui-per-language.md, desktop-is-not-web.md, thin-screens.md, one-screen-all-stacks.md, transpiler-everything.md, fail-safe-code.md
