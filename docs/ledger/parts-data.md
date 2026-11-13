# Tables, lists and trees

Status: built. Parts that show a list of records: `table` (cell editors, columns, order, pages, adding and removing rows), `list` (rows or cards), `tree` and `treeTable`, each able to reorder rows by hand.

Progress: 90%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (Data group), `trees.rs` (trees of several kinds of records: `level`, `group`, `on select Entity`, lowered to a tree of rows), `check.rs` (columns, `order by`, `reorder by`, `sortable` refused with `reorder by`); rows dragged from another part, `on drop from part: handler` (`model.rs` `DropFrom` :1205, `parser.rs:2296`, `check.rs:2289` `check_drops`)
- Generators: tables, trees and tree levels through window markup and `views::placed`; per-stack edits of tables in logic.rs
- Runtimes: shown rows decided in the runtime (DESIGN 1.7): order and pages R `runtimes/rust/src/tables.rs`, trees `trees.rs`, reorder `reorder.rs`, views `views.rs`; Go `clarity/tables.go`, `trees.go`, `reorder.go`, `views.go`; C and C++ `clarity/tables.*`, `trees.*`, `reorder.*`, `views.*`; C# `Reorder.cs`, `RowOrder.cs`, `TablePages.cs`, `Trees.cs`
- Clarity.UI: `runtimes/rust-ui/src/parts/data/table.rs` (table, tree, treeTable; 2287 lines), `data/table/reach.rs`, `data/rows.rs` (row drag and reorder; `drop_handler` :692 for a row from another part), `data/list.rs`
- IDE: `app/src/lib/PageProperties.svelte` (Columns in their order with Move up and Move down, Order, Reorder by the person, Link handlers, Takes rows from), header edges sized on the design canvas
- Docs: manual `parts.data.md` (Showing a list, Binding, Choosing and opening rows, Reordering rows, Dragging rows between parts, `table`, `list`, `tree`, `treetable`)

## Tests
- `conformance/reorder.json`: run by R, Go, C, C++, C#, rust-ui
- `conformance/table-order.json`, `table-pages.json`, `trees.json`: run by R, Go, C, C++, rust-ui; skipped by C# (`TablePages.cs`, `Trees.cs` name them in a comment, no test loads them)
- `conformance/tables.json`: run by rust-ui only
- spec `table_columns.rs`, `table_pages.rs`, `table_edit.rs`, `table_rows.rs`, `trees.rs`, `tree_levels.rs`, `reorder.rs`, `list_cards.rs`, `striped_rows.rs`, `links.rs` and `typing.rs` (drops)
- gen `table_pages.rs`, `trees.rs`, `tree_levels.rs`, `list_looks.rs`, `typing_build.rs` (drops)
- rust-ui `tests/data/data_table.rs`, `data_tree.rs`, `data_list.rs` (also drops), `table_add_remove.rs`, `table_edge.rs`, `table_key.rs`, `table_row.rs`, `table_sideways.rs`, `table_tip.rs`, `table_width.rs`, `striped_rows.rs`

## Open
- Moving several rows at once: NOT BUILT (DESIGN.md:1531).
- Trees of levels: code sets `selected`, a row label from more than its own fields, the designer edits levels: NOT BUILT (DESIGN.md:1574; levels are edited as text).
- Splitting `parts/data/table.rs` into columns and cells, sorting, paging, editing, adding and removing, reordering and drag (MODULAR-PLAN step 4): NOT BUILT.

## Conflicts
- Decision needed: table order puts an empty value last (`runtimes/rust/src/tables.rs:3`); comparisons in code put none first on R, Go, C and C++. No rule says which is meant.
- C# runtime files `TablePages.cs`, `Trees.cs` have no callers outside tests (copies of rules Clarity.UI or the runtime already decides).

## Decisions
- 2026-10-22: table columns take a width in pixels (`columns title 240, artist`), `resizable` and `sortable` are options, `order by field [desc]` sets the first order; widths dragged by the person are kept per window and table.
- 2026-10-23: a pager is a table option (`page 50`), not a part.
- 2026-10-23: `tree` and `treetable` bind all the rows in the window, arranged by the one `ref Self` field; not loaded as expanded.
- 2026-10-30: reordering is an option of list, table, tree and treetable (`reorder by field`), set in the designer; a table follows the person's order or a sort, never both. User: "Lists, trees and tree tables ... are parts that can reorder!!", "I want all re-order able... but as option", "in design mode!"
- 2026-10-30: a move is an edit of the part; Ctrl+Z takes back the order and the saved rows together.
- 2026-10-31: a table's order and page, a tree's open rows and the shown rows are the runtime's (`view_event`); the screen sends the action.
- 2026-11-09: a tree may hold `level` lines (records of an entity under the row of the level around), with `on select Entity: handler` and `reload()`.
- 2026-11-13: `fold buttons` on a tree and a tree table: Expand all and Collapse all in a bar above its rows, open and close every row; a property like any other, in its line and window file (`foldButtons="true"`), the designer's "Expand and collapse all", and code reads and sets `tree.foldButtons` on every runtime (part state `foldButtons`; C# starts it from the design). Spec `WidgetProperty::FoldButtons`, `code/check.rs`; Clarity.UI `parts/data/table.rs` (`folds`, `fold_bar`, `fold_box`, `fold_all`); words `expandAll`, `collapseAll`; tests spec `fold_buttons.rs`, rust-ui `data_tree.rs` `fold_buttons_open_and_close_every_row`; the every-part fixture's tree and the maintenance ledger tree use it. User: "the app (or clarity in general) need collapse and expand buttons for lists and trees (as an option that can be set to these type of parts)", "for trees and lists!!!", "a part has properties and that describes is behaviour, lands in cxaml and code bindings!". A plain list has no rows under rows, so it has none.

## Was
- DESIGN 1.7 (table, tree and reorder rows of the table; the shown rows), 5.30, 5.33 (A tree of several kinds of records)
- Roadmap: U1c (pager, tree, tree table), U1e
