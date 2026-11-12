# What is not built

This file lists what `docs/DESIGN.md`, `docs/LANGUAGE.md` and the `docs/*-PLAN.md` files promise that the code does not do. Its partner `docs/IMPLEMENTED.md` says what the code does. Each item names where it was promised and what the code has instead (paths from the repository root).

Written 2026-11-12 from the code at that date. When something here gets built, move its line to `IMPLEMENTED.md` in the same piece of work.

Status words:
- **NOT BUILT:** no code for it.
- **PARTIAL:** some of it exists; the line says what is missing.
- **UNTRIED:** the code exists but has never run where it was promised (an operating system, a real device or program).

## Contents

1. [The language](#1-the-language)
2. [Data model and databases](#2-data-model-and-databases)
3. [Engines](#3-engines)
4. [Parts and windows](#4-parts-and-windows)
5. [Diagrams](#5-diagrams)
6. [Data and app services](#6-data-and-app-services)
7. [Pictures, sound, video and camera](#7-pictures-sound-video-and-camera)
8. [Generation, builds and systems](#8-generation-builds-and-systems)
9. [The IDE](#9-the-ide)
10. [AI and tools](#10-ai-and-tools)
11. [Plans with nothing built](#11-plans-with-nothing-built)
12. [Code structure (modular plan)](#12-code-structure-modular-plan)
13. [Untried on Windows and macOS](#13-untried-on-windows-and-macos)
14. [The first design: rules, packs, gates](#14-the-first-design-rules-packs-gates)
15. [Docs that say less than the code](#15-docs-that-say-less-than-the-code)

## 1. The language

- **`while`, `break`, `continue`, `match`.** NOT BUILT. None of these words is in `crates/clarity-spec/src/code/parse.rs` or the lexer.
- **Date arithmetic and date members** (`d + 1`, `d.year`). NOT BUILT. Dates only compare (`crates/clarity-spec/src/code/check.rs:854`; `member()` has no date arm, :3872-3936).
- **Durations scaled by a decimal** (`duration * 1.5`). PARTIAL. Promised LANGUAGE.md:2884; only whole numbers are accepted (code/check.rs:4999-5001).
- **`computed` fields with a formula.** PARTIAL. Promised LANGUAGE.md:391, DESIGN.md:416-423, 1765. The word only leaves the field out of the required check (`crates/clarity-gen/src/lib.rs:270`); there is no syntax for the value.
- **Contract helpers `old(expr)`, `sum(list.field)`, `all(list, cond)`, `any(list, cond)`.** NOT BUILT. Promised DESIGN.md:449.
- **`require` and `ensure` turned into generated tests** (property tests). NOT BUILT. Promised DESIGN.md:757, 1676, 2215, 2369. They become run-time checks only (`crates/clarity-gen/src/rust/logic.rs:190-215`); no generator writes a test.
- **A context object that exposes only an action's declared effects.** NOT BUILT. Promised DESIGN.md:1677. Effects are enforced by the checker only.
- **`progress n of total` in handlers and actions.** NOT BUILT. Promised DESIGN.md:809, 830, 2528. `progress(...)` outside an engine is an error (code/check.rs:4455).
- **Text functions `encoded(text)` and `plainText(html)`.** NOT BUILT. Promised APP-CAPABILITIES-PLAN.md:43.
- **`context` for one text with two meanings in translations.** NOT BUILT. Promised DESIGN.md:2495.
- **`main.view = OtherView` to pick a content's view.** NOT BUILT as written. Promised LANGUAGE.md:779. Code uses `main.swap(OtherView)` or `main.tab` (code/check.rs:2608-2620, 3812).
- **Generator gaps on C:** list literals other than records or plain values (`crates/clarity-gen/src/c/logic.rs:1902`); a position in such a list (:1926); window data of `list<bool>` or `list<Enum>` (:437); a list of values given to a window (:1621); `json of` a `list<bool>` (:2027). PARTIAL, C only.
- **`files f in store where ...` with a condition that can fail.** PARTIAL, missing on Go (`crates/clarity-gen/src/go/logic.rs:701`).

## 2. Data model and databases

### Fields

- **`list<Entity>` on a stored entity** ("the records of another entity that belong to this one", LANGUAGE.md:372; DESIGN.md:169). NOT BUILT on all five stacks. No column or table is written (`crates/clarity-gen/src/ddl.rs:155, 185`), and reading one in code is refused with "list fields like ... are not loaded yet" (rust/logic.rs:903, go/logic.rs:1543, c/logic.rs:1740, cpp/logic.rs:1101, csharp/logic.rs:1017). Only web-API entities hold such lists.
- **`owned`** ("the listed records live and die with this record", LANGUAGE.md:388). NOT BUILT. Parsed and checked only (`crates/clarity-spec/src/parser.rs:811`, check.rs:2455); no generator or runtime reads it.
- **`pattern "regex"`** (LANGUAGE.md:383). NOT BUILT. Parsed and checked (parser.rs:819, check.rs:2501), never enforced.
- **`min` and `max`** (LANGUAGE.md:382). PARTIAL. They only set the range of sliders and spin boxes (`runtimes/rust-ui/src/parts/inputs/number.rs:147`, slider.rs:48); `save` checks only required fields (`runtimes/rust/src/lib.rs:826`).
- **`text(n)`** (LANGUAGE.md:355). PARTIAL. VARCHAR(n) on server databases only; nothing enforces it on SQLite.
- **`time` field and part.** NOT BUILT. Promised DESIGN.md:2474.
- **`bitmap` and `vector` field types.** NOT BUILT. Promised DESIGN.md:2538. Painting stores into an `image` field.
- **`video` field kept in a file store, played by a bound video part.** NOT BUILT. Promised VIDEO-PLAN.md:26-27, 40. The video part binds nothing (`crates/clarity-spec/src/widgets.rs:1332`).
- **`steps` between enum values** (allowed changes, a refused save, a warning on unreachable values). NOT BUILT. Promised DESIGN.md:1005-1022, 2530.
- **Unbound input parts reported as warnings.** NOT BUILT. Promised DESIGN.md:489; the catalog flag describes it (widgets.rs:540) but no check reads it.

### Databases

- **PostgreSQL, MySQL and SQL Server on C and C++.** NOT BUILT. Promised LANGUAGE.md:539. Refused: "the C backend stores records in SQLite for now" (`crates/clarity-gen/src/c/mod.rs:248`, cpp/mod.rs:253).
- **Import, live-table checks and baseline steps for server databases.** NOT BUILT. Promised DESIGN.md:136, 2502; LANGUAGE.md:580, 609. "importing is for SQLite and files only for now" (`crates/clarity-project/src/lib.rs:1382`); "a baseline step is for SQLite only for now" (`crates/clarity-gen/src/schema.rs:426`).
- **Keys that are not one integer column.** NOT BUILT. Promised LANGUAGE.md:602, DESIGN.md:2503 (`crates/clarity-db/src/lib.rs:42-49`).
- **TLS to PostgreSQL and MySQL.** PARTIAL. Promised DESIGN.md:2502. Rust connects without TLS (`runtimes/rust/src/postgres.rs:144`, mysql crate without TLS features in `runtimes/rust/Cargo.toml:57`); SQL Server uses rustls. Go and C# not checked.
- **One transaction across databases for data events.** PARTIAL. Promised DESIGN.md:2501. One transaction per source (`runtimes/rust/src/lib.rs:977-995`).
- **Testing a data source's connection from the Project tab.** NOT BUILT. Promised DESIGN.md:131.
- **REST entities mapped onto endpoints** (list, get, create, update, delete). PARTIAL. Promised DESIGN.md:140. An OpenAPI file gives entities and one action per operation (`crates/clarity-project/src/openapi.rs`); entities are never found or saved through the API.
- **Copying data when a source changes kind** (SQLite to PostgreSQL). NOT BUILT. Promised DESIGN.md:160, 2505.

### Migrations and backups

- **A default or fill expression when a field becomes required; counting rows a narrowing change would not fit.** NOT BUILT. Promised DESIGN.md:151-154, 2499. The data-loss confirmation is built.
- **Data migrations** (Clarity code that changes rows), **down steps**, **one-way steps**, **renaming enum values.** NOT BUILT. Promised DESIGN.md:155-156, 2499. `Op` holds schema operations only (`crates/clarity-spec/src/migrate.rs:163-191`).
- **A migrate switch in the generated program** for server deployments. NOT BUILT. Promised DESIGN.md:158. `clarity data migrate` exists for the IDE's SQLite only.
- **Backups of server databases** in a portable form. NOT BUILT. Promised BACKUP-PLAN.md:37-39, DESIGN.md:157, LANGUAGE.md:534.
- **`backups off` on a data source** (user: "dont keep backups for the examples"). NOT BUILT. Promised BACKUP-PLAN.md:41-49.
- **`back up main "name"` in app code; backups in a folder the person picks.** NOT BUILT. Promised BACKUP-PLAN.md:53-54.

### Row versions

- **The IDE's data browser saves through the version check.** NOT BUILT (there is no data browser). Promised APP-CAPABILITIES-PLAN.md:229, 248.
- **ETag and If-Match on web API writes.** NOT BUILT. Promised APP-CAPABILITIES-PLAN.md:241, 400.
- **Other windows reload a record saved elsewhere.** NOT BUILT. Promised APP-CAPABILITIES-PLAN.md:252-254.

## 3. Engines

- **Engines written in Clarity:** dates, durations and files (`crates/clarity-wasm/src/lower.rs:25, 36`); statements other than let, assign, if, for, return, call, tell, refuse (:246); keys with values (:295); `a to b` outside `for` (:220-228); `/` giving a decimal (:405); every built-in except count, sin, cos, round, random, progress, so no `join`, `length`, `today` or text functions (:432); the project's own functions (:432); record fields of date or ref type (:484). PARTIAL, all refused with "not compiled yet". Promised LANGUAGE.md:2441-2502, 2673; DESIGN.md:855, 892.
- **Picture functions in engines; a video frame as a picture.** NOT BUILT. Promised VIDEO-PLAN.md:99.
- **`clarity engines build`, `clarity watch`, engine templates for C, C++, TinyGo, Zig, AssemblyScript; WIT.** PARTIAL. Promised DESIGN.md:2527. Only the Rust engine flow exists (`crates/clarity-project/src/engines.rs`).

## 4. Parts and windows

- **Reflow of the grid on narrow windows and the web.** NOT BUILT. Promised DESIGN.md:487, 2462.
- **A check that a container's contents fit its height.** NOT BUILT. Promised DESIGN.md:2462.
- **Window size presets (narrow, medium, wide) in the designer.** NOT BUILT. Promised DESIGN.md:2063.
- **A view that brings its own ribbon tab and menu items.** NOT BUILT. Promised DESIGN.md:1502, 1506; WINDOWS-TESTS.md:115. A frame on a non-window page is an error (`crates/clarity-spec/src/check.rs:467`).
- **The menu key and Shift+F10 open a part's context menu.** PARTIAL. Promised DESIGN.md:1122. Only a right press does (`runtimes/rust-ui/src/element.rs:336`).
- **Ctrl means Cmd on macOS**, in matching and in what is shown. NOT BUILT. Promised DESIGN.md:1504; WINDOWS-TESTS.md:114 (`runtimes/rust-ui/src/shortcuts.rs:134-137`).
- **Moving several rows at once; dragging a row between parts.** NOT BUILT. Promised DESIGN.md:1531.
- **Level trees: code sets `selected`, row labels from more than own fields, the designer edits levels.** NOT BUILT. Promised DESIGN.md:1574.
- **Dropdown search on the labels people see; accent-blind sorting and search.** NOT BUILT. Promised DESIGN.md:300 (`runtimes/rust/src/dropdown_options.rs:26-38` lowercases the stored value only).
- **Dates and numbers in the system's locale, with a choice apart from the language.** PARTIAL. Promised DESIGN.md:186. Formats follow the shown language, 8 locales known, others fall back to en-US (`runtimes/rust-ui/src/parts/inputs/formats.rs:303-318`).
- **Timer `once`, a settable `every`, `start()` and `stop()`, `conformance/timers.json`.** PARTIAL. Promised DESIGN.md:2490. Code starts and stops it through `enabled` only.
- **Canvas `surface svg|bitmap|gpu`.** PARTIAL. Promised DESIGN.md:609, 636. Parsed and stored (`crates/clarity-spec/src/parser.rs:2531`), read by nothing.
- **A custom part's `preview "arena.png"` on the design canvas.** PARTIAL. Promised DESIGN.md:1272. Parsed (`crates/clarity-spec/src/code/parse.rs:980`), read by nothing.
- **A failed build of a part library shown as a problem at its declaration.** PARTIAL. Promised DESIGN.md:1274. Missing folders are problems; build failures are not.
- **Files dropped from the system onto a part.** NOT BUILT. Promised DESIGN.md:1921.
- **Real italic faces in Markdown.** PARTIAL. Promised DESIGN.md:1978. Italic is a skewed regular face (`runtimes/rust-ui/src/parts/containers/markdown.rs:460`).
- **Mermaid diagrams shown inside generated apps.** NOT BUILT. Promised DESIGN.md:916.

### Graph part (GRAPH-PLAN.md)

- **Looks bound to fields** (size by a number, color by a value, label templates, edge thickness). NOT BUILT (:26). `NodeLook` is per kind only (`runtimes/rust-ui/src/parts/data/graph.rs:71-85`).
- **Icons per node kind and label placement.** NOT BUILT (:77, :86).
- **A panel beside the graph for the picked node, references as links.** NOT BUILT (:27, :86).
- **Filters on any field.** NOT BUILT (:28). Only the legend's kinds.
- **Edges through a link entity; choosing which relations to follow.** NOT BUILT / PARTIAL (:32, :91).
- **Property entities** (open-ended typed properties as rows). NOT BUILT (:37-71).
- **Optional controls, a node and edge count, keeping drags optional.** PARTIAL (:79-80).
- **Edge events and a focus-changed event.** NOT BUILT (:81, :86). Events are click, select and open on nodes (`crates/clarity-spec/src/widgets.rs:1171`).
- **The modelling example uses the graph** (user: "I'm going to need it in the data modelling app"). NOT BUILT (:97).

### Usability rules (DESIGN.md 5.18)

- **Named rules as checker warnings** (confirm-delete, say-what-happens, label-every-input, keyboard-order, default-and-cancel, long-form, long-list, empty-state, slow-work-shown, small-target, contrast, menu-has-key), with thresholds, turned off with a reason, shown on the canvas, read by the agent. NOT BUILT. Promised DESIGN.md:1155-1191, 2482, 2563.
- **Usability from use** (noting where people struggle in a running app). NOT BUILT. Promised DESIGN.md:1189, 2492.

## 5. Diagrams

- **State, sequence, activity and architecture diagrams** written from the project and designed on the canvas, with their toolbox items. PARTIAL. Promised DESIGN.md:913, 920, 974, 1026-1028, 2530-2531. The kinds and their Mermaid headers exist (`crates/clarity-spec/src/diagrams/mod.rs:45-78`); no parser, view or designer; New diagram offers ERD and class only.
- **A diagram's problems in the project's Problems list.** PARTIAL. Promised DESIGN.md:994. Shown only in the diagram's own editor (`app/src/lib/DiagramView.svelte:338`).
- **The agent tool `diagram_view` gives the class view too.** PARTIAL. Promised DESIGN.md:1002. It returns the data model only (`crates/clarity-tools/src/lib.rs:578-582`).

## 6. Data and app services

### Formats (FORMATS-PLAN.md)

- **The data browser shows file sources.** NOT BUILT (:262, :268); there is no data browser.
- **An agent tool to add a file data source.** NOT BUILT (:263).
- **Reading formats from bytes.** NOT BUILT (:40).
- **A `table "Orders"` option for files with several tables.** NOT BUILT (:52).
- **xlsx and ods; toml and ini.** NOT BUILT (:344, :364; APP-CAPABILITIES-PLAN.md:404-405).
- **Formats from libraries.** NOT BUILT (:345).
- **JSON Schema checks for JSON and YAML.** NOT BUILT (:362).
- **A file source at a `setting:` path reloading when the setting changes.** NOT BUILT (:365).

### Encrypted storage, mail, jobs (APP-CAPABILITIES-PLAN.md)

- **Encrypted SQLite, file sources, file stores and backups** (`encrypted "setting:..."`, SQLCipher), the `forget` option, `clarity encrypt`, re-encrypting on a key change. NOT BUILT (:45-62).
- **Sending mail** (`smtp` source, `send mail`), **reading mail** (`imap`, `mails m in`, mark seen, move, delete), clarity-mail, Clarity.Mail, `conformance/mail.json`. NOT BUILT (:66-108).
- **Background jobs** (`at most N`, `start ... for ... on done/failed/finished`, `stop`, `conformance/jobs.json`) and the `examples/erasure` example (user: "add a background job with at most N to the plan"). NOT BUILT (:258-331).
- **Scheduled work** (`every day at 08:00`) and runs that outlive their window. NOT BUILT (:398, :417).

### Sign-in

- **Strategies for apps:** OpenID (web, desktop and cli flows), maillink, providers (GitHub, Microsoft, OAuth) with registration and a register part, TOTP, passkeys (user: "passkeys come later then"), LDAP, strategies from a library. NOT BUILT (APP-CAPABILITIES-PLAN.md:161-203, 338, 410-414). OpenID exists only for the IDE agent (`runtimes/rust-auth/src/openid.rs`); an untracked `runtimes/rust-auth/src/providers.rs` is not wired into the crate (MODULAR-PLAN.md:22).
- **Switching person on the desktop without a restart.** NOT BUILT (:416).

### Secrets

- **Secrets stored alike on every stack.** PARTIAL. Promised DESIGN.md:1488. On macOS Go's go-keyring stores `go-keyring-base64:` values that no other runtime reads (`runtimes/go/secrets/secrets.go:11-40`).

## 7. Pictures, sound, video and camera

- **Picture functions in Go, C and C++ programs that are web-only or cli.** PARTIAL. Promised IMAGE-PLAN.md:28, LANGUAGE.md:2786. Refused (`crates/clarity-gen/src/lib.rs:971`).
- **An example that uses the picture functions.** NOT BUILT (IMAGE-PLAN.md:90).
- **Layer opacity; `board.picture()` from code; stamps; scripts on the toolbox subject.** NOT BUILT (IMAGE-PLAN.md:46-48, 82-85). The toolbox is built as a `.cxaml` page instead of a module declaration (:56-77).
- **`conformance/sound.json` for play, pause and seek; `soundTags(path)`; AAC playback.** NOT BUILT (AUDIO-PLAN.md:19, 63, 70).
- **Project line `video openh264 | ffmpeg | system`.** NOT BUILT (VIDEO-PLAN.md:22, 39).
- **Camera part** (preview, stills, `on denied`, `on missing`), **recording** (encoder, MP4 writer, Opus), the browser camera, macOS camera text, a fake camera for tests. NOT BUILT (VIDEO-PLAN.md:41-84).
- **Frame buffer part, MP4 export, ffmpeg and system encoders, the video diary example.** NOT BUILT (VIDEO-PLAN.md:81-108).

## 8. Generation, builds and systems

- **`clarity gen --verify`** for CI. NOT BUILT. Promised DESIGN.md:1715, 2541.
- **Round tripping:** per-file hashes, finding hand-edited files, a three-way merge and its view, hand-written items in the spec. PARTIAL. Promised DESIGN.md:2536. One hash of the whole program exists, used only to say "press Generate first" (`crates/clarity-gen/src/lib.rs:1153-1171`).
- **`AGENTS.md` and `CLAUDE.md` in generated projects.** NOT BUILT. Promised DESIGN.md:1691, 2536.
- **The generated README carries the project's README.** NOT BUILT. Promised DESIGN.md:1596 (`crates/clarity-gen/src/readme.rs:41`).
- **Offline builds:** `clarity prepare`, `clarity vendor`, the offline sign beside Build, runtimes in offline form in the installer, an offline CI test. NOT BUILT. Promised OFFLINE-PLAN.md:31-65. Only the Clarity.UI library and browser files are carried.
- **macOS app bundles** (Info.plist, .icns) **and a macOS IDE installer.** NOT BUILT.
- **The tools check finds C and C++ libraries on Windows and macOS.** NOT BUILT. Promised DESIGN.md:2487. The check runs on Linux only (`crates/clarity-gen/src/tools.rs:402`).
- **Install hints for Fedora, Arch, Homebrew.** NOT BUILT. Promised DESIGN.md:2487.
- **`CMAKE_PREFIX_PATH` from the chosen compiler; Visual Studio's build environment (vcvars).** NOT BUILT. Promised DESIGN.md:1707-1708, 2487.
- **A Windows (and macOS) job in `checks.yml`.** NOT BUILT. Promised DESIGN.md:1704, 2484. Windows runners appear only in release workflows.
- **The C# program driven live in tests.** NOT BUILT. Promised DESIGN.md:272 (`crates/clarity-gen/tests/everything_live.rs` has go, rust, c, cpp).
- **Conformance cases for `enter`, `leave` and pictures in forms.** NOT BUILT. Promised DESIGN.md:277.
- **Chart grouping and vector drawing in one place.** PARTIAL. Promised DESIGN.md:2000. Still in `runtimes/rust/src/charts.rs` and `runtimes/rust-ui/src/parts/data/groups.rs`.
- **Leftover:** `Ui::Webview` is kept only to be refused (`crates/clarity-project/src/lib.rs:297, 317, 901`), and the comment at lib.rs:876 still speaks of the shared web screen.

## 9. The IDE

- **Translations of the IDE's own words.** NOT BUILT. Promised ACP-PLAN.md:46 and the project rule. The words are English in the Svelte files.
- **A data browser.** NOT BUILT. Promised FORMATS-PLAN.md:262, APP-CAPABILITIES-PLAN.md:229.
- **A debugger** (marks per statement, DAP in Clarity.UI, breakpoints, stepping, watches, the Debug button and panels, design debugging). NOT BUILT. Promised DEBUGGER-PLAN.md, all of it.
- **A report designer.** NOT BUILT (section 11).
- **Project-wide search.** NOT BUILT.
- **Code behind in the person's own editor** (LSP or an editor command). NOT BUILT. Promised DESIGN.md:2051.
- **The app's icon in the designer and preview.** NOT BUILT. From the removed icon and splash plan.
- **The file browser starts in the open project's folder.** PARTIAL. From the removed project browser plan (`app/src-tauri/src/browse.rs:56`).
- **The system dialog plugin removed.** PARTIAL. From the removed project browser plan. No screen uses it, but `tauri-plugin-dialog` is still a dependency and initialised (`app/src-tauri/Cargo.toml:36`, `app/src-tauri/src/lib.rs:858`).
- **Learning mode** (beginner projects, generated code beside Clarity code, a tutor). NOT BUILT. Promised DESIGN.md:2537.
- **Designer edits that name parts by key instead of a place in the file; the key translator removed; `none=` compared with an empty attribute.** PARTIAL. From the removed .cxaml plan. Edits still use a file offset (`crates/clarity-spec/src/design_edit.rs:154`); `keyed::key_module` is still used (`crates/clarity-spec/src/edit.rs:282`); `cxaml.rs:492-509` compares with `widgets::new_props`.

## 10. AI and tools

- **Local models** (`ollama`, `openai`-compatible) and a smaller tool set for small models. NOT BUILT. Promised AGENT-TOOLS-PLAN.md:21-22, DESIGN.md:2535. Other kinds are refused (`app/src-tauri/src/agent.rs:393`).
- **Foundry's OpenAI models; a Clarity app registration so `clientId` is optional; checked against a real tenant.** NOT BUILT. Promised FOUNDRY-PLAN.md:26, 44-45.
- **Tool schemas that list existing entities, windows and parts as choices.** PARTIAL. Promised AGENT-TOOLS-PLAN.md:20. Only fixed lists are enums.
- **The hook that stops Claude Code writing `.clarity` and `.cxaml` files.** PARTIAL. Promised AGENT-TOOLS-PLAN.md:38. It exists as a user-level Claude Code hook outside the repository; nothing in the repository installs it or documents it.
- **Claude Code as an outside agent in the panel.** NOT BUILT (waits on Anthropic). Promised ACP-PLAN.md:56, 64.
- **Codex and Copilot checked by hand against the real programs.** UNTRIED. Promised ACP-PLAN.md:53; open notes remain (`crates/clarity-acp/src/vendors.rs:84`).
- **A missing agent program links to Clarity's manual page.** PARTIAL. Promised ACP-PLAN.md:44. It links the vendor's install page (`vendors.rs:42-51`).
- **The questionnaire** (MCP-PLAN.md step 7, user: "a questionaire to tell the ai what components to use and a first explaining what the app should do"): a Clarity.UI form started by `clarity mcp`, answers kept in README.md, the same form in the IDE agent, decision-tree forms. NOT BUILT. Promised MCP-PLAN.md:44-65.
- **AI proposals inside a project** (layout, data model, slow handlers, duplicated logic), **patterns across projects**, **reading run-time measurements.** NOT BUILT. Promised DESIGN.md:82-85. The only proposals are for diagrams.

## 11. Plans with nothing built

- **Hardware** (HARDWARE-PLAN.md): `device` declarations with tells and hears; serial, bluetooth, tcp and mqtt transports; Firmata; connection events; a port picker; Web Serial; simulated devices; device editor and monitor; home automation; a starter Arduino sketch; examples. NOT BUILT.
- **Parts library** (PARTS-LIBRARY-PLAN.md): `package` files, `use package`, an index, a lock file, Sigstore, `clarity package check/publish/vendor`, a Get parts browser. NOT BUILT.
- **Reports and PDF** (REPORT-PLAN.md, user: "make a clarity-pdf crate? It's should function like pandoc"): clarity-pdf on Typst, its ffi, `<Report>` pages with bands, the flow part, report parts, `pdf of`, a formula part. NOT BUILT.
- **Debugger** (DEBUGGER-PLAN.md): see section 9. NOT BUILT.
- **Offline builds** (OFFLINE-PLAN.md): see section 8. NOT BUILT.

## 12. Code structure (modular plan)

From MODULAR-PLAN.md:
- **Step 1, sign-in as its own modules.** PARTIAL. Spec, Clarity.UI, Clarity.Auth and IDE modules exist (some uncommitted); the generators still emit sign-in from each stack's `model.rs` and one shared `crates/clarity-gen/src/signin.rs`.
- **Step 2, split `check.rs` and `parser.rs` by area.** NOT BUILT (2810 and 2811 lines).
- **Step 3, split Clarity.UI's large files** (table.rs 2287, canvas.rs 1956, desktop/mod.rs 1428, window/mod.rs 1242, screen.rs 1228, text_editor.rs 1198). NOT BUILT.
- **Step 4, part families in the catalog** (`widgets.rs`, 2526 lines). NOT BUILT.
- **Step 5, shared Rust cores for row versions and undo, migrations, file sources, backups.** NOT BUILT; each runtime has its own.

## 13. Untried on Windows and macOS

The code exists; it has not been run on these systems (WINDOWS-TESTS.md, DESIGN.md 7.9):
- Go, C and C++ programs on Clarity.UI on Windows and macOS (`CLARITY_UI`, rpath, the main thread on macOS) (WINDOWS-TESTS.md:144).
- The web targets in Edge; C# on Clarity.UI on Windows and macOS; the walker example in a browser on Windows (:88, :96, :104-110).
- The Windows and macOS video decoders (VIDEO-PLAN.md:3, 13).
- The folder part's picker on Windows and macOS (:139); progress, Stop and long engine calls on Windows and macOS (:140-142).
- Vault keys, the eye and copy in the vault, AltGr on European layouts (:41-51).
- One name for secrets in the Credential Manager on every stack, needing a Rust runtime release (:29-35).
- Screen readers (Orca, NVDA, VoiceOver, TalkBack) with AccessKit and the browser mirror (DESIGN.md:2032-2034).

## 14. The first design: rules, packs, gates

DESIGN.md's opening sections (:13-17, :331-378) and sections 9 to 18 (:2043-2449) still describe Clarity as a rules engine over hand-written code. None of it is in the code. These items need a decision: build them, or strike them from DESIGN.md.

- Directive layer and rule engine checking every change, `@clarity` directive comments, an effects vocabulary for hand-written code.
- Modularity, TDD state machine, DRY and other rules; tunable rules with metrics, waivers, false-positive marks, ratchet baselines, profiles, `clarity tune`.
- Gates: Claude Code hooks, pre-commit, CI and CODEOWNERS written by `clarity init`.
- `clarity.toml` rule settings (the project file replaced the file but took none of the rules).
- A source index in `.clarity/index.db`.
- Language packs with tree-sitter queries (rust, c, cpp, csharp, go, python, ruby, typescript), WASM plugin packs.
- `clarity init`, `clarity adopt`, project templates; CLI commands fmt, index, check, test, graph, explain, mark-fp, tune, report.
- `clarity lsp` for `.clarity` files and rule violations.
- MCP tools spec_for, directives_for, module_map, check(diff), tdd_state, tdd_advance, explain_violation.
- The control room view and init and adopt wizards in the IDE.
- Workspace crates clarity-core, -index, -lang, -rules, -metrics, -lsp.
- `clarity fmt` and a canonical form (a code printer exists, `crates/clarity-spec/src/code/print.rs`).

## 15. Docs that say less than the code

Moved to [`docs/CONFLICTING.md`](CONFLICTING.md), with every other place where two sources disagree.
