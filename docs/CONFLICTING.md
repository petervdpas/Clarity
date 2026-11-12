# Where sources disagree

This file lists the places where two sources say different things: a doc against the code, docs against each other, or code against code (stacks that behave differently, or two places deciding the same thing). Its partners are `docs/IMPLEMENTED.md` (what the code does) and `docs/UNIMPLEMENTED.md` (what is promised but missing); a missing feature is listed there, not here.

Written 2026-11-12 from the code at that date. When a conflict is settled, remove its line in the same piece of work. Paths are from the repository root.

## Contents

1. [Decisions needed](#1-decisions-needed)
2. [Stacks that behave differently](#2-stacks-that-behave-differently)
3. [Conformance files some stacks skip](#3-conformance-files-some-stacks-skip)
4. [Two places deciding the same thing](#4-two-places-deciding-the-same-thing)
5. [Versions and stale code comments](#5-versions-and-stale-code-comments)
6. [LANGUAGE.md against the code](#6-languagemd-against-the-code)
7. [DESIGN.md against the code](#7-designmd-against-the-code)
8. [The manual and READMEs against the code](#8-the-manual-and-readmes-against-the-code)
9. [Docs against docs](#9-docs-against-docs)

## 1. Decisions needed

Here a rule and the code (or two rules) disagree, and either side could be the one to change.

- **System folder and file pickers.** CLAUDE.md:19 says no dialog of the system, in the IDE and in generated apps. Generated apps open the system's folder and file pickers (`runtimes/rust-ui/src/desktop/files/mod.rs`: osascript, the Windows COM dialog), and LANGUAGE.md:675, 994, DESIGN.md:1843, manual design.parts.md:26 and data.pictures-files.md:58 describe that as intended.
- **Whether Clarity's agent executes.** CLAUDE.md:21 says "Clarity's own agent never executes anything". The agent builds and runs after asking, or without asking when Runs is Allow (`app/src-tauri/src/agent.rs:52, 108, 211`; manual ai.agent.md:38; README.md:142).
- **Undo after an outside agent.** DESIGN.md:82 says an AI's changes are undone by Undo. Manual ai.mcp.md:89 says Undo does not take back an outside agent's turn. (The IDE's own agent turn is one Undo step.)
- **Whole test suites.** CLAUDE.md:31-32 says run only the changed module's tests. README.md:109-113 says run `cargo test`; README also says plain `cargo test` writes the TS bindings, CLAUDE.md:46 says `cargo test -p clarity-spec --lib`.
- **Builds in /tmp.** CLAUDE.md:35 forbids them; README.md:118-119 copies shop to `/tmp/try` and runs it there.
- **Empty values in order.** Table order puts an empty value last (`runtimes/rust/src/tables.rs:3`); comparisons in code put none first on R, Go, C and C++ (section 2). No rule says which is meant.

## 2. Stacks that behave differently

None of these is pinned by a conformance file.

- **Ordered comparisons in C#.** The C# generator writes a plain `(l < r)` for every comparison (`crates/clarity-gen/src/csharp/logic.rs:1277`). On dates (`DateOnly?`) that makes `none < d` false; on R, Go, C and C++ none comes first (`go/logic.rs:1917`, `runtimes/go/clarity/values.go:124`, `runtimes/c/clarity/values.h:37`, `runtimes/cpp/clarity/values.hpp:45`). On text, which the checker allows (`crates/clarity-spec/src/code/check.rs:4939`), C# has no `<` operator for strings, so such code would not compile; the others compare bytes.
- **Decimal division.** Go, C and C++ keep 16 decimals (`runtimes/c/clarity/decimal.h:13`, `runtimes/cpp/clarity/decimal.hpp:20`, shopspring's default in Go); R and C# keep up to 28 digits (`rust/logic.rs:1488`), so `1 / 3` differs. Dividing a decimal by zero gives a value error on C and C++ (`runtimes/c/clarity/decimal.c:114`) and a panic or exception on R, Go and C#. Only `div` and `mod` share one "division by zero" error.
- **Integer overflow in `+ - *`.** R panics in debug builds and wraps in release; Go and C# wrap; C and C++ are undefined behaviour. Only `div` has a shared wrap rule (`runtimes/c/clarity/code.c:60`).
- **Decimal range.** C and C++ hold about 38 digits (`__int128`), R and C# about 28, Go has no limit.
- **Edit bus size.** R, Go, C and C++ keep 500 edits per window (`runtimes/rust/src/edits.rs:37`, `go/clarity/edits.go:40`, `c/clarity/edits.c:44`, `cpp/clarity/edits.cpp:15`); C# never drops any (`runtimes/csharp/Clarity.Runtime/EditBus.cs:29-35`). `conformance/edit-bus.json` has no limit case.
- **The person's keys in C# have two writers.** Clarity.UI keeps them through the program's settings (`runtimes/rust-ui/src/window/mod.rs:104-109`), and the C# program also writes `keys` from `PersonKeys`, read once at start, on every settings save (`crates/clarity-gen/src/csharp/model.rs:287`, `runtimes/csharp/Clarity.Runtime/PersonKeys.cs:25`). A save after the person changes keys may write the old keys back.
- **HTTP timeout text.** C# says "no answer within 30 seconds" (`runtimes/csharp/Clarity.Runtime/Http.cs:260`); the others pass the library's text. `http.json` checks only the start.
- **Refusal wording.** C and C++ say "is a PostgreSQL/MySQL/SQL Server database" in the desktop generator and "is a server database" in the cli generator (`crates/clarity-gen/src/c/mod.rs:248` vs `c/cli.rs:142`; `cpp/mod.rs:253` vs `cpp/cli.rs:123`).

## 3. Conformance files some stacks skip

From grepping each stack's tests for each file name (a name in a source comment does not count).

| File | Skipped by |
|---|---|
| accordions, charts, dropdown-options, folding, ribbons, sidebars, steps, table-pages, trees, ranges | C# (it has its own `PartState.cs`, `Charts.cs`, `DropdownOptions.cs`, `TablePages.cs`, `Trees.cs`, `Ranges.cs`; each names the file in a comment, no test loads it) |
| table-order, custom-parts | C# |
| migrations | R (no runtime test reads it); C# runs it only on server databases, which CI does not have |
| shortcut-keys | Go, C, C++; and `runtimes/web/src/host/keys.ts`'s own tests (the IDE's `app/src/lib/shortcutKeys.test.ts` does load it) |
| secret-store | Go, C, C++ (they write the keychains themselves) |
| picture-functions | every stack's wrapper (only `runtimes/rust-image` runs it) |
| formats (how values are shown and read) | R, Go, C, C++ runtime tests, although their cli duration readers say "as formats.json reads them" (`runtimes/rust/src/cli.rs:308`, `go/clarity/cli.go:387`) |
| calls | C# `Program/Members.cs:116` parses calls with no test against it |

Go's file-source tests are also skipped in CI by the `-run Formats` filter (`.github/workflows/checks.yml:207`); see IMPLEMENTED.md section 13.

## 4. Two places deciding the same thing

Each of these rules is written more than once. "Equal" means the copies agree today.

| Rule | Places | Today |
|---|---|---|
| Color palette | `crates/clarity-spec/src/colors.rs:8`, `runtimes/rust-ui/src/parts/inputs/color.rs:44`, `runtimes/csharp/Clarity.Runtime/Palette.cs:13` | Equal |
| Melodies | `crates/clarity-spec/src/sound.rs:73`, `runtimes/rust-ui/src/melodies.rs:16`, `runtimes/csharp/Clarity.Runtime/Melodies.cs:20` | Equal |
| Key rules | `runtimes/rust/src/keys.rs:99`, `runtimes/csharp/Clarity.Runtime/KeyRules.cs:79`, `runtimes/web/src/host/keys.ts:151` | Equal, except C# counts UTF-16 units for one-letter keys (Rust counts chars, keys.rs:107) |
| Display formats | `runtimes/rust-ui/src/parts/inputs/formats.rs`, `runtimes/web/src/screen/format.ts`, `runtimes/csharp/Clarity.Runtime/Formats.cs` | Equal on the cases |
| Markdown blocks | `runtimes/rust/src/markdown.rs`, `runtimes/web/src/screen/markdownBlocks.ts` | Equal on the cases |
| Ranges, display, translations, list fields, themes, sidebars | rust-ui and `runtimes/web/src/screen/*.ts`, `frame/sidebars.ts` | Equal on the cases |
| Code-language list | `crates/clarity-spec/src/widgets.rs:596`, `runtimes/rust-ui/src/parts/code_colors.rs:9` | Equal |
| Part kinds and their makers | `crates/clarity-spec/src/widgets.rs`, `runtimes/rust-ui/src/parts/mod.rs:585` and the `KINDS` lists | Equal (70 and 70), no test |
| Part properties code can read (34) | each generator's logic.rs (`rust/logic.rs:674`, `go/logic.rs:1568`, `cpp/logic.rs:1128`, `csharp/logic.rs:199`, c) | Equal, no test |
| Layout constants | `crates/clarity-spec/src/layout.rs`; `runtimes/rust-ui/src/grid.rs:16-42`, `window/view.rs:28`, `window/messages.rs:20`, `parts/containers/popover.rs:39`; `runtimes/web/src/screen/grid.ts:9-55` | Equal; `crates/clarity-spec/tests/layout.rs` checks some, not rust-ui's GAP, CELL_HEIGHTS or popover column |
| Port 8080 | `crates/clarity-project/src/lib.rs:285`, `app/src/lib/ProjectSettings.svelte:369` | Equal |
| Sign-in defaults (8 h, 5 tries, 15 min) | `crates/clarity-spec/src/signin.rs:141`, `runtimes/rust-auth/src/attempt.rs:157`, `runtimes/rust/src/signin.rs:74`, `runtimes/go/clarity/signin.go:220`, `runtimes/cpp/clarity/people.hpp:65` | Equal |
| HTTP timeout 30 s, backups keep 10, undo 100 steps | written in each runtime and clarity-db | Equal |
| Non-SQLite refusal on C and C++ | two places per stack (section 2) | Wording differs |
| Highlighter words | `crates/clarity-spec/src/highlight.rs:48-90` against the parsers | `codec` is missing from the keywords |

C# runtime files with no callers outside tests, each a copy of a rule Clarity.UI or clarity-runtime already decides: `Melodies.cs`, `KeyRules.cs`, `TablePages.cs`, `Charts.cs`, `Ranges.cs`, `Palette.cs`, `PersonKeys.Set`.

## 5. Versions and stale code comments

- **Runtimes changed without a version bump.** The uncommitted sign-in work changes `runtimes/go/clarity/signin.go`, `runtimes/rust/src/signin.rs`, `runtimes/rust-ui/src`, `runtimes/csharp/Clarity.Auth/Signins.cs`, `runtimes/rust-auth/src` and `runtimes/c|cpp/auth`, but `version.go` (v0.59.0), `runtimes/rust/Cargo.toml` (0.56.0) and `Directory.Build.props` (0.92.0) are unchanged. Generated programs fetch the published versions, which lack the change.
- **Two Clarity.UI versions.** `runtimes/rust-ui/Cargo.toml` is 0.21.0 and `runtimes/rust-ui-ffi/Cargo.toml` is 0.20.0; downloads follow the ffi version.
- **The tree's own version.** The workspace, `app/package.json` and `tauri.conf.json` say 0.1.0; releases are v0.40.0 (stamped from the tag by `release.yml:8-10`).
- `runtimes/rust/README.md:26` shows `clarity-runtime = "0.10.0"`; the crate is 0.56.0.
- `crates/clarity-gen/src/csharp/logic.rs:557-560` says custom parts are refused "until CP1 builds them"; the code right after generates them.
- `crates/clarity-spec/src/model.rs:1004` says engines run as generated code; they are compiled to WebAssembly.
- `crates/clarity-project/src/lib.rs:876-878` says web targets serve the shared web screen; they serve Clarity.UI.
- `.github/workflows/release-csharp-runtime.yml` says eleven packages; it publishes 13.
- Conformance "about" texts still name Avalonia: `custom-parts.json`, `forms.json`, `tables.json`, `table-order.json`, `display.json`, `themes.json`.
- **VS Code snippet.** The `page` snippet writes a page block (`editors/vscode/snippets/clarity.json`), which the loader refuses (`crates/clarity-spec/src/cxaml.rs:1385`); its test only checks that each snippet parses.

## 6. LANGUAGE.md against the code

### Names

- :37 says `and`, `or`, `not`, `in` are reserved; `contains` is too (`crates/clarity-spec/src/code/parse.rs:1457`).
- :37 says a part may be called `close_button`; part names must be camelCase (`check.rs:1087`).
- :29 names four engine hooks left as written; the code also exempts `clarity_open`, `clarity_alloc`, `clarity_free` and `_` (`names.rs:39-47`).
- Examples with snake_case names the checker refuses: :2038 `draw_mark`, :2269 `log_change`, :2314 `list_templates`, :2653 `loan.returned_on`, :2684 `approve_button`, `ordered_on`.
- :143-161 (sign-in) and :257-260 (sidebar) are written as page blocks, which the loader refuses (`cxaml.rs:1376-1399`).

### Project file

- :93-95 says only `datasource` and `filestore` repeat and has no `codec`; `codec` is a setting and repeats (`crates/clarity-project/src/file.rs:550`).
- :87 and the grammar leave out `beside` and the `file` kind with `using Mapping` (file.rs:484-519).
- :194 says C and C++ use "the shared screen (webview)"; every desktop draws with Clarity.UI (lib.rs:875-881; LANGUAGE.md:77 agrees with the code).

### Statements, expressions, parts

- :2653 says only dates and references can be `none`; pictures and record files can too (`code/check.rs:4867`).
- :2673 gives a precedence order; in the parser `to`, comparisons, `in`, `contains` and `matches` share one level that does not chain, and `div`/`mod` sit at `*` (`code/parse.rs:1293-1388`).
- :2673 says engines have no `to`; `for i in a to b` compiles (`crates/clarity-wasm/src/lower.rs:218`).
- :2611-2640 has no `tell` row (`code/parse.rs:659`).
- :1622 says `nuget` and `dll` custom parts are not generated; C# generates them (`crates/clarity-gen/src/csharp/parts.rs:26, 56, 138`).
- Parts catalog: markdown has `link` (:975 says no events; `widgets.rs:699`); memo has `link` and `toolbox` (:982); number takes `duration` (:986); password takes `secret` (:983); canvas has `connect` and `resize` (:1044); treetable has `frame` (:1034); `store` works on `file` too (:393; `check.rs:2462`).

### Grammar summary (:3028-3149)

Behind the parser in: `stack` (takes two words, not three or more); project settings (no icon, version, output keeps, signin, keys, engines, port, row height, filestore, codec); declarations (no record, const, mapping); enum labels (keys too); entity (no version, savedSince, readable/writable when); types (no duration, image, file, color, secret, folder, value lists); modifiers (no search, hashed, store; label and help take keys); roots (no SignIn, no `allow`); frame commands (no `allow`); custom parts (no crate, go, description); functions and commands (no `allow when`); http (no patch, every page); engines (no progress, tells, hears, beside the screen, const); `find` order by (one dot only); translations (no `resource`); migrations (no search, version, table, key lines); `code` (views too).

## 7. DESIGN.md against the code

### Replaced designs still written as current

- **Shells and UIs.** :46, :112, :1931 describe a UI word in the stack line; naming a UI is refused (`crates/clarity-project/src/file.rs:414-426`). :60 gives C and C++ the webview; :57-58, :64, :1746, :1750, :2025-2026 describe the web renderer, axum and ASP.NET as what web programs serve; every web program serves Clarity.UI's browser files (`crates/clarity-gen/src/web_browser.rs`) and C# uses its own socket server (`runtimes/csharp/Clarity.Ui.Server/WebServer.cs`). :174 says Rust's shell is Tauri. :1906 and :1363 use `stack go webview`.
- **wxWidgets (7.7, :1820-1868, G5 :2518)** is described as the C++ plan with no replaced note; no wx code exists.
- **Web windows as generated code (7.8, :1880-1896)**, with `@clarity/ui` built into apps, `kinds/`, `windowMarkup.ts`; none exists, `runtimes/web` serves only the IDE.
- **Binding table (7.5, :1762-1767)** puts the renderer in a frontend runtime; Clarity.UI in Rust draws for every stack.
- **Clarity.UI per language** (:1904, G7 :2520); there is one Rust library behind `runtimes/rust-ui-ffi`. The routes table (:1937-1943) names SkiaSharp, Silk.NET, CanvasKit; C# `Clarity.Ui` has no such packages.
- **Runtimes copied in** (:168, :2471, :2506, :2513); Rust, C# and Go use crates.io, NuGet and the Go module (`crates/clarity-gen/src/rust/mod.rs:6-8`).
- **Effects** (:441, :768, :2114) point to a directive vocabulary; the language has db.read, db.write, files.read, files.write, net (`crates/clarity-spec/src/code/mod.rs:1086`).
- **Translations** (:203, :214, :255, :1719) as key maps in a UI bundle; programs carry them in `files.json` and the model (C# also `.resx`).
- **Page blocks** (:657, :730 and examples at :469, :566, :660, :1078, :1109, :1138, :1225, :1396); windows are `.cxaml` files.

### Names that moved

- C# Clarity.Ui classes named as current (:269 parts, :72/:177/:1964 `GridLayout`, :1069 `AskDialog`, :1102 `StepsPart`, :1124 `PopoverPart`, :1478 `TrendPart`/`TimerPart`, :1493 `TextBoxPart`, :1507 `KeysWindow`, :1529 `RowsPart`, :876 `EngineCanvasPart`, :485 `WindowStack`/`WindowView`, :1491 `IUnconnectedScreen`) live in `runtimes/rust-ui/src` now; `runtimes/csharp/Clarity.Ui` holds `DesktopWindow.cs` and `Custom/`.
- `Clarity.Ui.Tests` (:270, :1481, :1494, :1998) does not exist; the tests are in `runtimes/rust-ui/tests`.
- Gone files cited as current: `clarity-gen/tests/avalonia.rs`, `frameKeys.ts`, `appKeys.ts`, `KeysWindow.svelte`, `settings.ts`, `Runtime.svelte`, `engineHost.ts`, `screenWorker.ts`, `screenEngine.ts`, `EngineCanvas.svelte` (:880, :1489, :1506-1507, :1520). `app/src/lib/keys.ts` (:1507) is `runtimes/web/src/host/keys.ts`.
- `clarity_gen::toolchain` (:175) is `clarity_gen::tools`. `clarity_spec::diagrams::edit` (:984) does not exist; models are read and written whole (`crates/clarity-spec/src/diagrams/erd_model.rs:80, 184`).
- `ui library` / `ui crate` (:1612) are read and dropped (`crates/clarity-project/src/file.rs:366-368`).
- `clarity-tools` (:2439) is described as toolchain running; it holds the agents' design tools. The workspace list (:2428-2446) and the MCP tool list (:2406-2413) do not match `Cargo.toml` and `crates/clarity-mcp`.

### Counts and status lines

- NuGets: "eight" (:1996, :2012, :273) and "nine" (:2000); there are 13.
- Theme roles: "21" (:240); there are 29 (`crates/clarity-spec/src/themes.rs:33`). Theme JSON "for C# only" (:243, :256, :1739); every stack ships it.
- Examples: "eight" (:1363); there are 20. The removed dice example is cited at :809, :1612, :1617, :1704, :2023, :2025, :2483-2486.
- C# database access: "EF Core or Dapper" (:138); plain ADO.NET (`runtimes/csharp/Clarity.Runtime.Sqlite/SqliteDriver.cs:5`). DESIGN.md:2506 agrees with the code.
- Engines written in Clarity "generated by each stack" (:792); compiled to WebAssembly (:849 agrees with the code). Beside-the-screen hosts (:799, :805) describe the web renderer; Clarity.UI asks the program's host (`runtimes/rust-ui/src/engines.rs:1-7`). Progress (:809, :827-829) describes Tauri, Wails and webview shells.
- Status lines behind the code: :821 (N5 "the rest is not yet"), :855 (records "not yet"), :2526 (N5), :913 and DG1 :2529 (diagrams "nothing is built"; DG1 says mermaid.js draws the designer), :2474 (menu keys "not yet pressed"), TI1 :2490 (timer "planned"), A1 :2534 and M8 :2547 (MCP), P3 :2468 (WebKitGTK).
- :597 lists quit and close as the built-in targets; there are also language, theme, keys, undo, redo, back, forward, sign-in, sign-out (`crates/clarity-spec/src/frame.rs:187, 195`).
- :926 says New diagram copies in a `.mermaid` file; it offers a name, a kind and "from the project or a small example".
- :1505 lists IDE key defaults that differ from `runtimes/web/src/host/keys.ts:25-59` (Ctrl+, is global, F7 Check, Ctrl+Shift+G and B, vault split in two layers).
- :1576 says tabs, steps and sections show a view's tells; :1572 says only `content` shows a view.
- :272 says `drive.mjs` drives the web renderer; it drives the Clarity.UI browser build.

## 8. The manual and READMEs against the code

### Old syntax in the manual

- Page blocks and part lines in modules, which the loader refuses: code.handlers.md:31-34, :80, :120; design.parts.md:46-56; themes.md:215, :246; design.custom-parts.md:122, :127.
- snake_case names the checker refuses: code.handlers.md:97, code.language.md:36, code.functions.md:115-121, code.data-events.md:178, code.queries.md:31-58, code.web-apis.md:117-121 (OpenAPI actions are camelCase, `crates/clarity-project/src/openapi.rs:260-283`).
- Quoted texts in `refuse`, `message`, `label` and `help`, which must be keys (`crates/clarity-spec/src/translations.rs:569, 650, 1025`): code.functions.md:161-163, code.data-events.md:46, code.queries.md:54, code.web-apis.md:16, data.entities.md:53-54.
- code.language.md:17: `contains` is reserved too. code.functions.md:152-157 leaves out the `net` effect.

### The IDE as described

- README.md:25 (manual): Text shows a window as Clarity text; it shows the `.cxaml`.
- start.overview.md:41: the tree names (i18n, not Languages; also README, Sign-in, Views, Mappings, Engines, Custom parts, Libraries, Commands) and the sidebar (also Main window). :52: settings sections are Display, Keys, Folders, Environment, Agent, Logging, System.
- start.first-project.md:48 (and design.windows.md:70-84, design.frame.md:47): no heading "The window"; the fieldsets are Data and Events. :50 vs :73: a button named `add` and markup with `addButton`/`addClick`.
- project.file.md:66: settings hold Migrations and Backups per database; translations open from the tree. :26-38 leaves out `version`, `icon`, `signin`.
- design.windows.md:88: grid columns go to 96, not 24 (`crates/clarity-spec/src/model.rs:594`). :51: New window offers views, splash, toolbox, sign-in and settings too.
- design.text.md:55-56 and app.keys.md:76-83: Tab is always four spaces (`app/src/lib/SourceEditor.svelte:72`); app.keys.md says Escape closes the panel, design.text.md says it goes back to Properties.
- design.parts.md:24: sign-in is set in the tree's Sign-in item, not "Who is using it" in Project settings.
- code.handlers.md:140: Code is an inspector tab, not a button in the Window group.
- code.engines.md:48: the section is "Texts it gives", not "Keys it says".
- data.entities.md:87: the editor also has Search and Hashed columns.
- data.diagrams.md:173-179: New diagram offers ERD and class only.

### Behaviour as described

- data.entities.md:45, :57: `list<Entity>`, `min`, `max`, `pattern` (see UNIMPLEMENTED.md section 2).
- data.pictures-files.md:146 leaves out `.jpeg` and `.svg`.
- languages.md:154 and themes.md:253-256: translations and themes go through "the web renderer" and CSS on Rust, Go, C, C++; every stack uses Clarity.UI and theme JSON. languages.md:132: the locale follows the language shown, not the system.
- design.custom-parts.md:139 says Clarity.UI on Rust and C#; it is all five stacks.
- design.video.md:42 describes Windows and macOS decoders with no caveat; they have not run (VIDEO-PLAN.md:3).
- code.engines.md:321-323 lists `progress` as not built; :97 of the same page documents it, and it is built.

### READMEs

- README.md: `clarity data migrate` needs the project file; the layout leaves out `rust-auth`, `rust-auth-ffi`, `rust-formats`, `rust-formats-ffi`, `rust-graph`; "Rust 1.88" is not set anywhere (no `rust-version`).
- editors/vscode/README.md:9 lists the `page` snippet (section 5); :37 installs version 0.1.3, the extension is 0.1.38.

## 9. Docs against docs

Only those not already covered above.

- **Released or not.** FORMATS-PLAN.md:3 says steps 2-5 unreleased; they are in tag v0.39.0 (APP-CAPABILITIES-PLAN.md:3 agrees). APP-CAPABILITIES-PLAN.md:3 says sign-in and row versions unreleased; they are before tag v0.40.0. GRAPH-PLAN.md:3 and AGENT-TOOLS-PLAN.md:3 say unreleased; both are in v0.39.0.
- **Status behind other docs.** FOUNDRY-PLAN.md:3 says step 2 is being built; ai.connections.md documents it working. IMAGE-PLAN.md has no status; the image tools are built. WINDOWS-TESTS.md:5 says C#'s parts and dialogs come later; DESIGN G7 and the walker example say they are built.
- **Toolbox syntax.** IMAGE-PLAN.md:60-77 uses a `toolbox Painting` block; LANGUAGE.md:2184 and the code use a `<Toolbox>` page (`crates/clarity-spec/src/views.rs:290`).
- **Page blocks in plans.** AUDIO-PLAN.md:40, VIDEO-PLAN.md:30, HARDWARE-PLAN.md:35 use page blocks.
- **Removed things.** PARTS-LIBRARY-PLAN.md:12 and LANGUAGE.md:1491-1512 use the dice example, which was removed. WINDOWS-TESTS.md:11 points to `runtimes/c/shell/shell.c`; it is `runtimes/c/desktop/desktop.c`. WINDOWS-TESTS.md:88 treats C# in the browser as missing; it is built (`crates/clarity-gen/src/csharp/web.rs`). FORMATS-PLAN.md:334 says no example uses a mapping; todo does (`examples/todo/ideas.clarity:13`).
- **Rust and Skia.** setup.tools.md:16, :64 and OFFLINE-PLAN.md:13 say cargo downloads Skia; stacks.rust.md:87 and the code use the ready-made library (`runtimes/rust/Cargo.toml:45`). README.md:74 and setup.tools.md:7 name the library for Go, C and C++ only; OFFLINE-PLAN.md:14 leaves Go out.
- **Releases.** WINDOWS-TESTS.md lists hand release steps and says C and C++ need no release; CLAUDE.md uses the Release Helper and README.md:74 says C and C++ fetch the Clarity.UI release.
- **Data source kinds.** DESIGN.md:121 lists sqlite, postgres, mysql, mssql, rest; LANGUAGE.md:91 and project.file.md:50 add the file formats.
- **macOS keys.** DESIGN 5.28 says Ctrl is Cmd on macOS; WINDOWS-TESTS.md section 6 says it is not done (the code agrees with WINDOWS-TESTS).
- **Roadmap.** DESIGN.md:2455 still says C# proves a native toolkit with Avalonia and lists G5 wxWidgets; DESIGN.md:65 says both were replaced. DESIGN's A2 puts ollama first; Foundry was built and DESIGN does not mention it.
