# IDE window designer

Status: partial. The Design tab: windows, views and the app's frame placed on Clarity.UI's drawing, edited on the `.cxaml` model.

Progress: 75%

## Code
- Spec: `crates/clarity-spec/src/cxaml.rs` (read and write `.cxaml`; roots Window, Dialog, View, Splash, Toolbox, SignIn, App; a part writes only what differs from `widgets::new_props`, :500), `design_edit.rs:110` (each edit on the page's or app's model; `split` sends the code half to the module, :123), `design_keys.rs` (a key per thing: `design/Main.cxaml#Main/saveButton`), `templates.rs` + `templates/` (new window, view, app as models), `fit.rs` (every design edit cuts parts to the rows the window holds, a slide-out to those rows, a view's parts to its content area), `page_rename.rs` (rename a window or view, from the tree), `keyed.rs` (texts become keys on the model), `edit.rs` (module text edits only: entities, fields, enums, uses, code), `sample.rs` (sample data)
- Generators: `crates/clarity-gen/src/preview.rs` (the IDE's preview server and `PreviewProgram`: opens windows with sample data, runs no handler), `web_windows.rs` (markup with `designItem` keys)
- IDE:
  - `app/src/lib/PageDesigner.svelte` (981 lines), `pageDesignerStore.svelte.ts`, `selection.ts`, `applyEdit.ts`; Tauri `apply_edit` (`app/src-tauri/src/lib.rs:238`)
  - Canvas: `DesignCanvas.svelte`, `designCanvas.ts`, `DesignLayer.svelte`, `designLayer.ts` (pick, move, resize, dock frame regions, drag a splitter's divider and a slide-out's inner edge), `ShownWindow.svelte` (the running size), `ClarityWindow.svelte` + `clarityWindow.ts` (Clarity.UI's browser build draws canvas and preview), `browserFiles.ts` + `browserFilesStore.svelte.ts` (missing or stale browser files said in the canvas's place, Rebuild), `ZoomBar.svelte`
  - Palette: `PartTile.svelte`; outline `PageOutline.svelte`
  - Inspector: `PageProperties.svelte` (2096 lines; Properties), `PageTells.svelte` + `ViewLineTells.svelte` (a view's tells, wired on the content line that shows it), `FrameProperties.svelte` (regions, ribbon tabs and groups), `CodePanel.svelte` (Code tab checked as typed, Text tab with the `.cxaml`), `IconField.svelte`
  - Double-click a part writes its handler; templates: blank window, blank dialog, blank view, splash, toolbox, sign-in, settings, form or browse per entity (`NewWindowDialog.svelte:86-100`; a dialog is `addPage` kind dialog, `App.svelte:1085`); New app asks for a name and a module (`NewAppDialog.svelte`)
  - Preview tab: run any window with sample data, Back and breadcrumb, language switch (`ClarityWindow.svelte`)
- CLI: `clarity convert-pages <project>` moves page and app blocks out of modules (`crates/clarity-cli/src/main.rs:576`)
- Docs: manual/design.windows.md, design.frame.md, design.parts.md, design.text.md

## Tests
- `crates/clarity-spec/tests/design_edit.rs` (every edit on every example reads back clean), design_keys.rs, cxaml.rs, cxaml_values.rs, templates.rs, keyed.rs, page_edit.rs, frame_edit.rs, table_edit.rs, fit.rs, rename_page.rs, pane_span.rs, layout.rs
- `crates/clarity-gen/tests/web_windows.rs`
- `app/src/lib/designCanvas.test.ts`, designLayer.test.ts, viewTells.test.ts, partProperties.test.ts, browserFiles.test.ts (what a canvas says when the browser files are missing or stale)
- `crates/clarity-gen/tests/browser_fresh.rs` (browser files fine, missing or stale)

## Open
- Edits named by key instead of a place in the file: PARTIAL. `design_edit.rs:154` finds things by span start
- Key translator removed: PARTIAL. `keyed::key_module` still used (`crates/clarity-spec/src/edit.rs:303`)
- `none=` (and `orientation`, `measure`) compared with an empty attribute: PARTIAL. `cxaml.rs:499-515` compares with `widgets::new_props`
- Window size presets (narrow, medium, wide): NOT BUILT (DESIGN 8.1)
- A check that a container's contents fit its height: NOT BUILT (S3b open)
- The app's icon in the designer and preview: NOT BUILT; see [project-file](project-file.md)
- Grid reflow on narrow windows: see [clarity-ui](clarity-ui.md)

## Conflicts
- `app/src/lib/CodePanel.svelte:2-4` and manual/README.md:26 say Text shows the window as Clarity text; it shows the `.cxaml`
- manual/design.windows.md:96 says grid columns go to 24; they go to 96 (`crates/clarity-spec/src/model.rs:594`)
- manual/design.windows.md:57 says New window always makes a window and a dialog is made by editing the root; New window offers Blank dialog (`NewWindowDialog.svelte:89`), and the line leaves out view, splash, toolbox, sign-in and settings
- manual/start.first-project.md:48, design.windows.md:76-80, design.frame.md:47: no heading "The window"; the fieldsets are Data and Events. start.first-project.md:50 vs :73 name the button `add` vs `addButton`
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
- 2026-11-12 (overturned by 2026-11-13 below): in the designer rows keep their height, as docs/LANGUAGE.md says: a growing part (list, table, tree) stands on the rows it is written on, on the window and in a sidebar alike, and only a running program shares the height to spare (`place_of` in `runtimes/rust-ui/src/parts/mod.rs`, by the part's `designItem`). A resize moves a row per row of the design (`rowStep` in `app/src/lib/designLayer.ts`), not per share of a stretched grid. Tests: rust-ui `the_designers_layer_finds_each_part_and_grid_by_its_design_item`, app `designLayer.test.ts`. User: "these list should follow grid!!!!!", "there is clearly something wrong with the grid!!!!"
- 2026-11-13: the design canvas shows a window as it runs: parts that grow grow there too (`runtimes/rust-ui/src/parts/mod.rs` `place_of`; before, rows kept their written height in the designer). User: "that isnt wysiwyg... memo in design is not the same height as in preview!", then "this overlap of project and items in design is also a problem with wysiwyg!!!". Guards: `runtimes/rust-ui/tests/parts/design_grows.rs`, `tests/hosts/browser_host.rs` (a part's height is its laid-out rows), and the live `the_design_canvas_lays_out_a_window_as_the_preview_does_wysiwyg` in `crates/clarity-gen/tests/web_windows.rs` (every named part in the same place on both canvases, headless Chromium, ignored by default).
- 2026-11-13: the design canvas and preview say in their place when Clarity.UI's browser files are missing or behind its source, instead of a raw "page script did not load" over a blank canvas. One check with three answers (`clarity_gen::clarity_ui::browser_files`, `BrowserFiles` fine, missing or stale; command `browser_files` replaces `browser_files_stale`), asked at start and whenever a canvas cannot draw. Run from the repository the canvas offers Rebuild (asks first, builds in the Output panel); an installed Clarity says to install it again. When the rebuild ends the canvases load again by themselves (`browserFilesStore.svelte.ts`, `resetPageScript` in `clarityWindow.ts`); the folder named at start is named even when missing, and found again at each use. User: "that does not sound like a solution, but symptom prevention". Guards: `the_browser_files_read_fine_missing_or_stale` (`crates/clarity-gen/tests/browser_fresh.rs`), `app/src/lib/browserFiles.test.ts`. The live headless test does not cover a missing folder yet.
- 2026-11-13: the Parts panel holds Clarity's parts only; the window's data (a record's fields, lists, plain values, the app's settings) is no longer listed there to drag, and a part is bound in its properties. User: "what is ledger folder????", "what is with the data shit in the parts?", chose "Nowhere" for the data list (`app/src/lib/PageDesigner.svelte`).
- 2026-11-13: the Parts tab shows every part type Clarity has, in every project and on every page (tools and sign-in parts too); what a part means is settled where it is dropped. Custom part is one generic tile: dropped, it asks which of the project's custom parts it is or makes a new one (`PageDesigner.svelte` `pickCustom`, App `newPartForDrop`), and the properties' Custom part picker changes it. User: "Even if there is no project open it should exist!!!!!", "it should show all part types Clarity has to offer!!!!", "It only means something if its dragged onto the app!", "that how it worked in the beginning".

## Was
- DESIGN 8 (designer bullets; the init, stub and LSP lines were struck, see [vision](vision.md)), DESIGN 8.1, S3, S3b, S3d, S3e
- CXAML-SOURCE-PLAN.md (status steps 1-4, "no translator" steps 1-3, the cleanup; the language rule that pages live in `.cxaml` is [language](language.md))
- memory: full-width-layouts.md
