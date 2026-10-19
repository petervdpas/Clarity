# Migrations

How a database follows your entities through migration steps, written as Clarity files that Clarity proposes and the program runs when it starts.

## Steps are Clarity files

Each SQL data source has its own steps, in a folder of the project's `migrations` folder named after the source. They are numbered in the order they run:

```text
shop/
    migrations/
        main/
            0001.clarity
            0002.clarity
```

A step holds one `migration` block with a title and its changes. It changes only the entities of its own source.

```clarity
migration "Phone and VAT"
    rename field Customer.vat_number to vat
    add field Customer.phone: text(20)
    change field Order.number: text(20)
    add unique Customer.email
```

## The changes a step can hold

- `add entity Name`, with its field lines indented below it, creates an entity's records.
- `remove entity Name` removes an entity and its records.
- `rename entity Old to New` renames an entity; its records stay.
- `add field Entity.name: type, modifiers` adds a field. Existing records get its `default`, or the empty value of its type.
- `remove field Entity.name` removes a field and its values.
- `rename field Entity.old to new` renames a field; its values stay.
- `change field Entity.name: type` changes a field's type and converts the values.
- `add unique Entity.name` and `remove unique Entity.name` make a field unique, or not.

Only what is stored counts in a step: the type, `unique`, `column`, and `default` for records that already exist. Labels, help texts and checks such as `min` belong to the entity alone. The `map table`, `map key` and `map column` changes are in [the language reference](../LANGUAGE.md).

## Pending changes

Replaying the steps in order gives the schema as last migrated. What your entities have and the steps do not are the pending changes. The checker shows them as warnings on the entity, and Clarity will not generate the application until a step covers them.

## The Migrations section

Open **Settings** on the Project tab. Below the data sources and file stores is a **Migrations** section for each SQL data source Clarity owns. It lists the pending changes as the step will say them, and marks the ones that **can lose data**: removing an entity or a field, and changing a type.

Type a title and click **Create step**. When a change can lose data, you first tick **I checked the changes that can lose data**. While the project has errors, the section says so and writes no step. Undo removes a step you just wrote.

## Proposed renames

When a field disappears and a field of the same type appears in the same entity, Clarity proposes a rename. It does the same when an entity disappears and one with mostly the same fields appears. The proposals show under **Renames** with a tick each. Kept as a rename, the data stays; untick it and the step removes the old one and adds the new one.

## From the command line

```shell
clarity migrate shop.clarity
```

This writes the pending changes of every SQL data source as its next step, titled "Step N" unless you give `--title`. `--check` only lists the changes. `--no-renames` takes proposed renames as a removal and an addition. When a change can lose data, the command stops until you run it again with `--allow-data-loss`.

## What the program does at start

The generated application, and Run in Clarity, apply the steps its database does not have yet, in order. The database keeps the number of steps it has in a table called `clarity_schema`. Each step runs in a transaction where the database allows it: SQLite, PostgreSQL and SQL Server undo a failed step as a whole, while MySQL keeps what ran before the failure. On SQLite, the references between records are checked before a step is kept.

A step that ran never changes. To undo it, write a new step. Clarity never writes steps for an external source; see [Data sources](data.sources.md).
