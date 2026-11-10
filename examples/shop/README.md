# Shop

A small shop administration: customers, their orders with order lines, and
a dashboard of how sales go. The data is kept in a SQLite database. An
engine prices each order from its lines, and data events guard the
customers' credit.

## What you see and can do

The app opens on the Customer card. Its menu has File (Customer card with
`Ctrl+K`, Language, Theme, Quit with `Ctrl+Q`), Customers (All customers),
Orders (All orders, and the Sales dashboard, which opens in a window of
its own) and Suppliers (All suppliers). The toolbar opens the same windows,
and the footer shows the status.

- Customer card: a customer's name, kind, active box and credit limit, the shop logo, and tabs with their orders and their notes.
- Customers: a search box and a table of customers. Every word typed is matched at the start of a word of the name, e-mail, notes or tags. The table adds and removes customers; opening one (a double click or Enter) slides out the customer editor from the right.
- Customer editor: three tabs. General holds the name, kind, e-mail, customer since, active and tags. Finance holds the credit limit, the VAT number and the contract file. Notes holds the notes. Only business customers can fill in a VAT number, and Save (`Ctrl+S`) shows problems at the input they are about.
- Orders: a filter on the order number and a table of orders, newest first. Opening one slides out the order editor.
- Order editor: the number, date, customer (a dropdown with search) and status, a table of order lines that is edited in place, the total, remarks, **Approve** and **Save**. The total is worked out again whenever a line changes.
- Suppliers: a filter on the name and a table of suppliers with their e-mail. **New**, **Open** or a double click opens the supplier editor.
- Supplier editor: a supplier's name, e-mail, phone number and website, with **Save** and **Close**. A supplier needs a name.
- Sales dashboard: three charts of the orders that are not cancelled: orders by status, sales per day (a line) and sales per customer (a pie).

The rules on the data:

- A private customer cannot have a credit limit above 1000.
- An order number looks like `ORD-12345678`.
- An order's total is always what its stored lines cost.
- An order can be approved only while it is pending and within its customer's credit limit; otherwise **Approve** shows a warning.
- Only pending orders can be deleted, and their lines go with them.

## Stack

Rust, desktop. An example builds only for the stack its project file names.

## What it shows

- An engine: `engine Pricing from "pricing.wasm"` declares `total`, which the Clarity code calls like its own function. Its source is the Rust crate in `engines/pricing`, built for `wasm32-unknown-unknown` into `engines/pricing.wasm`. A line costs its quantity times its price, with 5% off from 10 pieces and 10% off from 50; an order above 1000 gets 2% off the whole.
- Data events (`before save`, `before delete`) that run whenever a customer or order is saved or deleted, from any window.
- An action, `approve`, with its requirements and its effect on the database.
- Views that tell their window something changed (`Tells message="saved"`), so its table shows it at once.
- Full-text search on customers, money in euros (`currency EUR`), a file field and a list of tags.
- Three themes: Shop (follows the system, green accent), Night (always dark, amber accent) and Day (always light, blue accent, serif headings, its own look per part).
- Texts with slots, such as the credit note of an order, translated wherever the code makes them.

## Files

- `../shop.clarity`: the project file (stack, language, currency, the three themes, the SQLite data source).
- `shop.clarity`: the themes.
- `customers.clarity`: the Customer entity and its kinds, the code behind the customer windows and editor, and the customer's data event.
- `sales.clarity`: the Order and OrderLine entities and the order statuses, the Pricing engine, the credit check, the `approve` action, the code behind the order windows, editor and dashboard, and the order's data events.
- `suppliers.clarity`: the Supplier entity (name, e-mail, phone and website) and the code behind its browse window and editor.
- `counter.clarity`: the module of the Customer card window; it holds no code of its own.
- `design/`: `Shop.cxaml` (the app with its menu, toolbar and footer), `CustomerCard.cxaml` and `CustomerCardView.cxaml` (the card), `CustomerBrowse.cxaml` and `CustomerBrowseView.cxaml` (the customer table with its slide-out), `CustomerEditor.cxaml` (the customer form), `OrderBrowse.cxaml` and `OrderBrowseView.cxaml` (the order table with its slide-out), `OrderEditor.cxaml` (an order and its lines), `SalesDashboard.cxaml` and `SalesDashboardView.cxaml` (the charts), `SupplierBrowse.cxaml` and `SupplierBrowseView.cxaml` (the supplier table), `SupplierEditor.cxaml` and `SupplierEditorView.cxaml` (a supplier).
- `engines/pricing/`: the engine's Rust source; `engines/pricing.wasm` is the built engine the app loads.
- `resources/logo.png`: the logo on the Customer card.
- `translations/`: English and Dutch.
- `migrations/main/`: eight steps: customers and orders, order lines, tags and contracts, search on customers, names in camelCase, suppliers, supplier phone numbers and supplier website.
- `diagrams/`: `shopTables.mermaid`, the shop's ERD (its entities, fields and relations), and `shopClasses.mermaid`, a class diagram of its modules with their entities, enums, the pricing engine and the modules' functions.
