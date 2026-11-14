# Maintenance for agents

## What the app is for

Maintenance keeps reviews of a project's dossiers. A dossier is one subject under review in a category (Manual, Potentials, or one the person adds), often with its file in the category's folder, such as a page of Clarity's manual. Each dossier has one review, which says how much of it is done and holds review items. Reviewing is adding, removing and marking items. Reviews and items are kept in the app's own database; only a dossier's own text is saved into its file.

## The app's words

- A project (`Project`) is what is reviewed, by its name (`Project.name`).
- A category (`Category`) is what a dossier is, by its name (`Category.name`), with the folder its dossiers' files are in (`Category.folder`, Markdown files) and the color its dossiers' cards show (`Category.color`).
- A dossier (`Dossier`) has a name (`Dossier.name`), a category (`Dossier.category`) and, when it has a file, that file's name in its category's folder (`Dossier.inFolder`, like parts.input.md).
- A review (`Review`) belongs to one dossier: whether it is reviewed (`Review.reviewed`), by whom (`Review.reviewer`, one of `Reviewer`: `Peter` or `Claude`), on which day (`Review.reviewedOn`) and notes in Markdown (`Review.notes`). How much is done (`Review.percentDone`) is counted from its items: the green ones out of all of them, 0 while it has none.
- A review item (`ReviewItem`) says what it is (`ReviewItem.summary`), its kind (`ReviewItem.kind`, one of `ItemKind`), notes (`ReviewItem.notes`), and whether it is done (`ReviewItem.done`): green when done, red when not. The kinds: `Missing`, missing in the dossier for this category; `New`, new in the dossier and category; `Differs`, a piece (the item) differs from its origin; `Wrong`, the item is wrong in this dossier and category and might be deleted or moved to another dossier.

## How work flows

1. `project-list` gives the projects; `project-add` adds one.
2. `dossier-list` gives a project's dossiers with their files; `dossier-add` adds one under a category, with its review.
3. Read a dossier's file in its category's folder; the app does not serve it.
4. `review-show` gives a dossier's review with its items; `review-set` keeps whether it is reviewed and the notes.
5. `item-list`, `item-add` and `item-remove` keep the items of a dossier's review; `item-set` marks one green (with its done option) or red. Each of them counts the review's percentage again.

## Rules the app does not check

- An agent reviews as `Claude`, the default of `review-set`; never as `Peter`.
- Give every item its kind. An item of kind `Wrong` is only marked: deleting it or moving it to another dossier is the person's, after looking at it.
- Leave a review by `Peter` and its items as they are unless the person asks to change them.
