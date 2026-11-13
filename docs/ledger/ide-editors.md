# IDE editors

Status: partial. The editors that open from the project tree: Clarity text, other languages, part and library folders, engines, modules, README and resources.

Progress: 85%

## Code
- Spec: `crates/clarity-spec/src/highlight.rs` (colors from the lexer, the same words the grammar has), `native.rs` (custom part folders)
- Generators: `crates/clarity-gen/src/part_code.rs` (reads a part's own code without building it; each stack's `code_mismatches`)
- IDE:
  - Clarity text: `app/src/lib/SourceEditor.svelte` (CodeMirror, problems while typing, Tab is four spaces :72), `highlight.ts`, `codeMirrorLook.ts`
  - Other languages: `CodeEditor.svelte` (C++, Rust, Go, C#, CMake, Cargo), `codeLanguages.ts`
  - Custom parts and libraries: `PartView.svelte`, `LibraryView.svelte`, `FolderEditor.svelte` (the folder edited like a project, with Build); Tauri `app/src-tauri/src/parts.rs`; CLI `clarity parts <project>`; see [custom-parts](custom-parts.md)
  - Engines: `EngineEditor.svelte` (inspector, source, stubs, Build), `engines.ts`, `engineBuilds.svelte.ts`; Tauri `app/src-tauri/src/engines.rs`; the Rust engine crate and its template in `crates/clarity-project/src/engines.rs`; see [engines](engines.md)
  - Modules: `ModuleView.svelte` (a module's file and what it declares)
  - README: `ReadmeView.svelte` (Markdown left, rendered right, `save_readme`, "+ Write README" with Goal, Scope, Order of work)
  - Resources: `ResourceView.svelte` (Markdown view), `NewResourceDialog.svelte`, `markdown.ts`; Tauri `resources.rs`, `resource_urls.rs`
- Docs: manual/design.text.md, design.custom-parts.md, code.libraries.md, code.engines.md

## Tests
- `app/src/lib/highlight.test.ts`, codeLanguages.test.ts, codeParts.test.ts, engines.test.ts, markdown.test.ts
- `crates/clarity-spec/tests/highlight.rs` (grammar kept equal to the highlighter; `CLARITY_WRITE_GRAMMAR=1`), editor_snippets.rs
- `crates/clarity-project/tests/engines.rs`, part_stacks.rs

## Open
- Code behind in the person's own editor (LSP or an editor command): NOT BUILT (DESIGN 8, :2051)
- A failed build of a part library shown as a problem at its declaration: PARTIAL. Missing folders are problems; build failures are not
- Engine templates for C, C++, TinyGo, Zig, AssemblyScript; `clarity engines build`, `clarity watch`: PARTIAL. Only the Rust engine flow (`crates/clarity-project/src/engines.rs`)
- The generated program's README does not carry the project's README: NOT BUILT (`crates/clarity-gen/src/readme.rs:41`); see [generation](generation.md)

## Conflicts
- `codec` is missing from the highlighter's keywords (`crates/clarity-spec/src/highlight.rs:48-90`) while the project file reads it
- manual/design.text.md:55-56 and app.keys.md:76-83 differ on Tab and Escape in the editor (`SourceEditor.svelte:72`)
- manual/code.engines.md:48: the section is "Texts it gives", not "Keys it says"

## Decisions
- 2026-10-20: every project thing opens in the IDE with its inspector and its code; double-click jumps to the code and writes stubs; never an outside editor or script for a normal step (memory: cpp-builder-ide.md; see [ide-shell](ide-shell.md))
- 2026-11-08: a project's README.md is its design goal, opened at the top of the tree; nothing in it is checked or generated (DESIGN 5.35)

## Was
- DESIGN 5.35
