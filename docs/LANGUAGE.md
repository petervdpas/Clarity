# The Clarity language

This is the reference for the Clarity language: every construct the checker accepts today, with its syntax and its rules. It is written for people and for AI assistants alike. When this file and the parser disagree, the parser is right and this file has a bug.

A Clarity project is written entirely in Clarity: the project settings, the data, the windows and the behavior. Clarity transpiles it to a real application in the stack the project names. Why the language looks the way it does is in [DESIGN.md](DESIGN.md); this file only says what it is.

Contents:

1. [Lines, names and values](#1-lines-names-and-values)
2. [The project file](#2-the-project-file)
3. [Modules](#3-modules)
4. [Data: enums and entities](#4-data-enums-and-entities)
5. [Windows](#5-windows)
6. [Parts](#6-parts)
7. [The app and its frame](#7-the-app-and-its-frame)
8. [Code behind](#8-code-behind)
9. [Functions and actions](#9-functions-and-actions)
10. [Statements and expressions](#10-statements-and-expressions)
11. [A complete example](#11-a-complete-example)
12. [Grammar summary](#12-grammar-summary)

## 1. Lines, names and values

Clarity is line based. Each declaration, setting, part or statement is one line, and a block is the lines indented below it.

- **Indentation** uses spaces; a tab is an error. Four spaces per level is the convention, and the designer writes four.
- **Comments** start with `//` and run to the end of the line. They can stand on their own line or follow code. The designer keeps them when it rewrites a file.
- **Blank lines** are free and are kept too.
- **Names** start with a letter or `_`, followed by letters, digits and `_`: `customer`, `vat_number`, `OrderLine`. Names are case sensitive. By convention, entities, enums, windows and the app are `PascalCase`, and everything else is `snake_case`.
- **Strings** are written in double quotes: `"Customer card"`. Inside them, `\"` is a quote, `\\` a backslash and `\n` a new line. Other backslashes are kept as written, so `"^[0-9]\d+$"` is a valid pattern.
  In code, a string is also read for slots: `{customer.name}` is a value, and `{{` and `}}` are braces (see [Text with slots](#text-with-slots)). Strings in settings, such as titles and patterns, have no slots.
- **Numbers** are written without quotes: `12`, `9.95`.
- **Booleans** are `true` and `false`.

Words such as `page`, `data` and `bind` are keywords only where the grammar expects them. Outside those places they are ordinary names, so a field may be called `data` and a part may be called `close_button`. The words `and`, `or`, `not` and `in` are reserved in expressions.

When a name has a label that people see, Clarity derives it from the name unless one is given: `vat_number` becomes "Vat number", and `CustomerCard` becomes "Customer card".

## 2. The project file

A project is one file, `<name>.clarity`. Next to it is a folder with the same name, which holds the modules, the generated output and the data:

```
shop.clarity           the project file: open this one
shop/
    customers.clarity  modules
    sales.clarity
    migrations/        the steps that bring each database to the dictionary
        main/          one folder per SQL data source
            0001.clarity
    out/               the generated application
    data/
```

The project file holds one `project` block:

```
project Shop
    title "Shop"
    stack rust web desktop
    output "out"
    language "en"
    currency EUR
    datasource main sqlite "data/shop.db"
    datasource payments rest "https://api.example.com/v2"
        auth bearer "env:PAYMENTS_TOKEN"
```

| Setting | Required | Meaning |
|---|---|---|
| `title "text"` | no | The name people see. When it is left out, it is derived from the project name. |
| `stack backend ui target...` | yes | The stack the project is transpiled into, read by position. Backends: `rust`, `go`, `csharp`, `c`, `cpp`. UIs: `web` (the shared web renderer) or `avalonia`. Targets: `desktop` and `web`, one or both. |
| `output "folder"` | no | Where the generated application goes, relative to the project folder. The default is `out`. |
| `language "tag"` | no | The language the project's texts are written in, as a language tag: `"en"`, `"nl-NL"`. Dates and numbers follow the locale of the person using the application; this language where the system names none. |
| `currency CODE` | no | The currency of `money` fields, as its ISO 4217 code: `EUR`, `USD`. Without it, amounts show without a currency sign. |
| `theme Name...` | no | The themes the application offers, declared in modules, the first being the one it starts with: `theme Shop Night`. People switch with the click target `theme`. Without any, the application looks as Clarity does, and parts cannot have a `style`. See [Themes](#themes). |
| `datasource name kind "where"` | no | A data source. Kinds: `sqlite`, `postgres`, `mysql`, `mssql`, `rest`. "Where" is a file path for SQLite (relative to the project's folder, for Clarity and for the generated application), a connection string or URL for the other databases (see Databases in section 4), and the base URL of a web API. `external` after it marks a database Clarity does not own, and `readonly` one whose records are only read. |
| `auth kind "secret"` | no | Indented under a `datasource` line, for web APIs. Kinds: `bearer`, `api-key`, `basic`. The secret names where to find it, such as `"env:NAME"` or `"keychain:NAME"`; it is never written into the file itself. |

Each setting appears once, except `datasource`. An unknown setting or value is an error that names the line and lists the accepted words.

Today the Rust backend with the web UI is generated; the other stacks are on the roadmap in DESIGN.md.

### Translations

Every text a person sees is in the project already: labels (written or derived from a name), help texts, titles, captions, placeholders, tooltips, tab titles, enum labels, menu items, the status text, messages, refusals, and the captions and errors code sets. Clarity collects them, so nobody marks a text for translation. The project's `language` is the language they are written in.

A file per language, in the `translations` folder of the module folder, gives them in that language:

```
// shop/translations/nl.clarity
translation nl
    "Customer card" "Klantkaart"
    "Saved {customer.name}" "{customer.name} opgeslagen"
    "Yes" "Ja"
```

- The file's name is its language: `nl.clarity` says `translation nl`, `nl-BE.clarity` says `translation nl-BE`.
- A text is found by what it says, for the whole project. When a text changes, its translation no longer applies: the checker warns that the old line is no longer shown anywhere, and the new text shows as written until it has a translation.
- A translation keeps the slots of its text and may move them; a slot missing or added is an error.
- A text without a translation shows as written, so a language can grow text by text. The Project tab lists every text with a column per language, says how many each misses, and writes what is typed into the files. `Yes` and `No`, which the screen shows for bools, can be translated too.
- The application shows the translation for the locale of the person using it: `nl-BE` takes `nl-BE.clarity`, else `nl.clarity`, else any other region of Dutch, and the project's own language shows as written. The designer can show the canvas and Preview in any of the project's languages.
- A menu item or button with `on click -> language` lets the person choose another language while the application runs: it lists the project's languages, each by its own name (Nederlands, Deutsch), and the application remembers the choice for its next start.

## 3. Modules

Every other `.clarity` file in the project folder is a module. It starts with `module`, then the modules it uses, then its declarations:

```
module sales

use customers

entity Order
    customer: ref Customer
```

A module of entities that live in another database:

```
module erp

use customers
source erp

entity Invoice
    number: text(20), unique
    customer: ref Customer
```

A reference to an entity in another database is stored as its id, without a foreign key, so the database cannot stop a referenced record from being deleted.

- `module name` is the first line of the file (comments and blank lines may come before it).
- `use other` makes the declarations of module `other` visible in this file. Without it, a name from another module is an error that tells you which `use` is missing.
- `source name` places the entities of this module in that data source of the project. Without it, they live in the first data source. The source must be a SQL database (sqlite, postgres, mysql or mssql); entities in a web API come later. One module has at most one `source` line.
- A name is declared once per module. Two modules may declare the same name, but a file that sees both (its own module and the modules it uses) cannot use that name: it is ambiguous, and the checker says so.

The declarations a module can hold:

| Declaration | Section |
|---|---|
| `enum Name` | [4](#4-data-enums-and-entities) |
| `entity Name` | [4](#4-data-enums-and-entities) |
| `page Name: kind` | [5](#5-windows) |
| `app Name` | [7](#7-the-app-and-its-frame), one per project |
| `code Name` | [8](#8-code-behind) |
| `function name(...)` and `action name(...)` | [9](#9-functions-and-actions) |

## 4. Data: enums and entities

### Enums

An enum is a fixed list of values. Each value can have a label; without one, the label is derived from the name.

```
enum CustomerKind
    Private label "Private person"
    Business
```

### Entities

An entity is a kind of record that is stored in a data source. It is the data dictionary: everything about a field is said once, here, and the windows, the database schema and the validation all follow from it.

```
entity Customer
    name: text(80), required, label "Name"
    email: email, unique, label "E-mail"
    kind: CustomerKind, default Private
    vat_number: text(14), help "Only for business customers"
    credit_limit: money, min 0, default 0
    active: bool, default true
    since: date
    notes: text, help "Internal notes, not shown to the customer"
```

A field is `name: type`, followed by modifiers separated by commas. Every record also has an `id`, which Clarity adds itself.

**Field types**

| Type | Holds |
|---|---|
| `text` | text of any length |
| `text(n)` | text of at most `n` characters |
| `integer` | a whole number |
| `decimal(p, s)` | a decimal number with `p` digits, `s` of them after the point |
| `money` | an amount of money, exact |
| `bool` | true or false |
| `date` | a calendar date |
| `datetime` | a date with a time |
| `email` | an e-mail address |
| `url` | a web address |
| `id` | the id of a record |
| an enum name | one value of that enum |
| `ref Entity` | a reference to a record of another entity |
| `list<Entity>` | the records of another entity that belong to this one |

**Modifiers**

| Modifier | Meaning |
|---|---|
| `required` | The field must have a value. Windows mark its label. |
| `unique` | No two records have the same value. Empty text counts as no value, so several records can leave a unique text field empty. |
| `min value`, `max value` | The lowest and highest value allowed. |
| `pattern "regex"` | Text must match the regular expression. |
| `default value` | The value of a new record: a number, a string, `true`, `false` or an enum value. |
| `label "text"` | The label people see, instead of the derived one. |
| `help "text"` | A help text, shown next to the field's input in windows. |
| `format name` | How the value is shown, by name; see [Formats](#formats). |
| `owned` | For a `list<...>`: the listed records live and die with this record. |
| `computed` | The value is computed, not entered. Accepted, but not used by the generated application yet. |
| `column "Name"` | The existing column the field is stored in, for an entity of an external database. |

Each modifier appears at most once per field. The checker reports defaults and limits that do not fit the type, such as `min "x"` on a number, or a default that is not one of the enum's values.

### Formats

Dates, times, numbers and money show the way the person using the application writes them: the locale comes from their system (or, where it names none, the project's `language`). The value stored is the same in every locale.

| Type | Shown by default | `format` |
|---|---|---|
| `integer` | with thousands separators: `1,234,567`, `1.234.567` | `plain`: without them, for codes and years |
| `decimal(p, s)` | with `s` decimals: `-1,250.50`, `-1.250,50` | `plain`: without thousands separators; `percent`: `0.25` shows `25%` |
| `money` | with the project's currency and its decimals: `€1,250.50`, `€ 1.250,50`, `1.250,50 €` | `plain`: the amount without the currency |
| `date` | in digits: `10/17/2026`, `17-10-2026`, `17.10.2026` | `short` (the default), `long`: `17 October 2026`, `iso`: `2026-10-17` everywhere |
| `datetime` | the date and the time: `10/17/2026 9:30 AM`, `17-10-2026 09:30` | `short`, `long`, `iso` as for dates |

```
entity Invoice
    issued: date, format long
    rate: decimal(5, 4), format percent
    year: integer, format plain
```

- A number input reads what is typed in the same locale: in Dutch `1.250,50` is 1250.5 and a point is a thousands separator. The currency sign, the percent sign and thousands separators may be typed or left out. Text that is no number marks the input and keeps the last value.
- Every stack shows the same thing: the cases in `conformance/formats.json` are what each renderer must show and read.
- Values in text that is shown follow the locale when they are in slots: `message "Saved on {order.placed}"`. See [Text with slots](#text-with-slots).

### Migrations

A database follows the dictionary through migration steps. Every SQL data source has its own steps, in a folder of the project's `migrations` folder named after the source and numbered in the order the steps run: `migrations/main/0001.clarity`, `migrations/main/0002.clarity`. A step changes only the entities of its source. A step holds one `migration` block with a title and its changes:

```
migration "Phone and VAT"
    rename field Customer.vat_number to vat
    add field Customer.phone: text(20)
    change field Order.number: text(20)
    add unique Customer.email
```

| Change | Does |
|---|---|
| `add entity Name` with field lines below it | creates the records of an entity |
| `remove entity Name` | removes an entity and its records |
| `rename entity Old to New` | renames an entity; its records stay |
| `add field Entity.name: type, modifiers` | adds a field; existing records get its `default`, or the empty value of its type |
| `remove field Entity.name` | removes a field and its values |
| `rename field Entity.old to new` | renames a field; its values stay |
| `change field Entity.name: type` | changes a field's type, converting the values |
| `add unique Entity.name`, `remove unique Entity.name` | makes a field unique, or not |
| `map table Entity to "name"` | stores the entity in a table of another name (renames the table); `to default` goes back to the entity's name |
| `map key Entity to "name"` | names the key column (renames it); `to default` is `id` |
| `map column Entity.field to "name"` | stores a field in a column of another name (renames the column); `to default` is the field's name |

Fields in a step are written as in an entity. Only what is stored counts: the type, `unique`, `column`, and `default` for the records that already exist. Labels, help texts and checks such as `min` belong to the dictionary alone. An added entity can name its table and key with `table` and `key` lines, as in the dictionary; renaming a field whose column is mapped changes nothing in the database.

- **Replaying the steps** in order gives the schema as last migrated. Each step must fit the schema the steps before it made; one that renames a field that does not exist is an error.
- **Pending changes** are what the dictionary has and the steps do not. The checker reports them as warnings on the entity, and the generator refuses to write the application until a step covers them.
- **Writing a step.** The Project tab lists the pending changes and writes them as the next step; `clarity migrate shop.clarity` does the same from the command line. Clarity proposes a rename when a field disappears and a field of the same type appears in the same entity, or an entity disappears and one with mostly the same fields appears. You confirm it or take it as a removal and an addition. Changes that can lose data (removing, changing a type) need a confirmation.
- **Running.** The generated application applies the steps its database does not have yet when it starts, each in a transaction where the database allows it (see Databases below). On SQLite, which does not enforce references while it builds a table again, it checks the references between records before a step is kept.
- **A step that ran never changes.** To undo it, write a new step.

### Databases

The same entities and steps work on every SQL database. The generated application carries a driver for each kind of database its data sources are, and writes each step in that database's own SQL.

```
datasource main postgres "env:SHOP_DATABASE"
```

| Kind | Where the database is | Example |
|---|---|---|
| `sqlite` | a file, relative to the project's folder | `"data/shop.db"` |
| `postgres` | a connection string or URL | `host=db.example.com user=shop dbname=shop`, `postgresql://shop@db.example.com/shop` |
| `mysql` | a URL | `mysql://shop@db.example.com:3306/shop` |
| `mssql` | an ADO.NET connection string | `server=tcp:db.example.com,1433;database=shop;user=shop;TrustServerCertificate=true` |

A connection that needs a password is written as `"env:NAME"`: the generated application reads it from that environment variable when it starts, so the password is never in the project. The `keychain:` form is for web APIs only for now.

How fields are stored:

| Type | SQLite | PostgreSQL | MySQL | SQL Server |
|---|---|---|---|---|
| `text(n)` | TEXT | VARCHAR(n) | VARCHAR(n); TEXT above 768 | NVARCHAR(n); NVARCHAR(MAX) above 4000 |
| `text`, `url` | TEXT | TEXT | TEXT | NVARCHAR(MAX) |
| `email` | TEXT | VARCHAR(320) | VARCHAR(320) | NVARCHAR(320) |
| `integer`, `id`, `ref` | INTEGER | BIGINT | BIGINT | BIGINT |
| `decimal(p, s)` | TEXT | NUMERIC(p,s) | DECIMAL(p,s) | DECIMAL(p,s) |
| `money` | TEXT | NUMERIC(19,4) | DECIMAL(19,4) | DECIMAL(19,4) |
| `bool` | INTEGER | BOOLEAN | BOOLEAN | BIT |
| `date` | TEXT | DATE | DATE | DATE |
| `datetime` | TEXT | TIMESTAMP | DATETIME | DATETIME2 |
| an enum | TEXT | TEXT | VARCHAR(100) | NVARCHAR(100) |

SQLite keeps decimals and dates as text, so no precision is lost. The key is an integer the database counts up, and a `ref` field gets a foreign key when its entity is in the same database.

What differs between the databases:

- **Unique text.** MySQL and SQL Server index text up to a length (768 and 850 characters), so a `unique` text field there needs a maximum length that fits, such as `text(200)`; the generator says which field. Empty text is never a duplicate: SQLite, PostgreSQL and SQL Server leave it out of the index, MySQL indexes it as no value.
- **Changing a type.** SQLite builds the table again; the others add a column of the new type, copy the values over converted, and drop the old one. Empty text that becomes a number or a date gets the field's default.
- **A failed step.** SQLite, PostgreSQL and SQL Server undo the whole step. MySQL cannot undo changes to tables, so it keeps the statements of the step that ran before the failure; the error says how many.
- **Comparing text.** `==` and `contains` follow the database: MySQL and SQL Server usually ignore case in `==` too, and sort text by their own rules.
- **Baseline steps**, Import and the check of external tables are for SQLite only for now. An external PostgreSQL, MySQL or SQL Server database can be mapped by hand and is read and written as it is.

### External databases

A database that already exists, and that Clarity does not own, is a data source marked `external` in the project file:

```
datasource erp sqlite "legacy/erp.db" external
```

Its entities live in a module with `source erp`, and they map onto the tables and columns that are there:

```
entity Invoice
    table "tblInvoice"
    key "InvoiceID"
    inv_no: text(20), required, unique, column "InvNo"
    cust_id: ref Customers, column "CustID"
    amount: decimal(18, 6), column "Amount"
```

- `table` names the existing table; without it, the table is the entity's name in snake case (`invoice`).
- `key` names the integer column that identifies a record; without it, that column is `id`. Tables whose key is not one integer column cannot be used yet.
- `column` names the existing column of a field; without it, the column has the field's name.
- Two fields cannot share a column, and no field can use the key column. The mapping is meant for existing tables; it stays when Clarity takes ownership (below).

Clarity reads and writes the records of an external database, but never changes its tables: it writes no migration steps for it, and the generated application never migrates it.

- **Import.** The Import button of an external SQLite source on the Project tab, or `clarity import shop.clarity erp`, writes the chosen tables as a new module of mapped entities. Names follow Clarity's style (`tblCustomers` becomes `Customers`, `CustName` becomes `cust_name`), column types become field types, foreign keys become `ref` fields when both tables are imported, `NOT NULL` becomes `required` and a unique index `unique`. Binary columns are left out.
- **Checking.** Each time the project loads, Clarity compares the entities with the tables of the database. A missing table or column is an error; a column whose type does not fit the field, and a required column that no field fills (so new records cannot be saved), are warnings. When the database cannot be read, a warning says its tables were not checked.
- **Read-only.** With `readonly` on the data source, `save` and `delete` of its records are errors.
- **Take ownership.** The Own button of an external source on the Project tab, or `clarity own shop.clarity erp`, makes Clarity the owner. It writes the source's first step, a `baseline`, which describes the mapped tables as they are and creates only what is missing, so it changes nothing in the database it was taken from (and a new, empty database gets the tables). Then it removes `external`. From there on the dictionary drives the tables through ordinary steps, and the mapped names stay: `table`, `key` and `column` work on owned entities too.

## 5. Windows

A window is a `page` block. Its settings come first, then its parts, which sit on a snap grid.

```
page CustomerForm: window
    title "Customer"
    data customer: Customer
    data orders: list<Order>
    grid 12 columns
    on return: window_returned

    label "Name" at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name
    table name order_table at 1,3 span 12 rows 6 bind orders columns number, total
    button "Save" name save at 11,10 span 2 default on click: save_clicked
```

### Kinds

`page Name: kind`, where the kind is one of:

| Kind | Is |
|---|---|
| `window` | a window of its own, which can have a menu, toolbar, ribbon, sidebars and a footer |
| `dialog` | a modal window |
| `view` | content shown inside the app's frame |

Only a `window` has frame regions of its own.

An application has one desktop window, on every stack. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. Only a `dialog` opens a desktop window of its own: a new window appears only where the design says so.

### Settings

| Setting | Meaning |
|---|---|
| `title "text"` | The window's title. |
| `data name: Entity` | One record the window works on. |
| `data name: list<Entity>` | A list of records the window works on. |
| `grid N columns` | The number of grid columns, 1 to 24. The default is 12. |
| `on open: handler` | Runs when the window has opened and its data is loaded. |
| `on close: handler` | Runs when the window is about to close. |
| `on return: handler` | Runs when a window this one opened has closed, and this one is on top again. This is where a list reloads after a form saved a record. |

The window's data is what its parts bind to and what its handlers work on. When a window is opened with `open CustomerForm with customer = ...`, the data is filled by name. Data that is not filled starts as a new, empty record, or for a list, as all records of the entity.

### The grid

Every part sits on the grid of the window or of the container it is in:

- `at col,row` is the top-left cell, counted from 1. Every part needs one.
- `span n` is the number of columns it covers. The default comes from the part's kind.
- `rows n` is the number of rows it covers. The default also comes from the part's kind.

Rows have a fixed height in the designer. In a running application the columns share the width of the window, and rows that hold something that grows (a table, a list, an image, a canvas, or a container with one of these inside) share the extra height. Everything else keeps its height.

The checker reports parts that start outside the grid, run past its last column, or overlap another part in the same container.

## 6. Parts

A part is one line: its kind, then an optional caption in quotes, then its properties.

```
textbox name email at 4,2 span 6 bind customer.email placeholder "name@example.com" on change: email_changed
```

Properties may be written in any order. The designer writes them in a fixed order, which is the order the tables below use.

### Properties every part has

| Property | Meaning |
|---|---|
| `"caption"` | The text it shows, right after the kind, for the kinds that have one. |
| `name n` | The part's name, unique in its window (sidebars, footer, menu, toolbar and ribbon included). Handlers refer to the part by it. The designer names new parts `button1`, `textbox2` and so on; templates name inputs after their field. |
| `at col,row`, `span n`, `rows n` | Place and size on the grid; see [the grid](#the-grid). |
| `tooltip "text"` | Shown when the pointer rests on the part. |
| `style name` | A style of the project's theme: its background, text and border colors. See [Themes](#themes). |
| `on event: handler` | Runs `handler` in the window's code behind when `event` happens. Each event is wired at most once per part. |

### The catalog

**Text**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `label` | nothing | `for input` | none |
| `help` | nothing | `for input` | none |
| `heading` | nothing | none | none |
| `text` | a field of any type: `bind customer.name` | none | none |

**Input**

| Part | Binds to fields of type | Own properties | Events |
|---|---|---|---|
| `textbox` | text, email, url | `placeholder`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `memo` | text (several lines) | `placeholder`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `password` | text (hidden while typed) | `placeholder`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `number` | integer, decimal, money, id | `placeholder`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `checkbox` | bool | `readonly`, `taborder` | `change` |
| `switch` | bool | `readonly`, `taborder` | `change` |
| `dropdown` | an enum, `ref Entity` | `readonly`, `taborder` | `change` |
| `radio` | an enum, as one button per value | `horizontal` or `vertical`, `readonly`, `taborder` | `change` |
| `date` | date, datetime | `readonly`, `taborder` | `change` |
| `button` | nothing | `taborder`, `key`, `default`, `cancel` | `click` |

**Containers**

| Part | Holds | Own properties | Events |
|---|---|---|---|
| `form` | parts, on its own grid; binds to one record: `bind customer` | none | `submit` |
| `panel` | parts, on its own grid, in a frame with the caption | none | none |
| `tabs` | `tab "Title"` blocks, each with parts | none | `change` |
| `splitter` | exactly two `pane` blocks, each with parts | `horizontal` or `vertical` | none |

**Data**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `table` | a list: `bind orders` | `columns field, field, ...` | `select`, `open` |
| `list` | a list: `bind orders` | `show field, field, ...` | `select`, `open` |

**Graphics**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `image` | nothing | `source "file"` | `click` |
| `canvas` | nothing | `surface svg`, `surface bitmap` or `surface gpu`; `renderer Name` | `pointer`, `key` |

`select` fires when a row is chosen, and `open` on a double-click or Enter. `enter` and `leave` fire when the focus moves into and out of an input.

### Binding

`bind` connects a part to the window's data:

- An input or `text` binds to one field of a single record: `bind customer.name`. The field's type must be one the part accepts; a checkbox needs a `bool`.
- A `table` or `list` binds to a list: `bind orders`. Its `columns` or `show` name fields of the listed entity.
- A `form` binds to a single record: `bind customer`.

What the user types goes into the data, and handlers read and change the data by name. A part that cannot be bound, such as a label, may not have `bind`. An input that is not bound yet is allowed while designing.

### Labels, help texts and forms

A label, its input and the input's help text are three separate parts. `for` ties a label or a help text to an input by the input's name:

```
form name details at 1,1 span 12 rows 3 bind customer
    label "Name" at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name placeholder "Full name"
    help at 10,1 span 3 for name
    label "Kind" at 1,2 span 3 for kind
    radio name kind at 4,2 span 6 bind customer.kind
```

- **A label** shows its caption. Clicking it focuses its input. When the input is bound to a `required` field, the label shows a marker. A label is never bound to data; to show a value, use `text`.
- **A help text** shows its caption, or, when it has none, the `help` of the field its input is bound to. When a handler sets the input's `error`, the help text shows that message instead, and the input is marked.
- **`for`** must name an input in the same window.
- **A form** binds to one record and holds the inputs of that record, nested in panels and tabs if needed. Every input inside it binds to that record, and a form cannot contain another form. Pressing Enter in one of its inputs fires `submit`; when `submit` is not wired, Enter clicks the window's default button. A form without a caption has no frame, so its grid lines up with the grid around it.

### Input and button properties

| Property | On | Meaning |
|---|---|---|
| `placeholder "text"` | textbox, memo, password, number | Shown while the input is empty. |
| `readonly` | inputs | Shows the value, which cannot be changed. Handlers can change this. |
| `taborder n` | inputs, buttons | Position in the keyboard order, from 1. Parts without one follow in reading order. |
| `key "Ctrl+S"` | buttons | The key that clicks the button. |
| `default` | buttons | Enter clicks this button. One per window. |
| `cancel` | buttons | Escape clicks this button. One per window. |
| `horizontal`, `vertical` | radio, splitter | Side by side or stacked. |

A key is written as modifiers, then one key, joined by `+`: `"Ctrl+S"`, `"Ctrl+Shift+F2"`, `"Alt+Delete"`. The modifiers are `Ctrl`, `Alt`, `Shift` and `Meta`. The key is a letter, a digit, `F1` to `F12`, or one of `Enter`, `Escape`, `Tab`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`.

### Clicking to open a window

A button, an image, a menu item, or a toolbar or ribbon button can open a window without code:

```
button "All customers" at 1,12 span 3 on click -> CustomerBrowse
```

`on click -> Target` opens the window `Target`. In menus and toolbars, `close` closes the current window, `quit` ends the application, `language` lets the person choose the language the application shows (see [Translations](#translations)), and `theme` one of the themes it offers (see [Themes](#themes)). A click either opens a window this way or runs a handler (`on click: handler`), not both.

### Containers

A container has its own grid, as many columns wide as its `span`. Its parts are indented below it, and their positions count from its own top-left cell:

```
tabs at 1,5 span 12 rows 7
    tab "Orders"
        table at 1,1 span 12 rows 5 bind orders columns number, total
    tab "Notes"
        text at 1,1 span 12 bind customer.notes
splitter at 1,1 span 12 rows 10 vertical
    pane
        list at 1,1 span 12 rows 4 bind customers show name
    pane
        panel "Details" at 1,1 span 12 rows 4
            text at 1,1 span 6 bind customer.name
```

`tabs` need at least one `tab`, and a `splitter` has exactly two `pane` blocks.

## 7. The app and its frame

A project has one `app` block. It names the main window and declares the frame every window shares: a menu, a toolbar, a ribbon, sidebars and a footer.

```
app Shop
    title "Shop"
    main CustomerCard
    data session: Session
    on start: app_started
    menu
        item "File"
            item "New customer" key "Ctrl+N" on click -> CustomerForm
            separator
            item "Quit" key "Ctrl+Q" on click -> quit
    toolbar
        button "Customers" icon "list" on click -> CustomerBrowse
        separator
        button "Orders" on click: orders_clicked
    ribbon
        tab "Home"
            group "Customers"
                button "New" icon "add" large on click -> CustomerForm
                button "Browse" icon "list" on click -> CustomerBrowse
    sidebar left span 3
        list at 1,1 span 3 rows 10 bind recent show name
    footer
        status "Ready"
        text at 10,1 span 3 bind session.user
```

### App settings

| Setting | Meaning |
|---|---|
| `title "text"` | The application's title. |
| `main Window` | The window shown at start; it must be a `window`. |
| `data name: Entity` | Data for the parts in the sidebars and the footer. |
| `on start: handler` | Runs when the application starts, before its main window opens. Its code is in `code AppName`. |

### Regions

| Region | Docks | Holds |
|---|---|---|
| `menu` | at the top | `item "Caption"` lines, which can hold more items below them (a submenu), and `separator` |
| `toolbar` | at the top, below the menu | `button` and `separator` |
| `ribbon` | at the top | `tab "Title"` blocks with `group "Title"` blocks of `button` and `separator` |
| `sidebar left` or `sidebar right` | at that side, `span n` columns wide (default 3) | parts, on its own grid |
| `footer` | at the bottom | an optional `status "text"` and parts on a one-row grid of 12 columns |

A window can declare its own regions. A window's region replaces the app's region of the same kind, and a window's sidebar replaces the app's sidebar on the same side. The other regions still come from the app.

### Commands

Menu items and toolbar and ribbon buttons are commands. Their properties:

| Property | On | Meaning |
|---|---|---|
| `"caption"` | all | The text, right after the kind. A menu item needs one; a button needs a caption or an icon. |
| `name n` | all | The command's name, for handlers. |
| `icon "name"` | all | An icon. |
| `key "Ctrl+N"` | menu items | The key for it, shown in the menu. |
| `large` | ribbon buttons | A large button. |
| `on click -> Target` | all | Opens a window, or `close`, `quit`, `language` or `theme`. |
| `on click: handler` | all | Runs a handler. |

An item with a submenu opens the submenu, so it has no `on click` of its own.

### Themes

A theme says how the application looks, as named values rather than a style sheet, so every stack can carry it: the web renderer as a CSS file of variables, Avalonia as resources and styles. It is declared in a module, and the project file names the themes the application offers: `theme Shop Night`, the first one the one it starts with.

Every theme starts from Clarity's own, the built-in `theme Clarity`, which sets every value below; a theme sets only what it changes, and the rest stays Clarity's. The Module view shows Clarity's own theme, and "New theme" starts from a full copy of it.

```
theme Shop
    mode system
    color accent "#1f6f5c" dark "#4fb393"
    color brand "#f5c400"
    font "Inter"
    font heading "Georgia, serif"
    size 14
    size heading 20
    radius 4
    radius button 12
    padding 5
    shadow flat
    part button background panel, border accent, text accent
    part button hover background accent_soft
    part header background accent_soft, text accent
    style primary background accent, text on_accent, weight bold
    style quiet border none
```

| Line | Meaning |
|---|---|
| `title "Night owl"` | The name people see when they choose the theme; without it, one derived from the name (`NightOwl` reads "Night owl"). It is translated like every other text. |
| `mode light`, `mode dark`, `mode system` | The theme is always light, always dark, or follows the system. |
| `color name "#rrggbb"` | A color, written `#rrggbb` or `#rgb`; `dark "#rrggbb"` after it gives its value in dark mode. |
| `font "Families"`, `font heading ...`, `font mono ...` | The fonts of text, headings and code, as a list of families: `"Inter, sans-serif"`. |
| `size n`, `size small n`, `size heading n` | Text sizes in pixels: the text, small text (help, captions) and headings. |
| `radius n`, `radius input n`, `radius button n`, `radius panel n` | Corner radii in pixels: of everything, or of one kind. |
| `padding n`, `border n` | The space inside inputs and buttons, and the width of borders, in pixels. |
| `shadow none`, `shadow flat`, `shadow raised` | The shadow under menus, popups and messages. |
| `part kind [state] look` | How a kind of part looks: `button`, `input`, `header` (of a table), `tab` or `menu`; with `hover`, `pressed` or `focus`, in that state. |
| `style name look` | A named style that parts pick with `style name`. |

A look (of a part or a style) sets `background`, `text` and `border` to a color of the theme by name or written out (a border can be `none`), `radius n`, and `weight bold` or `weight normal`, separated by commas.

The colors a theme can set by role, which the application itself uses:

| Role | Used for |
|---|---|
| `background`, `panel`, `raised` | the window, panels and inputs, raised surfaces such as buttons and table headers |
| `frame`, `frame_text` | the menu bar, toolbar, sidebars and footer |
| `text`, `muted` | text, and quieter text such as help and labels |
| `border`, `strong_border` | lines between things, and the borders of inputs and buttons |
| `accent`, `accent_soft`, `on_accent` | what is chosen, active or important, a soft tint of it, and text on it |
| `selection`, `focus`, `input` | a chosen row, the focus ring, the inside of inputs |
| `error`, `warning`, `success` and their `_soft` tints | what went wrong, what needs care, what went well |

Other names are colors of the theme's own, for its parts and styles.

- A part picks a style with `style primary`. Every theme the application offers must have that style, so switching themes never drops one; a theme that misses it is an error.
- The generator writes Clarity's own theme as `ui/themes/clarity.css` and each offered theme as a CSS file of its own, `ui/themes/shop.css`, linked from the application's page after it. The application holds the theme's class (`theme-shop`), and a part with a style holds its class (`style-primary`), as a web developer would write it. The application's own CSS reads only these values. The designer uses the same CSS for its canvas, and the Module view shows a sample window for the theme being written.
- An application that offers several themes lets people switch with `on click -> theme`: a menu item or button that lists the themes by their titles, switches at once and remembers the choice for the next start. The Project tab chooses the themes the application offers and the one it starts with, and can show the designer's canvas in any of them.

## 8. Code behind

A window's handlers are in a `code` block named after the window, in the same file as the window. The app's handlers are in a `code` block named after the app, and an entity's data events in one named after the entity (see [Data events](#data-events)).

```
code CustomerForm
    handler kind_changed
        // Only business customers have a VAT number.
        vat_number.enabled = customer.kind == Business

    handler save_clicked
        name.error = ""
        if customer.name == ""
            name.error = "Name is required"
            return
        save customer
        message "Saved {customer.name}"
        close
```

- `handler name` declares a handler. It has no parameters. It runs when an event wired to it happens.
- A handler that is wired but not written is an error. A handler that is written but not wired anywhere is a warning.
- Handlers run in the backend. The window sends its data with the event, the handler runs, and the window shows what changed: data, part properties, messages, and windows to open or close.

### What a handler sees

- **The window's data**, by its `data` names: `customer`, `orders`.
- **The window's parts**, by their names: `save`, `vat_number`.
- **Its own locals**, made with `let`.
- **Enum values**, written bare (`Business`) where the expected type says which enum is meant, and qualified (`CustomerKind.Business`) where it does not.
- **Functions and actions** of its own module and of the modules it uses.

A name that is both data and a part in the same window is an error.

### Part properties

| Property | On | Type | Can be set |
|---|---|---|---|
| `visible` | every part | bool | yes |
| `enabled` | every part | bool | yes |
| `caption` | labels, help texts, headings, buttons, commands | text | yes |
| `error` | inputs | text | yes; `""` clears it |
| `readonly` | inputs | bool | yes |
| `text` | text boxes, memos and password inputs that are not bound | text: what the user typed | yes |
| `selected` | tables, lists | the record of the selected row | no |

A bound input is changed through its data: `customer.name = "Ada"` changes the text box bound to `customer.name`.

### Data events

An entity's code block holds its data events: code that runs whenever one of its records is saved or deleted, from any window, action or other event. The block has the entity's name and is in the same file as the entity.

```
code Order
    before save
        if order.status == Approved and not within_credit(order)
            refuse "Order {order.number} is over its customer's credit limit"

    after save
        log_change("saved " + order.number)

    before delete
        if order.status != Pending
            refuse "Only pending orders can be deleted"
```

| Event | Runs |
|---|---|
| `before save` | before a record is stored; what it changes in the record is stored |
| `after save` | after it is stored; a new record has its id by then |
| `before delete` | before a record is removed |
| `after delete` | after it is removed |

- **The record** is named after the entity in snake case: `order` in the events of `Order`, `order_line` in those of `OrderLine`. A new record has id 0 in `before save`.
- **An event is an action** on its record. It can read and change records, `save` and `delete` others, and call functions and actions; it declares no effects. Like an action, it does not talk to the screen: no `message`, `open`, `close` or part properties.
- **`refuse "text"`** stops the save or delete. The text is the error the window shows, so a refusal reads as the message for the person using the application.
- **All or nothing.** The save or delete runs in one transaction with its events, and with the saves and deletes they make in the same database. When an event fails or refuses, none of it is kept, and a new record is still new.
- An event that saves its own record in a save event, or deletes it in a delete event, would run itself again without end; the checker reports it. Each event is written once per entity, and an entity's code block has no handlers.
- Renaming an entity renames its code block and the record's name in its events.

## 9. Functions and actions

Logic that is not tied to one window goes into functions and actions in a module. Handlers, functions and actions can call them.

```
function within_credit(order: Order) -> bool
    return order.total <= order.customer.credit_limit

function order_total(order: Order) -> money
    ensure result >= 0
    let total: money = 0
    for line in order.lines
        total = total + line.amount
    return total

action approve(order: Order)
    require order.status == Pending
    require within_credit(order)
    effects db.write
    order.status = Approved
    save order
```

- **The signature** is `name(param: type, ...)`, optionally followed by `-> type` for the result. The types are `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, an entity (one record), an enum, and `list<Entity>`.
- **`require expr`** checks the arguments before the body runs. **`ensure expr`** checks the result after it, as `result`. Both must be bool, and both come before the statements.
- **`effects`** lists what an action does to data: `db.read`, `db.write` or both, comma separated. An action that uses `save` or `delete`, or that calls an action with `db.write`, must declare `db.write`.
- **A function** has no effects. It cannot save or delete, call actions, show messages or open windows. That makes it safe to call anywhere, including in conditions.
- **An action** can change data. Neither functions nor actions talk to the screen: `message`, `open`, `close` and part properties belong in handlers.
- A function or action with a result returns a value on every path; the checker reports a path that does not.

## 10. Statements and expressions

### Statements

One statement per line; `if`, `else` and `for` take an indented block.

| Statement | Meaning | Where |
|---|---|---|
| `let name = expr` | A local, with the type of the expression. | everywhere |
| `let name: type = expr` | A local with a declared type. | everywhere |
| `target = expr` | Sets a local, a data field (`customer.credit_limit`) or a part property (`save.enabled`). | everywhere; part properties in handlers |
| `if expr` / `else if expr` / `else` | Runs a block when the condition, which must be bool, is true. | everywhere |
| `for item in list` | Runs the block once per item of a list. | everywhere |
| `name(args)` | Calls a function or action on its own line. | everywhere |
| `handler_name()` | Runs another handler of the same window or app. | handlers |
| `return` | Leaves the handler or action. | handlers, actions without a result |
| `return expr` | Leaves with a result. | functions and actions with a result |
| `message expr` | Shows a message. `message info`, `message warning` and `message error` set its level; the default is info. | handlers |
| `open Window` | Opens a window. | handlers |
| `open Window with name = expr, ...` | Opens a window and fills its data by name. | handlers |
| `close` | Closes this window. | handlers |
| `save expr` | Stores a record in its data source. A new record gets its id. | handlers, actions, data events |
| `delete expr` | Removes a record from its data source. | handlers, actions, data events |
| `refuse expr` | Stops with this text (or value) as the error; a save or delete it is part of does not happen. See [Data events](#data-events). | everywhere |
| `reload data` | Reads the window's data again: a list loads all its records again, and a record loads again by its id (a new record stays as it is). | handlers |

### Expressions

| Expression | Meaning |
|---|---|
| `12`, `9.95`, `"text"`, `true`, `false` | literals |
| `"Saved {customer.name}"` | text with slots, for text that is shown; see [Text with slots](#text-with-slots) |
| `customer`, `customer.name`, `grid.selected.total` | names and paths |
| `Business`, `CustomerKind.Business` | enum values |
| `name(arg, ...)` | a call to a function or action; `count(list)` gives the number of items |
| `(expr)` | grouping |
| `-x` | negation |
| `a * b`, `a / b` | multiplication and division |
| `a + b`, `a - b` | addition and subtraction; `+` also joins text |
| `a == b`, `a != b`, `a < b`, `a <= b`, `a > b`, `a >= b` | comparison |
| `x in [a, b, c]` | whether `x` equals one of the values |
| `a contains b` | whether text `a` contains text `b`, ignoring case |
| `find c in Entity where ... order by ...` | the records that meet a condition; see [Queries](#queries) |
| `not a` | negation of a bool |
| `a and b`, `a or b` | both, either |

From tightest to loosest: member access (`.`), unary `-`, `*` and `/`, `+` and `-`, comparisons, `in` and `contains`, `not`, `and`, `or`.

### Text with slots

Text that people read names its values in slots, instead of adding them together, so a translator can move them and each value shows in the person's format:

```
code OrderForm
    handler approve_clicked
        approve(order)
        message "Order {order.number} of {order.total} is approved"
        approve_button.caption = "Approved on {order.ordered_on}"
```

- A slot is a name or a member path in braces: `{total}`, `{order.customer.name}`. It is checked like any expression; a whole record or list is no value to show, so `{order.customer}` is an error that suggests one of its fields.
- `{{` and `}}` are braces in the text; a `}` on its own is an error.
- A value shows as its field shows it: money with its currency, a date in the locale and the field's `format`, an enum value by its label. A slot of a local or a parameter shows as its type does.
- Text with slots is text to show: it goes in `message`, `refuse`, and a part's `caption` or `error`. Anywhere else it is an error.
- Text joined with `+` in a message, a refusal, a caption or an error is a warning that suggests slots, since such text cannot be translated and its values do not follow the locale.
- The backend never formats a value for a person: it sends the text as written and the value of each slot, and the screen fills the slots. Text read back in code (a caption, an error) has its slots filled plainly.

### Queries

`find` reads the records of an entity that meet a condition, in an order. The result is a list, used like any other: assign it to a window's list data, loop over it with `for`, count it, or return it.

```
code Customers
    handler filter_changed
        customers = find c in Customer where c.name contains name_filter.text and c.active order by c.name

    handler card_opened
        orders = find o in Order where o.customer == customer and o.total > 100 order by o.total desc
```

- `find c in Customer` names each record `c`. The name exists only inside the query, so `c.name` is never confused with a part or local that is also called `name`.
- `where` is optional. Without it, `find` reads every record.
- The condition combines, with `and`, `or`, `not` and parentheses, comparisons between a field of the record and a value: `c.name == x`, `c.total > 100`, `c.name contains x`, `c.kind in [Private, Business]`. A bool field stands on its own: `c.active`, `not c.active`. A reference field is compared with a record: `o.customer == customer`.
- The value side can be anything that does not use the record: a literal, a local, the window's data, a part property.
- Only the record's own fields can be compared. `o.customer.name` goes through a reference and is not allowed yet.
- `order by` takes one or more fields, each optionally followed by `desc`. Without it, records come in the order they were stored.
- The database does the work: the condition becomes a query with parameters, so values never become part of the SQL text.

### Types

The types of values follow the data dictionary:

| Type | Values of fields of type |
|---|---|
| `text` | text, email, url |
| `integer` | integer, id |
| `decimal` | decimal, money |
| `bool` | bool |
| `date`, `datetime` | date, datetime |
| an enum | that enum |
| a record | an entity, a `ref` |
| a list of records | `list<Entity>`, a list in the window's data |

An integer is accepted where a decimal is expected. Nothing else converts by itself. The checker reports unknown names, wrong types, setting something that cannot be set, and calls with the wrong arguments.

## 11. A complete example

A project file and one module: a customer list with a filter that opens a form, the form that saves a customer, and a data event that clears the VAT number of a private customer.

`shop.clarity`:

```
project Shop
    stack rust web desktop
    datasource main sqlite "data/shop.db"
```

`shop/customers.clarity`:

```
module customers

enum CustomerKind
    Private label "Private person"
    Business

entity Customer
    name: text(80), required
    kind: CustomerKind, default Private
    vat_number: text(14), help "Only for business customers"
    notes: text

app Shop
    main CustomerBrowse

page CustomerBrowse: window
    title "Customers"
    data customers: list<Customer>
    on return: window_returned

    label "Name" at 1,1 span 2 for name_filter
    textbox name name_filter at 3,1 span 4 placeholder "Part of the name" on change: filter_changed
    table name grid at 1,2 span 12 rows 8 bind customers columns name, kind on open: open_clicked
    button "New" at 9,10 span 2 on click -> CustomerForm
    button "Open" name open_button at 11,10 span 2 on click: open_clicked

page CustomerForm: window
    title "Customer"
    data customer: Customer

    form name customer_form at 1,1 span 12 rows 4 bind customer
        label "Name" at 1,1 span 3 for name
        textbox name name at 4,1 span 6 bind customer.name
        help at 10,1 span 3 for name
        label "Kind" at 1,2 span 3 for kind
        radio name kind at 4,2 span 6 bind customer.kind on change: kind_changed
        label "Vat number" at 1,3 span 3 for vat_number
        textbox name vat_number at 4,3 span 4 bind customer.vat_number
        help at 8,3 span 5 for vat_number
        label "Notes" at 1,4 span 3 for notes
        textbox name notes at 4,4 span 9 bind customer.notes
    button "Save" name save at 9,6 span 2 key "Ctrl+S" default on click: save_clicked
    button "Close" name close_form at 11,6 span 2 cancel on click: close_clicked

code Customer
    before save
        // Only business customers keep a VAT number, whichever window saves them.
        if customer.kind == Private
            customer.vat_number = ""

code CustomerBrowse
    handler window_returned
        // The form may have saved a customer.
        filter_changed()

    handler filter_changed
        customers = find c in Customer where c.name contains name_filter.text order by c.name

    handler open_clicked
        open CustomerForm with customer = grid.selected

code CustomerForm
    handler kind_changed
        vat_number.enabled = customer.kind == Business

    handler save_clicked
        name.error = ""
        vat_number.error = ""
        if customer.name == ""
            name.error = "Name is required"
            return
        if customer.kind == Business and customer.vat_number == ""
            vat_number.error = "Needed for business customers"
            return
        save customer
        message "Saved {customer.name}"
        close

    handler close_clicked
        close
```

The full shop example, with orders, a menu and a ribbon, is `examples/shop.clarity`.

## 12. Grammar summary

A compact view of the grammar. `[x]` is optional, `x*` is zero or more, `a | b` is one of them, and an indented line is a line in the block above it. Words in quotes are written as shown.

```
project-file  = "project" Name
                    setting*
setting       = "title" String | "stack" word word word+ | "output" String | "language" String | "currency" word | "theme" Name+
              | "datasource" name kind String ["external"] ["readonly"]
                    ["auth" kind String]

module-file   = "module" name
                ("use" name)*
                ["source" name]
                declaration*
declaration   = enum | entity | page | app | theme | code | function | action

enum          = "enum" Name
                    (Value ["label" String])+
entity        = "entity" Name
                    ["table" String] ["key" String]
                    (name ":" type ("," modifier)*)+
type          = "text" ["(" n ")"] | "decimal" "(" p "," s ")" | "integer" | "money" | "bool"
              | "date" | "datetime" | "email" | "url" | "id" | Enum | "ref" Entity | "list" "<" Entity ">"
modifier      = "required" | "unique" | "owned" | "computed"
              | ("min" | "max" | "default") value | ("pattern" | "label" | "help" | "column") String | "format" name

page          = "page" Name ":" ("window" | "dialog" | "view")
                    (page-setting | region | part)*
page-setting  = "title" String | "data" name ":" (Entity | "list" "<" Entity ">")
              | "grid" n "columns" | "on" ("open" | "close" | "return") ":" handler
part          = kind [String] property*
                    (part | "tab" String | "pane")*        // containers only
property      = "name" name | "at" col "," row | "span" n | "rows" n | "bind" path
              | "for" name | "placeholder" String | "tooltip" String | "style" name | "readonly" | "taborder" n
              | "key" String | "default" | "cancel" | "horizontal" | "vertical"
              | "columns" names | "show" names | "source" String | "surface" word | "renderer" Name
              | "on" event ":" handler | "on" "click" "->" Target

app           = "app" Name
                    ("title" String | "main" Window | "data" ... | "on" "start" ":" handler | region)*
region        = "menu" | "toolbar" | "ribbon" | "sidebar" ("left" | "right") ["span" n] | "footer"
command       = ("item" | "button" | "separator") [String]
                ("name" name | "icon" String | "key" String | "large" | "on" "click" (":" handler | "->" Target))*

code          = "code" (Window | App)
                    ("handler" name
                        statement*)*
              | "code" Entity
                    (("before" | "after") ("save" | "delete")
                        statement*)*
function      = ("function" | "action") name "(" [param ("," param)*] ")" ["->" type-ref]
                    ("require" expr | "ensure" expr | "effects" effect ("," effect)*)*
                    statement*

find          = "find" name "in" Entity ["where" condition]
                ["order" "by" name "." field ["desc"] ("," name "." field ["desc"])*]

theme         = "theme" Name
                    ("title" String | "mode" ("light" | "dark" | "system") | "color" name String ["dark" String]
                    | "font" [role] String | "size" [role] n | "radius" [kind] n | "padding" n | "border" n
                    | "shadow" ("none" | "flat" | "raised")
                    | "part" kind [state] look | "style" name look)*
look          = ("background" | "text" | "border") (name | String) | "radius" n | "weight" ("bold" | "normal")
                    ("," look)*
translation   = "translation" language
                    (String String)*
migration     = "migration" String ["baseline"]
                    (("add" "entity" Entity
                        field*)
                    | "remove" "entity" Entity | "rename" "entity" Entity "to" Name
                    | "add" "field" Entity "." field | "remove" "field" Entity "." name
                    | "rename" "field" Entity "." name "to" name | "change" "field" Entity "." name ":" type
                    | ("add" | "remove") "unique" Entity "." name
                    | "map" ("table" Entity | "key" Entity | "column" Entity "." name) "to" (String | "default"))*
```
