# Queries

How to read records with `find`, and how code lists, reads and writes files in a file store.

## Finding records

`find` reads the records of an entity that meet a condition, in an order. The result is a list.

```clarity
code Customers
    handler filterChanged
        customers = find c in Customer where c.name contains nameFilter.text and c.active order by c.name

    handler cardOpened
        orders = find o in Order where o.customer == customer and o.total > 100 order by o.total desc
```

- `find c in Customer` names each record `c`. The name exists only inside the query, so `c.name` never mixes up with a part or a local called `name`.
- `where` is optional. Without it, `find` reads every record.
- `order by` takes one or more fields, each with `desc` if you want them from high to low. `order by c.id` is the order the records were stored in, which every record has. Without `order by`, records come in the order they were stored. "order by a field of c, as in 'c.name'"; "cannot order by 'tags', a list".

```text
find name in Entity [where condition] [order by name.field [desc], ...]
```

## Conditions

A condition compares a field of the record with a value, and combines such comparisons with `and`, `or`, `not` and parentheses:

- `c.name == x`, `c.total > 100`, and the other comparisons.
- `c.name contains x`: the text holds `x`, ignoring case.
- `c.kind in [Private, Business]`: one of these values.
- A bool field on its own: `c.active`, `not c.active`.
- A reference field compared with a record: `o.customer == customer`.
- A date or reference field compared with `none` finds the records where it is empty: `l.returnedOn == none`.
- A list field is looked into with `contains`: `a.tags contains x`.

The value side can be anything that does not use the record: a literal, a local, the window's data, a part property, or a call such as `today()`. Two fields of the same record cannot be compared with each other.

The checker says: "a query compares a field of c with a value, as in 'c.name == x'"; "Customer has no field 'nme'"; "cannot compare integer with text"; "a query looks into list<text> with contains, as in 'a.tags contains x'"; "unknown entity 'Custmer'"; "Dot is a record, which is never stored, so 'find' has nothing to look through".

The database does the work. The condition becomes a query with parameters, so values never become part of the SQL text.

## Full-text search

A field marked `search` is in its record's full-text index, and `a matches x` finds the records that hold every word of `x`, each as the start of a word, in any of those fields:

```clarity
module albums

entity Album
    title: text(120), required, search
    notes: text, search
    tags: list<text>, search

code AlbumBrowse
    handler searchChanged
        albums = find a in Album where a matches searchBox.text order by a.title
```

- Typing `blue no` finds an album titled *Blue Train* with the notes *on Blue Note*, and one tagged `blues` and `north`, but not *Kind of Blue* alone: every word must be there. Case does not count.
- A word is a run of letters and digits; `hard-bop` is the two words `hard` and `bop`. Text without words, such as an empty search box, matches every record.
- `matches` looks in the record of the query itself (`a matches x`), and combines with other conditions: `where a matches x and a.year > 1960`. The entity needs at least one field marked `search`.
- `contains` stays the exact test: `a.title contains "lue"` finds *Blue Train*; `a matches "lue"` does not, as no word starts with it.
- Every stack writes the same SQL per database; the cases are in `conformance/queries.json`.
- "'matches' looks for words in the records found, as in 'a matches x'"; "'matches' looks for text, not integer"; "Album has no field marked search to look for words in". Outside a `find`: "'matches' looks for words in the records a find looks through, as in 'find a in Album where a matches x'".

## Through references

A condition and an order can go through reference fields to the record they point at:

```clarity
albums = find a in Album where a.artist.name contains artistFilter.text order by a.artist.name, a.title
```

A record whose reference is empty does not meet such a condition. A query reads one database, so it cannot go through a reference into another data source.

## Using the list

Assign it to a window's list data, so a table shows it; loop over it with `for`; count it with `count(list)`; or return it from a function.

```clarity
let tasks = find t in Task where t.finishedBy == person
if count(tasks) > 0
    message error "{person.name} has finished tasks and cannot be deleted"
    return
```

A table showing a list that gets new rows has no row selected afterwards. `today()` gives the date and `now()` the date and time where the application runs: `loan.lentOn = today()`.

## Files in a file store

Code reaches files only in the file stores of the project settings (`filestore photos "data/photos"`); see [Pictures and files](data.pictures-files.md).

```clarity
action exportCollection()
    effects files.write
    let lines = ""
    for album in find a in Album order by a.title
        lines = lines + album.title + "\n"
    write lines to exports "collection.txt"
```

- `files f in photos where f.extension == "jpg" order by f.path` lists the files of the store and its folders, leaving out names that start with a dot. The condition may be any bool, since it runs in memory.
- A file has `name`, `path` inside the store, `extension` in lower case without the dot, `size` in bytes and `modified` (a date and time, in UTC).
- `read text of f` gives the text a file holds, as UTF-8.
- `read bytes of f` gives a file's contents as they are, as `bytes`: a picture, a PDF, anything that is no text. `b.size` is how many there are. Bytes are no window data: the screen has nothing to show of them, so code keeps them while it works and hands them to engines and functions.
- `write t to exports "lists/albums.txt"` writes text as a file, making its folders; `write b to ...` writes bytes as they are. A path that would leave the store is refused.
- `artist.photo = f` makes a file a record's picture. In the field's own store (`store photos`) it is the file itself: the column holds its path, nothing is copied, as a catalog of photos wants. Kept anywhere else, the picture is a copy of the file. A file that is no picture is refused.
- The picture functions work on a record's picture or on bytes read from a file: `book.cover = resize(read bytes of f, 600, 900)`; `zip(paths, texts)` gives the bytes of a zip archive to write. Each, with what it takes and on which stacks it runs, is under [Picture functions](code.expressions.md#picture-functions) and [Text functions](code.expressions.md#text-functions).
- What a program touches on disk is in its settings: a store that differs per machine comes from an environment variable.

```text
files name in store [where condition] [order by name.detail [desc], ...]
```

The checker says: "a file has name, path, extension, size and modified, not 'title'"; "'read text of' needs a file, not text"; "'read bytes of' needs a file, not text"; "bytes have size, not 'length'"; "action 'findPhotos' lists files, which needs 'effects files.read'".

Listing and reading need `effects files.read` in an action, writing `files.write`; handlers may do both, and functions cannot reach files. See [Functions and actions](code.functions.md).

See also [Expressions](code.expressions.md) and [Statements](code.statements.md#write).
