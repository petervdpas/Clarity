# Project file

Status: built. The `<name>.clarity` project file: title, version, icon, stack, output, language, currency, themes, sign-in, keys, engines, port, row height, data sources, file stores, codecs.

## Code
- Spec: `crates/clarity-project/src/file.rs:293-569` (reading and writing; an unknown line is an error listing the valid ones); `Settings` in `crates/clarity-project/src/lib.rs:213-277`, `SourceKind` :66-75, `Backend`, `Ui`, `Target`; new projects start as `stack rust desktop` with one SQLite source (lib.rs:396)
- Generators: read through `clarity_project`; no project-file code of their own
- IDE: `app/src/lib/ProjectSettings.svelte`, `NewProjectDialog.svelte`; the Project tab writes only lines whose value changed
- Docs: `manual/project.file.md`; LANGUAGE.md section 2

## Tests
- `crates/clarity-project/tests/`: settings.rs, version.rs, icon.rs, signin.rs, folders.rs, ide.rs, part_stacks.rs
- `app/src/lib/version.test.ts`

## Open
- none. Splash is set on the app, not here (`crates/clarity-gen/src/web_windows.rs:112`); see [windows-dialogs](windows-dialogs.md).

## Conflicts
- `Ui::Webview` is kept only to be refused (`crates/clarity-project/src/lib.rs:297, 317`); comments at lib.rs:128 and :894 still speak of the shared web screen.
- DESIGN :46, :112, :1931 put a UI word in the stack line; naming a UI is refused (`file.rs:414-426`). DESIGN :1906, :1363 use `stack go webview`.
- DESIGN :1612 (`ui library`, `ui crate`): read and dropped (`file.rs:366-368`).
- LANGUAGE.md:93-95 says only `datasource` and `filestore` repeat and has no `codec`; `codec` repeats (`file.rs:550`).
- LANGUAGE.md:87 and the grammar leave out `beside` and the `file` kind with `using Mapping` (`file.rs:484-519`).
- LANGUAGE.md:194 says C and C++ use the shared screen (webview); every desktop draws with Clarity.UI (LANGUAGE.md:77 agrees).
- Grammar summary (LANGUAGE.md:3028-3149): `stack` takes two words; no icon, version, output keeps, signin, keys, engines, port, row height, filestore, codec.
- `manual/project.file.md:26-38` leaves out `version`, `icon`, `signin`; :66 says settings hold Migrations and Backups per database.
- Port 8080 written twice: `crates/clarity-project/src/lib.rs:285`, `app/src/lib/ProjectSettings.svelte:369`. Equal.

## Decisions
- 2026-10-16: a project is one file `<name>.clarity` in Clarity, with a folder of the same name; replaces `clarity.toml`; reading is strict.
- 2026-11-03: webview desktops dropped; Clarity.UI is the only UI, so naming one in the stack line is an error (see [clarity-ui](clarity-ui.md)).
- 2026-11-08: `ui library` / `ui crate` gone; every Rust desktop links the ready-made library.

## Was
- DESIGN 1.2 "The project file (2026-10-16)" and the stack line
- Roadmap P1, P2
- IMPLEMENTED.md section 2; UNIMPLEMENTED.md section 8 "Leftover"; CONFLICTING.md section 5 (lib.rs comment), section 6 "Project file"
