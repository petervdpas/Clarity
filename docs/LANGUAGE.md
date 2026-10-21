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
- **Names** start with a letter or `_`, followed by letters, digits and `_`: `customer`, `vat_number`, `OrderLine`. Names are case sensitive. Entities, enums, windows and the app are written in `PascalCase` by convention. The parts of a window and their handlers must be `camelCase`: letters and digits only, starting with a small letter (`newGame`, `square1`, `saveClicked`); the checker reports any other. Fields, data and functions are `snake_case` by convention.
- **Every part has a name** (`button @saveCaption name save at 1,3`), which its events and code go by. The designer names a part when it is placed (`button1`); a part written without one is an error, and **Name them** under Problems names every such part in the file.
- **Keys** name the texts people read: `@saveCaption`, `@customerCardTitle`. A key's text is in the language files, one per language (see [Translations](#translations)); a title, a caption, a label, a message are keys. Only a text without letters, such as a calculator's `"7"`, is written as it is.
- **Strings** are written in double quotes: `"logo.png"`. Inside them, `\"` is a quote, `\\` a backslash and `\n` a new line. Other backslashes are kept as written, so `"^[0-9]\d+$"` is a valid pattern.
  In code, a string is also read for slots: `{customer.name}` is a value, and `{{` and `}}` are braces (see [Text with slots](#text-with-slots) and [Texts by key](#texts-by-key)). Strings in settings, such as patterns and file names, have no slots.
- **Numbers** are written without quotes: `12`, `9.95`.
- **Booleans** are `true` and `false`.

Words such as `page`, `data` and `bind` are keywords only where the grammar expects them. Outside those places they are ordinary names, so a field may be called `data` and a part may be called `close_button`. The words `and`, `or`, `not` and `in` are reserved in expressions.

A text people see is a key (`label @customerVatNumberLabel`), whose text the language files give; see [Translations](#translations). **Make texts keys** writes a key for each text a project wrote as it was, and gives it a text derived from the name where there was none: `vat_number` becomes "Vat number", and `CustomerCard` becomes "Customer card".

## 2. The project file

A project is one file, `<name>.clarity`. Next to it is a folder with the same name, which holds the modules, the generated output and the data:

```text
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

```clarity
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
| `filestore name "folder"` | no | A file store: the folder that keeps the pictures of `image` fields with `store name`. A relative folder is in the project's folder, as a SQLite file is; `"env:NAME"` takes it from that environment variable, for a folder that differs per machine. |
| `auth kind "secret"` | no | Indented under a `datasource` line, for web APIs. Kinds: `bearer`, `api-key`, `basic`. The secret names where to find it, such as `"env:NAME"` or `"keychain:NAME"`; it is never written into the file itself. |

Each setting appears once, except `datasource` and `filestore`. An unknown setting or value is an error that names the line and lists the accepted words.

Five stacks are generated today: Rust with the web UI (Tauri), C# with Avalonia, Go with the web UI (Wails), and C and C++ with the web UI (webview; SQLite only for now).

### Translations

Every text a person reads is a key, written with `@`: titles, captions, labels, help texts, placeholders, tooltips, tab titles, enum labels, menu items, the status text, messages, refusals, and the captions and errors code sets. A file per language, in the `translations` folder of the module folder, gives each key its text in that language. The project's `language` is the default language, which has a file too:

```clarity
// shop/translations/en.clarity: the default language
translation en
    customerCardTitle "Customer card"
    savedName "Saved {name}"
    yes "Yes"
    no "No"
```

```clarity
// shop/translations/nl.clarity
translation nl
    customerCardTitle "Klantkaart"
    savedName "{name} opgeslagen"
    yes "Ja"
    no "Nee"
```

- The file's name is its language: `nl.clarity` says `translation nl`, `nl-BE.clarity` says `translation nl-BE`.
- Every language file has the same keys: a key one file has and another lacks is an error. A key is camelCase, as names are, and is used anywhere in the project.
- The default language gives every key a text; an empty one is an error. Another language may leave a key empty (`savedName ""`): it shows the default's text until it is translated.
- A key's text may have slots, which code gives values by name: `message @savedName(name = customer.name)`. Each language keeps the slots of the default's text and may move them; a slot missing or added is an error, and so is a use that leaves one out or gives one the text does not have.
- A key no design or code uses is a warning; it can go. A text with letters written in quotes where people read it is an error that asks for a key.
- `yes` and `no` are what the screen shows for bools; `noPicture`, `loadPicture` and `removePicture` what an image part bound to a picture shows of its own. Every project gives them their texts.
- The application shows the language for the locale of the person using it: `nl-BE` takes `nl-BE.clarity`, else `nl.clarity`, else any other region of Dutch, else the default. The designer can show the canvas and Preview in any of the project's languages.
- A project written before texts had keys gets them with **Make texts keys**, or `clarity spec keys <folder>`: each text becomes a key named after where it is (`saveCaption`, `customerNameLabel`, `cardTitle`), its text the default language's value, and old translation files give the same keys their translations.
- A menu item or button with `on click -> language` lets the person choose another language while the application runs: it lists the project's languages, each by its own name (Nederlands, Deutsch), and the application remembers the choice for its next start.

### Pictures

The `images` folder of the module folder holds the fixed pictures of the application, such as a logo. An image part shows one by its file name:

```clarity
// records/images/logo.png
page AlbumBrowse: window
    image name image1 at 12,1 span 1 rows 1 source "logo.png"
```

- The picture must be in the `images` folder; a name that is not there is an error that lists what is.
- A picture is a `.png`, `.jpg`, `.gif`, `.bmp` or `.webp` file, which every stack shows without extra libraries.
- Every generated application carries the folder: next to the renderer on the web stacks, as embedded resources on Avalonia. The designer shows the pictures as they are in the folder, and shows them again when the folder changes.
- The picture keeps its proportions inside the part.

## 3. Modules

Every other `.clarity` file in the project folder is a module. It starts with `module`, then the modules it uses, then its declarations:

```clarity
module sales

use customers

entity Order
    customer: ref Customer
```

A module of entities that live in another database:

```clarity
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
| `engine Name`, `engine Name from "file.wasm"` | [9](#engines) |

## 4. Data: enums and entities

### Enums

An enum is a fixed list of values. Each value can have a label; without one, the label is derived from the name.

```clarity
enum CustomerKind
    Private label @customerKindPrivate
    Business label @customerKindBusiness
```

### Entities

An entity is a kind of record that is stored in a data source. It is the data dictionary: everything about a field is said once, here, and the windows, the database schema and the validation all follow from it.

```clarity
entity Customer
    name: text(80), required, label @customerNameLabel
    email: email, unique, label @customerEmailLabel
    kind: CustomerKind, default Private, label @customerKindLabel
    vat_number: text(14), help @customerVatNumberHelp, label @customerVatNumberLabel
    credit_limit: money, min 0, default 0, label @customerCreditLimitLabel
    active: bool, default true, label @customerActiveLabel
    since: date, label @customerSinceLabel
    notes: text, help @customerNotesHelp, label @customerNotesLabel
```

A field is `name: type`, followed by modifiers separated by commas. Every record also has an `id`, which Clarity adds itself. Code can read it, to tell two records apart (`other.id != loan.id`); only `save` sets it, and a new record has id 0.

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
| `duration` | a length of time in whole seconds, shown and typed as `3:45` or `1:02:03` everywhere |
| `image` | a picture (png, jpg, gif, bmp or webp), kept in the database, or with `store` as a file; see [Pictures in records](#pictures-in-records) |
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
| `label @key` | The label people see, by its key. Every field has one. |
| `help @key` | A help text, by its key, shown next to the field's input in windows. |
| `format name` | How the value is shown, by name; see [Formats](#formats). |
| `owned` | For a `list<...>`: the listed records live and die with this record. |
| `computed` | The value is computed, not entered. Accepted, but not used by the generated application yet. |
| `column "Name"` | The existing column the field is stored in, for an entity of an external database. |
| `store name` | For an `image`: its pictures are files in that file store of the project settings, not in the database. |

Each modifier appears at most once per field. The checker reports defaults and limits that do not fit the type, such as `min "x"` on a number, or a default that is not one of the enum's values.

### Pictures in records

An `image` field holds a picture, or none. Without `store`, the picture is kept in the database. With `store`, it is a file in a file store, a folder that the project settings name, and the column holds the file's name there:

```clarity
entity Album
    cover: image, help @albumCoverHelp, label @albumCoverLabel

entity Artist
    photo: image, store photos, label @artistPhotoLabel
```

```clarity
// records.clarity
    filestore photos "data/photos"
```

- An image part shows it: `image name cover at 1,1 span 4 rows 6 bind album.cover`. It shows its `noPicture` text while there is none, and, while the pointer is on it, a button to load a picture from a file and one to remove it. A label or help text can be for it.
- Reading records never carries their pictures, so a list of thousands stays light; the screen fetches each picture on its own when it shows.
- A picture is a png, jpg, gif, bmp or webp file, told by its first bytes; any other file is refused.
- In a file store, a picture's file is named after the SHA-256 of its bytes, so the same picture is kept once. Files are never deleted: a store can be a folder of photos that are not the application's alone.
- In code, a picture is compared with `none` and set to `none`: `if album.cover != none`, `album.cover = none`. Copying a picture from one record to another comes later. A query can find the records with or without one: `where a.cover == none`.
- A table or list does not show a picture column; an image part does.
- Moving a field's pictures between the database and a file store, or between `image` and another type, is a change that can lose data: pictures do not move by themselves.

### Formats

Dates, times, numbers and money show the way the person using the application writes them: the locale comes from their system (or, where it names none, the project's `language`). The value stored is the same in every locale.

| Type | Shown by default | `format` |
|---|---|---|
| `integer` | with thousands separators: `1,234,567`, `1.234.567` | `plain`: without them, for codes and years |
| `decimal(p, s)` | with `s` decimals: `-1,250.50`, `-1.250,50` | `plain`: without thousands separators; `percent`: `0.25` shows `25%` |
| `money` | with the project's currency and its decimals: `€1,250.50`, `€ 1.250,50`, `1.250,50 €` | `plain`: the amount without the currency |
| `date` | in digits: `10/17/2026`, `17-10-2026`, `17.10.2026` | `short` (the default), `long`: `17 October 2026`, `iso`: `2026-10-17` everywhere |
| `datetime` | the date and the time: `10/17/2026 9:30 AM`, `17-10-2026 09:30` | `short`, `long`, `iso` as for dates |

```clarity
entity Invoice
    issued: date, format long
    rate: decimal(5, 4), format percent
    year: integer, format plain
```

- A number input reads what is typed in the same locale: in Dutch `1.250,50` is 1250.5 and a point is a thousands separator. The currency sign, the percent sign and thousands separators may be typed or left out. Text that is no number marks the input and keeps the last value.
- Every stack shows the same thing: the cases in `conformance/formats.json` are what each renderer must show and read.
- Values in text that is shown follow the locale when they are in slots: `message @savedOn(day = order.placed)`. See [Texts by key](#texts-by-key).

### Migrations

A database follows the dictionary through migration steps. Every SQL data source has its own steps, in a folder of the project's `migrations` folder named after the source and numbered in the order the steps run: `migrations/main/0001.clarity`, `migrations/main/0002.clarity`. A step changes only the entities of its source. A step holds one `migration` block with a title and its changes:

```clarity
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

```clarity
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

```clarity
datasource erp sqlite "legacy/erp.db" external
```

Its entities live in a module with `source erp`, and they map onto the tables and columns that are there:

```clarity
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

```clarity
page CustomerForm: window
    title @customerFormTitle
    data customer: Customer
    data orders: list<Order>
    grid 12 columns
    on return: windowReturned

    label @nameLabel name label1 at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name
    table name orderTable at 1,3 span 12 rows 6 bind orders columns number, total
    button @saveButtonCaption name saveButton at 11,10 span 2 default on click: saveClicked
```

### Kinds

`page Name: kind`, where the kind is one of:

| Kind | Is |
|---|---|
| `window` | a window of its own, which can have a menu, toolbar, ribbon, sidebars and a footer |
| `dialog` | a modal window |
| `view` | content shown inside the app's frame |

Only a `window` has frame regions of its own.

An application has one desktop window, on every stack. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A desktop window of its own opens only where the design says so: a target marked `in new window` starts one, and a `dialog` may get one too (on Avalonia it does; the web stacks show it on top of the window that opened it).

### Settings

| Setting | Meaning |
|---|---|
| `title "text"` | The window's title. |
| `size W x H` | The size its desktop window opens at, in pixels, like `size 320 x 460`: 200 to 3840 wide and 150 to 2160 high. Without it a window opens at 1100 x 760. A desktop window takes the size of the window it shows first; a window opened in place keeps the desktop window's size. The designer shows a window at its size. |
| `data name: Entity` | One record the window works on. |
| `data name: list<Entity>` | A list of records the window works on. |
| `data name: type` | A plain value the window holds: `data shown: decimal`, `data pending: Operation`. The type is `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration` or an enum. It is no record and is never stored; it lives as long as the window. |
| `data name: type, default value` | A plain value that starts at `value`: `data fresh: bool, default true`. Without a default it starts empty: 0, empty text, false, no date, or the enum's first value. |
| `grid N columns` | The number of grid columns, 1 to 24. The default is 12. |
| `on open: handler` | Runs when the window has opened and its data is loaded. |
| `on close: handler` | Runs when the window is about to close. |
| `on return: handler` | Runs when a window this one opened has closed, and this one is on top again. This is where a list reloads after a form saved a record. |

The window's data is what its parts bind to and what its handlers work on. When a window is opened with `open CustomerForm with customer = ...`, the data is filled by name. Data that is not filled starts as a new, empty record, for a list as all records of the entity, and for a plain value at its default. A plain value cannot be read again with `reload`.

### The grid

Every part sits on the grid of the window or of the container it is in:

- `at col,row` is the top-left cell, counted from 1. Every part needs one.
- `span n` is the number of columns it covers. The default comes from the part's kind.
- `rows n` is the number of rows it covers. The default also comes from the part's kind.

Rows have a fixed height in the designer. In a running application the columns share the width of the window, and rows that hold something that grows (a table, a list, an image, a canvas, or a container with one of these inside) share the extra height. Everything else keeps its height.

The checker reports parts that start outside the grid, run past its last column, or overlap another part in the same container.

## 6. Parts

A part is one line: its kind, then an optional caption in quotes, then its properties.

```clarity
textbox name email at 4,2 span 6 bind customer.email placeholder "name@example.com" on change: emailChanged
```

Properties may be written in any order. The designer writes them in a fixed order, which is the order the tables below use.

### Properties every part has

| Property | Meaning |
|---|---|
| `"caption"` | The text it shows, right after the kind, for the kinds that have one. |
| `name n` | The part's name, unique in its window (sidebars, footer, menu, toolbar and ribbon included). Handlers refer to the part by it. The designer names new parts `button1`, `textbox2` and so on; templates name inputs after their field. |
| `at col,row`, `span n`, `rows n` | Place and size on the grid; see [the grid](#the-grid). |
| `tooltip "text"` | Shown when the pointer rests on the part. |
| `style name` | A style of the project's theme: its colors, and the size, font and alignment of its text. See [Themes](#themes). |
| `on event: handler` | Runs `handler` in the window's code behind when `event` happens. Each event is wired at most once per part. |
| `on event: handler(values)` | Runs `handler` with these values, one per parameter of the handler: `on click: digit(7)`, `on click: operation(Add)`. A value is a number, text in quotes, `true`, `false` or a value of the parameter's enum. |

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
| `form` | parts, on its own grid; binds to one record: `bind customer` | `foldable`, `foldable folded` | `submit` |
| `panel` | parts, on its own grid, in a frame with the caption | `foldable`, `foldable folded` | none |
| `tabs` | `tab @key` blocks, each with parts | none | `change` |
| `splitter` | exactly two `pane` blocks, each with parts | `horizontal` or `vertical` | none |

A panel or form with `foldable` has a header the person clicks to fold it to that header and unfold it again; `foldable folded` starts it folded. Code reads and sets whether it is folded as `part.folded`, so a handler can open the section a message is about. In the designer it stays open, to edit what it holds. On Avalonia it is an Expander; on the web stacks the renderer folds it.

**Data**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `table` | a list: `bind orders` | `columns field, field, ...`; `edit field [as switch], ...` | `select`, `open`, `change` |
| `list` | a list: `bind orders` | `show field, field, ...` | `select`, `open` |

**Graphics**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `image` | an `image` field, or nothing | `source "file"` for a fixed picture of the images folder | `click` |
| `display` | an integer, decimal or money: `bind shown` | `digits n`, 1 to 24, 12 without it | none |
| `canvas` | nothing | `units W x H`, 1 to 10000 each, 100 x 100 without it; `surface svg`, `surface bitmap` or `surface gpu`; `renderer Name` | `draw`, `click`, `pointer`, `key` |

`select` fires when another row is chosen, and `open` on a double-click or Enter. `enter` and `leave` fire when the focus moves into and out of an input.

`change` fires when the person changes an input's value to one that differs, and only then, on every stack:

- Typed text is taken when the input is left or Enter is pressed; typing alone is no change, and neither is leaving without typing or typing the same text again. In a `memo`, Enter is a new line and the text is taken when it is left.
- A number reads in the person's format. Text that is no number stays as typed and is marked, and the value stays; a cleared number is zero.
- A checkbox, switch, dropdown, radio group or date changes when the person ticks or picks another value. An enum offers no empty choice; a `ref` offers one first, for no record.
- What a handler sets, and a record read again, is never a change.
- Enter in an input of a `form` takes the input first, then runs the form's `on submit`.

### Editing in a table

`edit` names the columns whose cells are inputs, so a row can be changed where it is shown:

    table name grid at 1,2 span 12 rows 8 bind tasks columns title, done edit done on change: taskChanged

    handler taskChanged
        save grid.selected

- A `bool` column holds a checkbox, or a switch when it is written `as switch`: `edit title, done as switch`. An enum column is a dropdown of its values, and a `ref` column a dropdown of the records it can point at, as a dropdown in a form shows them. Dates are picked with a date picker, as in a date input. A moment (`datetime`) is a day from the date picker and a time typed on a 24-hour clock (`14:30`), in a cell and in a date input alike. Text and numbers are edited in place, like a text box or number input.
- An edited column must be one of the `columns`, of one of these types: text, `email`, `url`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, an enum or a `ref`. Pictures and durations are not edited in a cell.
- Only a value that differs is an edit: a checkbox or dropdown when it changes, a text or number cell when it is left or Enter is pressed. An edit changes the row on screen. Then the row is selected and `on change` runs, so `grid.selected` is the changed record. Saving it is up to the handler, as with a form: `save grid.selected`. When the record breaks a rule, `save` refuses it as anywhere else.
- A table that edits but has no `on change` is a warning: its edits are lost when the data is read again.

### Binding

`bind` connects a part to the window's data:

- An input or `text` binds to one field of a single record: `bind customer.name`, or to a plain value by its name: `bind shown`. The type must be one the part accepts; a checkbox needs a `bool`. A plain `decimal` shows the digits it has.
- A `table` or `list` binds to a list: `bind orders`. Its `columns` or `show` name fields of the listed entity.
- A `form` binds to a single record: `bind customer`.

What the user types goes into the data, and handlers read and change the data by name. A part that cannot be bound, such as a label, may not have `bind`. An input that is not bound yet is allowed while designing.

### Labels, help texts and forms

A label, its input and the input's help text are three separate parts. `for` ties a label or a help text to an input by the input's name:

```clarity
form name details at 1,1 span 12 rows 3 bind customer
    label @nameLabel name nameLabel at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name placeholder @fullName
    help name nameHelp at 10,1 span 3 for name
    label @kindLabel name kindLabel at 1,2 span 3 for kind
    radio name kind at 4,2 span 6 bind customer.kind
```

- **A label** shows its caption. Clicking it focuses its input. When the input is bound to a `required` field, the label shows a marker. A label is never bound to data; to show a value, use `text`.
- **A help text** shows its caption, or, when it has none, the `help` of the field its input is bound to. When a handler sets the input's `error`, the help text shows that message instead, and the input is marked.
- **`for`** must name an input in the same window.
- **A form** binds to one record and holds the inputs of that record, nested in panels and tabs if needed. Every input inside it binds to that record, and a form cannot contain another form. Pressing Enter in one of its inputs fires `submit`; when `submit` is not wired, Enter clicks the window's default button. A form without a caption has no frame, so its grid lines up with the grid around it.

### Input and button properties

| Property | On | Meaning |
|---|---|---|
| `placeholder @key` | textbox, memo, password, number | Shown while the input is empty. |
| `readonly` | inputs | Shows the value, which cannot be changed. Handlers can change this. |
| `taborder n` | inputs, buttons | Position in the keyboard order, from 1. Parts without one follow in reading order. |
| `key "Ctrl+S"` | buttons | The key that clicks the button. |
| `default` | buttons | Enter clicks this button. One per window. |
| `cancel` | buttons | Escape clicks this button. One per window. |
| `horizontal`, `vertical` | radio, splitter | Side by side or stacked. |

A key is written as modifiers, then one key, joined by `+`: `"Ctrl+S"`, `"Ctrl+Shift+F2"`, `"Alt+Delete"`. The modifiers are `Ctrl`, `Alt`, `Shift` and `Meta`. The key is a letter, a digit, one of `+ - * / . , =`, `F1` to `F12`, or one of `Enter`, `Escape`, `Tab`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`; `"Ctrl++"` is Ctrl and `+`. A key without Ctrl, Alt or Meta that is a character (`"7"`, `"+"`) is what is typed, from the keyboard or the number pad, on any keyboard layout; while the person types in a text input it is their typing and presses nothing.

### Clicking to open a window

A button, an image, a menu item, or a toolbar or ribbon button can open a window without code:

```clarity
button @allCustomersCaption name allCustomers at 1,12 span 3 on click -> CustomerBrowse
```

`on click -> Target` opens the window `Target`, in place, on top of the window the click is in. `on click -> Target in new window` opens it in a desktop window of its own instead: the one way, besides a `dialog`, that an application gets a second desktop window. Closing the last window there closes that desktop window, and the window that opened it gets its `on return`. In menus, toolbars and on buttons, `close` closes the current window and `quit` ends the application. In menus and toolbars, `language` lets the person choose the language the application shows (see [Translations](#translations)), and `theme` one of the themes it offers (see [Themes](#themes)). A click either opens a window this way or runs a handler (`on click: handler`), not both.

### Containers

A container has its own grid, as many columns wide as its `span`. Its parts are indented below it, and their positions count from its own top-left cell:

```clarity
tabs at 1,5 span 12 rows 7
    tab @ordersTab
        table at 1,1 span 12 rows 5 bind orders columns number, total
    tab @notesTab
        text at 1,1 span 12 bind customer.notes
splitter at 1,1 span 12 rows 10 vertical
    pane
        list at 1,1 span 12 rows 4 bind customers show name
    pane
        panel @detailsCaption name details at 1,1 span 12 rows 4
            text at 1,1 span 6 bind customer.name
```

`tabs` need at least one `tab`, and a `splitter` has exactly two `pane` blocks.

## 7. The app and its frame

A project has one `app` block. It names the main window and declares the frame every window shares: a menu, a toolbar, a ribbon, sidebars and a footer.

```clarity
app Shop
    title @shopTitle
    main CustomerCard
    data session: Session
    on start: appStarted
    menu name mainMenu
        item @fileItem name fileItem
            item @newCustomerItem name newCustomerItem key "Ctrl+N" on click -> CustomerForm
            separator
            item @quitItem name quitItem key "Ctrl+Q" on click -> quit
    toolbar name mainToolbar
        button @customersButtonCaption name customersButton icon "list" on click -> CustomerBrowse
        separator
        button @ordersButtonCaption name ordersButton on click: ordersClicked
    ribbon name mainRibbon collapsible
        tab @homeTab name homeTab
            group @customersGroup name customersGroup
                button @newButtonCaption name newButton icon "add" large on click -> CustomerForm
                button @browseButtonCaption name browseButton icon "list" on click -> CustomerBrowse
    sidebar left name leftSidebar width 192
        list name list1 at 1,1 span 3 rows 10 bind recent show name
    footer
        status @shopStatus
        text name text1 at 10,1 span 3 bind session.user
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
| `menu` | at the top | `item @key` lines, which can hold more items below them (a submenu), and `separator` |
| `toolbar` | at the top, below the menu | `button` and `separator` |
| `ribbon` | at the top | `tab @key` blocks with `group @key` blocks of `button` and `separator` |
| `sidebar left` or `sidebar right` | at that side, `width` pixels wide (240 without it) | parts, on its own grid of `grid n columns` (3 without it) |
| `footer` | at the bottom | an optional `status "text"` and parts on a one-row grid of 12 columns |

Every menu item and button has a name, in camelCase like a part's (`item @newCustomerItem name newCustomerItem`); a separator has none. Its name is unique among the window's parts, including the menu and the toolbar themselves. The designer names a new one after its caption and kind, as `newCustomerItem`, which keeps it apart from data and parts of the same word.

The menu and the toolbar are named parts: `menu name mainMenu`, `toolbar name mainToolbar`. Code hides one with its `visible` and turns it off with its `enabled`.

A ribbon is a named part too, and so are its tabs and groups: `ribbon name mainRibbon collapsible collapsed`, `tab @homeTab name homeTab`, `group @clipboardGroup name clipboardGroup`. With `collapsible` the person double-clicks a tab to fold the ribbon to its tabs, and opening a tab unfolds it; `collapsed` starts it folded. Code reads and sets the open tab as `part.tab` (from 1; a hidden tab gives way to the first visible one) and the folding as `part.collapsed`, and hides a tab or group with its `visible`.

A sidebar is a named part: `sidebar left name nav width 280 min 200 max 480 resizable collapsible collapsed maximizable grid 4 columns`. Its width is in pixels (240 without `width`, 100 to 1600). With `resizable` the person drags its inner edge between `min` and `max`; with `collapsible` a button folds it to a narrow strip and back, and `collapsed` starts it folded; with `maximizable` a button lets it fill the window. `grid n columns` gives its own grid (3 without it). Code reads and sets `part.collapsed` and `part.maximized`. The old form `sidebar left span 3` is an error that **Update the sidebars** fixes.

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

```clarity
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
| `title @nightOwlTheme` | The name people see when they choose the theme, by its key. A theme the application offers needs one. |
| `mode light`, `mode dark`, `mode system` | The theme is always light, always dark, or follows the system. |
| `color name "#rrggbb"` | A color, written `#rrggbb` or `#rgb`; `dark "#rrggbb"` after it gives its value in dark mode. |
| `font "Families"`, `font heading ...`, `font mono ...` | The fonts of text, headings and code, as a list of families: `"Inter, sans-serif"`. |
| `size n`, `size small n`, `size heading n` | Text sizes in pixels: the text, small text (help, captions) and headings. |
| `radius n`, `radius input n`, `radius button n`, `radius panel n` | Corner radii in pixels: of everything, or of one kind. |
| `padding n`, `border n` | The space inside inputs and buttons, and the width of borders, in pixels. |
| `shadow none`, `shadow flat`, `shadow raised` | The shadow under menus, popups and messages. |
| `part kind [state] look` | How a kind of part looks: `button`, `input`, `header` (of a table), `tab` or `menu`; with `hover`, `pressed` or `focus`, in that state. |
| `style name look` | A named style that parts pick with `style name`. |

A look (of a part or a style) sets `background`, `text` and `border` to a color of the theme by name or written out (a border can be `none`), `radius n`, and `weight bold` or `weight normal`, separated by commas. A style also sets how its text shows: `size n` in pixels (6 to 200), `font` with a font of the theme (`text`, `heading` or `mono`), and `align left`, `align center` or `align right`; aligned text keeps an input's padding from the edge. A calculator's display:

```clarity
theme Calculator
    color lcd "#c7d3a8" dark "#1d2618"
    style display background lcd, size 40, font mono, align right
```

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

```clarity
code CustomerForm
    handler kindChanged
        // Only business customers have a VAT number.
        vatNumber.enabled = customer.kind == Business

    handler saveClicked
        name.error = ""
        if customer.name == ""
            name.error = @nameIsRequired
            return
        save customer
        message @savedName(name = customer.name)
        close
```

- `handler name` declares a handler. It runs when an event wired to it happens.
- `handler name(param: type, ...)` declares a handler that takes values, which a part gives it: `button "7" on click: digit(7)` runs `handler digit(d: integer)` with `d` 7. Each parameter is `text`, `integer`, `decimal`, `money`, `bool` or an enum. The values must fit: as many as it has parameters, each of its type. Events of the window itself (`on open`) give no values, so their handlers take none.
- A handler that is wired but not written is an error. A handler that is written but not wired anywhere is a warning.
- Handlers run in the backend. The window sends its data with the event, the handler runs, and the window shows what changed: data, part properties, messages, and windows to open or close.

### What a handler sees

- **The window's data**, by its `data` names: `customer`, `orders`.
- **The window's parts**, by their names: `save`, `vatNumber`.
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
| `checked` | checkboxes and switches that are not bound | bool: whether it is on | yes |
| `selected` | tables, lists | the record of the selected row | no |
| `folded` | foldable panels and forms | bool: whether it is folded, by the person or by code | yes |
| `collapsed` | collapsible sidebars | bool: whether it is folded to its button | yes |
| `maximized` | maximizable sidebars | bool: whether it fills the window | yes |
| `tab` | ribbons | integer: the open tab, counted from 1 | yes |
| `collapsed` | collapsible ribbons | bool: whether it is folded to its tabs | yes |

A bound input is changed through its data: `customer.name = "Ada"` changes the text box bound to `customer.name`.

### Drawing on a canvas

A canvas is drawn by its draw handler, wired with `on draw`. The handler draws shapes in the canvas's own units (`units 200 x 100`, 100 x 100 without it), x to the right and y down; the canvas scales them to fit the part, in proportion and centered.

```clarity
page Main: window
    data speed: integer, default 80
    canvas name dial at 1,1 span 12 rows 7 on draw: drawDial
    button @button1Caption name button1 at 7,8 span 6 key "+" on click: change(10)

code Main
    handler drawDial
        let angle = 210 - speed * 240 / 200
        dial.clear("panel")
        dial.ring(50, 50, 42, "border", 3)
        dial.line(50, 50, 50 + 34 * cos(angle), 50 - 34 * sin(angle), "accent", 3)
        dial.text(50, 72, "{speed} km/h")

    handler change(by: integer)
        speed = speed + by
```

| Shape | Values | What it draws |
|---|---|---|
| `canvas.clear(color)` | | Fills the whole canvas. |
| `canvas.line(x1, y1, x2, y2, color, width)` | width 1 when left out | A line with round ends. |
| `canvas.rect(x, y, width, height, color)` | | A filled rectangle. |
| `canvas.circle(x, y, r, color)` | | A filled circle around x, y. |
| `canvas.ring(x, y, r, color, width)` | width 1 when left out | A circle's outline. |
| `canvas.text(x, y, text, color, size)` | color `"text"` and size 10 when left out | Text centered on x, y; slots are filled in the person's format, like a message. |

- A color is a color of the theme by name (`"accent"`, `"text"`, a color the theme declares) or written out (`"#101810"`); a name the theme does not have is its text color.
- The draw handler runs when the window opens and after every other handler of the window, each time on an empty drawing, so the canvas always shows the data as it is. Only a canvas's own draw handler draws on it, and the handlers it calls that nothing else runs: `draw_mark(5, 100, 100)` can draw one square of a board. A handler that an event or another handler also runs cannot draw, since its shapes would land outside a drawing.
- `on click: handler` on a canvas runs the handler when the canvas is clicked, with `canvas.x` and `canvas.y` saying where, in the canvas's units, rounded down. They are read, not set, and 0 before the first click. A click beside the drawing, where the part shows none of it, runs nothing. The window draws the canvas anew after the handler, as after any other.

```clarity
page Game: window
    data status: text
    canvas name board at 1,1 span 12 rows 8 units 300 x 300 on draw: drawBoard on click: boardClicked

code Game
    handler drawBoard
        board.clear("panel")

    handler boardClicked
        let square = 1
        if board.x >= 200
            square = 3
        else if board.x >= 100
            square = 2
        if board.y >= 200
            square = square + 6
        else if board.y >= 100
            square = square + 3
        status = "Square {square}"
```
- `sin(degrees)` and `cos(degrees)` give a decimal, rounded to 10 decimals.

### Data events

An entity's code block holds its data events: code that runs whenever one of its records is saved or deleted, from any window, action or other event. The block has the entity's name and is in the same file as the entity.

```clarity
code Order
    before save
        if order.status == Approved and not within_credit(order)
            refuse @orderOverCreditLimit(number = order.number)

    after save
        log_change("saved " + order.number)

    before delete
        if order.status != Pending
            refuse @onlyPendingDeletable
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

```clarity
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
- **`effects`** lists what an action does to data: `db.read`, `db.write`, `files.read` and `files.write`, comma separated. An action that uses `save` or `delete`, or that calls an action with `db.write`, must declare `db.write`; one that lists or reads files declares `files.read`, and one that writes a file `files.write`. A function cannot reach files at all; handlers can, as they can save.
- **A function** has no effects. It cannot save or delete, call actions, show messages or open windows. That makes it safe to call anywhere, including in conditions.
- **An action** can change data. Neither functions nor actions talk to the screen: `message`, `open`, `close` and part properties belong in handlers.
- A function or action with a result returns a value on every path; the checker reports a path that does not.

### Engines

An engine is a processing module that every stack runs the same way: pricing, scheduling, a solver, the logic of a game. It is written in Clarity (see [An engine written in Clarity](#an-engine-written-in-clarity)) or compiled to WebAssembly from another language. A module declares a WebAssembly engine with its file and the functions it has; code calls those functions like its own.

```clarity
engine Pricing from "pricing.wasm"
    uses clock, random
    says @outOfStock
    limit 2 seconds, 64 MB
    state per app
    on open
    function total(lines: list<Line>) -> money
    function size_of(count: integer) -> Size
    function discounted(order: Order) -> Order
```

```clarity
function order_total(order: Order) -> money
    return Pricing.total(order.lines)
```

- **The file** is in the `engines` folder of the module folder (`shop/engines/pricing.wasm`). Every generated application carries it inside the program.
- **Each function** has a signature like a function's, and always a result: `-> type`. Parameters and results are values (`text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration`), `bytes`, enums, records and `list<Entity>`; not `file` or `list<file>`. An engine that works on a document gets its bytes: `Reader.pages(read bytes of f)`.
- **A call** is `Engine.function(args)`. The checker types it against the declaration, as it does a call to a function. Handlers, functions and actions can all call an engine: it has no effects.
- **The engine computes; the application does all input and output.** An engine reaches no database, file, network or screen. Code reads the records, hands them in, and saves what comes back: a record an engine returns is a copy, saved only when code saves it.
- **`uses`** names the services of Clarity the engine may call, comma separated: `clock` (the time), `random` (a random number), `log` (a line in the application's log) and `language` (the language the person picked, such as `nl`). Each is a function the file imports from module `clarity`; a file that imports anything else, or a service the declaration does not name, is refused. Without `uses`, an engine imports nothing.
- **`says`** lists the keys of the texts the engine gives people to read, comma separated: `says @youWin, @aDraw`. The engine returns a key (`"youWin"`), whose text the language files give in the person's language; an engine's refusal by a key shows the same way. A text an engine gives that is no key shows as it is.
- **`limit`** says what one call may take: `limit 2 seconds`, `limit 64 MB` or both, `limit 2 seconds, 64 MB`. A call that runs longer is stopped, and fails with an error that says so; the engine never gets more memory than its limit. Without a limit line, a call may take 5 seconds and the engine 256 MB. A time limit is between 1 and 3600 seconds, a memory limit between 1 and 4096 MB.
- **State.** Without a state line, each call starts the engine afresh, so nothing carries over from one call to the next. `state per app` keeps one instance for the whole application: what the engine holds in its memory carries from call to call, such as a loaded dictionary. `state per window` keeps an instance for each open window, made at its first call there and dropped when the window closes: a game's board, a document being edited. Only a window's handlers call an engine with state per window, each in its own window. A call that fails or runs past its limit drops the instance, since its memory may be half changed; the next call starts a new one. A refusal keeps it.
- **`on open`** runs the engine's own `clarity_open` whenever an instance is made: once per instance with state, before every call without. There is no `on close`: an engine cannot save or show anything, so a window that must keep what an engine holds asks it in its own `on close` handler and saves the answer.
- **Checked twice.** The designer reads the file and reports a file that is not in the folder, is no WebAssembly, imports what the engine does not use, or lacks a declared function. The application checks it again when it starts, and stops there rather than at the first call.
- **An error** that the engine gives back (its own refusal) reaches code as a refusal, in the engine's words. An engine that fails while it runs is an error that names the engine.
- The declaration is visible to the modules that `use` the module, like its functions. Two engines of one module have different names, and an engine has each function once.

#### An engine written in Clarity

An engine without `from` is written in Clarity: what it keeps between calls, and its functions with their code. Every stack generates it as code of its own, like a function, so it needs no file and no other language.

```clarity
engine Board
    uses random
    state per window
    keep squares: list<integer> = [0, 0, 0, 0, 0, 0, 0, 0, 0]
    keep moves: integer = 0

    function play(square: integer) -> text
        if squares[square] != 0
            refuse @thatSquareIsTaken
        squares[square] = 1
        moves = moves + 1
        return @yourMove

    function mark(square: integer) -> integer
        return squares[square]
```

- **`keep name: type = value`** is a value the engine keeps from call to call, with what it starts as. Its functions read and set it by name. A keep is a plain value or a list of them (`list<integer>`); the value it starts with is written out.
- **`state`** works as for any engine: none gives every call fresh values, `state per app` one set for the whole application, `state per window` one set for each open window, dropped when the window closes.
- **A call changes nothing when it fails.** A function works on a copy of what the engine keeps, which it keeps only when the call succeeds; a refusal or an error leaves the values as they were.
- **Its functions** are written as functions are, with `require` and `ensure` if wanted, and call each other by name (`answer()`). Code outside calls them as `Board.play(5)`.
- **It keeps to itself.** An engine has no window, data, parts or database: `message`, `open`, `save`, `find` and the like are errors in it. It returns what the screen should show, and its texts are keys, translated like the project's own.
- **`uses random`** gives it `random(n)`: a whole number from 1 to `n`. No other service is needed in Clarity: the language the person picked is already how its texts show.

How to build an engine in another language, and what it exports, is in the manual: [Engines](manual/code.engines.md).

## 10. Statements and expressions

### Statements

One statement per line; `if`, `else` and `for` take an indented block.

| Statement | Meaning | Where |
|---|---|---|
| `let name = expr` | A local, with the type of the expression. | everywhere |
| `let name: type = expr` | A local with a declared type. | everywhere |
| `target = expr` | Sets a local, a data field (`customer.credit_limit`), an item of a list (`squares[3] = 1`) or a part property (`save.enabled`). | everywhere; part properties in handlers |
| `if expr` / `else if expr` / `else` | Runs a block when the condition, which must be bool, is true. | everywhere |
| `for item in list` | Runs the block once per item of a list. | everywhere |
| `name(args)` | Calls a function or action on its own line. | everywhere |
| `handler_name()`, `handler_name(values)` | Runs another handler of the same window or app, with values for its parameters. | handlers |
| `canvas.shape(values)` | Draws a shape on a canvas; see [Drawing on a canvas](#drawing-on-a-canvas). | its draw handler |
| `return` | Leaves the handler or action. | handlers, actions without a result |
| `return expr` | Leaves with a result. | functions and actions with a result |
| `message expr` | Shows a message. `message info`, `message warning` and `message error` set its level; the default is info. | handlers |
| `open Window` | Opens a window. | handlers |
| `open Window with name = expr, ...` | Opens a window and fills its data by name. | handlers |
| `close` | Closes this window. | handlers |
| `save expr` | Stores a record in its data source. A new record gets its id. | handlers, actions, data events |
| `delete expr` | Removes a record from its data source. | handlers, actions, data events |
| `refuse expr` | Stops with this text (or value) as the error; a save or delete it is part of does not happen. See [Data events](#data-events). | everywhere |
| `reload data` | Reads the window's data again: a list loads all its records again, and a record loads again by its id (a new record stays as it is). A table or list showing a list read again has no row selected afterwards, as when the list is given new rows (`orders = find ...`): they are other records. | handlers |
| `write expr to store "path"` | Writes text as a file of a file store, at a path inside it, making its folders. See [Files](#files). | handlers, actions with `files.write` |

### Expressions

| Expression | Meaning |
|---|---|
| `12`, `9.95`, `"text"`, `true`, `false` | literals |
| `@savedName(name = customer.name)` | a text by its key, its slots filled by name; see [Texts by key](#texts-by-key) |
| `"{album.title}.txt"` | text with slots, filled plainly; see [Text with slots](#text-with-slots) |
| `customer`, `customer.name`, `grid.selected.total` | names and paths |
| `[1, 2, 3]`, `["a", "b"]` | a list of values of one type; an empty one needs its type, as in `let squares: list<integer> = []` |
| `squares[3]` | an item of a list, counted from 1; a position outside the list is an error |
| `Business`, `CustomerKind.Business` | enum values |
| `none` | the empty value of a date, a date and time or a reference field: `loan.returned_on == none`, `order.customer = none`. Nothing else is ever none; empty text is `""` |
| `name(arg, ...)` | a call to a function or action; `count(list)` gives the number of items, `round(value, places)` a decimal or money with at most `places` decimals, rounded half away from zero (`round(2.345, 2)` is 2.35), `sin(degrees)` and `cos(degrees)` a decimal rounded to 10 decimals, `today()` the date and `now()` the date and time where the application runs |
| `(expr)` | grouping |
| `-x` | negation |
| `a * b`, `a / b` | multiplication and division |
| `a + b`, `a - b` | addition and subtraction; `+` also joins text |
| `a == b`, `a != b`, `a < b`, `a <= b`, `a > b`, `a >= b` | comparison |
| `x in [a, b, c]` | whether `x` equals one of the values |
| `a contains b` | whether text `a` contains text `b`, ignoring case |
| `find c in Entity where ... order by ...` | the records that meet a condition; see [Queries](#queries) |
| `new Entity` | a new record of the entity, with its defaults; `save` stores it |
| `files f in store where ... order by ...` | the files of a file store that meet a condition; see [Files](#files) |
| `read text of f` | the text a file holds |
| `read bytes of f` | the contents of a file as they are; see [Files](#files) |
| `not a` | negation of a bool |
| `a and b`, `a or b` | both, either |

From tightest to loosest: member access (`.`) and items (`[i]`), unary `-`, `*` and `/`, `+` and `-`, comparisons, `in` and `contains`, `not`, `and`, `or`.

### Texts by key

A text people read is a key, and its slots get their values by name, so a translator can move them and each value shows in the person's format:

```clarity
code OrderForm
    handler approveClicked
        approve(order)
        message @orderApproved(number = order.number, total = order.total)
        approve_button.caption = @approvedOn(day = order.ordered_on)
```

```clarity
translation en
    orderApproved "Order {number} of {total} is approved"
    approvedOn "Approved on {day}"
```

- A key is `@` and its name; with slots, `@key(slot = value, ...)` gives each slot of its text a value. Every slot the default language's text has gets one, and no other.
- A value is checked like any expression; a whole record or list is no value to show, so `customer = order.customer` is an error that suggests one of its fields.
- A value shows as its field shows it: money with its currency, a date in the locale and the field's `format`, an enum value by its label. A local or a parameter shows as its type does.
- A keyed text is a text like any other: code keeps it in a local, passes it to a function, returns it, and puts it in a text value.
- Where it goes straight to the screen (a message, a refusal, a caption, an error, a text value a handler sets), the backend sends the key with each value, and the screen shows its text in the person's language and fills the slots in the person's format.
- Anywhere else, such as a function's result, the backend takes the key's text in the language the screen picked and fills its slots plainly.
- Text joined with `+` in a message, a refusal, a caption or an error is a warning that suggests a key with slots, since such text cannot be translated and its values do not follow the locale.

```clarity
function credit_note(order: Order) -> text
    if within_credit(order)
        return @withinCredit(number = order.number)
    return @overCredit(number = order.number)
```

### Text with slots

A string in code may have slots too: `"{album.title}.txt"`, `{{` and `}}` for braces. It is filled plainly, in no language, for what people do not read: a file's name, a line written to a file, a value to compare. Where people read it, it is a key.

### Queries

`find` reads the records of an entity that meet a condition, in an order. The result is a list, used like any other: assign it to a window's list data, loop over it with `for`, count it, or return it.

```clarity
code Customers
    handler filterChanged
        customers = find c in Customer where c.name contains nameFilter.text and c.active order by c.name

    handler cardOpened
        orders = find o in Order where o.customer == customer and o.total > 100 order by o.total desc
```

- `find c in Customer` names each record `c`. The name exists only inside the query, so `c.name` is never confused with a part or local that is also called `name`.
- `where` is optional. Without it, `find` reads every record.
- The condition combines, with `and`, `or`, `not` and parentheses, comparisons between a field of the record and a value: `c.name == x`, `c.total > 100`, `c.name contains x`, `c.kind in [Private, Business]`. A bool field stands on its own: `c.active`, `not c.active`. A reference field is compared with a record: `o.customer == customer`. A date or reference field compared with `none` finds the records where it is empty: `l.returned_on == none`.
- The value side can be anything that does not use the record: a literal, a local, the window's data, a part property.
- A condition can go through references to the fields of the record they refer to: `o.customer.name contains x`, `t.album.artist.country == "UK"`. A record whose reference is empty does not meet such a condition.
- `order by` takes one or more fields, each optionally followed by `desc`, and may go through references too: `order by a.artist.name, a.title`. Without it, records come in the order they were stored.
- A query reads one database, so it cannot go through a reference to an entity in another data source.
- The database does the work: the condition becomes a query with parameters, so values never become part of the SQL text.

### Files

Code reaches files only in the file stores of the project settings, the folders that also keep pictures (`filestore photos "data/photos"`). What a program touches on disk is in its settings, and a store that differs per machine comes from an environment variable.

```clarity
// Photos dropped in the photos folder, named after the artist
// (Miles Davis.jpg), become the photos of artists that have none.
action find_artist_photos()
    effects db.write, files.read
    let found = files f in photos where f.extension == "jpg" or f.extension == "png"
    let artists = find a in Artist where a.photo == none
    for artist in artists
        for f in found
            if f.name == artist.name + "." + f.extension
                artist.photo = f
                save artist

action export_collection()
    effects files.write
    let lines = ""
    for album in find a in Album order by a.title
        lines = lines + album.title + "\n"
    write lines to exports "collection.txt"
```

- `files f in photos` lists every file of the store and its folders, leaving out files and folders whose name starts with a dot, sorted by path. `where` and `order by` work as in `find`, on the details of a file, and run in memory: the condition may be any bool.
- A file has `name` (`Blue Train.jpg`), `path` inside the store (`2024/Blue Train.jpg`, with `/` between folders), `extension` in lower case without the dot (`jpg`), `size` in bytes and `modified` (a date and time, in UTC). A list of files is `list<file>`, one file `file`.
- `read text of f` gives the text a file holds, as UTF-8. `write t to exports "lists/albums.txt"` writes one, making its folders. A path that would leave the store is refused.
- `read bytes of f` gives the contents of a file as they are, of type `bytes`: a picture, a PDF, anything that is no text. `write b to exports "copies/cover.png"` writes bytes as they are. `b.size` is how many bytes there are. Bytes are no window data: the screen has nothing to show of them, so code keeps them while it works and hands them to engines and functions.
- A file becomes a record's picture with `photo.picture = f`. In the field's own store (`store photos`) it is the file itself: the column holds its path, nothing is copied, as a catalog of photos wants. Kept anywhere else, the picture is a copy of the file. A file that is no picture is refused.
- Listing, reading and writing are effects, `files.read` and `files.write`, which an action declares; a function cannot reach files.

### Types

The types of values follow the data dictionary:

| Type | Values of fields of type |
|---|---|
| `text` | text, email, url |
| `integer` | integer, id |
| `decimal` | decimal, money |
| `bool` | bool |
| `date`, `datetime` | date, datetime |
| `duration` | duration; durations add up (`+`, `-`), scale by a number (`*`, `/`), compare, and take a number of seconds: `let total: duration = 0` |
| an enum | that enum |
| a record | an entity, a `ref` |
| a list of records | `list<Entity>`, a list in the window's data |
| a list of values | `list<text>`, `list<integer>`, `list<decimal>`, `list<bool>`: in code and in what an engine keeps; a list is a value, so a copy changes on its own |
| `image` | an `image` field; compared with and set to `none`, or set to a file |
| `file`, `list<file>` | a file of a file store, and a list of them; see [Files](#files) |
| `bytes` | the contents of a file as they are (`read bytes of f`); `size` is their number |

An integer is accepted where a decimal is expected. Nothing else converts by itself. The checker reports unknown names, wrong types, setting something that cannot be set, and calls with the wrong arguments.

## 11. A complete example

A project file and one module: a customer list with a filter that opens a form, the form that saves a customer, and a data event that clears the VAT number of a private customer.

`shop.clarity`:

```clarity
project Shop
    stack rust web desktop
    datasource main sqlite "data/shop.db"
```

`shop/customers.clarity`:

```clarity
module customers

enum CustomerKind
    Private label @customerKindPrivate
    Business label @customerKindBusiness

entity Customer
    name: text(80), required, label @customerNameLabel
    kind: CustomerKind, default Private, label @customerKindLabel
    vat_number: text(14), help @customerVatNumberHelp, label @customerVatNumberLabel
    notes: text, label @customerNotesLabel

app Shop
    main CustomerBrowse

page CustomerBrowse: window
    title @customerBrowseTitle
    data customers: list<Customer>
    on return: windowReturned

    label @nameFilterLabel name label1 at 1,1 span 2 for nameFilter
    textbox name nameFilter at 3,1 span 4 placeholder @nameFilterPlaceholder on change: filterChanged
    table name grid at 1,2 span 12 rows 8 bind customers columns name, kind on open: openClicked
    button @button1Caption name button1 at 9,10 span 2 on click -> CustomerForm
    button @openButtonCaption name openButton at 11,10 span 2 on click: openClicked

page CustomerForm: window
    title @customerFormTitle
    data customer: Customer

    form name customerForm at 1,1 span 12 rows 4 bind customer
        label @nameLabel name label1 at 1,1 span 3 for name
        textbox name name at 4,1 span 6 bind customer.name
        help name help1 at 10,1 span 3 for name
        label @kindLabel name label2 at 1,2 span 3 for kind
        radio name kind at 4,2 span 6 bind customer.kind on change: kindChanged
        label @vatNumberLabel name label3 at 1,3 span 3 for vatNumber
        textbox name vatNumber at 4,3 span 4 bind customer.vat_number
        help name help2 at 8,3 span 5 for vatNumber
        label @notesLabel name label4 at 1,4 span 3 for notes
        textbox name notes at 4,4 span 9 bind customer.notes
    button @saveButtonCaption name saveButton at 9,6 span 2 key "Ctrl+S" default on click: saveClicked
    button @closeFormCaption name closeForm at 11,6 span 2 cancel on click: closeClicked

code Customer
    before save
        // Only business customers keep a VAT number, whichever window saves them.
        if customer.kind == Private
            customer.vat_number = ""

code CustomerBrowse
    handler windowReturned
        // The form may have saved a customer.
        filterChanged()

    handler filterChanged
        customers = find c in Customer where c.name contains nameFilter.text order by c.name

    handler openClicked
        open CustomerForm with customer = grid.selected

code CustomerForm
    handler kindChanged
        vatNumber.enabled = customer.kind == Business

    handler saveClicked
        name.error = ""
        vatNumber.error = ""
        if customer.name == ""
            name.error = @nameIsRequired
            return
        if customer.kind == Business and customer.vat_number == ""
            vatNumber.error = @neededForBusinessCustomers
            return
        save customer
        message @savedName(name = customer.name)
        close

    handler closeClicked
        close
```

The full shop example, with orders, a menu and a ribbon, is `examples/shop.clarity`.

## 12. Grammar summary

A compact view of the grammar. `[x]` is optional, `x*` is zero or more, `a | b` is one of them, and an indented line is a line in the block above it. Words in quotes are written as shown.

```text
project-file  = "project" Name
                    setting*
setting       = "title" String | "stack" word word word+ | "output" String | "language" String | "currency" word | "theme" Name+
              | "datasource" name kind String ["external"] ["readonly"]
                    ["auth" kind String]

module-file   = "module" name
                ("use" name)*
                ["source" name]
                declaration*
declaration   = enum | entity | page | app | theme | code | function | action | engine

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
page-setting  = "title" String | "size" n "x" n
              | "data" name ":" (Entity | "list" "<" Entity ">" | type-ref ["," "default" value])
              | "grid" n "columns" | "on" ("open" | "close" | "return") ":" handler
part          = kind [String] property*
                    (part | "tab" String | "pane")*        // containers only
property      = "name" name | "at" col "," row | "span" n | "rows" n | "bind" path
              | "for" name | "placeholder" String | "tooltip" String | "style" name | "readonly" | "taborder" n
              | "key" String | "default" | "cancel" | "horizontal" | "vertical"
              | "columns" names | "edit" name ["as" "switch"] ("," name ["as" "switch"])* | "show" names | "source" String | "surface" word | "renderer" Name
              | "on" event ":" handler ["(" [value ("," value)*] ")"] | "on" "click" "->" Target ["in" "new" "window"]

app           = "app" Name
                    ("title" String | "main" Window | "data" ... | "on" "start" ":" handler | region)*
region        = "menu" | "toolbar" | "ribbon" | "sidebar" ("left" | "right") ["span" n] | "footer"
command       = ("item" | "button" | "separator") [String]
                ("name" name | "icon" String | "key" String | "large" | "on" "click" (":" handler | "->" Target ["in" "new" "window"]))*

code          = "code" (Window | App)
                    ("handler" name ["(" [param ("," param)*] ")"]
                        statement*)*
              | "code" Entity
                    (("before" | "after") ("save" | "delete")
                        statement*)*
function      = ("function" | "action") name "(" [param ("," param)*] ")" ["->" type-ref]
                    ("require" expr | "ensure" expr | "effects" effect ("," effect)*)*
                    statement*

engine        = "engine" Name "from" String
                    ["uses" service ("," service)*]
                    ["says" Key ("," Key)*]
                    ["limit" n ("seconds" | "MB") ["," n ("seconds" | "MB")]]
                    ["state" "per" ("app" | "window")]
                    ["on" "open"]
                    ("function" name "(" [param ("," param)*] ")" "->" type-ref)*
              | "engine" Name                                // written in Clarity
                    ["uses" "random"]
                    ["state" "per" ("app" | "window")]
                    ("keep" name ":" type-ref "=" expr)*
                    function*
service       = "clock" | "random" | "log" | "language"
engine-call   = Engine "." name "(" [expr ("," expr)*] ")"
list-of       = "[" [expr ("," expr)*] "]"
item          = expr "[" expr "]"

find          = "find" name "in" Entity ["where" condition]
                ["order" "by" name "." field ["desc"] ("," name "." field ["desc"])*]

theme         = "theme" Name
                    ("title" String | "mode" ("light" | "dark" | "system") | "color" name String ["dark" String]
                    | "font" [role] String | "size" [role] n | "radius" [kind] n | "padding" n | "border" n
                    | "shadow" ("none" | "flat" | "raised")
                    | "part" kind [state] look | "style" name look)*
look          = ("background" | "text" | "border") (name | String) | "radius" n | "weight" ("bold" | "normal")
                    | "size" n | "font" role | "align" ("left" | "center" | "right")
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
