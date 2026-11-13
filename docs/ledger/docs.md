# Docs: LANGUAGE.md, the manual, the READMEs

Status: partial. The written docs people and agents read, and where they disagree with the code. A conflict here is fixed in the doc unless marked "needs decision".

Progress: 80%

## Code
- `docs/LANGUAGE.md` (3,225 lines): the language reference. Served by the agents' `reference` tool (`crates/clarity-tools/src/manual.rs:83`, `lib.rs:30`); its code blocks are tested.
- `manual/*.md` (41 pages, about 3,800 lines; moved from `docs/manual/`, not yet committed): one topic per page, English only. Opened by F1 in the IDE (`app/src-tauri/src/manual.rs`, `app/src/lib/manual.ts`) and by the `manual` tool (`crates/clarity-tools/src/manual.rs:11`, `include_str!` per page); index `manual/README.md`.
- READMEs: `README.md` (repo layout, setup, dev loop); `runtimes/*/README.md` (one per published crate), `runtimes/csharp/*/README.md` (one per NuGet), `runtimes/web/README.md`, `editors/vscode/README.md`; `examples/*/README.md` (each project's goal, read by `project_overview`); generated projects' README (`crates/clarity-gen/src/readme.rs`).
- `docs/DESIGN.md` is embedded by the `design` tool (`crates/clarity-tools/src/lib.rs:33`); see [mcp-tools](mcp-tools.md).
- All docs are Markdown.

## Tests
- `crates/clarity-spec/tests/language_doc.rs`, `crates/clarity-project/tests/settings.rs:476` (LANGUAGE.md)
- `crates/clarity-spec/tests/manual_doc.rs`, `doc_snippets.rs` (manual snippets parse; untracked)
- `app/src/lib/manual.test.ts`; `crates/clarity-spec/tests/editor_snippets.rs` (VS Code snippets parse)

## Open
- The manual in other languages: NOT BUILT (English only).
- Old references to `docs/manual/` after the move: UNTRIED; check links and comments.

## Conflicts
README.md
- :109-112 runs whole `cargo test` and says it writes the TS bindings; CLAUDE.md:31-32 says run only the changed module's tests, CLAUDE.md:46 says `cargo test -p clarity-spec --lib` writes them (needs decision).
- :118-119 copies shop to `/tmp/try`; CLAUDE.md:35 forbids builds in /tmp (needs decision).
- :142 the agent runs when settings allow; CLAUDE.md:21 says it never executes (needs decision; see [agent](agent.md)).
- :69 "Rust 1.88 or newer": no `rust-version` is set anywhere.
- :14-40 layout leaves out `runtimes/rust-auth`, `rust-auth-ffi`, `rust-formats`, `rust-formats-ffi`, `rust-graph`.
- :74 names the Clarity.UI library for Go, C and C++ only; Rust uses the ready-made library too (`runtimes/rust/Cargo.toml:45`).

runtimes/rust/README.md
- :26 `clarity-runtime = "0.10.0"`; the crate is 0.56.0.

editors/vscode/README.md
- :9 lists the `page` snippet, which writes a page block the loader refuses (`crates/clarity-spec/src/cxaml.rs:1385`).
- :34 installs 0.1.3; the extension is 0.1.38 (`editors/vscode/package.json:5`).

docs/LANGUAGE.md (not syntax; syntax items are in [language](language.md))
- :199 says C and C++ use "the shared screen (webview)"; every desktop draws with Clarity.UI (`crates/clarity-project/src/lib.rs:875-881`; LANGUAGE.md:77 agrees with the code).
- :1627-1628 says `nuget` and `dll` parts are not generated; C# generates them (`crates/clarity-gen/src/csharp/parts.rs:26, 56, 138`).
- :1496-1506 uses the removed dice example.
- :680 the folder part opens the system's folder picker; CLAUDE.md:19 forbids system dialogs (needs decision; code: `runtimes/rust-ui/src/desktop/files/mod.rs`).

manual/ (lines as of the move)
- Page blocks in modules, refused by the loader: code.handlers.md:31-34.
- snake_case names the checker refuses: code.language.md:36, :71 (`handler_name()`); code.functions.md:34-40 and code.web-apis.md:117-121 (`list_templates`; OpenAPI actions are camelCase, `crates/clarity-project/src/openapi.rs:260-283`); code.data-events.md:16; code.queries.md:31, :42, :52.
- Quoted texts where keys are required (`crates/clarity-spec/src/translations.rs:569, 650, 1025`): code.functions.md:82, code.data-events.md:46 (`refuse "text"`), code.web-apis.md:16, data.entities.md:53-54.
- code.language.md:17: `contains` is reserved too (`crates/clarity-spec/src/code/parse.rs:1457`). code.functions.md:69-76: the effects table leaves out `net`.
- README.md:25: Design as text shows "a window as Clarity text"; it shows the `.cxaml`.
- start.overview.md:41: tree names (i18n, README, Sign-in, Views, Mappings, Engines, Custom parts, Libraries, Commands) and the sidebar's Main window; :52: settings are Display, Keys, Folders, Environment, Agent, Logging, System.
- start.first-project.md:48, design.windows.md:70-74: no heading "The window"; the fieldsets are Data and Events.
- project.file.md:24-38 leaves out `version`, `icon`, `signin`; :66: Migrations and Backups sit per database in settings, translations open from the tree.
- design.windows.md:88: grid columns go to 96, not 24 (`crates/clarity-spec/src/model.rs:594`); :51: New window also offers views, splash, toolbox, sign-in and settings.
- design.text.md:55: Tab is always four spaces (`app/src/lib/SourceEditor.svelte:72`); app.keys.md:81 says Escape closes the panel, design.text.md says it goes back to Properties.
- design.parts.md:24: sign-in is set in the tree's Sign-in item, not in Project settings.
- design.parts.md:26, data.pictures-files.md:58: the system's folder picker (needs decision with CLAUDE.md:19, as above).
- code.handlers.md:140: Code is an inspector tab, not a button in the Window group.
- code.engines.md:48: the section is "Texts it gives", not "Keys it says"; :323 lists `progress` as not built, :97 documents it, and it is built.
- data.entities.md:45, :57: `list<Entity>`, `min`, `max`, `pattern` as working; see [entities](entities.md). :87: the editor also has Search and Hashed columns.
- data.pictures-files.md:17 leaves out `.jpeg` and `.svg`.
- languages.md:154, themes.md:97-100: translations and themes through "the web renderer" and CSS on Rust, Go, C, C++; every stack uses Clarity.UI and theme JSON. languages.md:132: the locale follows the language shown, not the system.
- design.custom-parts.md:48: Clarity.UI on "Rust and C#"; it is all five stacks.
- design.video.md:42: Windows and macOS decoders with no caveat; never run ([os-checks](os-checks.md)).

Plans against plans and code (the plans go away with the ledger; listed so no entry repeats them)
- Released or not: FORMATS-PLAN.md:3 (steps 2-5 are in v0.39.0), APP-CAPABILITIES-PLAN.md:3 (sign-in and row versions before v0.40.0), GRAPH-PLAN.md:3 and AGENT-TOOLS-PLAN.md:3 (in v0.39.0).
- Status behind: FOUNDRY-PLAN.md:3 (step 2 built, manual/ai.connections.md documents it); IMAGE-PLAN.md has no status (built); WINDOWS-TESTS.md:5 (C# parts and dialogs built).
- Syntax: IMAGE-PLAN.md:60-77 `toolbox Painting` block, the code uses a `<Toolbox>` page (`crates/clarity-spec/src/views.rs:290`); AUDIO-PLAN.md:40, VIDEO-PLAN.md:30, HARDWARE-PLAN.md:35 use page blocks.
- Removed things: PARTS-LIBRARY-PLAN.md:12 (dice); WINDOWS-TESTS.md:11 (`runtimes/c/desktop/desktop.c`), :88 (C# in the browser is built); FORMATS-PLAN.md:334 (todo uses a mapping, `examples/todo/ideas.clarity:13`).
- Skia for Rust: manual/setup.tools.md:16, :64 and OFFLINE-PLAN.md:13 say cargo downloads Skia; Rust uses the ready-made library (`runtimes/rust/Cargo.toml:45`, manual/stacks.rust.md:87). README.md:74 and setup.tools.md:7 name it for Go, C and C++ only; OFFLINE-PLAN.md:14 leaves Go out.
- Releases: WINDOWS-TESTS.md lists hand release steps; CLAUDE.md uses the Release Helper.
- Data source kinds: DESIGN.md:121 lists sqlite, postgres, mysql, mssql, rest; LANGUAGE.md:91 and manual/project.file.md add the file formats.
- macOS keys: DESIGN 5.28 says Ctrl is Cmd; WINDOWS-TESTS section 6 and the code say not done.
- Roadmap: DESIGN.md:2455 still has Avalonia and G5 wxWidgets; DESIGN.md:65 says both were replaced. DESIGN A2 puts ollama first and omits Foundry.

## Decisions
- 2026-10-17: all documentation, in the repo and generated, is Markdown. User: "all documentation written or stored should be markdown, bc that the best thing for ai and human"
- 2026-11-12: the manual moved from `docs/manual/` to `manual/` at the repository root; `crates/clarity-tools/src/manual.rs` embeds it from there, its links reach the reference as `../docs/LANGUAGE.md`. User: "yes move manual more toplevel in app?", then the root over `app/manual/`.
- The manual is in English (MCP-PLAN step 6: "in English, as the whole manual is").

## Was
- CONFLICTING.md sections 8 and 9; section 6 items not about syntax (:194, :1622, :1491-1512)
- memory: markdown-docs.md
