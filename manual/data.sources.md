# Data sources

Where your records live: the project file's data sources (SQLite, the server databases, web APIs and files), keeping passwords out of the project, several sources and a module's `source`, external databases that Clarity reads without owning, how each database stores a field, and the migration steps of a source.

## Declaring a source

A data source is a `datasource` line in the project file: a name, a kind and where the data is. A new project gets one SQLite source named `main`.

```clarity
datasource main sqlite "data/shop.db"
datasource sales postgres "env:SHOP_DATABASE"
```

You can also add, change, reorder and remove sources in the **Data sources** section of **Settings** on the Project tab. For SQLite, **Browse...** picks the file in Clarity's file browser, or names a new one there.

Entities live in the first data source unless their module names another with a `source` line; see [Several sources](#several-sources).

### `datasource`

```text
datasource name kind "where" [external] [readonly] [beside]
    auth kind "secret"                         a web API
    paging ...                                 a web API
    openapi "file.yaml"                        a web API
    pick "operationId", ...                    a web API with openapi
    option "value"                             a file's format options
```

| Part | Type | Default | Meaning |
|---|---|---|---|
| `name` | a name | | How modules name the source (`source erp`); camelCase, each source once |
| `kind` | `sqlite`, `postgres`, `mysql`, `mssql`, `rest`, `csv`, `json`, `xml`, `yaml`, `file` | | What the source is; see [Kinds](#kinds) |
| `"where"` | text | | A file, a connection, a URL, `"env:NAME"` or `"setting:name"` |
| `external` | flag | off | A database Clarity does not own: it reads and writes the records but never changes the tables |
| `readonly` | flag | off | Records are only read: `save` and `delete` of them are errors |
| `beside` | flag | off | For an owned SQLite file: the built program keeps it in the project's folder (see [Where a SQLite file is](#where-a-sqlite-file-is)) |
| `using Mapping` | a mapping | | For a source of kind `file`: the mapping that says its format; see [Mappings](code.formats.md) |

The checker refuses, with the line:

- a missing name, kind or place: "write a name, a kind and where it is, like: datasource main sqlite \"data/app.db\"";
- a word after the place other than these: "unexpected 'x'; a data source can be external, readonly or beside, and a file can be laid over by a mapping: using OrderFeed", and a flag written twice;
- two sources of one name, and a name that is no Clarity name;
- an empty place ("the connection is empty"), a SQLite source that is not a file path ("a SQLite source is a file path, like data/shop.db"), a web API whose URL does not start with `http://` or `https://`;
- `auth` or `paging` under a source that is no web API ("only web API sources have auth", "only a web API pages"), and `external` on a web API ("a web API is always external; leave out 'external'").

## Kinds

| Kind | Where the data is | Example |
|---|---|---|
| `sqlite` | a file, relative to the project's folder, for Clarity and for the generated application alike | `"data/shop.db"` |
| `postgres` | a connection string or URL | `"host=db.example.com user=shop dbname=shop"`, `"postgresql://shop@db.example.com/shop"` |
| `mysql` | a URL | `"mysql://shop@db.example.com:3306/shop"` |
| `mssql` | an ADO.NET connection string | `"server=tcp:db.example.com,1433;database=shop;user=shop;TrustServerCertificate=true"` |
| `rest` | the base URL of a web API | `"https://api.example.com/v2"` |
| `csv`, `json`, `xml`, `yaml` | a file in that format, always `external` | `"imports/logins.csv"` |
| `file` | a file laid over by a mapping, always `external` | `"imports/orders.xml" external using OrderFeed` |

The same entities and steps work on the four SQL databases; the generated application carries a driver for each kind its sources are, and writes each step in that database's own SQL. The C and C++ stacks store records in SQLite: Generate refuses a server database for them, naming the source ("data source 'sales' is a PostgreSQL database; the C backend stores records in SQLite for now").

A `rest` source is a web API: its entities live in the service, so no migration covers them and `find`, `save` and `delete` do not reach them; the module's actions read and send them with `http ... from name`. See [Calling web APIs](code.web-apis.md).

## Passwords stay out of the project

A connection that needs a password is written as `"env:NAME"`: the generated application reads the whole connection from that environment variable when it starts, so the password is never in the project. It may also be `"setting:name"`, a [secret setting](data.settings.md#secret-settings) of the person, which the system's protected store keeps.

```clarity
datasource main postgres "setting:shopDb"
```

- The checker refuses a connection that contains a password itself: "the connection contains a password; put it in an environment variable (env:NAME) or a secret setting (setting:name), which the system's keychain keeps".
- A `"keychain:NAME"` is refused: the keychain is reached through a secret setting, `setting:NAME`, with `NAME: secret` in the settings.
- `env:` or `setting:` with nothing after it is refused.
- A `setting:name` that is no secret setting is an error: "database main connects with setting 'shopDb', which is no secret setting; add it to settings as shopDb: secret".
- While a secret connection is not kept yet, the database does not open: the application shows the window where it is entered, saying why, and connects once it is kept and that window closes. The app's `on unconnected` handler can show something else instead. A secret connection needs one of the two; else the checker says "database main connects with setting 'shopDb', which no window shows and no 'on unconnected' handles".
- Why the databases did not open is said in Clarity's words `unconnectedNotKept` and `unconnectedFailed`, the second with the database's own message in `{problem}`; see [Languages](languages.md).
- A SQLite source is always a file path.

## A web API

The lines under a `rest` source, indented:

### `auth`

How the calls are signed: `bearer`, `api_key` or `basic`, and where the secret is, `"env:NAME"` or `"setting:name"` (a secret setting). The secret itself is never in the file.

```clarity
datasource payments rest "https://api.example.com/v2"
    auth bearer "env:PAYMENTS_TOKEN"
```

Once per source. A secret that is not a reference is refused ("the secret must be a reference (env:NAME or setting:name), not the secret itself"), and a `setting:name` that is no secret setting is an error ("web API payments signs with setting 'x', which is no secret setting; add it to settings as x: secret"). An `auth` line that is not under a `datasource` is refused.

### `paging`

How the API hands out a long list, for `every page`:

- `paging page "page" size "per_page" 100` sends page numbers from 1;
- `paging offset "offset" size "limit" 100` sends offsets from 0;
- `paging cursor "after" next "meta.next"` sends the cursor each page names at that path;
- `paging link` follows the `Link` header's next address.

`items "data"` at the end says where a page's items are in its JSON. Once per source; a page holds at least 1 item. See [Calling web APIs](code.web-apis.md).

### `openapi` and `pick`

`openapi "file.yaml"` names the API's OpenAPI (Swagger) description, a YAML or JSON file of the project's `apis` folder. Clarity writes the module `apis/<source>.clarity` from it, for what the project picks, and writes it again when the file or the picks change. `pick "operationId", ...` names the operations the application uses: each becomes an action, and the schemas it takes or gives entities. `pick schema "Name", ...` makes schemas entities on their own. A pick the description does not have is a warning; only a web API has these lines, and `pick` needs an `openapi` line. In Settings, **API...** next to the source picks them. See [Calling web APIs](code.web-apis.md).

```clarity
datasource forms rest "https://forms.example.com/api"
    openapi "forms.yaml"
    pick "listTemplates", "getTemplate"
```

## A file as a data source

A CSV, JSON, XML or YAML file can hold an entity's records, so a table part over it is a CSV editor with no code:

```clarity
datasource logins csv "imports/logins.csv" external readonly
    separator ";"
```

- The kind is the format's name. The path is relative to the project's folder.
- A file source is always `external`: Clarity reads and writes its records but never changes its shape. Without it: "a data source that is a file is external, since Clarity does not own its shape". Add `readonly` when it may only be read.
- The format's options are indented lines under it, each once. CSV: `separator ";"` (any one character, `"\t"` for a tab), `quote "'"`, `noHeader true` (the file has no header row; its columns are the entity's fields in order) and `encoding "windows-1252"` (or `"iso-8859-1"`; UTF-8 by default). JSON: `at "data.logins"`, where the list of records is inside the file. XML: `at "shop/orders/order"`, `namespace`, `indent true` and `encoding`; see [An XML file](#an-xml-file). The options of every format are in [Data formats](code.formats.md).
- The file holds one entity's records. Its fields map onto the file's columns as on an external database's: by name, or with `column "Login URL"`. A column the entity wants that the file lacks stops the program when it opens, with a message that names it.
- `find`, `save` and `delete` work as on a database. The program reads the whole file when it starts and writes it again after each change: to a new file that then replaces the old one, so a crash never leaves half a file. The file's own column order and the columns no field maps are kept.
- When the file has no `id` column, a record's id is its row's place, which is never written into the file.
- In Settings, pick **CSV file**, **JSON file**, **XML file** or **YAML file** as the kind; the format's options show under the location.
- A file can instead be laid over its entity by a mapping of the project, which says the format, where the records and fields are, and the options: `datasource feed file "imports/orders.xml" external using OrderFeed`. The kind is then `file`, and the entity is the mapping's (its module says `source feed`). A `file` source without `using` is refused, and so is `using` on a source that names its format. See [Mappings](code.formats.md).

### An XML file

```clarity
datasource feed xml "imports/orders.xml" external
    at "shop/orders/order"
```

- `at` names the record's element by its path, element names with slashes between: every `<order>` inside `<shop><orders>` is one record.
- A field takes a child element of its name, else an attribute of its name. Where its value is somewhere else, `column` says where, relative to the record's element: `column "@number"` is the attribute only, `column "customer/name"` the text of an element inside, `column "."` the record element's own text.
- Without a mapping the source is one flat table: an element inside a record that holds a record or a list is kept with the record, unmapped. A mapping with records inside (`lines of Line at "lines/line"` with `order is ".."`) makes the file hold both entities: `Line` lives in the same source (its module says `source feed` too), each line refers to the order it sits in by `order`, and a line saved or deleted is written back inside its order.
- Saving keeps the whole document: everything outside the records stays as it was (the declaration, comments, other elements), and so do the attributes and elements inside a record that no field maps, in their place. A new record gets an element with its mapped fields; a deleted record's element goes, with what it held.

## Several sources

Entities live in the first data source unless their module names another with a `source` line, after its `use` lines:

```clarity
module erp

use customers
source erp

entity Invoice
    number: text(20), unique, label @invoiceNumberLabel
    customer: ref Customer, label @invoiceCustomerLabel
```

### `source`

```text
source name
```

Places the entities of this module in that data source of the project. In a SQL database Clarity stores them; in a web API the service keeps them; in a file source the file holds them. A module has at most one `source` line ("a module has one 'source'"), and a name the project does not have is an error that lists the sources it has.

A reference to an entity in another database is stored as its id, without a foreign key, so the database cannot stop a referenced record from being deleted. A query reads one database, so it cannot go through such a reference. Each owned source has its own migration steps.

## Where a SQLite file is

Clarity's Run uses the project's own file. A built program keeps a SQLite database the project owns in the person's folder for application data, under the program's name (`~/.local/share/<program>` on Linux, `~/Library/Application Support/<program>` on macOS, `%APPDATA%\<program>` on Windows). `beside` keeps it in the project's folder for the built program too, for an app that lives in its own folder; in Settings it is the **Beside the project** box of an owned SQLite source. The environment variable `CLARITY_DATA` names another folder, where every SQLite file opens under its own file name, which is how Run chooses its data without changing the program.

## External databases

A database that already exists, and that Clarity does not own, is marked `external`. Add `readonly` when its records may only be read; `save` and `delete` of them are then errors ("Invoice is in data source erp, which is read-only; its records cannot be saved"). In Settings these are the **External** and **Read-only** boxes.

```clarity
datasource erp sqlite "legacy/erp.db" external
```

Its entities live in a module with `source erp`, and map onto the tables and columns that are there:

```clarity
entity Invoice
    table "tblInvoice"
    key "InvoiceID"
    version "RowVersion"
    invNo: text(20), required, unique, column "InvNo", label @invoiceNumberLabel
    custId: ref Customers, column "CustID", label @invoiceCustomerLabel
    amount: decimal(18, 6), column "Amount", label @invoiceAmountLabel
```

### `table`, `key` and `column`

```text
table "Name"
key "Column"
field: type, column "Column"
```

- `table` names the existing table; without it, the table is the entity's name in snake case (`invoice`).
- `key` names the integer column that identifies a record; without it, that column is `id`. A table whose key is not one integer column cannot be used.
- `column` names the existing column of a field; without it, the column is the field's name in snake case (`invNo` in `inv_no`).
- Each once per entity ("duplicate 'table'"). Two fields cannot share a column ("fields 'a' and 'b' both use column 'X'"), and no field can use the key column. A record has no `table` or `key`, as it belongs to no table.
- The mapping is meant for existing tables; it stays when Clarity takes ownership, and works on owned entities too.

### `version`

```text
version "Column"
version none
```

Names the column that holds each record's version, a whole number the database keeps as Clarity keeps its own `row_version` on the tables it owns: a save writes only when the version is still the one it read, and adds one, so a save never overwrites someone else's change (see [Entities](data.entities.md#a-save-never-overwrites-someone-elses-change)). `version none` says the entity's saves are not checked, on purpose. Without either line the checker warns: "Invoice has no version column, so saves of it are not checked: name its column with version \"Column\", or write version none". A `readonly` source needs neither. `version` is for external databases only: a database Clarity owns keeps `row_version` itself (there the line is refused: "Invoice is stored in a database Clarity owns, which keeps its version in row_version itself; remove the version line"), a file source's file is its own version, and a web API has no such column. The version column is neither the key nor a field's column.

### What Clarity does with an external database

Clarity reads and writes the records of an external database, but never changes its tables: it writes no migration steps for it, and the generated application never migrates it. Each time the project loads, Clarity compares the entities with the tables of the database. A missing table or column, the version column included, is an error; a column whose type does not fit the field, and a required column that no field fills (so new records cannot be saved), are warnings. When the database cannot be read, a warning says its tables were not checked.

## Import

**Import...** next to an external SQLite source, or a file source, opens the Import panel. It lists the tables; tick the ones you want, name the module and click the **Import** button below them. The tables become entities mapped onto them, in a new module. Names follow Clarity's style (`tblCustomers` becomes `Customers`, `CustName` becomes `custName`), column types become field types, foreign keys between imported tables become `ref` fields, `NOT NULL` becomes `required` and a unique index `unique`. Binary columns are left out. An integer column named `row_version`, as in a table Clarity made, becomes the entity's `version "row_version"`; any other version column you name yourself, as it is never guessed. From the command line:

```shell
clarity import shop.clarity erp --tables tblInvoice,tblCustomers
```

`--list` shows the tables, and `--module` names the module (the source's name by default).

A file source has one table, named after the file: its columns become text fields. Types are not guessed from the values; change them in the entity editor.

For an XML file source, Import lists the repeated elements found in the file by their path (`shop/orders/order`) and writes an entity with a text field per attribute and child element of the records ticked. Beside the entity, Import writes a mapping, `mapping OrderFeed of Order from xml`, with the `at` where the records are, a line for each field whose path is not its name (`note at "@note"`) and the source's format options, and lays the source over it: `datasource feed file "imports/orders.xml" external using OrderFeed`. The paths are then kept in one place, and changed on the mapping's page (**Mapping...** next to the source); see [Mappings](code.formats.md).

Import also takes an XSD schema of the file. Its types become entities with their real types: `xs:int` and the other whole numbers an `integer`, `xs:decimal` a `decimal`, `xs:date` a `date`, `xs:dateTime` a `datetime`, `xs:boolean` a `bool`, an enumeration an `enum`, `maxLength` a text's size, `minOccurs="1"` `required`, a complex type inside another a record field and `maxOccurs="unbounded"` a list.

## Taking ownership

**Own...** next to an external SQLite source, or `clarity own shop.clarity erp`, makes Clarity the owner. It writes the source's first step, a `baseline`, which describes the mapped tables as they are and creates only what is missing, so it changes nothing in the database it was taken from (and a new, empty database gets the tables). Then it removes `external`. From then on your entities drive the tables through ordinary steps, and the `table`, `key` and `column` names stay. Undo takes it back.

Import works for SQLite and file sources, and the table check, the baseline and Own for SQLite. An external PostgreSQL, MySQL or SQL Server database is mapped by hand and is read and written as it is.

## How each database stores a field

| Type | SQLite | PostgreSQL | MySQL | SQL Server |
|---|---|---|---|---|
| `text(n)` | TEXT | VARCHAR(n) | VARCHAR(n); TEXT above 768 | NVARCHAR(n); NVARCHAR(MAX) above 4000 |
| `text`, `url` | TEXT | TEXT | TEXT | NVARCHAR(MAX) |
| `email` | TEXT | VARCHAR(320) | VARCHAR(320) | NVARCHAR(320) |
| `color` | TEXT | VARCHAR(7) | VARCHAR(7) | NVARCHAR(7) |
| `file` | BLOB, and TEXT for its name | BYTEA, VARCHAR(255) | LONGBLOB, VARCHAR(255) | VARBINARY(MAX), NVARCHAR(255) |
| `integer`, `id`, `ref` | INTEGER | BIGINT | BIGINT | BIGINT |
| `decimal(p, s)` | TEXT | NUMERIC(p,s) | DECIMAL(p,s) | DECIMAL(p,s) |
| `money` | TEXT | NUMERIC(19,4) | DECIMAL(19,4) | DECIMAL(19,4) |
| `bool` | INTEGER | BOOLEAN | BOOLEAN | BIT |
| `date` | TEXT | DATE | DATE | DATE |
| `datetime` | TEXT | TIMESTAMP | DATETIME | DATETIME2 |
| an enum | TEXT | TEXT | VARCHAR(100) | NVARCHAR(100) |

SQLite keeps decimals and dates as text, so no precision is lost. The key is an integer the database counts up, and a `ref` field gets a foreign key when its entity is in the same database. The version column is stored as an integer, `row_version INTEGER NOT NULL DEFAULT 1` on SQLite and `BIGINT NOT NULL DEFAULT 1` on the others. A list field is kept in one column as JSON; see [Entities](data.entities.md#lists-in-records).

What differs between the databases:

- Unique text. MySQL and SQL Server index text up to a length (768 and 850 characters), so a `unique` text field there needs a maximum length that fits, such as `text(200)`; the generator says which field. Empty text is never a duplicate: SQLite, PostgreSQL and SQL Server leave it out of the index, MySQL indexes it as no value.
- Changing a type. SQLite builds the table again; the others add a column of the new type, copy the values over converted, and drop the old one. Empty text that becomes a number or a date gets the field's default.
- A failed step. SQLite, PostgreSQL and SQL Server undo the whole step. MySQL cannot undo changes to tables, so it keeps the statements of the step that ran before the failure; the error says how many.
- Comparing text. `==` and `contains` follow the database: MySQL and SQL Server usually ignore case in `==` too, and sort text by their own rules.
- Full-text search. Each database keeps its own index of the fields marked `search`: SQLite an FTS5 table that triggers keep (`album__search`), PostgreSQL a GIN index on `to_tsvector('simple', ...)`, MySQL a FULLTEXT index without stopwords, and SQL Server none, as its full-text search is seldom installed: it looks with LIKE at the start of each word, reading every record. A step that changes an entity builds its index again. Whether accents count follows the database: MySQL usually ignores them, the others do not. See [Queries](code.queries.md).

## The migration steps of a source

Each SQL data source Clarity owns follows the entities through its own migration steps, in a folder of the project's `migrations` folder named after the source and numbered in the order they run: `migrations/main/0001.clarity`, `migrations/main/0002.clarity`. A step changes only the entities of its source. How Clarity proposes and writes the steps, in the IDE and with `clarity migrate`, is in [Migrations](data.migrations.md).

### `migration`

```text
migration "Title" [baseline]
    change...
```

```clarity
migration "Phone and VAT"
    rename field Customer.vatNumber to vat
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
| `add search Entity.name`, `remove search Entity.name` | puts a field in its entity's full-text index, or takes it out |
| `map table Entity to "name"` | stores the entity in a table of another name (renames the table); `to default` goes back to the entity's name |
| `map key Entity to "name"` | names the key column (renames it); `to default` is `id` |
| `map column Entity.field to "name"` | stores a field in a column of another name (renames the column); `to default` is the field's name in snake case |
| `add version Entity` | gives the entity's table its version column, `row_version`, at 1 for the records already there, so saves of it are checked |

- Fields in a step are written as in an entity. Only what is stored counts: the type, `unique`, `column`, and `default` for the records that already exist. Labels, help texts and checks such as `min` belong to the entity alone.
- An added entity can name its table and key with `table` and `key` lines, as in the entity, and a line `version` gives its table the version column from the start. Clarity writes `version` in every entity it adds to a database it owns. Renaming a field whose column is mapped changes nothing in the database.
- `baseline` marks the step Own writes: it describes tables that are there and creates only what is missing. It leaves the version out, as the tables it describes have no such column; the next step adds it.
- Replaying the steps in order gives the schema as last migrated. Each step must fit the schema the steps before it made: one that renames a field that does not exist is an error.
- What the entities have and the steps do not are the pending changes: warnings on the entity, and the generator writes no application until a step covers them. A missing version column alone does not hold the application back: the table is written as it is and its saves stay unchecked until the step that adds the column exists, which the checker's warning says.

```clarity
migration "Versions"
    add version Customer
    add version Order
```

### Running the steps

The generated application applies the steps its database does not have yet when it starts, each in a transaction where the database allows it (see the differences above). On SQLite, which does not enforce references while it builds a table again, it checks the references between records before a step is kept. A step that ran never changes: to undo it, write a new step.

Before it applies steps to a SQLite database that already has a schema, the application backs it up beside it, in `backups/` (`data/backups/shop-20261105T205905Z-before-migration-0006.db`, the time in UTC), and keeps the newest ten of those; a backup taken by hand stays. When it cannot back up, it does not migrate. The Database section of the Project tab takes a backup by hand and restores or deletes one; `clarity data backup shop.clarity`, `clarity data backups shop.clarity` and `clarity data restore shop.clarity <file>` do the same from the command line. A restore first backs up what it replaces. A server database is backed up by its administrator.

See also [Entities and enums](data.entities.md), [Migrations](data.migrations.md), [Calling web APIs](code.web-apis.md) and [The project file](project.file.md).
