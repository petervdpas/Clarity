# What is built

This file says what Clarity does today, taken from the source code and its tests, never from the plans. `docs/DESIGN.md` and the `docs/*-PLAN.md` files record decisions and history; this file records the result. Every line points at the code that does it (paths from the repository root) and, where there is one, the test that pins it.

Written 2026-11-12 from the code at that date. When code changes what a line here says, change the line in the same piece of work.

Stack names: R = Rust, Go, C, C++, C#.

## Contents

1. [At a glance](#1-at-a-glance)
2. [The project file](#2-the-project-file)
3. [The language](#3-the-language)
4. [Parts](#4-parts)
5. [Windows, frame, themes and dialogs](#5-windows-frame-themes-and-dialogs)
6. [Data and app services](#6-data-and-app-services)
7. [Generation and build output](#7-generation-and-build-output)
8. [Runtimes and packages](#8-runtimes-and-packages)
9. [The IDE](#9-the-ide)
10. [AI: Clarity's agent, outside agents, tools](#10-ai-claritys-agent-outside-agents-tools)
11. [The command line](#11-the-command-line)
12. [Editor support](#12-editor-support)
13. [Checks and releases on GitHub](#13-checks-and-releases-on-github)
14. [Not built](#14-not-built)

## 1. At a glance

A project is a project file, `.clarity` modules and one `.cxaml` file per window or view in `design/`. `clarity gen` (or Generate in the IDE) writes a complete program in the stack's language; the stack's own toolchain builds it. Every program draws its windows with Clarity.UI, one Rust library (`runtimes/rust-ui`) reached through a C interface (`runtimes/rust-ui-ffi`) from Go, C, C++ and C#, and compiled to WebAssembly for the browser (`runtimes/rust-ui-browser`).

| Target | R | Go | C | C++ | C# |
|---|---|---|---|---|---|
| desktop | yes | yes | yes | yes | yes |
| web (server + Clarity.UI in the browser) | yes | yes | yes | yes | yes, as a second program |
| cli | yes | yes | yes | yes | yes |

- `stack <backend> desktop web` is one program on R, Go, C and C++ (`--serve` serves instead of opening a window: `crates/clarity-gen/src/rust/mod.rs:816`, `go/mod.rs:938`); on C# it is two programs, `out/desktop/` and `out/web/` (`crates/clarity-gen/src/lib.rs:953`).
- The web program keeps the application on the server; the browser runs Clarity.UI in a Web Worker, draws with Skia through WebGL2 and sends each command to the server (`runtimes/rust-ui/src/browser/`, `crates/clarity-gen/src/web_browser.rs`).
- Operating systems: Linux, Windows and macOS for programs (Clarity.UI library built for linux, win and osx, x64 and arm64: `.github/workflows/release-clarity-ui.yml`). The IDE installers are Linux and Windows only (section 13).

Examples and their stacks (`examples/*.clarity`):

| Stack line | Examples |
|---|---|
| rust desktop | dashboard, modelling, network, shop |
| go desktop | calculator, dial, pills, records, videoplayer, waves |
| go desktop cli | stock |
| go web | invaders |
| csharp desktop | mp3player, noughts, todo |
| csharp web | walker |
| csharp cli | ledger |
| c desktop | kitchen |
| cpp desktop | novel, orbs |

No example uses rust web, c web or cpp web; those are built by `crates/clarity-gen/tests/everything_build.rs` and the `the_web_target_serves_in_*` tests.

## 2. The project file

Parsed in `crates/clarity-project/src/file.rs:293-569`; the settings are `Settings` in `crates/clarity-project/src/lib.rs:213-277`. An unknown line is an error that lists the valid ones.

| Line | Meaning | Code |
|---|---|---|
| `title "..."` | Name people see; no `{...}` slots | file.rs:294 |
| `version "1.2.0"` | major.minor.patch, optional pre-release | file.rs:303 |
| `icon "app.png"` | A picture among the resources | file.rs:310 |
| `stack <backend> <targets>` | rust, go, csharp, c, cpp; desktop, web, cli. Naming a UI is an error (Clarity.UI is the only one) | file.rs:401-473, lib.rs:878 |
| `output "out"`, `output keeps "a"` | Output folder (not `.`) and folders in it Clarity leaves alone | file.rs:319-336 |
| `language "en"` | Default language | file.rs:337 |
| `currency EUR` | ISO 4217 | file.rs:385 |
| `theme A B` | Themes offered, first one starts | file.rs:393 |
| `signin ...` | Sign-in block (section 6) | file.rs:345 |
| `keys editable` | People can change the app's keys | file.rs:351 |
| `engines interpreted` / `compiled` | How engines written in Clarity are built | file.rs:358 |
| `port N` | Web port, default 8080, address `127.0.0.1:<port>` | file.rs:369, lib.rs:285 |
| `row height N` | | file.rs:377 |
| `datasource name kind "where"` | sqlite, postgres, mysql, mssql, rest, file or a format name; flags external, readonly, beside, `using Mapping`; sub-lines auth, paging, openapi, pick | file.rs:474-540 |
| `filestore`, `codec x from "path"` | File stores, video codecs | file.rs:541, 550 |

The splash is not a project line: it is set on the app (`<App splash=".." splashFor="..">`, `crates/clarity-gen/src/web_windows.rs:112`). New projects start as `stack rust desktop` with one SQLite source (lib.rs:396).

## 3. The language

Parser `crates/clarity-spec/src/parser.rs` and `code/parse.rs`, model `model.rs` and `code/mod.rs`, checker `check.rs` and `code/check.rs`. Tests are in `crates/clarity-spec/tests/` (142 files) unless said otherwise.

### Loading

- Modules are parsed, then each `.cxaml` page, then derived data, then the structural, sign-in and theme checks (`crates/clarity-spec/src/lib.rs:100-160`). Loading a folder adds translations, resources, engine files, native part files, diagrams and mapping schemas (lib.rs:763-795).
- A window or app written inside a module is an error: each is a `.cxaml` file in `design/` (`cxaml.rs:1385`). `clarity convert-pages` moves old ones out.

### Declarations

| Word | What it declares | Code | Test |
|---|---|---|---|
| `module X`, `use Y` | Module name (first line), visibility of another module | parser.rs:312, 325 | modules.rs |
| `source erp` | The module's SQL data source | parser.rs:332, check.rs:2636 | sources.rs |
| `enum` | Values with labels | parser.rs:426 | parse.rs |
| `entity` | Stored fields, `table`, `key`, `readable when`, `writable when`, `version`, `savedSince @key` | parser.rs:492-620 | external.rs, versions.rs |
| `record` | Values with fields, never stored | parser.rs:462 | records.rs |
| `settings` | Per-person values kept on the machine | parser.rs:624, check.rs:583 | settings.rs |
| `theme` | Colors (with dark variants), fonts, sizes, radii, part looks, styles | themes.rs | themes.rs |
| `code Name` | Handlers of a window or the app, or data events of an entity | code/parse.rs:14 | code.rs, data_events.rs |
| `function`, `action` | Parameters with defaults, `-> type`, `require`, `ensure`, effects, `allow when` | code/parse.rs:267, 326 | modules.rs |
| `const` | Named value, also `const x: picture = "file.svg"` | code/parse.rs:368 | consts.rs |
| `engine` | WebAssembly engine (below) | code/parse.rs:60 | engines.rs |
| `cli`, `command` | Command-line commands with `option` lines | code/parse.rs:785-849 | commands.rs |
| `part X` | Custom part drawn by a library in the stack's language | code/parse.rs:882 | custom_parts.rs |
| `library X` | Functions from a package (nuget, dll, crate, go, cmake) | code/parse.rs:1015 | libraries.rs |
| `mapping M of E from fmt` | How a file format lies over an entity | code/parse.rs:1091, code/mapping.rs | mapping.rs, xml.rs |

### Fields

- Types: text(n), integer, decimal(p,s), money, bool, date, datetime, duration, image, file, email, url, color, secret, folder, id, `ref E`, `list<E>`, lists of values and enums, a record held inside, an enum (parser.rs:718-777).
- Modifiers: required, unique, owned, computed, search, cascade, hashed, min, max, default, pattern, label, help, format, column, store (parser.rs:779), with per-field rules in check.rs:2427-2612 (defaults.rs, cascade.rs, search.rs, file_fields.rs).
- `computed` is parsed and only leaves the field out of the required check (`crates/clarity-gen/src/lib.rs:270`); there is no formula.
- Records held inside records are only allowed in entities of a web API (code/check.rs:5252, nested.rs).

### Pages and events

- Page kinds: window, dialog, view, splash, toolbox, signin (model.rs:512). Page lines: title, size, style, data, grid, allow, gives, tells, `on open/close/return` (parser.rs:941-1112).
- App: main, splash (`for N seconds`, 0.1 to 30), title, `on start`, `on unconnected` (parser.rs:1306-1355).
- Part events: click, change, enter, leave, select, open, pointer, key, submit, draw, drag, drop, bend, connect, resize, next, finish, reorder, add, remove, tick, end, fail, link (widgets.rs:162-220). Wiring is `on event: handler`; a wired handler that is not written is an error (code/check.rs:1428; events.rs, lifecycle.rs).
- Data events in `code Entity`: before/after save, before/after delete (code/mod.rs:43-124, data_events.rs).

### Statements

From `code/mod.rs:136-253`:
- `let`, assignment (name, member, part property, list item), `if`/`else if`/`else`, `for x in list`, `for i in a to b`, `return`. There is no `while`, `break`, `continue` or `match`.
- Window statements: `message info|warning|error`, `ask ... then handler(...)` (confirm, a typed value, or a dialog window), `open W with a = v`, `close`, `close with value`, `reload x`, `copy text`, `tell event(args)`.
- Data statements: `save x [anyway]`, `delete x`, `refuse "text"`, `write text to store "path"`.
- Command-line statements: `print value [columns a, b]`, `exit code`; `ask` as an expression reads the terminal.
- Drawing: `canvas.shape(...)` for clear, line, rect, circle, ring, text, link, object, ellipse, path, image (code/check.rs:2230).
- Statements that show something are refused outside handlers (code/check.rs:2809).
- Tests: code.rs, asks.rs, drawing.rs, commands.rs, files.rs.

### Expressions and operators

From `code/mod.rs:448-568` and `code/check.rs:4883-5024`:
- Literals (a number with `.` is a decimal), texts with slots `"Saved {customer.name}"` (slots.rs), keys `@key(slot = v)`, lists `[a, b]`, `none`, ranges `a to b`, `x in [a, b]`, `list[i]` from 1.
- `new Entity`; `find v in E where ... order by v.f [desc]`; `files f in store where ...`; `read text of f`, `read bytes of f`; `json of x`, `<format> of x [using M]`; `read list<T> from <format> text`; `http get|post|put|patch|delete ...` (section 6).
- `or and not`, comparisons (ordered on text, dates, durations, numbers), `+ - * /` (`/` on integers gives a decimal, anything with money gives money, `+` joins texts), `div` and `mod` on integers (whole_division.rs), `contains`, `matches` (full text, only in `find`).
- Dates: `today()` and `now()`, comparison, defaults. No date arithmetic and no date members (code/check.rs:854, 3872-3936).

### Built-in functions

- Text: words, split, trim, lower, upper, replace, startsWith, endsWith, slice, indexOf, zip (code/check.rs:2133; texts.rs, conformance/zip.json).
- Pictures: resize, crop, rotate, flip, blend, adjust, pictureWidth, pictureHeight (code/check.rs:2109; picture_functions.rs).
- Others: count, join, length, snapped, sin, cos (degrees), round (half away from zero), today, now, newToken, newRecoveryCodes, `me` (with sign-in), random and progress (engines only) (code/check.rs:2081-2180).
- Read-only members of files, http replies, bytes, record files and record `id` (code/check.rs:3872-3936).

### Effects and access

- Effects db.read, db.write, files.read, files.write, net. A function has none and cannot call an action; an action declares the effects of what it calls (code/check.rs:1861-1890).
- `allow when` on actions and commands, `allow` on pages, `readable/writable when` on entities, all type-checked (code/check.rs:1321-1365; signin.rs, signin_views.rs).

### Queries

`find` works on entities: conditions and `order by` through refs, `contains` on lists, `matches` on `search` fields; refs stay in one source; web-API entities are never found or saved; read-only sources are never saved (code/check.rs:4620-4800, 5114, 5157, 5336; queries.rs, sources.rs). SQL per dialect is in every runtime, pinned by `conformance/queries.json` on all five.

### Engines

- `engine X from "x.wasm"` declares a WebAssembly module's functions; Clarity reads its imports and exports and checks them (`crates/clarity-spec/src/engines.rs:111, 194`).
- Lines: `uses clock, random, log, language, progress`, `says @keys`, `state per app|window`, `on open`, `limit N seconds, M MB` (default 5 s and 256 MB, max 3600 s and 4096 MB; engines.rs:17-26).
- `beside the screen`: runs in the screen, one per canvas, with `tells` and `hears`, `clarity_tick` and `clarity_draw` (screen_engines.rs).
- Written in Clarity (no `from`): `const`, `keep`, functions with bodies. Compiled to WebAssembly at generation by `crates/clarity-wasm` (compiled, or interpreted with `engines interpreted`), never translated to the stack's language (`crates/clarity-gen/src/lib.rs:820-857`; `crates/clarity-wasm/tests/conformance.rs`, noughts.rs, sky.rs).
- Hosts: R wasmi, Go wazero, C and C++ wasm3 (vendored), C# Wasmtime (section 8); `conformance/engines.json`, `progress.json`, `screen-engines.json`.
- The engine compiler refuses with "not compiled yet" (`crates/clarity-wasm/src/lower.rs`): date, datetime, duration and file values (:25, :36); statements other than let, assign, if, for, return, call, tell, refuse (:246); keys with values (:295); calls other than count, sin, cos, round, random, progress and the engine's own (:432), so no text functions.

### Translations and resources

- `translations/<lang>.clarity`: `key "text"` or `key resource "file.md"` (translations.rs:911). Errors: file name and language differ, key not camelCase or doubled, key used but missing in the default language, slots missing or unknown, languages with different keys, a literal text people read (translations.rs:375, 987-1100). An unused key is a warning. Clarity's own words have built-in English (translations.rs:35-320).
- Resources: pictures (png, jpg, gif, bmp, webp, svg), sounds (wav, mp3, flac, ogg), video (mp4), Markdown (`crates/clarity-spec/src/resources.rs:25-32`); a missing file is an error (resources.rs).

### What the generators still refuse

- Reading a stored entity's `list<X>` field in code: "list fields like ... are not loaded yet" on all five (`crates/clarity-gen/src/rust/logic.rs:903`, csharp/logic.rs:1017, go/logic.rs:1543, c/logic.rs:1740, cpp/logic.rs:1101).
- C: list literals of non-record, non-value types (c/logic.rs:1902), a position in a list of records (:1926), window data of list<bool> or list<Enum> (:437), `json of` some value types (:2027).
- Go: a `files ... where` condition that calls something that can fail (go/logic.rs:701).

## 4. Parts

The catalog is `WidgetKind` in `crates/clarity-spec/src/widgets.rs:15-136` (70 kinds, entries at :656). Clarity.UI has a maker for every kind (`runtimes/rust-ui/src/parts/mod.rs:585` and the `KINDS` of inputs.rs, containers.rs, data.rs, signin_view.rs); a kind without one, or one that fails while made, shows an error in its own place (mod.rs:661-699, `tests/screen/markup.rs:180`). Programs can add or replace kinds (`register`, mod.rs:613).

Modules below are under `runtimes/rust-ui/src/parts/`.

| Group | Kind | What it does | Module |
|---|---|---|---|
| Text | label | Caption, can be `for` an input | label.rs |
| | help | Help under an input | containers/small.rs |
| | heading | Heading, can bind a field | heading.rs |
| | text | Read-only value in the person's format | containers/text.rs |
| | markdown | Markdown from a resource, key or field | containers/markdown.rs |
| | link | Opens an address or runs a handler | inputs/link.rs |
| | badge, progress, busy | Pill, bar from 0 to max, busy sign | containers/small.rs |
| | icon | One Lucide icon | containers/icon.rs |
| | separator | Line, level or upright | containers/separator.rs |
| Input | textbox, password | Text (text, email, url, secret), optional copy button | text_box.rs |
| | memo | Multi-line; `code <lang>` coloring (19 languages); `markdown` editor with preview | text_editor.rs, code_colors.rs, markdown_memo.rs |
| | number, spinbox | Number in the person's format, step arrows | inputs/number.rs |
| | slider | Number on a bar | inputs/slider.rs |
| | checkbox, switch | Bool | check_box.rs |
| | toggle | Button that stays pressed | inputs/toggle.rs |
| | dropdown | Enum, reference, language, theme or text choices; searching | inputs/dropdown.rs, typed_dropdown.rs |
| | radio | Enum as a radio group | inputs/radio.rs |
| | date | Date or datetime with a calendar | inputs/date.rs, calendar.rs |
| | color | `#rrggbb` typed or picked | inputs/color.rs |
| | file | A record's file: choose, open, save a copy, remove | inputs/file.rs |
| | folder | Folder path, typed or picked | inputs/folder.rs |
| | tags | List of values as chips | inputs/tags.rs |
| | button, buttons | Button; a joined row of buttons and toggles | button.rs, containers/buttons.rs |
| Containers | form, panel | Grid with or without a captioned frame, can fold | containers/panel.rs |
| | tabs | Tab strip and pages | containers/tabs.rs |
| | content | One view at a time, picked by code | containers/tabs.rs |
| | steps | Wizard with Back, Next, Finish | containers/steps.rs |
| | accordion | Folding sections | containers/accordion.rs |
| | scroll | Grid that scrolls | containers/scroll.rs |
| | splitter | Panes with a divider | containers/splitter.rs |
| | popover | Grid opened next to a part | containers/popover.rs |
| | slideout | Grid sliding in from an edge, can be pinned | containers/slideout.rs |
| Data | table | Rows with cell editors, order, pages | data/table.rs |
| | tree, treeTable | Table as a tree | data/table.rs |
| | list | Rows or cards, reorder by hand | data/list.rs |
| | chart | Bars, line or pie | data/chart.rs |
| | trend | Last values of one number | data/trend.rs |
| | graph | Nodes and edges laid out by Clarity.Graph | data/graph.rs |
| Graphics | image | Picture, or edits an image field | containers/image.rs, inputs/picture_field.rs |
| | display | Seven-segment number | containers/display.rs |
| | canvas | Drawing from code; objects, paint and vector modes; an engine's canvas | data/canvas.rs, canvas/ |
| Toolbox | tool, toolGroup, toolSize, toolOpacity, toolColors, toolLayers | Tools for a painting canvas | tools.rs |
| Media | sound | Sounds, tones, melodies; not visible | sound.rs |
| | video | Plays a resource or a file | video.rs |
| Time | timer | Runs `tick` every N seconds; not visible | timer.rs |
| Sign-in | signin, signinField, signinButton, signinRemember, signinLanguage, signinOther, signinRefusal | The sign-in part and the parts of a sign-in view | signin.rs, signin_view.rs |
| Custom | custom | Part drawn by an outside library | custom.rs |

- Context menus on parts: `Widget.menu` (`crates/clarity-spec/src/model.rs:662`), drawn by `designed_menu` (parts/mod.rs:470).
- Custom parts: declared with `part X` and a library (native, crate, go, nuget, dll); Clarity.UI's `CustomPart` trait (parts/custom.rs), C interface `runtimes/rust-ui-ffi/src/custom.rs`. Written in the stack's own language only (`crates/clarity-gen/src/lib.rs:871`). A custom part in a web-only program is refused (lib.rs:1628).
- Known limits in code: the file part runs no change handlers on open and save a copy (inputs/file.rs:1-9); the folder part has no picker in a browser (inputs/folder.rs:8).
- Tests: 1248 tests in `runtimes/rust-ui/tests/` (parts/, inputs/, data/, windows/, screen/, hosts/, language/); spec tests parts.rs, small_parts.rs, charts.rs, trees.rs, markdown.rs, video.rs, sound.rs, toolbox.rs and others. No test checks that every catalog kind has a maker.

## 5. Windows, frame, themes and dialogs

### Pages and layout

- `.cxaml` reading and writing: `crates/clarity-spec/src/cxaml.rs` (roots Window, Dialog, View, Splash, Toolbox, SignIn; a part writes only what differs from its defaults). Tests cxaml.rs, cxaml_values.rs.
- Layout values: window 1100 x 760 (min 200 x 150, max 3840 x 2160), row 36, gap 8, splash 480 x 280, sign-in 420 x 380 (`crates/clarity-spec/src/layout.rs`).
- Designer edits on design files: `crates/clarity-spec/src/design_edit.rs:110` (place, create, update, remove parts, tabs, menus, data, grid, size, frame regions and commands); the code half goes to the module (`split`, :123).
- Views shown in a slideout, section, tab, step or content join their window's parts, data and handlers (`crates/clarity-spec/src/views.rs`; views.rs, view_names.rs).

### Window stack

- `WindowStack` (`runtimes/rust-ui/src/window/mod.rs:216`): `open` shows a window in place; a dialog or an `in new window` target gets its own desktop window where the host allows, else a modal overlay (:698, :622). Closing the last window ends the program (:793). Back and Forward history (window/history.rs).
- Tests: `runtimes/rust-ui/tests/windows/window_stack.rs`, core_windows.rs, windows_life.rs, history.rs.

### Frame

- Regions: menu, toolbar, ribbon (top), sidebar (left or right), footer (bottom) (`crates/clarity-spec/src/frame.rs:16, 54-96`). Built-in commands: quit, close, language, theme, keys, undo, redo, back, forward (:187); sign-in and sign-out targets (:195).
- Clarity.UI: `runtimes/rust-ui/src/frame/` (menu bar with Alt letters and F10, dropdown menus, toolbar, ribbon with tabs and large buttons, sidebar that drags, folds and fills the window, footer).
- Keys window with `keys editable`: people change any command's key, swap, reset (`runtimes/rust-ui/src/dialogs/keys.rs`; conformance/shortcut-keys.json).
- Tests: spec frame.rs, ribbons.rs, sidebars.rs, menus.rs, commands.rs, new_windows.rs, keys.rs; rust-ui tests/windows/core_frame*.rs, frame_keyboard.rs, dialogs_keys.rs.

### Dialogs in generated apps

All drawn by Clarity.UI, never the system's:
- `message info` shows a note that closes itself; `warning` and `error` show an OK dialog (`runtimes/rust-ui/src/window/messages.rs`).
- `ask` asks a yes/no question, a typed or chosen value, or opens a dialog window answered by `close with value` (`runtimes/rust-ui/src/dialogs/ask.rs`, `runtimes/rust/src/ui.rs:444`).
- Long calls show a bar with Stop (dialogs/long_call.rs).
- Tests: tests/windows/core_dialogs.rs, dialogs_ask.rs, dialogs_long_call.rs; spec asks.rs.

### Splash, icon, sign-in window

- Splash before the main window, with status set by code (`runtimes/rust-ui/src/splash.rs`, window/mod.rs:349; tests/windows/splash.rs).
- App icon on the desktop window, Clarity's own when the project names none (`runtimes/rust-ui/src/desktop/mod.rs:98-213`); Windows icon and version resources in section 7.
- Sign-in window before the main window, again after sign-out (window/signin.rs; tests/windows/signin.rs, signin_view.rs).

### Themes

- Clarity's base theme (`crates/clarity-spec/themes/clarity.clarity`), 29 color roles, fonts, sizes, radii, part looks and states (`crates/clarity-spec/src/themes.rs:33-76`). A project theme lies over the base (`runtimes/rust-ui/src/theme.rs:91`).
- Light and dark follow the system on Linux (GTK, portal, GNOME), Windows (registry) and macOS (`runtimes/rust-ui/src/system_theme.rs`). The person's language and theme are kept (choices.rs).
- Tests: spec themes.rs, clarity_ui_theme.rs, colors.rs; rust-ui tests/screen/theme_look.rs, looks_contrast.rs, core_system_theme.rs.

## 6. Data and app services

| Feature | R | Go | C | C++ | C# | Shared cases |
|---|---|---|---|---|---|---|
| SQLite | yes | yes | yes | yes | yes | |
| PostgreSQL, MySQL, SQL Server | yes | yes | no | no | yes | |
| Migrations | yes | yes | yes | yes | yes | migrations.json |
| Row versions (savedSince) | yes | yes | yes | yes | yes | versions.json |
| Undo and redo of saves | yes | yes | yes | yes | yes | (tests per runtime: undo.rs, undo_test.*, UndoTests.cs) |
| Backups (SQLite) | yes | yes | yes | yes | yes | backups.json |
| Queries | yes | yes | yes | yes | yes | queries.json |
| Data folder | yes | yes | yes | yes | yes | data-folder.json |
| File stores, pictures, record files | yes | yes | yes | yes | yes | files.json, pictures.json, record-files.json |
| Formats (json, csv, xml, yaml) | yes | yes | yes | yes | yes | formats-runtime.json |
| Files as data sources | yes | yes | yes | yes | yes | |
| zip | yes | yes | yes | yes | yes | zip.json |
| HTTP client | yes | yes | yes | yes | yes | http.json |
| Settings per person | yes | yes | yes | yes | yes | settings.json |
| Secret settings in the keychain | yes | yes | yes | yes | yes | secret-store.json (R, C# only) |
| Sign-in | yes | yes | yes | yes | yes | signin.json |
| Picture functions | yes | desktop only | desktop only | desktop only | yes | picture-functions.json |
| Sound and video parts | yes | yes | yes | yes | yes | melody.json |

### Databases and migrations

- Kinds: `SourceKind` (`crates/clarity-project/src/lib.rs:66-75`). Locations can be `env:` or `setting:`; `keychain:` is refused (lib.rs:607).
- Drivers: R `runtimes/rust/src/{sqlite,postgres,mysql,mssql}.rs` (Cargo features); Go `runtimes/go/clarity/sqlite.go`, `postgresql/`, `mysql/`, `sqlserver/`; C and C++ `clarity/sqlite.*`; C# `Clarity.Runtime.Sqlite`, `.PostgreSql`, `.MySql`, `.SqlServer`.
- C and C++ refuse other databases: "the C backend stores records in SQLite for now" (`crates/clarity-gen/src/c/mod.rs:246`, `cpp/mod.rs:251`).
- Migrations are Clarity files `migrations/<source>/NNNN.clarity` (`crates/clarity-spec/src/migrate.rs:163-191, 363`); `pending()` diffs the model against the steps (:594). Each runtime applies missing steps and records them in `clarity_schema`. A baseline step is SQLite only (`crates/clarity-gen/src/schema.rs:426`).
- REST sources are reached from code only (`http ... from source`); their entities are never stored.
- Server-database tests need `CLARITY_TEST_POSTGRES`, `_MYSQL`, `_MSSQL` (`crates/clarity-gen/tests/servers.rs`, Go `server_test.go`, C# `ServerTests.cs`); CI does not set them.

### Row versions, undo, backups

- An owned table carries `row_version`; an external one names its column or says `version none` (`crates/clarity-spec/src/versions.rs`). A save over someone else's change is refused with savedSince; `save x anyway` writes, `reload x` reads again.
- Undo takes saves back, all or none per handler (R `runtimes/rust/src/edits.rs`, Go `undo.go`, C and C++ `edits.*`, C# `Undo.cs`).
- Backups use `VACUUM INTO` into `backups/`, keep 10 automatic ones, and are taken before a migration (`crates/clarity-db/src/backup.rs`; runtimes `backup.*`). Server databases have no backup.

### Files and formats

- File stores: `files f in store where ...`, `read text|bytes of f`, `write t to store "path"`; pictures in the database or in a store under a SHA-256 name; `file` fields (R `files.rs`, `picture.rs`, `record_file.rs` and the same in every runtime).
- Clarity.Formats (`runtimes/rust-formats`, no dependencies): json, csv, xml (with XSD), yaml; paths, mappings, tables and checks on one engine. CSV options separator, noHeader, quote, encoding (utf-8, windows-1252, iso-8859-1) (src/csv.rs:143). R calls the crate; the others go through `runtimes/rust-formats-ffi`.
- A file as a data source (`datasource x file "a.csv" external`) is kept in an in-memory SQLite table and written back (`crates/clarity-gen/src/lib.rs:96-136`; runtimes `file_source.*`).
- Tests: the `conformance/formats-*.json` files in `runtimes/rust-formats/tests`; `formats-runtime.json` on all five.

### HTTP and web APIs

- `http get|post|put|patch|delete url [from source] [with body] [header ..] [query ..] [every page]` never throws; the reply has status, ok, text (status 0 when nothing answered). Source auth bearer, api-key, basic; paging by page, offset, cursor. R `http.rs`, Go `http.go`, C and C++ libcurl `http/`, C# `Http.cs`.
- OpenAPI 3 and Swagger 2 files become a generated `apis/<source>.clarity` module (`crates/clarity-project/src/openapi.rs`).

### Settings and secrets

- Settings are JSON per app in the system's settings folder; a value that no longer fits is dropped (R `settings.rs` and every runtime).
- `secret` settings live in the Windows Credential Manager, macOS Keychain or Linux Secret Service (R `secrets.rs`, Go `runtimes/go/secrets`, C and C++ `secrets/`, C# `Secrets.cs`).
- The IDE's vault (section 10) is not used by generated apps.

### Sign-in

- Clarity.Auth (`runtimes/rust-auth`) decides every attempt for every stack, through a JSON call interface (`runtimes/rust-auth-ffi`). Strategies: password (Argon2id), account (the OS user, desktop and cli), token, recovery codes (`src/catalog.rs:33-39`); sessions, remember me, a limit on failed tries.
- The `signin` block: entity, targets, session, tries, remember, shows, language picker (`crates/clarity-spec/src/signin.rs`); `me` in code; `allow="me.role == Admin"` on windows and commands.
- People are managed with `clarity users` (section 11).
- OpenID / Microsoft Entra exists in `runtimes/rust-auth/src/openid.rs` but serves only the IDE agent's Foundry sign-in; it is not a strategy for apps.

### Pictures, sound, video

- Picture functions run on Clarity.Image (`runtimes/rust-image`): decode, resize, crop, rotate, flip, blend, adjust, size, giving PNG. On Go, C and C++ they need the desktop program: web-only and cli programs are refused (`crates/clarity-gen/src/lib.rs:971`).
- Sound part on Clarity.Audio (`runtimes/rust-audio`: mp3, flac, wav, ogg; play, pause, resume, seek, stop, tone, melody; `crates/clarity-spec/src/sound.rs`).
- Video part on Clarity.Video (`runtimes/rust-video`: a codec the project adds, else the system decoder (Media Foundation, AVFoundation, GStreamer), else openh264).
- In the browser, sound and video go through the page, not these libraries (`runtimes/rust-ui/src/browser/page.rs:16-22`).

## 7. Generation and build output

### Steps

`generate()` (`crates/clarity-gen/src/lib.rs:860-932`) checks stores, secrets and custom parts, places views, compiles engines written in Clarity, runs the stack's generator, adds codec libraries, stamps the version (Cargo.toml, csproj `<Version>`, CMake `project(VERSION)`), and writes README.md (plus THIRD_PARTY_NOTICES.md when Lucide icons are used).

### Output layout

- `out/desktop`, `out/web`, `out/cli` and a `.clarity-out` marker (`write_programs_listed`, lib.rs:1031). Clarity refuses to write into a folder holding files it did not make (lib.rs:1038) and keeps each stack's build folders (lib.rs:1580).

### Per stack

| Stack | Desktop | Web | Cli | Build command |
|---|---|---|---|---|
| R (`rust/mod.rs`) | Cargo crate on `clarity-runtime` from crates.io; build.rs links Clarity.UI | Same crate plus browser files embedded | Own crate without Clarity.UI | `cargo build` |
| Go (`go/mod.rs`) | Module on `github.com/petervdpas/Clarity/runtimes/go`, go.sum prefilled; cgo to Clarity.UI | browser files | Own module | `go build -o bin/<module>` |
| C (`c/mod.rs`) | CMake project, runtime copied in (cJSON, wasm3 vendored) | browser files carried in the program | Own CMake project | `cmake --workflow --preset` |
| C++ (`cpp/mod.rs`) | Same shape; needs nlohmann_json and SQLite3 | browser files | Own CMake project | `cmake --workflow --preset` |
| C# (`csharp/mod.rs`) | `net10.0` WinExe on the Clarity.Ui NuGet | Second program: App/ library on Clarity.Runtime plus Server/ on Clarity.Ui.Server (`csharp/web.rs`) | Console on Clarity.Runtime | `dotnet build` |

- Tools each stack needs and how they are found: `crates/clarity-gen/src/tools.rs:333-425` (pkg-config on Linux); people can pin 9 tools (`tools/environment.rs:25`). Engines from Rust need `wasm32-unknown-unknown`.
- The Clarity.UI library comes from the folder `CLARITY_UI` names, else the copy the installed Clarity carries, else a download from the `runtimes/rust-ui-ffi/v<ver>` release checked against SHA256SUMS (`crates/clarity-gen/src/clarity_ui.rs`). C# gets it in its NuGet.

### Operating systems

- Windows: Clarity converts the icon (PNG, JPEG, WebP, SVG) to `app.ico` and writes icon and VERSIONINFO resources for amd64, arm64 and 386 itself, no resource compiler (`crates/clarity-gen/src/windows_resources.rs`); R links them in build.rs, Go as `.syso`, CMake directly, C# with `ApplicationIcon`.
- macOS: rpath and Keychain linking only; no app bundle, Info.plist or .icns.
- Linux: rpath `$ORIGIN`; secrets link gio-2.0.

### Building without the internet

Not built as a mode. C and C++ copy their runtime into the output and the Clarity.UI library can come from the installed copy; Rust (crates.io), C# (nuget.org) and Go modules still download on first build.

### Generator refusals

- Custom part or library in another stack's language (lib.rs:871).
- Picture functions in a Go, C or C++ web-only or cli program (lib.rs:971).
- Custom part in a web-only program (lib.rs:1628).
- Non-SQLite database on C or C++ (c/mod.rs:246, cpp/mod.rs:251).
- `keychain:` secrets (lib.rs:67).
- Entities without a data source (lib.rs:53).

### Tests

- Not ignored: toolchain.rs, out_layout.rs, csharp_web.rs, windows_resources.rs, splash.rs, icons.rs, engines.rs, browser_fresh.rs, the per-stack generation tests (rust.rs, go.rs, csharp.rs, c.rs, cpp.rs, versions_*.rs, signin_*.rs).
- Ignored (need the real tools): examples_build.rs (one per example), everything_build.rs (every part on every stack and on the web for R, Go, C, C++), build.rs, go_build.rs, csharp_build.rs, c_build.rs, cpp_build.rs, http_build.rs, servers.rs, everything_live.rs (headless Chromium).

## 8. Runtimes and packages

| Runtime | Where | Version in tree | Published as |
|---|---|---|---|
| Rust runtime | `runtimes/rust` | 0.56.0 | crate `clarity-runtime` |
| Go runtime | `runtimes/go` | 0.59.0 | Go module, tag `runtimes/go/v*` |
| C runtime | `runtimes/c` | | copied into each program |
| C++ runtime | `runtimes/cpp` | | copied into each program |
| C# runtime | `runtimes/csharp` | 0.92.0 | 13 NuGets: Clarity.Runtime, .Sqlite, .PostgreSql, .MySql, .SqlServer, .Engines, Clarity.Ui, Clarity.Ui.Server, Clarity.Audio, Clarity.Video, Clarity.Image, Clarity.Formats, Clarity.Auth |
| Clarity.UI | `runtimes/rust-ui`, `-ffi`, `-browser` | ffi 0.20.0 | crate `clarity-ui`; native libraries and browser files as GitHub release archives |
| Audio, Video, Image, Formats, Graph, Auth | `runtimes/rust-*` (+ `-ffi`) | | crates `clarity-audio`, `-video`, `-image`, `-formats`, `-graph`, `-auth` |
| Engine library | `runtimes/rust-engine` | 0.3.1 | crate `clarity-engine` (for engine authors, not used by programs) |
| Web screen | `runtimes/web` (`@clarity/ui`) | | private; used only by the IDE |

Engine hosts: R wasmi 2 (`runtimes/rust/Cargo.toml`), Go wazero 1.12 (`runtimes/go/go.mod`), C# Wasmtime 48 (`Clarity.Runtime.Engines`), C and C++ wasm3 (`runtimes/c/third_party/wasm3`).

## 9. The IDE

A Tauri app (`app/`): Svelte front end in `app/src`, 133 commands in `app/src-tauri/src` (registered in lib.rs:866-1002). Shared screen code comes from `runtimes/web` (`@clarity/ui`).

- **Shell:** ribbon tabs Project, Design, Preview, Vault (`app/src/App.svelte:102`); a sidebar per tab; a Problems list with one-click fixes; right-click menus in the project tree, the design canvas and diagrams; splitters whose sizes are kept. No menu bar. Drawing errors are caught per area and Rust panics are reported (`fail_safe.rs`). Splash at start (`splash.rs`).
- **Project browser:** Clarity's own file browser with project cards, recent projects and an address bar (`lib/FileBrowser.svelte`, `browse.rs`). New project dialog: name, folder, backend, targets (`lib/NewProjectDialog.svelte`). The folder is watched; outside changes reload (`lib.rs:140`).
- **Project tree:** README, Sign-in, Windows, Views, Data, Mappings, Web APIs, Diagrams, Themes, i18n, Resources, Engines, Custom parts, Libraries, Commands, Modules, each with "+ new" (`lib/ProjectTree.svelte:145-406`).
- **Window designer:** drag parts from the palette onto a window drawn by Clarity.UI with sample data; pick, move, resize; dock frame regions; inspector tabs Properties, Code (checked as you type) and Text (the `.cxaml`); double-click a part to write its handler (`lib/PageDesigner.svelte`, `lib/PageProperties.svelte`, `apply_edit` lib.rs:251). New window templates: blank window or view, splash, toolbox, sign-in, settings, form or browse for an entity (`lib/NewWindowDialog.svelte:83`).
- **Code editors:** CodeMirror for Clarity with problems while typing (`lib/SourceEditor.svelte`); an editor for C++, Rust, Go, C#, CMake and Cargo files (`lib/CodeEditor.svelte`); folders of custom parts and libraries with Build (`lib/FolderEditor.svelte`, `parts.rs`); an engine editor that writes stubs and builds (`lib/EngineEditor.svelte`, `engines.rs`).
- **Diagram designer:** ERD and class diagrams on Clarity's own canvas, saved as `.mermaid`; other Mermaid kinds are shown read-only; edit as text; proposals from the project and Apply (`lib/DiagramView.svelte`, `diagrams.rs`). New diagrams offer only ERD and class (`lib/NewDiagramDialog.svelte:28`).
- **Data:** entity and enum editors (`lib/EntityEditor.svelte`, `lib/EnumEditor.svelte`); mapping page with a live sample (`lib/MappingPage.svelte`); web API view from an OpenAPI file (`lib/ApiView.svelte`).
- **Translations and resources:** a value per key per language with a filter and progress, resource keys, add a language, "make texts keys" (`lib/TranslationsPanel.svelte`, `translations.rs`); resources with a Markdown view (`lib/ResourceView.svelte`); README editor.
- **Themes:** edit a theme's text beside a sample window; new themes from System, Light, Dark or Cream and green (`lib/ThemeView.svelte`).
- **Project settings:** name, folders, title, stack with a tools check, language, formats, theme, data sources, file stores; per database migrations and backups; import tables; take ownership of an external source (`lib/ProjectSettings.svelte`, `MigrationPanel.svelte`, `BackupPanel.svelte`, `ImportPanel.svelte`). Sign-in settings (`lib/SigninSettings.svelte`).
- **Build and run:** Generate, Build, Run, Stop, open a web app in the browser; output panel; an input line for command-line programs; disabled while the project has errors (`App.svelte:1467-1497`, `build.rs`). Tools check per stack with "Check again" and Browse (`lib/EnvironmentSettings.svelte`).
- **Preview:** run any window with sample data, Back and a breadcrumb, switch language (`lib/ClarityWindow.svelte`).
- **Agent panel:** section 10.
- **Vault:** create, unlock, lock; secrets by category; connections; change password; destroy; locks itself after a while (`lib/VaultView.svelte`, `vault.rs`).
- **Clarity settings:** Display (IDE theme), Keys, Folders, Environment, Agent, Logging, System (CPU, memory, IO, disk charts) (`lib/AppSettingsView.svelte:24-31`).
- **Keys:** rebind any IDE command per layer, swap, reset (`lib/KeysSettings.svelte`, `runtimes/web/src/host/keys.ts:13-57`).
- **Status bar:** what is open, build state, error and warning counts, stack, load meters, last outside-agent call (`lib/StatusBar.svelte`).
- **Dialogs:** confirm, message, prompt, choose, never the system's (`lib/dialogs.ts`, `lib/DialogHost.svelte`).
- **Undo:** project-wide undo and redo of edits, 200 steps; an agent turn is one step (`lib.rs:87`, `undo_edit` :646).
- **Search:** Ctrl+F in editors and filters in translations, vault and manual. No project-wide search.
- **Manual:** separate window, F1 opens the current tab's topic (`src/Manual.svelte`, `manual.rs`).
- **The IDE's own words** are English written in the Svelte files; there are no translation files for the IDE itself.
- No debugger and no report designer.

## 10. AI: Clarity's agent, outside agents, tools

### Clarity's agent

- **Loop:** the model answers, its tool calls run, results go back, until no tool calls, Stop, 40 steps or the token budget (default 200k) (`crates/clarity-agent/src/lib.rs:216-278`; `app/src-tauri/src/agent.rs:26`).
- **Providers:** the Anthropic Messages API with streaming and prompt caching (5m or 1h) (`crates/clarity-agent/src/anthropic.rs`), and Microsoft Foundry with an API key or a Microsoft work account through OpenID (`foundry.rs`). Any other kind is refused (`app/src-tauri/src/agent.rs:393`).
- **What it may run:** only building and running, after asking. The setting is Ask, Allow or Never; with Never the build tool is not offered (`agent.rs:52, 108, 211`). Restoring a backup asks too.
- **Tests:** `crates/clarity-agent/tests/agent.rs`, anthropic.rs, foundry.rs; `app/src-tauri/src/agent.rs:536-722`.

### Outside agents over ACP

- Codex (`codex-acp`) and Copilot (`copilot --acp --stdio`) run in the agent panel with their own sign-in (`crates/clarity-acp/src/vendors.rs`). JSON-RPC over stdio, written by hand (`src/lib.rs`).
- They get Clarity's tools through `clarity mcp --turn <file>` (`app/src-tauri/src/acp.rs:177`).
- They may read and write files only inside the project. They may never write Clarity's own files (`.clarity`, `.cxaml`, README.md, `resources/*.md`), and terminals are refused. Anything else is asked, once at a time (`src/lib.rs:444-503, 636, 648`).
- **Tests:** `crates/clarity-acp/tests/client.rs`, vendors.rs.

### MCP server

- `clarity mcp` serves every shared tool except build, build_output, the backup tools and show_window/show_diagram (those two come with `--turn`) (`crates/clarity-mcp/src/lib.rs:27, 101`). It works on the project open in the IDE and logs each call without file contents (:126).
- Without an open project only manual, reference, design, parts and new_project answer. With outside agents turned off in the IDE, only the first four answer (:31, :35).
- **Tests:** `crates/clarity-mcp/tests/server.rs`.

### Shared tools

62 tools in `crates/clarity-tools`, none of which builds, runs or touches a database:
- **General:** project_overview, read_file, write_file, check, missing_translations, api, pick_api, migrate, rename, parts, manual, reference, design, new_project, generate, read_generated, diagram_view, write_diagram, diagram_proposals, apply_diagram (`src/lib.rs:807-920`).
- **Data model:** add_entity, add_enum, add_field, change_field, add_enum_value, change_enum_value, set_uses, remove_model_item (`src/structural/model.rs`).
- **Windows:** add_window, add_view, remove_page, change_page, add_part, place_field, change_part, rename_part, move_part, remove_part, add_tab, change_tab, remove_tab, add_data, remove_data, set_part_grows (`src/structural/windows.rs`).
- **App and frame:** add_app, set_main_window, set_splash, add_region, add_command, change_command, change_sidebar, change_ribbon, rename_frame_part, set_frame_title, move_frame_part, remove_frame_part, add_context_menu, set_event, wire_event, set_handler_code, name_parts, camel_names (`src/structural/frame.rs`).
- `write_file` refuses `.cxaml` files and files Clarity writes from a declaration (`src/lib.rs:491-499`).

### Vault

An encrypted folder: Argon2id key, one AES-256-GCM record per secret bound to the vault and its name, secrets by category, and connections whose fields can point into the vault (`crates/clarity-vault`; tests vault.rs, connections.rs). The IDE keeps agent connections here.

## 11. The command line

`clarity` (`crates/clarity-cli/src/main.rs:16-234`):

| Command | What it does |
|---|---|
| `spec check <dir>` | Parse and check, print problems |
| `spec names <dir>` | Name unnamed parts, make names camelCase |
| `spec keys <dir>` | Turn readable texts into keys |
| `spec texts <dir>` | Keys without a value per language |
| `spec model <dir> [--samples]` | The checked model as JSON |
| `spec windows <dir> <out>` | Windows and app as `.cxaml` |
| `gen <project> [--out]` | Generate the programs |
| `parts <project>` | Write the files of custom parts and libraries |
| `convert-pages <project>` | Move windows out of modules into `.cxaml` |
| `migrate <project> [--check] [--allow-data-loss] ...` | Write the next migration step |
| `own <project> <source>` | Take ownership of an external source |
| `import <project> <source> ...` | Import SQLite tables or a CSV, JSON or XML file as entities |
| `data backup / backups / migrate / restore` | The project's SQLite database |
| `users <project> list / add / password / token / codes / remove` | People of a project that signs in |
| `tools` | Which build tools each stack has |
| `mcp [--turn <file>]` | The MCP server on stdio |
| `libraries` | Rebuild the base libraries from source (in the repository) |

Tests: `crates/clarity-cli/tests/mcp.rs`, users.rs, data_migrate.rs.

## 12. Editor support

- The VS Code extension `editors/vscode` (`clarity-designer`) is declarative only. It gives `.clarity` files a language, an icon, a grammar (`syntaxes/clarity.tmLanguage.json`), snippets and bracket rules; `.cxaml` opens as XML.
- The grammar is kept equal to the IDE's highlighter (`crates/clarity-spec/tests/highlight.rs`), and every snippet parses (`editor_snippets.rs`).

## 13. Checks and releases on GitHub

| Workflow | When | What |
|---|---|---|
| `checks.yml` | push to main | Linux: Clarity.UI libraries, browser files, workspace tests, renderer checks, Go, C, C++ and C# runtime tests, 20 examples each on its stack |
| `release.yml` | tag `v*` | IDE installers: Windows NSIS and MSI; Linux .deb (amd64, arm64) and .rpm (Fedora 43, 44), each install-tested; the `.vsix`. No macOS installer |
| `release-clarity-ui.yml` | tag `runtimes/rust-ui-ffi/v*` | Native libraries for linux, win, osx (x64, arm64) and browser files |
| `release-csharp-runtime.yml` | tag `runtimes/csharp/v*` | The 13 NuGets to nuget.org |
| `release-rust-*.yml` | tag `runtimes/rust*/v*` | One crate each to crates.io |
| `release-vscode.yml` | tag `editors/vscode/v*` | VS Code Marketplace and Open VSX |

Go module tags need no workflow: the Go proxy reads the tag.

Gaps in the checks:
- Go's file-source tests are skipped: they carry the `clarityformats` tag, but CI runs `-run Formats`, which does not match their names (`checks.yml:207`, `runtimes/go/clarity/file_source_test.go`).
- The server-database tests are not run (no database variables set).

## 14. Not built

What the design and plans promise that the code does not do is in [`docs/UNIMPLEMENTED.md`](UNIMPLEMENTED.md).
