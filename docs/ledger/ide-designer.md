# IDE window designer

Status: partial. The Design tab: windows, views and the app's frame placed on Clarity.UI's drawing, edited on the `.cxaml` model.

Progress: 75%

## Code
- Spec: `crates/clarity-spec/src/cxaml.rs` (read and write `.cxaml`; roots Window, Dialog, View, Splash, Toolbox, SignIn, App; a part writes only what differs from `widgets::new_props`, :492), `design_edit.rs:110` (each edit on the page's or app's model; `split` sends the code half to the module, :123), `design_keys.rs` (a key per thing: `design/Main.cxaml#Main/saveButton`), `templates.rs` + `templates/` (new window, view, app as models), `keyed.rs` (texts become keys on the model), `edit.rs` (module text edits only: entities, fields, enums, uses, code), `sample.rs` (sample data)
- Generators: `crates/clarity-gen/src/preview.rs` (the IDE's preview server and `PreviewProgram`: opens windows with sample data, runs no handler), `web_windows.rs` (markup with `designItem` keys)
- IDE:
  - `app/src/lib/PageDesigner.svelte` (981 lines), `pageDesignerStore.svelte.ts`, `selection.ts`, `applyEdit.ts`; Tauri `apply_edit` (`app/src-tauri/src/lib.rs:251`)
  - Canvas: `DesignCanvas.svelte`, `designCanvas.ts`, `DesignLayer.svelte`, `designLayer.ts` (pick, move, resize, dock frame regions), `ShownWindow.svelte` (the running size), `ClarityWindow.svelte` + `clarityWindow.ts` (Clarity.UI's browser build draws canvas and preview), `ZoomBar.svelte`
  - Palette: `PartTile.svelte`; outline `PageOutline.svelte`
  - Inspector: `PageProperties.svelte` (1784 lines; Properties), `FrameProperties.svelte` (regions, ribbon tabs and groups), `CodePanel.svelte` (Code tab checked as typed, Text tab with the `.cxaml`), `IconField.svelte`
  - Double-click a part writes its handler; templates: blank window or view, splash, toolbox, sign-in, settings, form or browse per entity (`NewWindowDialog.svelte:80-90`)
  - Preview tab: run any window with sample data, Back and breadcrumb, language switch (`ClarityWindow.svelte`)
- CLI: `clarity convert-pages <project>` moves page and app blocks out of modules (`crates/clarity-cli/src/main.rs:574`)
- Docs: manual/design.windows.md, design.frame.md, design.parts.md, design.text.md

## Tests
- `crates/clarity-spec/tests/design_edit.rs` (every edit on every example reads back clean), design_keys.rs, cxaml.rs, cxaml_values.rs, templates.rs, keyed.rs, page_edit.rs, frame_edit.rs, table_edit.rs
- `crates/clarity-gen/tests/web_windows.rs`
- `app/src/lib/designCanvas.test.ts`, designLayer.test.ts

## Open
- Edits named by key instead of a place in the file: PARTIAL. `design_edit.rs:154` finds things by span start
- Key translator removed: PARTIAL. `keyed::key_module` still used (`crates/clarity-spec/src/edit.rs:282`)
- `none=` (and `orientation`, `measure`) compared with an empty attribute: PARTIAL. `cxaml.rs:492-509` compares with `widgets::new_props`
- Window size presets (narrow, medium, wide): NOT BUILT (DESIGN 8.1)
- A check that a container's contents fit its height: NOT BUILT (S3b open)
- The app's icon in the designer and preview: NOT BUILT; see [project-file](project-file.md)
- Grid reflow on narrow windows: see [clarity-ui](clarity-ui.md)

## Conflicts
- `app/src/lib/CodePanel.svelte:2-4` and manual/README.md:25 say Text shows the window as Clarity text; it shows the `.cxaml`
- DESIGN 8 says the designer edits `.clarity` spec files and offers no free layout; it edits `.cxaml` on a snap grid (`cxaml::apply`)
- manual/design.windows.md:88 says grid columns go to 24; they go to 96 (`crates/clarity-spec/src/model.rs:594`). :51 leaves out views, splash, toolbox, sign-in and settings in New window
- manual/start.first-project.md:48, design.windows.md:70-84, design.frame.md:47: no heading "The window"; the fieldsets are Data and Events. start.first-project.md:50 vs :73 name the button `add` vs `addButton`
- manual/code.handlers.md:140: Code is an inspector tab, not a button in the Window group

## Decisions
- 2026-10-18: the designer shows windows at the size they open (1100 x 760), and example windows use the full width. User: "not really wysiwyg"
- 2026-11-04: canvas and preview draw with Clarity.UI's browser build from the program's markup (DESIGN 8.1)
- 2026-11-09: `.cxaml` is the source of windows and views. User: "1, cxaml is the source, make it!!! and the .cxaml is used to derive the clarity code!!!!"
- 2026-11-09: all files straight in `design/`, the root naming module and page. User: "Do it the avalonia way!"
- 2026-11-09: the app moves to `design/<App>.cxaml`. User picked: "App to design/ too (Recommended)"
- 2026-11-09: page blocks leave the language; only the converter reads them. User picked: "Only the converter reads (Recommended)"
- 2026-11-09: templates build models; New window, view, splash, toolbox and app write `.cxaml` at once. User: "do the cleanup first"
- 2026-11-09: no translator between design and code (keys per part, edits on the model) proposed; design debugging follows from it, see [debugger](debugger.md). User: "how would you improve it? dont make it, tell it"
- 2026-11-12: a part in a sidebar or the footer is moved and resized as any: the designer's markup marks the sidebar and footer with their keys (`sidebar_element`, `footer_element` in `crates/clarity-spec/src/cxaml.rs`) and Clarity.UI reports their grids by them (`runtimes/rust-ui/src/frame/sidebar.rs`, `footer.rs`); their grids had no key, so a resize found no grid. Tests `the_designers_layer_finds_each_part_and_grid_by_its_design_item` (rust-ui tests/hosts), `the_designer_finds_a_sidebar_s_and_the_footer_s_grid` (gen sidebars.rs). User: "cant resize the list selected!"
- 2026-11-12: in the designer rows keep their height, as docs/LANGUAGE.md says: a growing part (list, table, tree) stands on the rows it is written on, on the window and in a sidebar alike, and only a running program shares the height to spare (`place_of` in `runtimes/rust-ui/src/parts/mod.rs`, by the part's `designItem`). A resize moves a row per row of the design (`rowStep` in `app/src/lib/designLayer.ts`), not per share of a stretched grid. Tests: rust-ui `the_designers_layer_finds_each_part_and_grid_by_its_design_item`, app `designLayer.test.ts`. User: "these list should follow grid!!!!!", "there is clearly something wrong with the grid!!!!"
- 2026-11-13: the design canvas shows a window as it runs: parts that grow grow there too (`runtimes/rust-ui/src/parts/mod.rs` `place_of`; before, rows kept their written height in the designer). User: "that isnt wysiwyg... memo in design is not the same height as in preview!", then "this overlap of project and items in design is also a problem with wysiwyg!!!". Guards: `runtimes/rust-ui/tests/parts/design_grows.rs`, `tests/hosts/browser_host.rs` (a part's height is its laid-out rows), and the live `the_design_canvas_lays_out_a_window_as_the_preview_does_wysiwyg` in `crates/clarity-gen/tests/web_windows.rs` (every named part in the same place on both canvases, headless Chromium, ignored by default).

## Was
- DESIGN 8 (designer bullets; the init, stub and LSP lines were struck, see [vision](vision.md)), DESIGN 8.1, S3, S3b, S3d, S3e
- CXAML-SOURCE-PLAN.md (status steps 1-4, "no translator" steps 1-3, the cleanup; the language rule that pages live in `.cxaml` is [language](language.md))
- memory: full-width-layouts.md
