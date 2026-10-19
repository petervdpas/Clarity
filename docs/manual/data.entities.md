# Entities and enums

How you describe your data with entities and enums, the field types and modifiers, and how the entity editor writes them for you.

## The data dictionary

An entity is a kind of record that is stored in a data source, such as a customer or an order. Everything about a field is said once, in the entity: windows take their labels, inputs and validation from it, and the database schema follows from it through [migrations](data.migrations.md).

```clarity
entity Customer
    name: text(80), required, label "Name"
    email: email, unique, label "E-mail"
    kind: CustomerKind, default Private
    credit_limit: money, min 0, default 0
    active: bool, default true
    since: date
    notes: text, help "Internal notes, not shown to the customer"
```

A field is `name: type`, followed by modifiers separated by commas. Every record also has an `id`, which Clarity adds itself. Only `save` sets it; a new record has id 0.

## Enums

An enum is a fixed list of values. Each value can have a label; without one, the label is derived from the name.

```clarity
enum CustomerKind
    Private label "Private person"
    Business
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

## Modifiers

- `required`: the field must have a value. Windows mark its label.
- `unique`: no two records have the same value. Empty text counts as no value.
- `default value`: the value of a new record (a number, a string, `true`, `false` or an enum value).
- `label "text"`: the label people see, instead of the one derived from the name.
- `help "text"`: a help text shown next to the field's input in windows.
- `format name`: how the value is shown, such as `plain` for an integer or `long` and `iso` for a date.
- `store name`: for an `image`, keep its pictures as files in that file store.
- `min value`, `max value` and `pattern "regex"`: limits a value must keep to.

Each modifier appears at most once per field. The checker reports a default or a limit that does not fit the type. Entities of a database you do not own also use `table`, `key` and `column`; see [Data sources](data.sources.md). The full list is in [the language reference](../LANGUAGE.md).

## The entity editor

The Data section of the project tree, on the Project and Design tabs, lists every entity and enum. Click one to open its editor; **+ New entity** and **+ New enum** ask for a name and add one.

The entity editor shows a row per field, with columns for the name, the type, **Req.**, **Unique**, **Label**, **Default**, **Format**, **Help** and **Store**. The type box suggests the built-in types, your enums, and `ref` and `list<...>` of your entities. **Format** offers the formats of the field's type, and **Store** appears for image fields. The arrows move a field, the cross removes it, and **Add field** at the bottom adds one. The name at the top renames the entity everywhere it is used, and **Delete entity** removes it.

The enum editor works the same way, with a row per value and its label.

Every change is written to the module file at once and checked. A change that would break a window is refused, with the reason. Modifiers without a column, such as `min`, `max` and `pattern`, are written in the module's text.

After you change entities, the database needs a migration step; see [Migrations](data.migrations.md).
