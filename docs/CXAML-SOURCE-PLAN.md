# .cxaml as the source of windows and views

Decided 2026-11-09. Until now `.cxaml` was only generated output (DESIGN 7.8).

## What the user said (2026-11-09, verbatim)

- "should the designer work with clarity code or .cxaml?! I think .cxaml!!"
- "1, cxaml is the source, make it!!! and the .cxaml is used to derive the clarity code!!!!"
- "I thought we already designer it this way... avalonia was the inspiration and that also turn axaml into code!"
- "I remember telling you that the cxaml for every stack and every os should, could and would be the same!!!"

Choices picked by the user from the options offered:

- Files: one `.cxaml` per window and per view.
- Split: the whole page is in the `.cxaml` (title, size, data, parts, which
  handler each event calls); the `.clarity` file keeps the app, entities,
  records, enums, themes and the code blocks (handlers).
- Module (asked because a folder often holds several modules): a folder per
  module, named after the module. Then, seeing `examples/calculator/calculator/`:
  "why all the double folders in the examples?", "or design/<module>/ ?",
  "why not straight under design???", and for how a file names its module:
  "Do it the avalonia way!". So all files sit in `design/` and the root names
  module and page as `x:Class` does: `novel/design/Desk.cxaml` with
  `<Window class="desk.Desk">`.
- Migration: a convert command writes the `.cxaml` files from the page blocks
  and removes the blocks; run on every example, each built on its own stack.

Already in DESIGN 7.10: every Clarity.UI version (Rust, Go, C, C++, C#) reads
the same window and app markup.

## Open: to confirm with the user before building

- The source `.cxaml` is the markup Clarity.UI reads, unchanged, on every stack
  and OS; the apps carry the source files as they are.
- What the source adds to today's markup (title, size, data, `on open`, a
  content naming its views) becomes part of that one format and of every
  Clarity.UI reader.
- The order of the slices: format and reader, loader, convert command,
  designer edits, IDE (new window and view, the Design text, agent, manual).

## Status (2026-11-09)

Built:

1. Format and reader: `clarity_spec::cxaml` (`page_source`, `read_page`) and
   `clarity_spec::xml`; every page of every example reads back unchanged.
2. Loader: each module folder's `.cxaml` files load as files of that module;
   the checks see a module as its `.clarity` file and its folder; the
   generators get one file per module (`folded`).
3. Convert command: `clarity convert-pages <project>`; every example is
   converted and checks clean. The dice example was removed (user,
   2026-11-09).
