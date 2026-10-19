# Data sources

Where your records live: SQLite and the server databases, keeping passwords out of the project, several sources, and external databases that Clarity reads without owning.

## Declaring a source

A data source is a `datasource` line in the project file: a name, a kind and where the data is. A new project gets one SQLite source named `main`.

```clarity
datasource main sqlite "data/shop.db"
datasource sales postgres "env:SHOP_DATABASE"
```

You can also add, change, reorder and remove sources in the **Data sources** section of **Settings** on the Project tab. For SQLite, **Browse...** picks the file.

## Kinds

- `sqlite`: a file, relative to the project's folder, for Clarity and for the generated application alike.
- `postgres`: a connection string or URL, such as `postgresql://shop@db.example.com/shop`.
- `mysql`: a URL, such as `mysql://shop@db.example.com:3306/shop`.
- `mssql`: an ADO.NET connection string, such as `server=tcp:db.example.com,1433;database=shop;user=shop`.

The same entities and steps work on all four; the program writes each step in that database's own SQL. The C and C++ stacks support SQLite only for now. A `rest` kind for web APIs is accepted in the project file, but entities cannot live in one yet.

## Passwords stay out of the project

A connection that needs a password is written as `"env:NAME"`. The generated application reads the whole connection from that environment variable when it starts. The checker refuses a connection that contains a password itself. A SQLite source is always a file path.

## Several sources

Entities live in the first data source unless their module names another with a `source` line. A module has at most one, and it must name a SQL source.

```clarity
module erp

use customers
source erp

entity Invoice
    number: text(20), unique
    customer: ref Customer
```

A reference to an entity in another database is stored as its id, without a foreign key. A query reads one database, so it cannot go through such a reference. Each owned source has its own [migrations](data.migrations.md).

## External databases

A database that already exists, and that Clarity does not own, is marked `external`. Add `readonly` when its records may only be read; `save` and `delete` of them are then errors. In Settings these are the **External** and **Read-only** boxes under **Access**.

```clarity
datasource erp sqlite "legacy/erp.db" external
```

Its entities map onto the tables and columns that are there:

```clarity
entity Invoice
    table "tblInvoice"
    key "InvoiceID"
    inv_no: text(20), required, unique, column "InvNo"
    amount: decimal(18, 6), column "Amount"
```

Without `table`, the table is the entity's name in snake case; without `key`, the key column is `id`, an integer; without `column`, a field's column has its name. Clarity reads and writes the records but never changes the tables, writes no steps for them and never migrates them. Each time the project loads, Clarity compares the entities with the tables: a missing table or column is an error, and a column whose type does not fit is a warning.

## Import

**Import...** next to an external SQLite source opens the Import panel. It lists the tables; tick the ones you want, name the module and click the **Import** button below them. The tables become entities mapped onto them, in a new module. Foreign keys between imported tables become `ref` fields, `NOT NULL` becomes `required`, and binary columns are left out. From the command line:

```shell
clarity import shop.clarity erp --tables tblInvoice,tblCustomers
```

`--list` shows the tables, and `--module` names the module (the source's name by default).

## Taking ownership

**Own...** next to an external SQLite source, or `clarity own shop.clarity erp`, makes Clarity the owner. It writes the source's first step, a baseline that describes the tables as they are and changes nothing in that database, and removes `external`. From then on your entities drive the tables through ordinary steps, and the `table`, `key` and `column` names stay. Undo takes it back.

Import, the table check and Own work for SQLite only for now. An external PostgreSQL, MySQL or SQL Server database can be mapped by hand.
