# Migrations

How a database follows your entities through migration steps, written as Clarity files that Clarity proposes and its migration tool, `clarity-migrate`, applies.

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
    rename field Customer.vatNumber to vat
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
- `rename value Enum.Old to New` renames a value of an enum: every record that keeps it, in a field of the enum or in a list of its values, takes the new name. Clarity writes it when a value is renamed; see [Entities](data.entities.md#enum).
- `change field Entity.name: type` changes a field's type and converts the values.
- `add unique Entity.name` and `remove unique Entity.name` make a field unique, or not.
- `add version Entity` gives the table its version column, `row_version`, at 1 for the records already there, so a save never overwrites someone else's change (see [Entities](data.entities.md#a-save-never-overwrites-someone-elses-change)). A new entity gets it from its first step: a line `version` in its `add entity`, which Clarity writes itself.

Only what is stored counts in a step: the type, `unique`, `column`, and `default` for records that already exist. Labels, help texts and checks such as `min` belong to the entity alone. The `map table`, `map key` and `map column` changes are in [Data sources](data.sources.md), under the migration steps of a source.

## Pending changes

Replaying the steps in order gives the schema as last migrated. What your entities have and the steps do not are the pending changes. The checker shows them as warnings on the entity, and Clarity will not generate the application until a step covers them.

A table that has no version column yet is proposed `add version Entity`, one line per entity. That one alone does not hold the application back: it is generated as it is, and saves of the entity are not checked until the step exists. The baseline that Own writes leaves the version out, as the tables it describes have none; the next step adds it.

## The Migrations section

Open **Settings** on the Project tab. Below the data sources and file stores is a **Migrations** section for each SQL data source Clarity owns. It lists the pending changes as the step will say them, and marks the ones that **can lose data**: removing an entity or a field, and changing a type.

Type a title (it is "Step N" when you leave it empty) and click **Create step**. When a change can lose data, you first tick **I checked the changes that can lose data**. While the project has errors, the section says so and writes no step. Undo removes a step you just wrote.

The section, the agents' migrate tool and `clarity migrate` write a step the same way: the same data sources (the SQL databases Clarity owns; a web API, a file and an external database have no steps), the same refusal while the project has errors or a change can lose data that nobody allowed, and the same answer when there is nothing to migrate.

## Renames

A rename gets into a step only when someone asked for it; Clarity never guesses one. Renaming an entity, a field or a value of an enum in Clarity (the rename in the project tree, the name cell of the entity and enum editors, or an agent's rename tool) writes the rename into the migration step at once, so the data stays.

When a field disappears and a field of the same type appears in the same entity, Clarity suggests a rename. It does the same when an entity disappears and one with mostly the same fields appears. The suggestions show under **Renames**, unticked. Tick one to keep it as a rename, so the data stays; left unticked, the step removes the old one and adds the new one.

## From the command line

```shell
clarity migrate shop.clarity
```

This writes the pending changes of every SQL data source as its next step, titled "Step N" unless you give `--title`. `--check` only lists the changes. A field or entity that went while another came is a removal and an addition; the command lists the renames it could be, and takes one only when you name it with `--rename`: `--rename Customer=Client` for an entity, `--rename Customer.vatNumber=vat` for a field, `--rename Kind.Private=Personal` for a value of an enum. Repeat it for more. When a change can lose data, the command stops until you run it again with `--allow-data-loss`.

## Applying the steps

Clarity's migration tool, `clarity-migrate`, applies the steps a database does not have yet, in order: Run in Clarity does it before the program starts, `clarity data migrate` from the command line (below), and the tool beside an app where it is installed. The database keeps the number of steps it has in a table called `clarity_schema`. Each step runs in a transaction where the database allows it: SQLite, PostgreSQL and SQL Server undo a failed step as a whole, while MySQL keeps what ran before the failure. On SQLite, the references between records are checked before a step is kept. A SQLite database that already has a schema is backed up first; see [Data sources](data.sources.md#running-the-steps).

A step that ran never changes. To undo it, write a new step. Clarity never writes steps for an external source; see [Data sources](data.sources.md).

## What the program does at start

The generated application does not migrate. When it starts, before any window, it checks that each database it owns has exactly the steps it was made with. A database that is behind or ahead stops it with one line in English on the console, and it exits with 1:

```text
The database main needs migrating: run clarity-migrate
The database main belongs to a newer version of this app
```

Whoever installs the app migrates its database with `clarity-migrate`, or with Clarity. With `include migrate` in the project file (**Include data migration tools** in the Project settings view), Generate puts `clarity-migrate` and its `clarity-migrate.json` in the output folder, so the tool travels with the app; see [The project file](project.file.md#include-migrate).

## Creating the database from the command line

`clarity data migrate shop.clarity` creates every database the project owns, or brings it up to date, with Clarity's migration tool, `clarity-migrate`: SQLite, PostgreSQL, MySQL and SQL Server alike, each SQLite database the one in the project's folder where Clarity runs it, each server database as the app connects to it (a connection kept as a secret setting is read from the system's store). A database that changes is backed up first. A database that has more steps than the project is refused: it belongs to a newer version of the app. `--source` takes only that data source. Handy before seeding data, or on a machine with no display.
