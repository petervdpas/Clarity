# Stock

Keeps count of what a shop has: each item, how many there are, and when
there are so few that more should be ordered. The same items can be kept
in a window and from the command line, against one SQLite database.
Deliveries on their way are kept in an XML file.

## What you see and can do

The Items window holds one view, ItemsView, with a table of the items: the
name, the count and the low mark. The table edits them in place. Add above
it makes a new item named "New item" (or "New item 2", and so on, when the
name is taken), and the trash on each row removes one after a question.

The Windows menu also opens Deliveries: a table of the deliveries kept in
`data/deliveries.xml` (the item, the number, the day it arrives and a
note), edited in place in the same way. Each change is written into the
file, which keeps everything else it holds.

The command-line program `stock` has these commands:

- `list` prints every item with its count and low mark; `list --low` prints only the items that run low.
- `add <name>` adds an item, with `--count` and `--low` (5 unless given). An item whose name exists is refused.
- `take <name> <number>` lowers the count, refusing to go below zero, and warns when the item runs low.
- `bring <name> <number>` raises the count.
- `export` prints every item as XML (`<stock><item>...</item></stock>`, indented).
- `import <text>` reads items from XML as `export` writes it: an item already there takes the count and low mark given, a new one is added.
- `receive` books every delivery onto its item's count and removes it from the deliveries file.
- `check` prints the items that run low and ends with exit code 3 when there are any, for use in scripts.

An item runs low when its count is at or below its low mark.

## Stack

Go, with two targets: desktop and CLI. "CLI" is the name of the
command-line target. An example builds only for the stack its project file
names.

## What it shows

- One project with a window and a command-line program that share the entity, the function `runsLow` and the database.
- A `cli` block with commands, arguments, options with defaults and limits, `print` of a table's columns, `refuse` and an exit code.
- A table that adds, edits and removes rows itself.
- A data source that is an XML file: `datasource deliveries xml "data/deliveries.xml" external` with `at "deliveries/delivery"`, its records the `<delivery>` elements. `item`, `number` and `arrives` are attributes (`column "@item"`, `column "@date"`), so a new delivery is written the same way, and `note` is a child element, found by its name. The file is made at the first save.
- XML in code: `xml of ... at "stock/item" indent true` writes the items, and `read list<Item> from xml text at "stock/item"` reads them back.

## Files

- `../stock.clarity`: the project file (title, stack and targets, output folder, the SQLite data source and the XML one).
- `stock.clarity`: the Item entity, `runsLow`, the code behind the window, and the command-line program with its commands.
- `deliveries.clarity`: the Delivery entity, kept in the XML file, and the code behind its window.
- `design/`: `Stock.cxaml` (the app and its menu), `Items.cxaml` and `ItemsView.cxaml` (the items), `Deliveries.cxaml` and `DeliveriesView.cxaml` (the deliveries).
- `translations/`: English and Dutch.
- `migrations/main/`: one step that adds the items.
