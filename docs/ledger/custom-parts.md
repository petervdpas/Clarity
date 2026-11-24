# Custom parts and libraries

Status: built. `part X` (a part an outside library draws) and `library X` (functions from a package), both written in the project's own stack language and declared in Clarity.

Progress: 90%

## Code
- Spec: `part` in `crates/clarity-spec/src/code/parse.rs:887`, `library` at :1020; `crates/clarity-spec/src/native.rs` (part and library files)
- Generators: `crates/clarity-gen/src/parts.rs`, `part_code.rs` (reads each part's code before generating), `libraries.rs`, `{rust,go,c,cpp,csharp}/parts.rs`; language check `lib.rs:891`; a custom part in a web-only program is refused (`check_parts_in_browser`, `lib.rs:1668`)
- Runtimes: part state and hears by name in every runtime (`conformance/custom-parts.json`); C# adapter base in `runtimes/csharp/Clarity.Ui/Custom/`
- Clarity.UI: `runtimes/rust-ui/src/parts/custom.rs` (`CustomPart`); C interface `runtimes/rust-ui-ffi/src/custom.rs` (`clarity_custom_register`, `clarity_canvas_*`)
- IDE: `app/src/lib/FolderEditor.svelte` (folders with Build), `app/src-tauri/src/parts.rs`; **+ New custom part** and **+ New library** (`NewCustomDialog.svelte`, `ProjectTree.svelte:431`); a part's and a library's view is a form, one declaration line per change (`PartForm.svelte` in `PartView.svelte`, `LibraryForm.svelte` in `LibraryView.svelte`); CLI `clarity parts`
- Docs: manual `design.custom-parts.md`, `code.libraries.md`, `parts.graphics.md` (`custom part`)

## Tests
- `conformance/custom-parts.json`: run by R, Go, C, C++; skipped by C#
- `crates/clarity-spec/tests/`: custom_parts.rs, libraries.rs
- `crates/clarity-gen/tests/`: native_parts.rs, libraries.rs, rust_clarity_parts.rs; `crates/clarity-project/tests/part_stacks.rs`
- Examples: orbs (C++, SFML)

## Open
- `preview "arena.png"` on the design canvas: PARTIAL. Parsed (`code/parse.rs:985`) and set in the IDE's part form (the Picture field, `PartForm.svelte:158-160`); the canvas does not draw it.
- A failed build of a part library as a problem at its declaration: PARTIAL. Missing folders are problems; build failures are not.
- Owned part files listed in a generated `AGENTS.md`/`CLAUDE.md` (DESIGN 6.5): see [generation](generation.md).
- Whether Clarity's agent may write a part's C, C++ or C# adapter, or only the declaration: open (DESIGN 20 question 12).
- Undo of a rename or delete in a part's folder brings back its text files only: a file that is not text (a picture) is moved or deleted with no Undo, and a folder left empty by an Undo stays. Open question: should such a file be kept for Undo too (FileChange holds text)?

## Conflicts
- `crates/clarity-gen/src/csharp/logic.rs:588` says custom parts are refused "until CP1 builds them"; the code after it generates them.
- `conformance/custom-parts.json` "about" still names Avalonia.

## Decisions
- 2026-10-23: a custom part is a link to a library in the language of the project's stack, with that language's packages; never JS, never one C ABI for all. User: "a custom part is a link to external library", "if you pick a language, the custom part needs to be in that language!!"
- 2026-10-23: external libraries follow the same rule (`nuget`, `dll`, `crate`, `go`, `cmake`), called as `Name.function(...)`.
- 2026-10-23: the canvas's `renderer Name` removed; outside graphics are a custom part.
- 2026-11-02: `grows` is an option of the part, not of each place.
- 2026-11-03: every desktop draws custom parts with Clarity.UI through a table of callbacks.
- 2026-11-08: Rust parts go through the C interface too; `ui crate` removed.

- 2026-11-23: one rule and one write for the files of a part's folder, the IDE's and the agents' alike: `clarity_tools::parts::part_path` (plain names split by `/`, none empty, `.`, `..` or hidden, no backslash, nothing in a build folder) and `parts::part_file` (never Clarity's own file), written by `clarity_tools::files::write_text` (atomic, a newline at the end) and kept for Undo in the IDE too (`save_part_file`, `new_part_file`, and `rename_part_file` and `delete_part_file` through `parts.rs` `rename_in` and `delete_in`: a delete keeps each text file's text, a rename removes each old file and writes the new one, one Undo each; Undo makes a folder deleted with its files again, `replay` in `app/src-tauri/src/lib.rs`). The IDE allowed hidden names, backslashes and build output before and wrote with no Undo. Test `a_path_in_a_part_s_folder_is_plain_and_outside_the_build`; the IDE's rename and delete in `app/src-tauri/src/parts.rs` tests (`deleting_a_file_keeps_its_text_for_undo`, `deleting_a_folder_keeps_the_text_of_every_file_in_it`, `renaming_removes_the_old_file_and_writes_the_new_in_one_change`, `renaming_and_deleting_keep_the_rule_of_part_files`). Test `a_path_in_a_part_s_folder_is_plain_and_outside_the_build`. User: "the must be only one way and 'ide/agents OR WHATEVER' can hook into it!!!", "that accounts for all tools... only ONE version should exist!!!"

## Was
- DESIGN 5.19, 5.21, 6.5 (with "Windows (2026-10-24, done)"), 5.39 paragraph "Rust custom parts on the library; `ui crate` gone"; DESIGN 20 question 12
- Roadmap CP1, CP2, EL1
- IMPLEMENTED.md section 4 "Custom parts", section 7 generator refusals (other language, web-only); UNIMPLEMENTED.md section 4 (preview, failed build)
- memory: custom-parts-are-libraries.md
