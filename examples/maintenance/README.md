# Maintenance

An app to review a project's dossiers. A project holds dossiers; each dossier has a category (Manual and Potentials to start with, more can be added) and one review, which holds review items, each green when done and red when not. How much of a dossier is done is its green items out of all of them. Reviewing is adding, removing and marking items. Each category has a folder and a color: a dossier can have its file in its category's folder, such as a page of Clarity's manual, and its card shows its category's color.

## What you see and can do

- The **Maintenance** menu holds Projects, Categories, Preferences, Dossiers from the folder and Quit; beside it, on the same row, the toolbar has Reload, Add dossier and Save dossier.
- **Projects** opens the table of projects, by name, edited in place, with Add and Remove.
- **Categories** opens the table of categories: name and color edited in place, and below it the folder of the category picked, chosen with the folder part.
- The sidebar at the left lists the projects. Picking one shows its dossiers, and the app opens it again at the next start (`lastProject`).
- Below them, the project's dossiers as cards: each its name, its category on the second line as a chip in the category's color, and how much of it is done on the third. Picking one shows its file and its review.
- **Dossiers from the folder** asks for a category and makes a dossier of every Markdown file in its folder that is no dossier yet, named by its first heading. **Add dossier** asks for a name and a category.
- Picking a dossier shows its file on the right, to read and edit; **Save dossier** writes it back into its category's folder.
- **Review** slides out the review of the dossier picked: its category (pick another to move the dossier, say from Manual to Potentials), reviewed or not, by whom, on which day, how much is done (a bar, counted from the items) and notes. **Save** stores the dossier and its review in the app's own database.
- The review slide-out also holds the review's items, as a table edited in place: what each is, whether it is done (a switch, green) or not (red), and its kind: missing in the dossier for this category, new in the dossier and category, differing from its origin, or wrong in this dossier and category (to delete or move to another dossier, by hand). **Add** above the table adds one, red until it is done; the trash button removes one, after asking. Every change counts the review's percentage again.
- The footer has three status items at its right: how much of the dossier picked is done, with a gauge; whether it is reviewed, or a warning when its file changed after the review; and whether the dossiers' files match their folders.
- **Reload** reads the folders again.

## AI agents

The app offers its commands to AI agents: `project list`, `project add`, `dossier list`, `dossier add`, `review show`, `review set`, `item list`, `item add`, `item set` and `item remove`, each called by its words joined with hyphens (`item-add`). The program serves them itself when started as `maintenance mcp`, with the app's guidance (`agent/guidance.md`) as what agents are told first; there is no separate command-line program. While the app is open, each call goes to it: the person allows or refuses it, and the window shows the change with Undo. While the app is closed, **Preferences** decides: "AI agents may read while the app is closed" and "AI agents may change while the app is closed".

    $ claude mcp add maintenance -- /path/to/maintenance mcp

## Stack

Rust, desktop. An example builds only for the stack its project file names.

## Data

The data model is drawn in `diagrams/reviews.mermaid` (ERD) and `diagrams/reviewClasses.mermaid` (class diagram).

## What it shows

- A folder held by a record, which is no store (`folder: folder, extensions "*.md"` on Category), chosen with a folder part, its files listed, read and written by code (`files f in folder d.category.folder where ...`, `write ... to folder ...`).
- A one-to-one relation kept by a unique ref (`Review.dossier`), and records of the window bound to slide-outs' parts, found or made new in code and stored with `save`.
- A list of cards (`cards`, `show name, category, done`) filled from code with records of a record type, each colored by its record (`colored by categoryColor chip category`).
- Asking the person for a text and then for a record of a list (`ask ... for text`, `ask ... for Category from categories`), and a yes or no before removing.
- A memo in Markdown mode bound to a text of the window.
- Slide-outs at the window's edge, opened by code (`reviewPanel.visible = true`).
- Toasts the app declares, each with its own lifespan, level, icon and corner, shown by name with their text's slots filled.
- Status items in the footer (`statusItem`): a caption set by code, a gauge of a number, a check mark (`checked`) and a warning (`error`).
- A file's `modified` time compared with the time a review was made (`reviewedAt`).
- A window's own menu bar with a width and its toolbar beside it on one row, a sidebar, and a second window with a table edited in place.
- Commands for AI agents (`agent`) in a desktop program without a command line, the guidance for agents, `maintenance mcp`, and the settings `agentReadsWhileClosed` and `agentChangesWhileClosed` in a Preferences dialog.
