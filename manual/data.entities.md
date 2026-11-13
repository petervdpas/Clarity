# Entities and enums

How you describe your data with entities and enums, the field types and modifiers, and how the entity editor writes them for you.

## The data dictionary

An entity is a kind of record that is stored in a data source, such as a customer or an order. Everything about a field is said once, in the entity: windows take their labels, inputs and validation from it, and the database schema follows from it through [migrations](data.migrations.md).

```clarity
entity Customer
    name: text(80), required, label @customerNameLabel
    email: email, unique, label @customerEmailLabel
    kind: CustomerKind, default Private, label @customerKindLabel
    creditLimit: money, min 0, default 0, label @customerCreditLimitLabel
    active: bool, default true, label @customerActiveLabel
    since: date, label @customerSinceLabel
    notes: text, help @customerNotesHelp, label @customerNotesLabel
```

A field is `name: type`, followed by modifiers separated by commas. Every record also has an `id`, which Clarity adds itself. Only `save` sets it; a new record has id 0.

## Enums

An enum is a fixed list of values. Each value can have a label; without one, the label is derived from the name.

```clarity
enum CustomerKind
    Private label @customerKindPrivate
    Business label @customerKindBusiness
```

A field whose type is an enum holds one of its values.

## Field types

- `text` holds text of any length; `text(n)` at most `n` characters.
- `integer` is a whole number; `decimal(p, s)` has `p` digits, `s` of them after the point.
- `money` is an exact amount, shown with the project's `currency`.
- `bool` is true or false.
- `date` is a calendar date; `datetime` a date with a time.
- `duration` is a length of time in whole seconds, shown and typed as `3:45` or `1:02:03`.
- `email` and `url` are an e-mail address and a web address.
- `ref Entity` refers to a record of another entity.
- `image` holds a picture; see [Pictures and files](data.pictures-files.md).
- `id` holds the id of a record, and `list<Entity>` the records of another entity that belong to this one.
- In an entity of a web API, a field named after another entity holds a record inside: `owner: User`, and `list<Line>` holds the lines inside it, as the service's JSON nests them. Code reads them as any record and list: `order.owner.name`, `for line in order.lines`. A record never holds itself; a list may. See [Calling web APIs](code.web-apis.md).

## Modifiers

- `required`: the field must have a value. Windows mark its label.
- `unique`: no two records have the same value. Empty text counts as no value.
- `default value`: the value of a new record (a number, a string, `true`, `false` or an enum value).
- `label "text"`: the label people see, instead of the one derived from the name.
- `help "text"`: a help text shown next to the field's input in windows.
- `format name`: how the value is shown, such as `plain` for an integer or `long` and `iso` for a date.
- `store name`: for an `image`, keep its pictures as files in that file store.
- `min value`, `max value` and `pattern "regex"`: limits a value must keep to.

Each modifier appears at most once per field. The checker reports a default or a limit that does not fit the type. Entities of a database you do not own also use `table`, `key`, `column` and `version`; see [Data sources](data.sources.md). The full list is in [the language reference](../docs/LANGUAGE.md).

## A save never overwrites someone else's change

Every record of an entity in a database Clarity owns carries a version, a whole number in a column of its own, `row_version`. Clarity keeps it: 1 for a new record, one more with each save, and you never set it. When two people (or two windows) open the same customer and Anna saves first, Ben's save is refused instead of writing Anna's old values back, and Ben sees "Someone else changed this record since you opened it. Reload it to see their change." A delete of a record someone changed since is refused the same way. What to do then is in [Handlers](code.handlers.md#messages-windows-and-data).

The column comes from a migration step, which Clarity proposes; see [Migrations](data.migrations.md). Until that step is written, saves of the entity are not checked.

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

The slots are `current.` with a field, `current.id`, and a field of a record it refers to. Clarity keeps no "changed by" of its own: a `before save` that sets `customer.changedBy = me` does; see [Data events](code.data-events.md).

## The entity editor

The Data section of the project tree, on the Project and Design tabs, lists every entity and enum. Click one to open its editor; **+ New entity** and **+ New enum** ask for a name and add one.

The entity editor shows a row per field, with columns for the name, the type, **Req.**, **Unique**, **Search**, **Hashed**, **Label**, **Default**, **Format**, **Help** and **Store**. The type is a list of every type, the same the agents' tools offer; a `ref`, an enum, a record or a `list` gets a second list of what it holds (your entities, enums and records; in a module of a data source, also the module's other entities, to hold inside), a text a box for its length and a decimal boxes for its precision and scale. **Format** offers the formats of the field's type, and **Store** appears for image fields. The arrow before a row shows the field's details: **Min**, **Max**, **Pattern**, **Computed**, **Owned** (for a `ref` or a `list`), **Cascade** (for a `ref`) and **Column** (in a module of a data source, or for an entity with its own table). The arrows move a field, the cross removes it, and **Add field** at the bottom adds one. A new name in a field's name cell renames the field everywhere it is named: windows, code and the translations. The name at the top renames the entity everywhere it is used, and **Delete entity** removes it.

**Settings** in the project tree (or **+ Settings** while there are none) opens the app's settings block in the same way: checkboxes for Clarity's own settings (`language`, `theme`, `agentReadsWhileClosed`, `agentChangesWhileClosed`), and a row per setting of the app with the types and modifiers a setting takes. Each person keeps a setting under its name, so a new name there changes only its line.

The enum editor works the same way, with a row per value and its label.

Every change is written to the module file at once and checked. A change that would break a window is refused, with the reason. Modifiers without a column, such as `min`, `max` and `pattern`, are written in the module's text.

After you change entities, the database needs a migration step; see [Migrations](data.migrations.md).
