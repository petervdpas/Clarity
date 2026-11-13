# Project file

Status: built. The `<name>.clarity` project file: title, version, icon, stack, output, language, currency, themes, sign-in, keys, engines, port, row height, agents (while closed, at most), data sources, file stores, codecs.

Progress: 100%

## Code
- Spec: `crates/clarity-project/src/file.rs` (`parse` :168-633, an unknown line is an error listing the valid ones; `write` :680; `agents` lines read :360-386, written :758-763); `Settings` in `crates/clarity-project/src/lib.rs:265-341` (`agents_read_closed`, `agents_change_closed`, `agents_at_most` :324-332), `SourceKind` :66-75, `Backend`, `Ui`, `Target`; new projects start as `stack rust desktop` with one SQLite source (`starter`, lib.rs:460)
- Generators: read through `clarity_project`; no project-file code of their own
- IDE: `app/src/lib/ProjectSettings.svelte`, `NewProjectDialog.svelte`, `AgentsView.svelte` (the `agents` lines); the Project tab writes only lines whose value changed
- Docs: manual `project.file.md`

## Tests
- `crates/clarity-project/tests/`: settings.rs, version.rs, icon.rs, signin.rs, folders.rs, ide.rs, part_stacks.rs
- `app/src/lib/version.test.ts`

## Open
- none. Splash is set on the app, not here (`crates/clarity-gen/src/web_windows.rs:112`); see [windows-dialogs](windows-dialogs.md).

## Conflicts
- `Ui::Webview` is kept only to be refused (`crates/clarity-project/src/lib.rs:48, 361`); comments at lib.rs:128 and :969 still speak of the shared web screen.
- `manual/project.file.md:34` says settings hold Migrations and Backups per database.
- Port 8080 written twice: `crates/clarity-project/src/lib.rs:349` (`DEFAULT_PORT`), `app/src/lib/ProjectSettings.svelte:416, 422`. Equal.

## Decisions
- 2026-10-16: a project is one file `<name>.clarity` in Clarity, with a folder of the same name; replaces `clarity.toml`; reading is strict.
- 2026-11-03: webview desktops dropped; Clarity.UI is the only UI, so naming one in the stack line is an error (see [clarity-ui](clarity-ui.md)).
- 2026-11-08: `ui library` / `ui crate` gone; every Rust desktop links the ready-made library.

## Was
- DESIGN 1.2 "The project file (2026-10-16)" and the stack line
- Roadmap P1, P2
- IMPLEMENTED.md section 2; UNIMPLEMENTED.md section 8 "Leftover"; CONFLICTING.md section 5 (lib.rs comment), section 6 "Project file"
