# Novel

A workplace for writing books, from fragment to manuscript. Each book has
its chapters with their text, and around them research, characters,
fragments and dialogues tried out before they go into a chapter. A book goes
out as Markdown, and a screenplay also as Fountain.

## What you see and can do

The Bookcase window opens first: the books in a table on the left (title,
form, author), the chosen book on the right with its title, subtitle,
author, language, form and a note. The table adds and removes books; a new
book gets a free title (New book, New book 2, ...). The buttons below:

- Export writes the manuscript to `export/<title>/`.
- Cover opens the CoverDesk window to paint the book's cover.
- Save keeps the book; Open goes to its Desk.

The Desk window is an accordion of the five parts of the book, side by
side, each opening to its view:

- Chapters: the chapters in a list you reorder by dragging; the numbers follow the places, the first being chapter 0, a prologue. The text of the chosen chapter is a Markdown memo, and Save counts its words. A slide-out beside the list, pinnable, holds the chapter's properties (title, layers, status, summary) and tabs with the research, characters, fragments and dialogues linked to it.
- Research: a searchable list of notes with a link and a Markdown text, and the chapters each is used in.
- Characters: the characters as cards with name, role and description, and the chapters they appear in.
- Fragments: a searchable list of fragments with a heading, a kind (fact, invented, question), who brought it in, a link and a text, and the chapters they belong to.
- Dialogues: trial balloons as a chat or a screenplay. Each has its kind (dialogue or monologue), its voice (aloud or in thought), the people taking part and its lines in an order you drag. The chosen participant speaks the next line; with nobody chosen the line is a direction.

Each part jumps to the others: from a chapter to its research, characters,
fragments or dialogues, and from those back to a chapter. Something still
used in a chapter is only removed once its links are gone.

In the chapter text, Ctrl+Space or a right-click calls up a toolbox at the
caret that puts in research, a fragment or a dialogue, with a link back to
it. A dialogue goes in as prose: spoken lines between quotes, thoughts in
italics. Following such a link in the preview goes to that item.

The CoverDesk window is a paint canvas of 600 by 900 with zoom and a popup
toolbox: select, lasso, pen, pencil, spray, eraser, fill, colour picker,
shapes, rotate, flip, crop, size, opacity, colours, import and export.

The app's menu holds Language (English, Dutch, German), Go (Back with
Alt+Left, Forward with Alt+Right) and Theme (Paper, Night, Sepia, Ink). The
language and the theme are settings each person keeps on their machine.

## Stack

C++, desktop. An example builds only for the stack its project file names.

## What it shows

- A large data model in one module: books, time layers, chapters, manuscripts with versions and cuttings, writing sessions, fragments, trial balloons with lines, characters, appearances, research, documents and events, each tied to its book with `cascade`.
- An action that writes files: `exportManuscript` writes a Markdown file per chapter and the whole book to the `export` file store, and for a screenplay a Fountain file with the dialogues as lines (thoughts as V.O.). `export/The Lighthouse Keeper/` is such an export.
- Views that talk to their window: the parts declare `Tells` messages (`chosen`, `jumpTo`, `toResearch` ...) and the Desk wires them to its handlers.
- Toolboxes designed as pages: `ManuscriptTools` at the caret of a memo, `CoverTools` as a popup on a paint canvas.
- Four themes of its own and full-text search on fragments and research.

## Files

- `../novel.clarity`: the project file (stack, output folder, the SQLite data source, the `export` file store, the themes).
- `books.clarity`: the enums, the entities and the `exportManuscript` action.
- `writing.clarity`: the language and theme settings and the code behind every window and part.
- `themes.clarity`: the four themes Paper, Night, Sepia and Ink.
- `design/`:
  - `Novel.cxaml`: the app with its menu and footer.
  - `Bookcase.cxaml`: the books and the chosen book.
  - `Desk.cxaml`: a book's desk with its five parts.
  - `ChaptersPart.cxaml`, `ResearchPart.cxaml`, `CharactersPart.cxaml`, `FragmentsPart.cxaml`, `DialoguesPart.cxaml`: the parts of the desk.
  - `ManuscriptTools.cxaml`: the toolbox of the chapter text.
  - `CoverDesk.cxaml`, `CoverTools.cxaml`: the cover window with its canvas, and its toolbox.
- `translations/`: English, Dutch and German.
- `migrations/main/`: three steps. The first makes the tables, the second adds the book's cover, the third the kind and voice of a dialogue.
- `export/`: an exported book.
