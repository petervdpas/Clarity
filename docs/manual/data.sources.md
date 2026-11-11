# Data sources

Where your records live: SQLite and the server databases, keeping passwords out of the project, several sources, and external databases that Clarity reads without owning.

## Declaring a source

A data source is a `datasource` line in the project file: a name, a kind and where the data is. A new project gets one SQLite source named `main`.

```clarity
datasource main sqlite "data/shop.db"
datasource sales postgres "env:SHOP_DATABASE"
```

You can also add, change, reorder and remove sources in the **Data sources** section of **Settings** on the Project tab. For SQLite, **Browse...** picks the file in Clarity's file browser, or names a new one there.

## Kinds

- `sqlite`: a file, relative to the project's folder, for Clarity and for the generated application alike.
- `postgres`: a connection string or URL, such as `postgresql://shop@db.example.com/shop`.
- `mysql`: a URL, such as `mysql://shop@db.example.com:3306/shop`.
- `mssql`: an ADO.NET connection string, such as `server=tcp:db.example.com,1433;database=shop;user=shop`.

The same entities and steps work on all four; the program writes each step in that database's own SQL. The C and C++ stacks support SQLite only for now. A `rest` source is a web API: its entities live in the service, and its module's actions read and send them; see [Calling web APIs](code.web-apis.md). A source can also be a file in a format, `csv`, `json`, `xml` or `yaml`; see [A file as a data source](#a-file-as-a-data-source).

## A file as a data source

A CSV, JSON or XML file can hold an entity's records, so a table part over it is a CSV editor with no code:

```clarity
datasource logins csv "imports/logins.csv" external readonly
    separator ";"
```

- The kind is the format's name. The path follows SQLite's rules: relative to the project's folder, `"env:NAME"` or `"setting:name"`.
- A file source is always `external`: Clarity reads and writes its records but never changes its shape. Add `readonly` when it may only be read.
- The format's options are indented lines under it. CSV: `separator ";"` (any one character, `"\t"` for a tab), `quote "'"`, `noHeader true` (the file has no header row; its columns are the entity's fields in order) and `encoding "windows-1252"` (or `"iso-8859-1"`; UTF-8 by default). JSON: `at "data.logins"`, where the list of records is inside the file. XML: `at "shop/orders/order"`, `namespace`, `indent true` and `encoding`; see [An XML file](#an-xml-file).
- The file holds one entity's records. Its fields map onto the file's columns as on an external database's: by name, or with `column "Login URL"`. A column the entity wants that the file lacks stops the program when it opens, with a message that names it.
- `find`, `save` and `delete` work as on a database. The program reads the whole file when it starts and writes it again after each change: to a new file that then replaces the old one, so a crash never leaves half a file. The file's own column order and the columns no field maps are kept.
- When the file has no `id` column, a record's id is its row's place, which is never written into the file.
- In Settings, pick **CSV file**, **JSON file**, **XML file** or **YAML file** as the kind; the format's options show under the location.
- A file can instead be laid over its entity by a mapping of the project, which says the format, where the records and fields are, and the options: `datasource feed file "imports/orders.xml" external using OrderFeed`. The kind is then `file`, and the entity is the mapping's (its module says `source feed`); see [Mappings](code.formats.md#mappings).

### An XML file

```clarity
datasource feed xml "imports/orders.xml" external
    at "shop/orders/order"
```

- `at` names the record's element by its path, element names with slashes between: every `<order>` inside `<shop><orders>` is one record.
- A field takes a child element of its name, else an attribute of its name. Where its value is somewhere else, `column` says where, relative to the record's element: `column "@number"` is the attribute only, `column "customer/name"` the text of an element inside, `column "."` the record element's own text.
- Without a mapping the source is one flat table: an element inside a record that holds a record or a list is kept with the record, unmapped. A mapping with records inside (`lines of Line at "lines/line"` with `order is ".."`) makes the file hold both entities: `Line` lives in the same source (its module says `source feed` too), each line refers to the order it sits in by `order`, and a line saved or deleted is written back inside its order.
- Saving keeps the whole document: everything outside the records stays as it was (the declaration, comments, other elements), and so do the attributes and elements inside a record that no field maps, in their place. A new record gets an element with its mapped fields; a deleted record's element goes, with what it held.

## Passwords stay out of the project

A connection that needs a password is written as `"env:NAME"`, or as `"setting:name"` for a [secret setting](../LANGUAGE.md#secret-settings) of the person. The generated application reads the whole connection from that environment variable, or from the system's protected store, when it starts; while a secret setting is not kept yet, it shows the window where it is entered first, saying why, and connects once it is kept. The checker refuses a connection that contains a password itself. A SQLite source is always a file path.

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
    invNo: text(20), required, unique, column "InvNo"
    amount: decimal(18, 6), column "Amount"
```

Without `table`, the table is the entity's name in snake case; without `key`, the key column is `id`, an integer; without `column`, a field's column has its name. Clarity reads and writes the records but never changes the tables, writes no steps for them and never migrates them. Each time the project loads, Clarity compares the entities with the tables: a missing table or column is an error, and a column whose type does not fit is a warning.

A save never overwrites someone else's change when the table has a version column, a whole number the database keeps as Clarity keeps its own `row_version`. Name it on the entity, as you name the key:

```clarity
entity Invoice
    table "tblInvoice"
    key "InvoiceID"
    version "RowVersion"
```

Without it the saves of the entity are not checked, and the checker says so with a warning: "Invoice has no version column, so saves of it are not checked: name its column with version \"Column\", or write version none". `version none` says it is on purpose. A `readonly` source needs neither. `version` is for external databases only: a database Clarity owns keeps `row_version` itself, and a file source's file is its own version.

## Import

**Import...** next to an external SQLite source, or a file source, opens the Import panel. It lists the tables; tick the ones you want, name the module and click the **Import** button below them. The tables become entities mapped onto them, in a new module. Foreign keys between imported tables become `ref` fields, `NOT NULL` becomes `required`, and binary columns are left out. An integer column named `row_version`, as in a table Clarity made, becomes the entity's `version "row_version"`; any other version column you name yourself. From the command line:

```shell
clarity import shop.clarity erp --tables tblInvoice,tblCustomers
```

`--list` shows the tables, and `--module` names the module (the source's name by default).

A file source has one table, named after the file: its columns become text fields. Types are not guessed from the values; change them in the entity editor.

For an XML file source, Import lists the repeated elements found in the file by their path (`shop/orders/order`) and writes an entity with a text field per attribute and child element of the records ticked.

Beside the entity, Import writes a mapping, `mapping OrderFeed of Order from xml`, with the `at` where the records are, a line for each field whose path is not its name (`note at "@note"`) and the source's format options, and lays the source over it: `datasource feed file "imports/orders.xml" external using OrderFeed`. The paths are then kept in one place, and changed on the mapping's page (**Mapping...** next to the source); see [Mappings](code.formats.md#mappings).

Import also takes an XSD schema of the file. Its types become entities with their real types: `xs:int` and the other whole numbers an `integer`, `xs:decimal` a `decimal`, `xs:date` a `date`, `xs:dateTime` a `datetime`, `xs:boolean` a `bool`, an enumeration an `enum`, `maxLength` a text's size, `minOccurs="1"` `required`, a complex type inside another a record field and `maxOccurs="unbounded"` a list.

## Taking ownership

**Own...** next to an external SQLite source, or `clarity own shop.clarity erp`, makes Clarity the owner. It writes the source's first step, a baseline that describes the tables as they are and changes nothing in that database, and removes `external`. From then on your entities drive the tables through ordinary steps, and the `table`, `key` and `column` names stay. Undo takes it back.

Import works for SQLite and file sources, the table check and Own for SQLite only for now. An external PostgreSQL, MySQL or SQL Server database can be mapped by hand.
