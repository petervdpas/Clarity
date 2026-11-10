# Dashboard

A one-window dashboard over sales and stock. The window is split into six
squares, each a content showing a view of its own.

## What you see and can do

The Board window opens with two rows of three squares:

- Sales per product: a bar chart of the sales, summed by product.
- Sales: a table of every sale (sold, product, amount), the newest first.
- Stock: a table of the products and how many are in stock.
- Total: a display of what all sales bring in.
- Notes: a memo for what to remember today.
- Running low: a list of the products with fewer than ten in stock.

When the window opens, its `boardOpened` handler reads the sales and the
products from the database, picks out the low stock and adds up the total.
Each view receives the list or value it shows.

## Stack

Rust, desktop. An example builds only for the stack its project file names.

## What it shows

- One window built from several contents, each holding a different view.
- The same data shown two ways: the sales as a chart and as a table.
- Queries with `find ... where ... order by` and a total worked out in a loop.

## Files

- `../dashboard.clarity`: the project file (title, stack, language, the SQLite data source).
- `dashboard.clarity`: the Sale and Product entities and the Board's code.
- `design/`: `Dashboard.cxaml` (the app), `Board.cxaml` (the window with its six squares), and one `.cxaml` per view: `SalesChart`, `SalesTable`, `StockList`, `TotalView`, `NotesView`, `LowStock`.
- `translations/`: English.
- `migrations/main/`: the step that adds the Sale and Product tables.
