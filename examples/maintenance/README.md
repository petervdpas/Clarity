# Maintenance

An app to review development ledgers: a project's ledger is a folder with an index (`README.md`) and one Markdown file per piece. Clarity is one of the projects.

## What you see and can do

- **Projects** in the toolbar opens the table of projects: each has a name and the folder of its ledger, edited in place, with Add and Remove.
- The sidebar at the left lists the projects. Picking one points the file store `ledger` at its ledger folder (the setting `ledgerFolder`), and the app opens it again at the next start (`lastProject`).
- Below them, the ledger's entries as a tree, grouped as the ledger's index groups them, with each entry's progress (its `Progress:` line, else its status) and whether it is reviewed. Files the index does not list yet come last, under their own group.
- Picking an entry shows it on the right: its title, its status and its text as Markdown.
- The footer has three status items at its right. **Progress** says how much of the entry picked is implemented, from its `Progress: 60%` line, with a gauge. **Review** says whether it is reviewed, with a check mark, or a warning when the entry changed after its review; a click opens the review. The third checks the saved reviews against the ledger: a check mark when they match, else a warning that counts the reviews without an entry and those whose entry changed since.
- **Review** slides out the review of the entry picked: reviewed or not, by whom, on which day, notes (Markdown), the person's own references, and, to read, the entries it links to, its code and tests, and its old references (the entry's Was section).
- **Save entry** writes the entry back and says so in a toast at the bottom right that goes after 3 seconds.
- **Save** stores the review in the app's own database (entity `Review`, one per project and entry) and marks the entry in the tree. Nothing is written into the ledger.
- **Reload** reads the ledger again and checks the reviews against it.

## AI agents

The app offers its commands to AI agents: `project list`, `project add`, `review list`, `review show` and `review set`, each called by its words joined with hyphens (`review-set`). The program serves them itself when started as `maintenance mcp`, with the app's guidance (`agent/guidance.md`) as what agents are told first; there is no separate command-line program. While the app is open, each call goes to it: the person allows or refuses it, and the window shows the change with Undo. While the app is closed, **Preferences** decides: "AI agents may read while the app is closed" and "AI agents may change while the app is closed".

    $ claude mcp add maintenance -- /path/to/maintenance mcp

## Stack

Rust, desktop. An example builds only for the stack its project file names.

## What it shows

- A file store whose folder is a setting, set from code (`filestore ledger "setting:ledgerFolder"`).
- Reading text files of a store, and finding them by path and extension (`files f in ledger where ...`).
- A record of the window bound to a slide-out's parts (`review.reviewed`, a date part on `review.reviewedOn`), found or made new in code and stored with `save`.
- A tree table of records by key and parent (`rowKey`, `parentKey`), filled from code.
- A markdown part bound to a text of the window, and a memo in Markdown mode.
- A slide-out at the window's edge, opened from a toolbar button by code (`reviewPanel.visible = true`).
- Toasts the app declares (`toast entrySaved`, `toast reviewSaved`), each with its own lifespan, level, icon and corner, shown by name with their text's slots filled.
- Status items in the footer (`statusItem`): a caption set by code, a gauge of a number, a check mark (`checked`) and a warning (`error`).
- A file's `modified` time compared with the time a review was made (`reviewedAt`, added by migration 0003).
- A window's own toolbar and sidebar.
- Commands for AI agents (`agent`) in a desktop program without a command line, the guidance for agents, `maintenance mcp`, and the settings `agentReadsWhileClosed` and `agentChangesWhileClosed` in a Preferences dialog.
