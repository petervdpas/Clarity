# Entities and enums

How you describe your data with enums, entities and records: the field types and modifiers, versions, lists in records, and how the entity editor writes them for you.

## The data dictionary

An entity is a kind of record that is stored in a data source, such as a customer or an order. Everything about a field is said once, in the entity: windows take their labels, inputs and validation from it, and the database schema follows from it through [migrations](data.migrations.md).

Entities, enums and records are declarations of a module, in any `.clarity` module file. Their names are PascalCase; field names are camelCase.

## The entity editor

The Data section of the project tree, on the Project and Design tabs, lists every entity and enum. Click one to open its editor; **+ New entity** and **+ New enum** ask for a name and add one.

The entity editor shows a row per field, with columns for the name, the type, **Req.**, **Unique**, **Search**, **Hashed**, **Label**, **Default**, **Format**, **Help** and **Store**. The type is a list of every type, the same the agents' tools offer; a `ref`, an enum, a record or a `list` gets a second list of what it holds (your entities, enums and records; in a module of a data source, also the module's other entities, to hold inside), a text a box for its length and a decimal boxes for its precision and scale. **Format** offers the formats of the field's type, and **Store** appears for image fields. The arrow before a row shows the field's details: **Min**, **Max**, **Pattern**, **Computed**, **Owned** (for a `ref` or a `list`), **Cascade** (for a `ref`) and **Column** (in a module of a data source, or for an entity with its own table). The arrows move a field, the cross removes it, and **Add field** at the bottom adds one. A new name in a field's name cell renames the field everywhere it is named: windows, code and the translations. The name at the top renames the entity everywhere it is used, and **Delete entity** removes it.

The enum editor works the same way, with a row per value and its label.

Every change is written to the module file at once and checked. A change that would break a window is refused, with the reason. Modifiers without a column, such as `pattern`, are written in the module's text.

The app's settings are edited the same way, under **Settings** in the project tree; see [Settings](data.settings.md).

After you change entities, the database needs a migration step; see [Migrations](data.migrations.md).

### `enum`

A fixed list of values. A field whose type is the enum holds one of its values.

```text
enum Name
    Value label @key
```

```clarity
enum CustomerKind
    Private label @customerKindPrivate
    Business label @customerKindBusiness
```

- Each value is a PascalCase name with its label by key. People read the label, so a value without one is an error: "Private of CustomerKind shows a text without a key; give it one: label @customerKindPrivate".
- An enum has at least one value ("enum 'CustomerKind' has no variants"), and each value once ("duplicate variant 'Private'").

### `entity`

```text
entity Name
    [table "Table"] [key "Column"] [version "Column" | version none]
    [savedSince @key]
    [readable when condition] [writable when condition]
    name: type, modifier, modifier...
```

```clarity
entity Customer
    name: text(80), required, label @customerNameLabel
    email: email, unique, label @customerEmailLabel
    kind: CustomerKind, default Private, label @customerKindLabel
    vatNumber: text(14), help @customerVatNumberHelp, label @customerVatNumberLabel
    creditLimit: money, min 0, default 0, label @customerCreditLimitLabel
    active: bool, default true, label @customerActiveLabel
    since: date, label @customerSinceLabel
    notes: text, help @customerNotesHelp, label @customerNotesLabel
```

A field is `name: type`, followed by modifiers separated by commas. Every record also has an `id`, which Clarity adds itself. Code can read it, to tell two records apart (`other.id != loan.id`); only `save` sets it, and a new record has id 0.

The lines that are no fields:

