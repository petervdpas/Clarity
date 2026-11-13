# Docs: the manual and the READMEs

Status: partial. The written docs people and agents read, and where they disagree with the code. A conflict here is fixed in the doc unless marked "needs decision".

Progress: 80%

## Code
- `manual/*.md` (56 pages and the index, about 9,300 lines; moved from `docs/manual/`): the only documentation for app builders and agents, one topic per page, English only; it holds the whole language (a page per tree section, palette group, declaration and statement). Opened by F1 in the IDE (`app/src-tauri/src/manual.rs`, `app/src/lib/manual.ts`) and by the `manual` tool (`crates/clarity-tools/src/manual.rs:11`, `include_str!` per page); index `manual/README.md`.
- READMEs: `README.md` (repo layout, setup, dev loop); `runtimes/*/README.md` (one per published crate), `runtimes/csharp/*/README.md` (one per NuGet), `runtimes/web/README.md`, `editors/vscode/README.md`; `examples/*/README.md` (each project's goal, read by `project_overview`); generated projects' README (`crates/clarity-gen/src/readme.rs`).
- `docs/LANGUAGE.md` and `docs/DESIGN.md` were removed on 2026-11-13, with the agents' `reference` and `design` tools; their content is in the manual. The plans and status documents in `docs/` remain (see [ledger](ledger.md)).
- All docs are Markdown.

## Tests
- `crates/clarity-spec/tests/manual_catalog.rs` (every parts page holds its palette group, every part its section), `manual_coverage.rs` (every statement, declaration word and tree section has its page; no page links to the old documents), `manual_doc.rs` (snippets parse, the complete example checks clean), `doc_snippets.rs`
- `crates/clarity-project/tests/settings.rs:478` (the manual's project files are valid)
- `app/src/lib/manual.test.ts`; `crates/clarity-spec/tests/editor_snippets.rs` (VS Code snippets parse)

## Open
- The manual in other languages: NOT BUILT (English only).
- Old references to `docs/manual/` after the move: UNTRIED; check links and comments.

## Conflicts
README.md
- :110-113 runs whole `cargo test` and says it writes the TS bindings; CLAUDE.md:31-32 says run only the changed module's tests, CLAUDE.md:46 says `cargo test -p clarity-spec --lib` writes them (needs decision).
- :119-120 copies shop to `/tmp/try`; CLAUDE.md:35 forbids builds in /tmp (needs decision).
- :147 the agent runs or builds when Clarity settings allow; CLAUDE.md:21 says it never executes (needs decision; see [agent](agent.md)).
- :70 "Rust 1.88 or newer": no `rust-version` is set anywhere.
- :11-64 layout leaves out `runtimes/rust-auth`, `rust-auth-ffi`, `rust-formats`, `rust-formats-ffi`, `rust-graph`, `rust-app-mcp`, `rust-app-mcp-ffi`.
- :75 names the Clarity.UI library for Go, C and C++ only; Rust uses the ready-made library too (`runtimes/rust/Cargo.toml:45`).

runtimes/rust/README.md
- :26 `clarity-runtime = "0.10.0"`; the crate is 0.59.0 (`runtimes/rust/Cargo.toml:6`).

editors/vscode/README.md
- :9 lists the `page` snippet, which writes a page block the loader refuses (`crates/clarity-spec/src/cxaml.rs:1382-1395`).
- :34 installs 0.1.3; the extension is 0.1.38 (`editors/vscode/package.json:5`).

manual/
- snake_case names the checker refuses: code.web-apis.md:119-123 and code.statements.md:493 (`list_templates`; OpenAPI actions are camelCase, `crates/clarity-project/src/openapi.rs:260-283`).
- Quoted texts where keys are required (`crates/clarity-spec/src/translations.rs:569, 650, 1025`): `refuse "text"` in code.handlers.md:237, code.functions.md:134, code.statements.md:358; `message "..."` in code.web-apis.md:16, parts.graphics.md:172.
- start.overview.md:41: tree names (i18n, README, Sign-in, Views, Mappings, Engines, Custom parts, Libraries, Commands); Clarity settings has Display, Keys, Folders, Environment, Libraries, Agent, Logging, System (`app/src/lib/AppSettingsView.svelte:27-37`).
- design.text.md:55: Tab is always four spaces (`app/src/lib/SourceEditor.svelte:72`); app.keys.md:81 says Escape closes the panel, design.text.md:13 says it goes back to Properties.
- parts.input.md:80, parts.signin.md:9: sign-in is set in the tree's Sign-in item, not under Who is using it in Project settings.
- parts.input.md:202, data.settings.md:137: the system's folder picker; CLAUDE.md:19 forbids system dialogs (needs decision; code: `runtimes/rust-ui/src/desktop/files/mod.rs`).
- code.handlers.md:270: Code is an inspector tab, not a button in the Window group.
- code.engines.md:79: the section is "Texts it gives", not "Keys it says".
- data.entities.md:97, :111-112: `list<Entity>`, `min`, `max`, `pattern` as working; see [entities](entities.md).
- project.file.md:34: Migrations and Backups sit per database in settings, translations open from the tree.
- languages.md:196: the locale follows the language shown, not the system.
- design.video.md:42: Windows and macOS decoders with no caveat; never run ([os-checks](os-checks.md)).

Plans against plans and code (the plans go away with the ledger; listed so no entry repeats them)
- Released or not: FORMATS-PLAN.md:3 (steps 2-5 are in v0.39.0), APP-CAPABILITIES-PLAN.md:3 (sign-in and row versions before v0.40.0), GRAPH-PLAN.md:3 and AGENT-TOOLS-PLAN.md:3 (in v0.39.0).
- Status behind: FOUNDRY-PLAN.md:3 (step 2 built, manual/ai.connections.md documents it); IMAGE-PLAN.md has no status (built).
- Syntax: IMAGE-PLAN.md:60-77 `toolbox Painting` block, the code uses a `<Toolbox>` page (`crates/clarity-spec/src/views.rs:290`); AUDIO-PLAN.md:40, VIDEO-PLAN.md:30, HARDWARE-PLAN.md:35 use page blocks.
- Removed things: PARTS-LIBRARY-PLAN.md:12 (dice); FORMATS-PLAN.md:334 (todo uses a mapping, `examples/todo/ideas.clarity:13`).
- Skia for Rust: manual/setup.tools.md:16, :64 and OFFLINE-PLAN.md:13 say cargo downloads Skia; Rust uses the ready-made library (`runtimes/rust/Cargo.toml:45`, manual/stacks.rust.md:87). README.md:75 and setup.tools.md:7 name it for Go, C and C++ only; OFFLINE-PLAN.md:14 leaves Go out.

## Decisions
- 2026-10-17: all documentation, in the repo and generated, is Markdown. User: "all documentation written or stored should be markdown, bc that the best thing for ai and human"
- 2026-11-12: the manual moved from `docs/manual/` to `manual/` at the repository root; `crates/clarity-tools/src/manual.rs` embeds it from there, its links reach the reference as `../docs/LANGUAGE.md`. User: "yes move manual more toplevel in app?", then the root over `app/manual/`.
- The manual is in English (MCP-PLAN step 6: "in English, as the whole manual is").
- 2026-11-13: agents read only the manual; docs/LANGUAGE.md and docs/DESIGN.md removed, their content split into manual pages that follow the code and the IDE (one per tree section, palette group, declaration and statement), guarded by crates/clarity-spec/tests/manual_catalog.rs, manual_coverage.rs and manual_doc.rs. User: "agent should only read manuals!", "I think those docs are way too unstructured!", "let reality (code + ide) lead you... nothing else!", "yes, do it".

## Was
- CONFLICTING.md sections 8 and 9; section 6 items not about syntax (:194, :1622, :1491-1512)
- memory: markdown-docs.md
