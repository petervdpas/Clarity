# Queries

How to read records with `find`, and how code lists, reads and writes files in a file store.

## Finding records

`find` reads the records of an entity that meet a condition, in an order. The result is a list.

```clarity
code Customers
    handler filter_changed
        customers = find c in Customer where c.name contains name_filter.text and c.active order by c.name

    handler card_opened
        orders = find o in Order where o.customer == customer and o.total > 100 order by o.total desc
```

- `find c in Customer` names each record `c`. The name exists only inside the query, so `c.name` never mixes up with a part or a local called `name`.
- `where` is optional. Without it, `find` reads every record.
- `order by` takes one or more fields, each with `desc` if you want them from high to low. Without it, records come in the order they were stored.

## Conditions

A condition compares a field of the record with a value, and combines such comparisons with `and`, `or`, `not` and parentheses:

- `c.name == x`, `c.total > 100`, and the other comparisons.
- `c.name contains x`: the text holds `x`, ignoring case.
- `c.kind in [Private, Business]`: one of these values.
- A bool field on its own: `c.active`, `not c.active`.
- A reference field compared with a record: `o.customer == customer`.
- A date or reference field compared with `none` finds the records where it is empty: `l.returned_on == none`.

The value side can be anything that does not use the record: a literal, a local, the window's data, a part property, or a call such as `today()`. Two fields of the same record cannot be compared with each other.

The database does the work. The condition becomes a query with parameters, so values never become part of the SQL text.

## Through references

A condition and an order can go through reference fields to the record they point at:

```clarity
albums = find a in Album where a.artist.name contains artist_filter.text order by a.artist.name, a.title
```

A record whose reference is empty does not meet such a condition. A query reads one database, so it cannot go through a reference into another data source.

## Using the list

Assign it to a window's list data, so a table shows it; loop over it with `for`; count it with `count(list)`; or return it from a function.

```clarity
let tasks = find t in Task where t.finished_by == person
if count(tasks) > 0
    message error "{person.name} has finished tasks and cannot be deleted"
    return
```

A table showing a list that gets new rows has no row selected afterwards. `today()` gives the date and `now()` the date and time where the application runs: `loan.lent_on = today()`.

## Files in a file store

Code reaches files only in the file stores of the project settings (`filestore photos "data/photos"`); see [Pictures and files](data.pictures-files.md).

```clarity
action export_collection()
    effects files.write
    let lines = ""
    for album in find a in Album order by a.title
        lines = lines + album.title + "\n"
    write lines to exports "collection.txt"
```

- `files f in photos where f.extension == "jpg" order by f.path` lists the files of the store and its folders, leaving out names that start with a dot. The condition may be any bool, since it runs in memory.
- A file has `name`, `path` inside the store, `extension` in lower case without the dot, `size` in bytes and `modified` (a date and time, in UTC).
- `read text of f` gives the text a file holds, as UTF-8.
- `read bytes of f` gives a file's contents as they are, as `bytes`: a picture, a PDF, anything that is no text. `b.size` is how many there are.
- `write t to exports "lists/albums.txt"` writes text as a file, making its folders; `write b to ...` writes bytes as they are. A path that would leave the store is refused.
- `artist.photo = f` makes a file a record's picture.

Listing and reading need `effects files.read` in an action, writing `files.write`; handlers may do both, and functions cannot reach files. See [Functions and actions](code.functions.md).

The full rules are in [the language reference](../LANGUAGE.md#queries), with files under [Files](../LANGUAGE.md#files).
