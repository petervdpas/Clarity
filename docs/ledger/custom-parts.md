# Custom parts and libraries

Status: built. `part X` (a part an outside library draws) and `library X` (functions from a package), both written in the project's own stack language and declared in Clarity.

## Code
- Spec: `part` in `crates/clarity-spec/src/code/parse.rs:882`, `library` at :1015; `crates/clarity-spec/src/native.rs` (part and library files)
- Generators: `crates/clarity-gen/src/parts.rs`, `part_code.rs` (reads each part's code before generating), `libraries.rs`, `{rust,go,c,cpp,csharp}/parts.rs`; language check `lib.rs:871`; a custom part in a web-only program is refused (`lib.rs:1628`)
- Runtimes: part state and hears by name in every runtime (`conformance/custom-parts.json`); C# adapter base in `runtimes/csharp/Clarity.Ui/Custom/`
- Clarity.UI: `runtimes/rust-ui/src/parts/custom.rs` (`CustomPart`); C interface `runtimes/rust-ui-ffi/src/custom.rs` (`clarity_custom_register`, `clarity_canvas_*`)
- IDE: `app/src/lib/FolderEditor.svelte` (folders with Build), `app/src-tauri/src/parts.rs`; CLI `clarity parts`
- Docs: `manual/design.custom-parts.md`, `manual/code.libraries.md`; LANGUAGE.md section 6 "Custom parts", "External libraries"

## Tests
- `conformance/custom-parts.json`: run by R, Go, C, C++; skipped by C#
- `crates/clarity-spec/tests/`: custom_parts.rs, libraries.rs
- `crates/clarity-gen/tests/`: native_parts.rs, libraries.rs, rust_clarity_parts.rs; `crates/clarity-project/tests/part_stacks.rs`
- Examples: orbs (C++, SFML)

## Open
- `preview "arena.png"` on the design canvas: PARTIAL. Parsed (`code/parse.rs:980`), read by nothing.
- A failed build of a part library as a problem at its declaration: PARTIAL. Missing folders are problems; build failures are not.
- Owned part files listed in a generated `AGENTS.md`/`CLAUDE.md` (DESIGN 6.5): see [generation](generation.md).
- Whether Clarity's agent may write a part's C, C++ or C# adapter, or only the declaration: open (DESIGN 20 question 12).

## Conflicts
- LANGUAGE.md:1622 says `nuget` and `dll` parts are not generated; C# generates them (`crates/clarity-gen/src/csharp/parts.rs:26, 56, 138`).
- `crates/clarity-gen/src/csharp/logic.rs:558` says custom parts are refused "until CP1 builds them"; the code after it generates them.
- DESIGN 6.5 bullets and "Windows (2026-10-24, done)", and roadmap CP1/CP2, describe native child windows and part hosts (`runtimes/parts`, `clarity_runtime::parts`, Go `parts`); removed 2026-11-03, parts draw on Clarity.UI.
- `manual/design.custom-parts.md:139` says Clarity.UI on Rust and C#; it is all five stacks. :122, :127 use page blocks.
- `conformance/custom-parts.json` "about" still names Avalonia.
- Grammar summary (LANGUAGE.md:3028-3149): custom parts have no `crate`, `go`, `description`.

## Decisions
- 2026-10-23: a custom part is a link to a library in the language of the project's stack, with that language's packages; never JS, never one C ABI for all. User: "a custom part is a link to external library", "if you pick a language, the custom part needs to be in that language!!"
- 2026-10-23: external libraries follow the same rule (`nuget`, `dll`, `crate`, `go`, `cmake`), called as `Name.function(...)`.
- 2026-10-23: the canvas's `renderer Name` removed; outside graphics are a custom part.
- 2026-11-02: `grows` is an option of the part, not of each place.
- 2026-11-03: every desktop draws custom parts with Clarity.UI through a table of callbacks.
- 2026-11-08: Rust parts go through the C interface too; `ui crate` removed.

## Was
- DESIGN 5.19, 5.21, 6.5 (with "Windows (2026-10-24, done)"), 5.39 paragraph "Rust custom parts on the library; `ui crate` gone"; DESIGN 20 question 12
- Roadmap CP1, CP2, EL1
- IMPLEMENTED.md section 4 "Custom parts", section 7 generator refusals (other language, web-only); UNIMPLEMENTED.md section 4 (preview, failed build)
- memory: custom-parts-are-libraries.md