- `table`, `key` and `version` map the entity onto an existing table; see [External databases](data.sources.md#external-databases).
- `savedSince @key` gives the refusal of a save the entity's own text; see [A save never overwrites someone else's change](#a-save-never-overwrites-someone-elses-change).
- `readable when` and `writable when` say whose records a person may read and change; see [Signing in](project.signin.md#who-signed-in-in-code).

The checker refuses an entity without fields ("entity 'Customer' has no fields"), a field written twice ("duplicate field 'name'"), a field name that is not camelCase, and a field without a label key: "field name of Customer shows its name as its label; give it a key: label @customerName".

## Field types

| Type | Holds |
|---|---|
| `text` | text of any length |
| `text(n)` | text of at most `n` characters, `n` at least 1 |
| `integer` | a whole number |
| `decimal(p, s)` | a decimal number with `p` digits, `s` of them after the point; `p` at least 1 and not less than `s` |
| `money` | an amount of money, exact, shown with the project's `currency` |
| `bool` | true or false |
| `date` | a calendar date |
| `datetime` | a date with a time |
| `duration` | a length of time in whole seconds, shown and typed as `3:45` or `1:02:03` everywhere |
| `image` | a picture (png, jpg, gif, bmp or webp), kept in the database, or with `store` as a file; see [Pictures and files](data.pictures-files.md) |
| `file` | any file, with its name, kept in the database, or with `store` as a file; see [Pictures and files](data.pictures-files.md) |
| `email` | an e-mail address |
| `url` | a web address |
| `color` | a color, kept as text `#rrggbb`: `default "#3b82f6"`; code sets it as such text |
| `id` | the id of a record |
| an enum name | one value of that enum |
| `ref Entity` | a reference to a record of another entity |
| an entity name | in an entity of a web API, a record of that entity held inside this one, as the service's JSON nests it: `owner: User` |
| `list<Entity>` | the records of another entity that belong to this one; in an entity of a web API, held inside it |
| `list<text>`, `list<integer>`, `list<decimal>`, `list<date>`, `list<Enum>` | a list of values, kept in one column; see [Lists in records](#lists-in-records) |

`secret` and `folder` are types of a setting only; an entity's field of either is an error. See [Settings](data.settings.md).

In an entity of a web API, code reads a record and a list held inside as any record and list: `order.owner.name`, `for line in order.lines`. A record never holds itself; a list may. See [Calling web APIs](code.web-apis.md).

## Modifiers

| Modifier | Type | Meaning |
|---|---|---|
| `required` | flag | The field must have a value. Windows mark its label, and `save` refuses a record while the field is empty (no value, or empty text; 0 and false are values) with Clarity's word `fieldRequired`, "{field} is required" with the field's label, after `before save`, so nothing is kept. The refusal shows at the window's input bound to that field, else as an error message; code can catch it. |
| `unique` | flag | No two records have the same value. Empty text counts as no value, so several records can leave a unique text field empty. |
| `search` | flag | The field is in its record's full-text index, which `where a matches x` looks in. For text (also `email` and `url`) and lists of text; see [Queries](code.queries.md). |
| `min value`, `max value` | number, or a date as text | The lowest and highest value allowed. |
| `pattern "regex"` | text | Text must match the regular expression. |
| `default value` | number, text, `true`, `false` or an enum value | The value of a new record. A date is written `"2026-10-17"` and a date and time `"2026-10-17T09:30:00"` (seconds may be left out); a color `"#3b82f6"`. |
| `label @key` | key | The label people see. Every field has one. |
| `help @key` | key | A help text, shown next to the field's input in windows. |
| `format name` | name | How the value is shown; see [Formats](languages.md#formats). |
| `owned` | flag | For a `list<Entity>`: the listed records live and die with this record. |
| `hashed` | flag | For text of an entity: assigning stores the hash the project's sign-in names for it (Argon2id when none does); code never reads it and no part shows it. See [Signing in](project.signin.md). |
| `cascade` | flag | For a `ref Entity`: deleting the record it refers to deletes this one first, with its own cascades and data events, all in the transaction of that delete. When one of them refuses, nothing is deleted. |
| `computed` | flag | The value is worked out, not entered. Clarity accepts it; the generated application does not compute it. |
| `column "Name"` | text | The column the field is stored in, when it is not the field's name in snake case; see [External databases](data.sources.md#external-databases). |
| `store name` | a file store | For an `image` or a `file`: kept as files in that file store of the project file, not in the database. |

Each modifier appears at most once per field ("duplicate modifier 'required'"), and an unknown one is an error. The checker also refuses:

- `owned` on anything but a list, `cascade` on anything but a `ref`, `store` on anything but an image or a file, `search` on anything but text and lists of text, `pattern` on anything but text;
- `unique` or `default` on a picture, a file or a list;
- a `min` or `max` that is no number on a number or no date on a date, and `min` greater than `max`;
- a default that does not fit the type ("default value does not match the field type ('active')"), a default that is not one of the enum's values ("'Gold' is not a variant of 'CustomerKind' ('kind')"), a date or color written another way;
- `computed` with a `default`; `computed` with `required` is a warning, since it has no effect;
- a `format` the type does not have;
- `hashed` on anything but text of a stored entity, on text shorter than 128 characters ("a hash is longer than 80 characters; write text without a length"), and with `search`, `computed`, `default`, `pattern`, `min`, `max` or `format`.

## A save never overwrites someone else's change

Every record of an entity in a database Clarity owns carries a version, a whole number in a column of its own, `row_version`. Clarity keeps it: 1 for a new record, one more with each save; code never sets it, and `find` reads it with the record. A save writes only when the version is still the one it read, and a delete is checked the same way. When two people (or two windows, or a background job) open the same customer and Anna saves first, Ben's save is refused and changes nothing: his handler stops, and he sees Clarity's word `savedSince`, "Someone else changed this record since you opened it. Reload it to see their change." One record is one unit: fields are never merged. `reload customer` reads the record as it is stored now, and `save customer anyway` writes regardless of the version; see [Handlers](code.handlers.md).

The column comes from a migration step (`add version Customer`, or `version` in an `add entity`), which Clarity proposes; see [Migrations](data.migrations.md). Until that step is written, saves of the entity are not checked. An entity of an external database names its own column with `version "Column"`; see [External databases](data.sources.md#external-databases).

### `savedSince`

An entity can show its own text for the refusal, by a key, with the slots of the record as it is stored now:

```clarity
entity Customer
    name: text(80), label @customerNameLabel
    changedBy: ref Person, label @customerChangedByLabel
    savedSince @customerChangedSince
```

```clarity
translation en
    customerChangedSince "{current.changedBy.name} changed this customer since you opened it"
```

- The slots are `current.` with a field (`{current.name}`), `{current.id}`, and a field of a record it refers to (`{current.changedBy.name}`).
- The text is a key: `savedSince "..."` is refused with "the text of savedSince is a key: write savedSince @customerChangedSince, with its text in the language files". Once per entity.
- Who changed a record is the project's choice: Clarity keeps no "changed by" column of its own, and a `before save` that sets `customer.changedBy = me` does; see [Data events](code.handlers.md#data-events).
- A record never has `savedSince` or `version`, as it is never stored.

## Lists in records

A field can hold a list of values: tags, the formats an album came out in, the days a show was played.

```clarity
entity Album
    tags: list<text>, label @albumTagsLabel
    formats: list<Format>, label @albumFormatsLabel
    plays: list<date>, label @albumPlaysLabel
```

- The item types are `text`, `integer`, `decimal`, `date` and an enum; any other is refused ("a list field holds text, integer, decimal, date or an enum"). A list is kept in one column as JSON (`["jazz","live"]`) and starts empty; migrations add it as an empty list. A field cannot become a list or stop being one; remove it and add a new field. A list cannot be unique, and has no default, `min` or `max`.
- The tags part shows the items as chips, each with a button that removes it, and a place to add one: `<Tags name="tagsBox" at="1,1" span="6" bind="album.tags" />`. Text and numbers are typed and taken on Enter or when the box is left; a typed item is trimmed, an item the list has is not added twice, and text that is no number stays as typed and is marked. A date is picked; an enum's values are picked from the ones the list has not. Adding and removing are its `change`. Its words are Clarity's `addTag` and `removeTag`.
- A table shows a list column as its items joined by a comma and a space, an enum's values by their labels. It is not edited in the cell.
- In code a list field is a list of values like any other: `for play in album.plays`, `count(album.tags)`, `album.tags[1]`, a new list `album.formats = [Vinyl, Cd]`, and `album.tags contains "live"` for whether it holds an item. A query finds the records whose list holds an item, exactly: `find a in Album where a.tags contains "live"` finds an album tagged `live`, not one tagged `live music`. Each database does it with its own JSON functions.
- Every stack keeps and reads a list alike: the cases are in `conformance/list-fields.json`.

## Pictures and files in records

An `image` field holds a picture and a `file` field any file with its name; both are kept in the database, or with `store` as files in a file store. How parts show them and code sets them is in [Pictures and files](data.pictures-files.md).

### `record`

A record is a value with fields that belongs to no table: declared like an entity, with `record`, never saved or found.

```text
record Name
    name: type, modifiers
```

```clarity
module shapes

record Dot
    x: integer
    y: integer
    size: decimal, default 1.5
    tags: list<text>
```

- `new Dot` has every field at its default: the one written with `default`, or 0, empty text, no, an empty list. A record's decimal needs no precision and scale, since it has no column. Its fields need no label.
- `d.x` reads a field and `d.x = 3` sets it. A record is a value: setting a field changes that record only, as an item of a list does.
- Copied, never shared. Putting a record into a list, giving it to a variable, a window's data or a function, or reading it from a list or from `grid.selected` makes a copy. After `let d = new Dot`, `dots = dots + [d]` and `d.x = 3`, the dot in the list still has its own `x`. Every stack behaves alike: Rust, Go, C and C++ copy the struct, C# copies the object.
- Lists of records (`list<Dot>`, `[a, b]`) are read by item and by `for`, and grow by a list of the same records: `dots + [d]`. An empty one names its type: `let dots: list<Dot> = []`.
- A record is never none. It always has its fields; compare a field instead. Only a part's selected row is set to `none`.
- Engines written in Clarity give and take records as JSON objects of their fields. An engine uses records of values only, without references.
- App code uses them too: functions and actions take and give them, a window holds one (`<Data name="picked" type="Node" />`, new when the window opens) or a list of them (`type="list&lt;Node&gt;"`, empty when it opens), and a `list`, `table`, `tree` or `treetable` shows such a list as it shows records of an entity: `columns` and `show` name its fields, `grid.selected` is the record of the chosen row, and `on select` and `on open` run. A record has no id, so a row is its place in the list. Giving the window a new list (`nodes = outline()`) selects no row; code selects one again with `nav.selected = ...`.

The checker refuses:

- a record without fields ("record 'Dot' has no fields"), and a field written twice;
- a picture or a file: "record Dot has field cover: a record's fields are values or references to stored records, not pictures or files";
- a reference to another record: "a reference names a stored entity; Node is a record, which is never stored";
- `table`, `key`, `version`, `savedSince`, `readable when` and `writable when` ("a record is never stored, so it has no 'version'"), and `hashed`;
- `save`, `delete`, `find` and `reload` of a record, and a table of records that the person edits (`edit`) or reorders by hand (`reorder`): build the list in code.

```clarity
module modelling

record Node
    key: text
    parent: text
    label: text
    project: ref Project

function outline(projects: list<Project>) -> list<Node>
    let all: list<Node> = []
    let top = new Node
    top.key = "land"
    top.label = "Landscape"
    all = all + [top]
    for p in projects
        let n = new Node
        n.key = p.name
        n.parent = "land"
        n.label = p.name
        n.project = p
        all = all + [n]
    return all
```

```xml
<!-- modelling/design/WorkspaceView.cxaml -->
<View class="modelling.WorkspaceView">
  <Data name="projects" type="list&lt;Project&gt;" />
  <Data name="nodes" type="list&lt;Node&gt;" />
  <Tree name="nav" at="1,1" span="4" rows="20" bind="nodes" rowKey="key" parentKey="parent" show="label" onselect="chosen" />
</View>
```

See also [Data sources](data.sources.md), [Migrations](data.migrations.md), [Queries](code.queries.md) and [Data events](code.handlers.md#data-events).