4. Designer edits on `.cxaml` (through the page's text of Clarity), code and
   module edits to the module's file, New window writing `.cxaml`, rename,
   and a view's parts edited in place inside a window's content.

Open:

- Done 2026-11-09, "yes, make Clarity.UI read the source cxaml", then picked
  "One format, compiled": the app's windows are the source dialect with what
  Clarity works out filled in, as Avalonia compiles `.axaml`; Clarity.UI reads
  that dialect (class, `<View>` pages, `<Data>`, `<Tells>`, `<Level>` passed over).
- Done 2026-11-09 ("rename the two desk modules and then do design text colors
  and leftover page blocks finally"): novel's `desk` module is `writing`,
  shop's is `counter`; the Design text of a `.cxaml` is colored as markup; a
  page block left in a module's file is an error with the fix Move pages into
  design, under Problems; Make texts keys reaches the `.cxaml` files.

## Next: no translator between design and code (proposed 2026-11-09, not built)

What the user said (verbatim), after hearing that a page is edited through a
temporary block of Clarity with positions translated back and forth, and
designer items carry a file number packed into their bits:

- "how would you improve it? dont make it, tell it"
- "would it improve (more stabile) the relation between clarity code and the views (design)"
- "you mean remove the translator between them... that might enable debugging design too?"
- "yes, add those to the debugger plan... but that are consequences of the change"

The change as proposed (to confirm with the user before building):

1. A part is known by its file and name everywhere (`design/Gallery.cxaml#saveButton`):
   the designer item, the code and the running app use the same key. This
   replaces `(file + 1) << 24 | offset`. A part without a name falls back to
   its element path, or `.cxaml` requires a name.
2. A designer edit changes the page's model ("move saveButton to 3,2") and the
   writer writes the file; no temporary block, no position mapping. The
   writer already writes every example back byte for byte.
3. `<Data>`, `<Tells>` and handler attributes are read by Clarity's type and
   name parsers on the attribute value itself, at its place in the file,
   instead of through a synthetic `module cxaml`.
4. Page blocks leave the language: one way to write a window.

The round-trip tests guard every step. Debugging the design follows from
step 1; see DEBUGGER-PLAN.md, "Debugging the design".

### Step 1 as built (2026-11-09)

- `clarity_spec::design_keys` gives every thing of a window, view or app
  its key: the file from the project's folder, `#`, then a path from the
  page's or app's name. A named thing is `Main/saveButton` wherever it
  stands (moving it keeps its key); one without a name is its element and
  number under what holds it (`Main/mainTabs/Tab[2]`); data and tells go
  by name in brackets (`Main/Data[orders]`); a footer's status is
  `Main/Footer[1]/Status`. Unnamed things keep a path rather than `.cxaml`
  requiring a name: every part of the examples already has one, only tabs,
  panes, levels, selects, sections and tells do not.
- The designer's markup writes keys as `designItem`; `designShown` and the
  preview's `pages=` go by key. `Preview.designKeys` gives the IDE each
  file's keys by place.
- The IDE picks, drags and drops by key; `placeOf(key)` gives the file,
  page and place an edit is made in, so the view's file changes when a
  view's part is edited in a window. `DESIGN_PLACE_BITS`, `design_item`,
  `designItem()` and `routedEdit` are gone.
- Edits still name places in a file; step 2 makes them name the model.

### Step 2: edits on the model (started 2026-11-09)

Asked whether the app (its frame of menus, toolbars, sidebars and footer,
written in the module's file) stays text or moves beside the windows, the
user chose: "App to design/ too (Recommended)", the option that read: "The
app gets its own design/App.cxaml (<App class="todo.Todo">) like windows.
Every designed thing is then edited on the model; the text engine keeps
only entities, enums and code. One frame editor."

- The app moves to `design/<App>.cxaml` with `<App class="module.App">`:
  read, written, loaded and converted as windows are; an app block left in
  a module is an error, as a page block is.
- `clarity_spec::design_edit` makes each designer edit on the page's or
  app's model; `cxaml::apply` reads the file, edits the model and writes
  it. No temporary block, no places matched up. The focus comes back as
  the edited thing's key.
- Edits still name the thing by where it starts in the file it was read
  from; the model read from the file has those places, so nothing is
  translated.
- Built: the app's file (reader, writer, loader, converter, every example
  and fixture converted, Problems' fix, New app), `design_edit` and the new
  `cxaml::apply`, the IDE splitting a wiring between the `.cxaml` and the
  module. Tests: clarity-spec `design_edit` compares 7764 edits on every
  example's windows, views and apps with the old path through the text, and
  the same with the project (keys made, fields placed); they match.
- Kept until step 4 (page blocks leave the language): the old path,
  `apply_through_block`, as the comparison's reference; and the keys an edit
  makes are still found through the page's block (`key_module`), as Make
  texts keys finds them.
### Step 3: page blocks leave the language (decided 2026-11-09)

Asked how far page blocks go (433 of them in 172 test files, and the
converter reads them to migrate old projects), the user chose: "Only the
converter reads (Recommended)", the option that read: "A page block is an
error everywhere, in-memory sources too; only the converter still reads
one, so old projects migrate. Tests load their pages through a helper that
converts them to .cxaml first (one call each), and new tests write .cxaml.
The internal translator (keys, rename, edits' old path) goes."

Built (2026-11-09):

- A page or app block left in a module is an error in every load, in-memory
  sources too. `clarity_spec::converted` and `load_converted` move old-form
  blocks into `.cxaml` in memory for tests (a module with an error inside a
  page block stays as written); `clarity convert-pages` and Move pages into
  design are the only readers of page blocks for people's projects.
- The reader reads `<Data>`, `<Tells>`, root events, `gives` and handler
  arguments in place by Clarity's rules (`parser::data_value` and friends);
  no synthetic `module cxaml`, errors point at the attribute's value.
- Texts become keys on the model (`keyed::make_keys_designed`,
  `key_designed`), renames work on the model (`cxaml::read_designed`,
  `written_again`); a file read with an error is refused, not written back.
- Gone: `apply_through_block`, `make_keys_with_pages_through_blocks`,
  `page_block`, `app_block`, `rewritten`, `block_page`, `places`,
  `module_text_with_pages`, `edit::with_items`. The comparisons became tests
  of the new paths (every edit on every example reads back clean; every
  example's texts become keys again).
- Bugs the move exposed, fixed: the window-handler clash check and
  `code::edit::set_code` looked for a window in its module's file.
- Cleanup after step 3 (user: "do the cleanup first", 2026-11-09), built:
  templates build the window and view as models (`templates::new_window`,
  `new_page`, `new_app`); New window, New view, the splash, the toolbox and
  New app write their `.cxaml` files at once (`cxaml::added`, the IDE's
  apply_edit), and their code goes into the module; deleting a window
  deletes its file and its code block (`cxaml::removed`, one undo step); a
  new project is written from the same models. The text engine (`edit.rs`)
  keeps only a module's own edits (entities, fields, enums, variants, uses,
  code); a window's edit on a module's text says `IN_DESIGN_FILE`. Its tests
  of window edits moved to `cxaml::apply` on markup; the old-form fixtures
  (tests/fixtures/old-pages) are read only by tests of the parser and the
  converter. `trees.rs` builds a tree's rows data itself.
- Open: the writer's `none="surface"` (and `orientation`, `measure`) comes
  from comparing with a new part's props rather than with what an empty
  attribute reads as; to look at after this step.
