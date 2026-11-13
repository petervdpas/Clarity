# Expressions

The values code computes with: literals, names, operators and their order, lists, text with slots, the types the checker knows, and the built-in functions.

## Values and names

An expression is one line's worth of value: the right side of `let` and `=`, a condition, a value given to a call.

### Literals

```text
12   9.95   -3   "text"   true   false   none
```

- Numbers are written without quotes. A whole number is an `integer`, one with a point a `decimal`.
- Text is written in double quotes; a text with `{...}` in it has slots (see [Text with slots](#text-with-slots)).
- `none` is the empty value of a date, a date and time, a reference, a picture or a file: `loan.returnedOn == none`, `order.customer = none`, `grid.selected = none`. Nothing else is ever none: "text is never none; empty text is """, "integer is never none; only a date, a date and time or a reference can be". Only `==` and `!=` compare with it: "'<' cannot compare with none; use == or !=".

### Names and paths

```clarity
code OrderForm
    handler totalChanged
        let total = order.total
        if grid.selected.id == 0
            message @pickARow
        statusText = "{order.customer.name}: {total}"
```

A name is one of these, looked up where the code stands:

- a local made with `let`, a `for` item, or a parameter;
- the window's data by its name (`customer`, `orders`), and its parts (`save`, `vatNumber`) in handlers;
- a constant of the module or engine; see [Modules](code.modules.md#const);
- an enum value, bare (`Business`) where the expected type says which enum, else qualified (`CustomerKind.Business`);
- `settings`, the app's settings (`settings.lastFolder`);
- `me`, the record of who signed in, where the project signs in (see [Signing in](project.signin.md)): not in a function ("'me' is not allowed in a function; make it an action") or an engine;
- `app.version`, the project's version as text, read and never set;
- `result`, the value a function gives, in its `ensure` lines.

A path goes from a record to its fields (`order.customer.name`), from a part to its properties (`grid.selected`), from a file to its details (`f.extension`) and from an HTTP reply to its `status`, `ok` and `text`. The checker says "unknown name 'totl'", "Order has no field 'totl'", "button 'save' has no property 'colour'", "CustomerKind has no value 'Privat'", "button 'save' is a part, not a value" and "CustomerKind is an enum, not a value".

### Lists

```clarity
function firstOfThree() -> integer
    let squares: list<integer> = [0, 0, 0]
    squares[2] = 5
    let more = squares + [7]
    return more[1]
```

- `[1, 2, 3]` is a list of values of one type: "a list holds values of one type: this is text, the list integer". An empty list needs its type: "an empty list needs its type, as in let squares: list<integer> = []".
- `squares[3]` is an item, counted from 1; a position outside the list is an error when the code runs. "a position in a list is an integer, counted from 1, not text"; "only a list has positions, not text".
- `readings + [value]` is a new list with the items of both. A list of records grows by a list of the same records: `found + [p]`. "a list grows by a list of its items, as in readings + [value]"; "cannot add a list of Album to a list of Order".
- `a to b` is the whole numbers from `a` to `b`, both counted, as a `list<integer>`; empty when `b` is below `a`: `for i in 1 to count(names)`. "'to' counts from one whole number to another, as in 1 to 10".

### Records and data

- `new Order` is a new record of the entity (or a [record](code.modules.md#record)), with its defaults; `save` stores it.
- `find c in Customer where ... order by ...` gives the records that meet a condition; see [Queries](code.queries.md).
- `files f in photos where ... order by ...` gives the files of a file store; `read text of f` the text a file holds, `read bytes of f` its contents as they are; see [Queries](code.queries.md#files-in-a-file-store).
- `json of order` and `read list<Order> from json text` write and read a format; see [Data formats](code.formats.md).
- `http get url` calls a web API and gives its reply; see [Calling web APIs](code.web-apis.md).
- `Pricing.total(lines)` calls a function of an engine; see [Engines](code.engines.md). `Words.spell(42)` calls a function of a library; see [Libraries](code.libraries.md).
- `ask @question ...`, in a command, is the answer typed on the terminal; see [Commands](code.commands.md#ask-in-a-command).

### Texts by key

`@savedName(name = customer.name)` is a text by its key, in the person's language, its slots filled by name. It is what a message, caption, refusal and error say; see [Languages and formats](languages.md).

## Text with slots

A string in code may hold slots: `"{album.title}.txt"`. A slot names a value or a field (`{total}`, `{customer.name}`), with spaces around it allowed; `{{` and `}}` are braces.

```clarity
action exportTitles(album: Album)
    effects files.write
    write album.notes to exports "{album.title}.txt"
```

- Text with slots is filled plainly, in no language: for what people do not read, such as a file's name, a line written to a file, a value to compare, or a URL. Where people read it, use a key.
- In a message, refusal, caption or error, text with slots still shows each value in the person's format. Text joined with `+` there is a warning: write one text with slots instead.
- A slot shows one value: "a slot shows a value, and {customer} is Customer; name one of its fields, like {customer.name}". "a slot names a value or a field, like {customer.name}, not {total + 1}". "'}' ends a slot; write '}}' for a brace". "'{' starts a slot; close it with '}', or write '{{' for a brace".

## Operators

| Operator | Meaning |
|---|---|
| `-x` | negation of a number |
| `a * b`, `a / b` | multiplication and division; `/` always gives a decimal (money when one side is money) |
| `a div b`, `a mod b` | whole-number division: the quotient of two integers rounded toward zero (`7 div 2` is 3, `-7 div 2` is -3), and what is left, with the sign of `a` (`7 mod 3` is 1). Both sides are integers. Dividing by zero is an error, in Clarity's word `divisionByZero`. Elsewhere `div` and `mod` are plain names |
| `a + b`, `a - b` | addition and subtraction; `+` also joins text with any single value, and joins lists |
| `a == b`, `a != b`, `a < b`, `a <= b`, `a > b`, `a >= b` | comparison; `<` and the others compare numbers, text, dates, dates and times, and durations |
| `x in [a, b, c]` | whether `x` equals one of the values |
| `a to b` | the whole numbers from `a` to `b` |
| `a contains b` | whether text `a` holds text `b`, ignoring case; for a list of values, whether it holds the item `b` |
| `a matches x` | in a `find`: the record holds every word of `x`; see [Queries](code.queries.md) |
| `not a` | negation of a bool |
| `a and b`, `a or b` | both, either |
| `(a)` | grouping |

From tightest to loosest:

1. member access (`.`), items (`[i]`) and calls
2. unary `-`
3. `*`, `/`, `div`, `mod`
4. `+`, `-`
5. one comparison: `to`, `contains`, `matches`, `==`, `!=`, `<`, `<=`, `>`, `>=` or `in`; comparisons do not chain, so `a < b < c` is not written
6. `not`
7. `and`
8. `or`

In `write text to store "path"`, the `to` belongs to `write`.

```clarity
function canApprove(order: Order) -> bool
    return order.status in [Pending, Draft] and not (order.total > 1000 or order.customer.blocked)
```

The checker says: "'not' needs bool, not integer"; "'-' needs a number, not text"; "'and' needs bool, not text"; "cannot compare date with text"; "'contains' needs text, not integer and text"; "cannot look for integer in list<text>"; "'div' divides whole numbers, not decimal and integer; '/' divides decimals"; "'+' needs numbers, not bool and integer"; "cannot join list<Order> with text"; "cannot use '*' on duration and duration".

## Types

The types of values follow the field types of entities:

| Type | Values of | Notes |
|---|---|---|
| `text` | text, email, url, color and the other text fields | |
| `integer` | integer, id | |
| `decimal` | decimal | |
| `money` | money | |
| `bool` | bool | |
| `date`, `datetime` | date, datetime | |
| `duration` | duration | durations add up (`+`, `-`), scale by a whole number (`*`, and `/` a duration by one), compare, and take a number of seconds: `let total: duration = 0` |
| an enum | that enum | |
| a record | an entity, a `ref`, a [record](code.modules.md#record) | |
| `list<Entity>` | a list in the window's data, one a query finds, a list field | code builds one too, of stored records or records made in code: `let found: list<Project> = []`, then `found = found + [p]` |
| `list<text>`, `list<integer>`, `list<decimal>`, `list<bool>`, `list<date>`, `list<Enum>` | list fields of values | in code, in what an engine keeps and in list fields; a list is a value, so a copy changes on its own |
| `image` | an `image` field | compared with and set to `none`, or set to a file |
| `file`, `list<file>` | a file of a file store, and a list of them | see [Queries](code.queries.md#files-in-a-file-store) |
| `bytes` | the contents of a file as they are (`read bytes of f`) | `size` is their number; no window data |
| `picture` | a picture of the resources folder, named by its file | only a constant's type (`const logo: picture = "logo.png"`); see [constants](code.modules.md#const) |
| `http reply` | what `http` gives | `status`, `ok`, `text`, read and not set |

A type is written in a parameter, a result, a typed local, an option and `ask ... for` as `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration`, `file`, `bytes`, the name of an entity, record or enum, or `list<...>` of one of them or of `text`, `integer`, `decimal`, `bool`, `date` or `file`. "unknown type 'Ordr'"; "unknown entity 'Ordr'".

Few values convert by themselves: an integer is accepted where a decimal or money is, decimal and money are accepted for each other, and an integer (a number of seconds) where a duration is. Nothing else converts. The checker reports unknown names, wrong types, setting what cannot be set, and calls with the wrong values.

## Built-in functions

Code calls these by name, as it calls its own functions. Their values go in order, without names: "function 'round' takes its values in order, without names".

| Function | Gives |
|---|---|
| `count(list)` | the number of items of a list, of records, values or files |
| `join(texts, separator)` | the texts of a `list<text>` in order, with the separator between each two: `join(names, ", ")` |
| `round(value, places)` | a decimal or money with at most `places` decimals, rounded half away from zero: `round(2.345, 2)` is 2.35 |
| `length(text)` | how many characters a text has: `length("Größe")` is 5 |
| `snapped(value, step)` | the multiple of the whole number `step` nearest to `value`, a half rounding up: `snapped(29, 20)` is 20; a step of 0 gives the value. A canvas's grid snaps so |
| `sin(degrees)`, `cos(degrees)` | a decimal, rounded to 10 decimals |
| `today()` | the date where the application runs |
| `now()` | the date and time where the application runs |
| `newToken()` | a new random token as text, which code gives a person and keeps in a hashed field |
| `newRecoveryCodes(n)` | that many new recovery codes as lines of text, shown once and kept in a hashed field |

### Text functions

Alike on every stack (`conformance/texts.json`). Characters are counted once however many bytes they take, and positions count from 1, as lists do.

| Function | Gives |
|---|---|
| `words(text)` | the words, apart by white space, as a `list<text>` |
| `split(text, separator)` | the pieces, empty ones kept; an empty separator gives the characters |
| `trim(text)` | the text without white space at its ends |
| `lower(text)`, `upper(text)` | the text in small or capital letters |
| `replace(text, find, with)` | the text with every `find` replaced |
| `startsWith(text, prefix)`, `endsWith(text, suffix)` | whether the text starts or ends so |
| `slice(text, from, count)` | `count` characters from position `from`, clamped to the text |
| `indexOf(text, find)` | the position of `find`, from 1; 0 when it is not there |
| `zip(paths, texts)` | the bytes of a zip archive with one file per path, holding the text at the same place as UTF-8, in that order and stored as it is; the same files give the same bytes on every stack (`conformance/zip.json`) |

```clarity
function wordCount(chapter: Chapter) -> integer
    return count(words(chapter.body))
```

`write zip(["mimetype", "OEBPS/ch1.xhtml"], ["application/epub+zip", chapter]) to exports "book.epub"` writes an EPUB, whose `mimetype` must come first and uncompressed.

### Picture functions

Each gives a new picture, as PNG, and leaves its values as they were. A picture is a record's picture or bytes read from a file: `book.cover = resize(read bytes of f, 600, 900)`.

| Function | Gives |
|---|---|
| `resize(p, width, height)` | the picture at that size |
| `crop(p, x, y, width, height)` | that part of it, clamped to the picture |
| `rotate(p, degrees)` | the picture turned; a quarter turn exactly, other angles on a transparent ground the size of the turned picture |
| `flip(p, horizontal)` | left to right with `true`, top to bottom with `false` |
| `blend(base, over, x, y, opacity)` | `over` laid on `base` at that place, its alpha times `opacity` from 0 to 100 |
| `adjust(p, hue, saturation, brightness, contrast)` | hue turned by degrees, the others from -100 to 100, 0 leaving it as it is |
| `pictureWidth(p)`, `pictureHeight(p)` | its size in pixels |

No picture, bytes that are no PNG, JPEG or WebP picture, or a side of 0 or above 30000 pixels is refused with a message when the code runs. Every stack gives the same pixels, through Clarity.Image (`conformance/picture-functions.json`); a program carries it only when its code calls one. On Go, C and C++ they run only in a desktop program (`stack go desktop`, which can also serve a browser): a project whose stack is Go, C or C++ with a `web` target and no `desktop`, or with a `cli` target, cannot be generated while its code calls one, and Generate says why. Rust and C# run them in every program. An engine has none: "'resize' is a picture function, which runs in the program, not in an engine".

A project function of the same name as a text or picture function, `join`, `length`, `snapped`, `newToken` or `newRecoveryCodes` is called instead of the built-in one.

The checker says what each takes when the values do not fit: "'count' needs a list, not text"; "'join' takes a list of texts and the text between them, as in join(names, ", ")"; "'round' needs a decimal or money, not integer"; "'length' takes one text, as in length(name)"; "'snapped' takes a whole number and the step it rounds to, as in snapped(x, 20)"; "'today' takes no arguments"; "'slice' takes a text, a whole number, a whole number".

An engine written in Clarity has two more, `random(n)` and `progress(done, total)`, with `uses random` and `uses progress`; see [Engines](code.engines.md).

See also [Statements](code.statements.md).
