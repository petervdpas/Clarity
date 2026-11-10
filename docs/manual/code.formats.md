# Data formats

Reading records from text in a format, and writing values in one: JSON first.

## Reading and writing

```clarity
code Orders
    handler loadClicked
        let reply = http get "https://api.example.com/orders"
        orders = read list<Order> from json reply.text at "data"

action sendOrder(order: Order) -> integer
    effects net
    let reply = http post "https://api.example.com/orders" with json of order header "Content-Type": "application/json"
    return reply.status
```

- `read Order from json text` reads one record, `read list<Order> from json text` a list. Options follow the text: `at "data"` starts inside the JSON, at the names given with dots between (`at "result.items"`).
- `json of value` writes a record, a list of records or a value.
- The format is a name of Clarity's catalog of formats. JSON is the one there now; more come the same way, each read and written with the same words.

## How what is read fits the record

- Names match by themselves, else ignoring case, underscores, hyphens and spaces: `creditLimit`, `credit_limit` and `Credit Limit` all fill `creditLimit`. When several match, the first written counts.
- Numbers and strings of numbers stand for each other, and a string may have spaces around it. A decimal keeps every digit, a whole number is any 64-bit integer, a date and time fills a date with its date, and an enum value matches in any case.
- What is left out, or null, keeps its default. What the entity does not have is left aside, and so are the id, references, pictures, files and lists of stored records. The records are new: `save` adds them.
- A record held inside an entity of a web API (`owner: User`), and a list of them (`lines: list<Line>`), are read from the nested object and list, each into its own entity.
- What does not fit stops the handler with a message that says where, such as `field 'lines' of Order, item 2: field 'quantity' of Line is an integer, not "three"`, as a refused save does.

## What is written

Every stack writes a value the same way, byte for byte: a record's id first, then its fields in the entity's order, decimals and money as exact text in their shortest form (`12.50` is written `12.5`), enums by name, dates as `2026-10-25`. So what one Clarity program sends, another reads, whatever stacks they are built on.

## In the built program

The formats are one library, Clarity.Formats, that every stack uses, so they read and write alike everywhere. A program carries it only when its code reads or writes a format:

- **Rust:** the crate `clarity-formats`, fetched with the runtime.
- **C#:** the NuGet `Clarity.Formats`.
- **Go, C and C++:** its library, which lies beside Clarity.UI's and comes with it. A Go program that uses a format builds with cgo, so it needs a C compiler, as a Go desktop program does.

See [Calling web APIs](code.web-apis.md) for where the text comes from, and [The language reference](../LANGUAGE.md#data-formats) for every rule.
