# By hand: everything the tools do, a person does in the IDE

Status: built. Every change an AI makes through Clarity's tools, a person makes by hand in the IDE, offline: a gesture on a canvas, a field, a menu or a dialog. An audit of 2026-11-13 compared every tool option with the IDE and found the gaps below; each is fixed in its own piece and leaves this list when it is.

Progress: 95%

## Code
- Tools: `crates/clarity-tools/src/structural/` (`windows.rs`, `frame.rs`, `model.rs`, `places.rs`), `lib.rs` (rename, missing_translations)
- IDE: `app/src/lib/PageProperties.svelte`, `FrameProperties.svelte`, `DesignLayer.svelte`, `designLayer.ts`, `NewWindowDialog.svelte`, `EntityEditor.svelte`, `ProjectTree.svelte`, `AgentsView.svelte`, `ThemeView.svelte`, `ProjectSettings.svelte`, `TranslationsPanel.svelte`; Tauri `app/src-tauri/src/lib.rs`
- Shared edits: `crates/clarity-spec/src/edit.rs`, `design_edit.rs`

## Tests
- `app/src/lib/designLayer.test.ts` (splitter divider, slide-out edge), `partProperties.test.ts`, `viewTells.test.ts`, `fieldType.test.ts`, `themeLines.test.ts`, `customDecl.test.ts`, `agentsPart.test.ts`
- spec `frame_edit.rs`, `rename_page.rs`, `decl_lines.rs`, `dictionary_edit.rs`, `agent_commands.rs`, `translations.rs`, `pane_span.rs`; project `modules.rs`; tools lib tests

## Open
Windows, frame and app:
- A blank dialog in New window: BUILT 2026-11-13 (`NewWindowDialog.svelte`, `addPage`).
- A separator in a submenu: BUILT 2026-11-13 (`FrameProperties.svelte`).
- Moving sidebars up and down: BUILT 2026-11-13 (`FrameProperties.svelte`, `frame.ts` index and count).
- A second item in a part's context menu: BUILT 2026-11-13.
- Naming a new app and picking its module: BUILT 2026-11-13.
- Renaming a window or view: BUILT 2026-11-13.
- A view's tells, declared on the view and wired on the content line that shows it: BUILT 2026-11-13.
- A slide-out's width by dragging its edge on the canvas: BUILT 2026-11-13.

Part properties:
- Canvas: units, zoom, grid, snap, background, dots, mode (objects), tools, brush, ink: BUILT 2026-11-13.
- A table column's link handler (`links`): BUILT 2026-11-13.
- Drop sources (`on drop from`): BUILT 2026-11-13.
- A toolbox tool's action (`does`): BUILT 2026-11-13.
- A tree table's resizable columns: BUILT 2026-11-13.
- The order of a table's columns and a list's shown fields: BUILT 2026-11-13.
- On click opening a slide-out or popover: BUILT 2026-11-13.
- A slide-out's view and the tells it hears: not built on purpose: the language shows a view only through a content (LANGUAGE.md:787), and the tools refuse an edit that adds an error, so neither side can.

Data model and project:
- Renaming a field everywhere: BUILT 2026-11-13.
- Field modifiers owned, computed, cascade, min, max, pattern, column: BUILT 2026-11-13.
- Field type with length, precision, scale and every type: BUILT 2026-11-13 (a type picker; the add-field form is still a text box with suggestions: PARTIAL).
- The app's settings block: BUILT 2026-11-13.
- A theme's values as a form: BUILT 2026-11-13.
- A new module: BUILT 2026-11-13.
- Custom parts and libraries declared and changed by hand: BUILT 2026-11-13.
- Translations the same as the default counted as missing, as the tool counts them: BUILT 2026-11-13.
- Offering a command to agents (`agent` line) by a checkbox: BUILT 2026-11-13.
- Project lines `output keeps` and `codec`: BUILT 2026-11-13.

## Conflicts
- none

## Decisions
- 2026-11-13: user: "Oh, but I SHOULD be able to do this by hand... what if the internet is down?! And maybe it should be set by properties, but thats okay... but I should be able to do it by hand!!!!! remember human/ai?!!!!", then "I want you to go through all design bits and fix the things I cannot do (easily) by hand!!"

## Was
- none
