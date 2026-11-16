# By hand: everything the tools do, a person does in the IDE

Status: built. Every change an AI makes through Clarity's tools, a person makes by hand in the IDE, offline: a gesture on a canvas, a field, a menu or a dialog. An audit of 2026-11-13 compared every tool option with the IDE; the gaps it found are built (Decisions).

Progress: 95%

## Code
- Tools: `crates/clarity-tools/src/structural/` (`windows.rs`, `frame.rs`, `model.rs`, `places.rs`), `lib.rs` (rename, missing_translations)
- IDE: `app/src/lib/PageProperties.svelte`, `FrameProperties.svelte`, `DesignLayer.svelte`, `designLayer.ts`, `NewWindowDialog.svelte`, `EntityEditor.svelte`, `ProjectTree.svelte`, `AgentsView.svelte`, `ThemeView.svelte`, `ProjectSettings.svelte`, `TranslationsPanel.svelte`, `PageTells.svelte`, `ViewLineTells.svelte` + `viewTells.ts`, `partProperties.ts`, `SettingsBlockEditor.svelte`, `ThemeForm.svelte` + `themeLines.ts`, `PartForm.svelte`, `LibraryForm.svelte`, `customDecl.ts`, `NewAppDialog.svelte`, `NewCustomDialog.svelte`, `fieldType.ts`; Tauri `app/src-tauri/src/lib.rs`
- Shared edits: `crates/clarity-spec/src/edit.rs`, `design_edit.rs`, `page_rename.rs`, `decl_lines.rs`

## Tests
- `app/src/lib/designLayer.test.ts` (splitter divider, slide-out edge), `partProperties.test.ts`, `viewTells.test.ts`, `fieldType.test.ts`, `themeLines.test.ts`, `customDecl.test.ts`, `agentsPart.test.ts`
- spec `frame_edit.rs`, `rename_page.rs`, `decl_lines.rs`, `dictionary_edit.rs`, `agent_commands.rs`, `translations.rs`, `pane_span.rs`; project `modules.rs`; tools lib tests

## Open
- The add-field form's type: PARTIAL. The field editor has a type picker; the add-field form is still a text box with suggestions.

## Conflicts
- manual/design.windows.md:57 says New window always makes a window and a dialog is made by editing the root; New window offers Blank dialog (`NewWindowDialog.svelte:89`); see [ide-designer](ide-designer.md)

## Decisions
- 2026-11-13: user: "Oh, but I SHOULD be able to do this by hand... what if the internet is down?! And maybe it should be set by properties, but thats okay... but I should be able to do it by hand!!!!! remember human/ai?!!!!", then "I want you to go through all design bits and fix the things I cannot do (easily) by hand!!"
- 2026-11-13: the audit's gaps, all BUILT that day. Windows, frame and app: a blank dialog in New window (`NewWindowDialog.svelte:89`, `addPage` kind dialog, `App.svelte:1085`); a separator in a submenu (`FrameProperties.svelte`); moving sidebars up and down (`FrameProperties.svelte`, `runtimes/web/src/screen/frame.ts:67-72`); a second item in a part's context menu; naming a new app and picking its module; renaming a window or view; a view's tells, declared on the view and wired on the content line that shows it; a slide-out's width by dragging its edge. Part properties: the canvas (units, zoom, grid, snap, background, dots, mode, tools, brush, ink); a table column's link handler (`links`); drop sources (`on drop from`); a toolbox tool's action (`does`); a tree table's resizable columns; the order of a table's columns and a list's shown fields; on click opening a slide-out or popover. Data model and project: renaming a field everywhere; field modifiers owned, computed, cascade, min, max, pattern, column; a field's type with length, precision, scale and every type (a type picker); the app's settings block; a theme's values as a form; a new module; custom parts and libraries; translations the same as the default counted as missing, as the tool counts them; offering a command to agents (`agent` line) by a checkbox; project lines `output keeps` and `codec`.
- 2026-11-13: a slide-out's view and the tells it hears are not built on purpose: the language shows a view only through a content (manual `design.windows.md`, Window, content, view), and the tools refuse an edit that adds an error, so neither side can.

## Was
- none
