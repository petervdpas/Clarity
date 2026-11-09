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
