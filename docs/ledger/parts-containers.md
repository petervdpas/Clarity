# Containers, content and views

Status: built. Parts that hold a grid of parts (form, panel, tabs, steps, accordion, scroll, splitter, popover, slideout), context menus on parts, and a window's content that shows one view at a time.

Progress: 95%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (Containers group; `Content` with `view X` lines), `views.rs` (`places` :33, `placed` :102 folds each view into its window for every generator), `templates.rs` (`in_content_of` :487: a new window is window, content, view), `code/check.rs:2617-2629` (`main.swap(View)`), `model.rs:663` (`Widget.menu`, a part's context menu); `fit.rs` cuts a slide-out and a view to the rows the window holds (see [windows-dialogs](windows-dialogs.md))
- Generators: no code of their own; `views::placed` writes `main.swap(X)` as `main.tab = n` and a view's tells as calls of the window's handlers
- Runtimes: part state per container (accordion `open`, steps `step`, tabs `tab`, folded panels, popover and slide-out `visible`, slide-out `pinned`): R `runtimes/rust/src/ui.rs:324`, `steps.rs`; Go `clarity/steps.go`; C `clarity/steps.c`; C++ `clarity/steps.cpp`; C# `PartState.cs`, `WindowViewModel.cs` (`IsPinned`)
- Clarity.UI: `runtimes/rust-ui/src/parts/containers/` `container.rs`, `panel.rs` (form, panel, fold, label column), `tabs.rs` (tabs, and content as tabs without a strip), `steps.rs`, `accordion.rs` (also horizontal), `scroll.rs`, `scroll_view.rs`, `splitter.rs` (a side by side first pane at `span` columns, a stacked one at `rows`, `<Pane rows="4">`), `popover.rs`, `slideout.rs` (with `place_slideouts`; takes no room in the window's content, `takes_room`); context menus `parts/mod.rs:470` (`designed_menu`)
- IDE: `app/src/lib/PageProperties.svelte` (Shows view, Add view, Remove view), `PageTells.svelte` (a view's Tells, Add and remove), `ViewLineTells.svelte` + `viewTells.ts` (Tells of a view on the content's view line, Wire), `DesignLayer.svelte` / `designLayer.ts` (dragging a splitter's divider), `PageDesigner.svelte`, `ProjectTree.svelte` (Views, New view); context-menu items by hand: see [by-hand](by-hand.md)
- Docs: manual `parts.containers.md`, `design.windows.md` (Window, content, view; Views)

## Tests
- `conformance/slideouts.json`: run by R, Go, C, C++, C#, rust-ui
- `conformance/accordions.json`, `folding.json`, `steps.json`: run by R, Go, C, C++, rust-ui; skipped by C#
- `conformance/overlays.json`: run by rust-ui only
- spec `content.rs`, `views.rs`, `view_names.rs`, `accordions.rs`, `steps.rs`, `slideouts.rs`, `overlays.rs`, `folding.rs`, `pane_span.rs`, `tabs_open.rs`, `part_frame.rs`, `form_labels.rs`, `placed_places.rs`
- gen `accordions.rs`, `folding.rs`, `overlays.rs`, `slideouts.rs`, `views.rs`, `wizards.rs`, `form_labels.rs`
- rust-ui `tests/parts/parts_accordion.rs`, `parts_content.rs`, `parts_panel.rs`, `parts_popover.rs`, `core_popover_route.rs`, `parts_splitter.rs`, `parts_steps.rs`, `parts_tabs.rs`, `parts_scroll_view.rs`, `parts_frames.rs`; `tests/windows/slideouts.rs` (also `a_tall_slide_out_gives_the_window_nothing_to_scroll`); `tests/screen/core_context_menu.rs`, `grid_around.rs`
- IDE `app/src/lib/viewTells.test.ts`, `designLayer.test.ts` (splitter divider)

## Open
- The menu key and Shift+F10 open a part's context menu: PARTIAL. Only a right press does (`runtimes/rust-ui/src/element.rs:337`, `context_menu_at`; F10 goes to the menu bar only, `window/view.rs:438`; DESIGN.md:1122).
- A view that brings its own ribbon tab and menu items: see [frame-keys](frame-keys.md).

## Conflicts
- Memory note ui-content-structure.md says `main.view = X`; the checker takes `main.swap(OtherView)` or `main.tab` (`crates/clarity-spec/src/code/check.rs:2617-2629`), as manual `design.windows.md` and `parts.containers.md` say.
- C# carries `PartState.cs` copies for accordions, folding, steps that no test runs against the conformance files.

## Decisions
- 2026-10-22: a wizard is a container part, `steps`; Next runs `on next` on the step shown and stays on a refusal, Back runs nothing, Finish runs `on finish`.
- 2026-10-22: a context menu is a `menu` block under any part; a right press on a table or list row chooses it first.
- 2026-10-22: a popover is a container with no place in the grid, opened next to the part `for` names, closed by a click outside or Escape.
- 2026-10-23: containers share the column lines of the grid around them; a form gets a label column (`labels 3`).
- 2026-11-05: a slide-out slides in over the window from an edge, with a handle tab, `resizable`, `pinnable`; width and pin kept per person; at the bottom its handles stand above the footer (the person asked for it above the footer, not in it).
- 2026-11-09: a view shows only in content; a slide-out, tab, step or section that names one is refused.
- 2026-11-09: every window is window, content, view, even one that never swaps; a dialog holds its parts; content has no look of its own.
- 2026-11-13: a stacked splitter's first pane starts at rows of its own, `<Pane rows="4">`, as a side by side one starts at `span`. User, on the maintenance sidebar: "I want a slider between projects and items panel", then "default heights 25%/75% ?!". By hand in the designer: drag the splitter's divider on the canvas (`DesignLayer.svelte` `pressDivider`, `designLayer.ts` `dividerOf`, `paneSizeAt`, tested in `designLayer.test.ts`), or the pane's Width or Height field; user: "I SHOULD be able to do this by hand... what if the internet is down?!". The edits of a container's pages (pane size, content view, view tells, section color) now find containers in sidebars and the footer too; before, "there is no pane there" for a splitter in a sidebar (`design_edit.rs` `page_anywhere`, test `pane_span.rs`).

## Was
- DESIGN 5.6 (Containers), 5.15, 5.16, 5.32, 5.33 (all but the tree levels and .cxaml bullets)
- Roadmap: U1b (accordion, scroll box, shared grid lines, label column), U1d (wizards, context menus, popovers)
- memory: ui-content-structure.md
