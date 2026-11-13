# Maintenance for agents

## What the app is for

Maintenance reviews development ledgers. A project's ledger is a folder with an index (README.md in it) and one Markdown file per piece; Clarity is one of the projects. The app keeps its own review of each entry in its database and never writes into a ledger.

## The app's words

- A project (`Project`) is a ledger to review: its name (`Project.name`) and the folder of its ledger (`Project.ledgerFolder`).
- An entry, or piece, is one Markdown file of a ledger, named by its file name without .md (app-mcp for app-mcp.md).
- A review (`Review`) is the app's word on one entry of one project: whether it is reviewed (`Review.reviewed`), who reviewed it (`Review.reviewer`, one of `Reviewer`: `Peter` or `Claude`), on which day (`Review.reviewedOn`), and notes in Markdown (`Review.notes`).

## How work flows

1. `project-list` gives the projects and their ledger folders; `project-add` adds one.
2. Read the entries of a ledger as files in its folder; the app does not serve them.
3. `review-show` gives the review an entry has, if any; `review-list` the entries reviewed so far, and with --all the ones with a review that is not done.
4. `review-set` keeps a review: give --reviewed only when the entry is reviewed, and write the notes as Markdown.

## Rules the app does not check

- An agent reviews as `Claude`, the default of `review-set`; never as `Peter`.
- Mark an entry reviewed only after reading its whole file and the entries it links to.
- Leave a review by `Peter` as it is unless the person asks to change it.
