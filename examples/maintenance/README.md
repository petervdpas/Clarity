# Maintenance

An app to review development ledgers: a project's ledger is a folder with an index (`README.md`) and one Markdown file per piece. Clarity is one of the projects.

## What you see and can do

- **Projects** in the toolbar opens the table of projects: each has a name and the folder of its ledger, edited in place, with Add and Remove.
- The sidebar at the left lists the projects. Picking one points the file store `ledger` at its ledger folder (the setting `ledgerFolder`), and the app opens it again at the next start (`lastProject`).
- Below them, the ledger's entries as a tree, grouped as the ledger's index groups them, with each entry's status and whether it is reviewed. Files the index does not list yet come last, under their own group.
- Picking an entry shows it on the right: its title, its status and its text as Markdown.
- **Review** slides out the review of the entry picked: reviewed or not, by whom, on which day, notes (Markdown), the person's own references, and, to read, the entries it links to, its code and tests, and its old references (the entry's Was section).
- **Save** writes the review to `reviews/<piece>.md` in the ledger, a file anyone (or any agent) can read and write too, and marks the entry in the tree.
- **Reload** reads the ledger again.

## Stack

Rust, desktop. An example builds only for the stack its project file names.

## What it shows

- A file store whose folder is a setting, set from code (`filestore ledger "setting:ledgerFolder"`).
- Reading and writing text files of a store, and finding them by path and extension (`files f in ledger where ...`).
- A tree table of records by key and parent (`rowKey`, `parentKey`), filled from code.
- A markdown part bound to a text of the window, and a memo in Markdown mode.
- A slide-out at the window's edge, opened from a toolbar button by code (`reviewPanel.visible = true`).
- A window's own toolbar and sidebar.
