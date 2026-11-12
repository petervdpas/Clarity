# Data formats

Reading records from text in a format, and writing values in one: JSON, CSV and XML.

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
- The format is a name of Clarity's catalog of formats: `json`, `csv`, `xml` and `yaml` now; more come the same way, each read and written with the same words.

## CSV

```clarity
function loginsIn(text: text) -> list<Login>
    return read list<Login> from csv text separator ";"

function exported(logins: list<Login>) -> text
    return csv of logins
```

- The first row names the columns, matched to the fields as JSON's names are. With `noHeader true` there is no header row, and the columns are the entity's fields in the order they are declared.
- `separator` sets the delimiter, any one character: `separator ";"`, `separator "|"`, or `separator "\t"` for tab-separated text. The default is a comma.
- `quote` sets the quote character (default `"`), and `encoding` reads and writes `"windows-1252"` or `"iso-8859-1"` text instead of UTF-8.
- Quoted fields may hold the separator, line breaks and doubled quotes. An empty field keeps the default, `true` and `false` fill a yes/no field, and a list of values is written as its JSON text.
- Writing gives a header row (unless `noHeader true`), then one row per record, with the fields that are one value: not the id, references, pictures, files or records inside. CSV is one flat table, so an entity with records inside, or a plain value, is refused when the program is checked.

## XML

```clarity
// <shop><orders><order number="A-1"><customer><name>Ada</name></customer>
//   <lines><line product="Coffee" quantity="2"/></lines></order></orders></shop>
function ordersIn(text: text) -> list<Order>
    return read list<Order> from xml text at "shop/orders/order"

function feed(orders: list<Order>) -> text
    return xml of orders at "shop/orders/order" indent true
```

- `at` says where the records are: element names with slashes between, the record's own element last. Every `<order>` inside `<shop><orders>` is one record. Without `at`, reading takes the root element's children.
- A field takes a child element of its name, else an attribute of its name: `number` above comes from the attribute. A field that is a record takes the element of its name (`customer`); a list takes the elements of its name, repeated, or one element of its name holding the items (`<lines><line/><line/></lines>`).
- Prefixes do not matter: `<o:order>` is an `order`. The encoding the XML declaration names is honoured.
- `xml of` always takes `at`, which names the elements written around the records and the record's own: `at "order"` writes one record as `<order>`. Each field becomes a child element, a list of records an element of the field's name holding one element per record, and a null value is left out, so what Clarity writes it reads back the same.
- `indent true` writes one element per line, indented, for files people read. `namespace "urn:example:orders"` writes that namespace as the document's default, and `encoding "windows-1252"` writes in that encoding instead of UTF-8.

## YAML

```clarity
function settingsIn(text: text) -> list<Shop>
    return read list<Shop> from yaml text at "shops"

function written(shops: list<Shop>) -> text
    return yaml of shops
```

- YAML reads as JSON does: the same records, lists and values, names matched the same way, `at "shops"` (or a longer path, `at "data/shops"`) where the records are.
- Comments, anchors (`&base`, `*base`), quoted and block texts (`|`, `>`) are read. `yes` and `no` are text, as YAML 1.2 has it; `true` and `false` are yes and no.
- Writing gives block YAML, quoting only a text that would read back as something else, such as `"007"` or `"true"`.
- As a data source, a YAML file keeps its comments and layout: only the lines of what changed are written again.

## Mappings

```clarity
mapping OrderFeed of Order from xml
    at "shop/orders/order"
    number at "@number"
    customer at "customer/name"
    lines of Line at "lines/line"
        order is ".."
    check "orders.xsd"
    indent true

function ordersIn(text: text) -> list<Order>
    return read list<Order> from text using OrderFeed

function feed(orders: list<Order>) -> text
    return text of orders using OrderFeed
```

- A mapping says, for this case only, where a file keeps the records (`at`) and each field (`number at "@number"`). A field left out is found by its name.
- `lines of Line at "lines/line"` reads the records inside, with their own lines; `order is ".."` names the field of a line that refers to the order it sits in.
- `check "orders.xsd"` names a schema file in the project's folder, for a format that has schemas (XML takes `xsd`). The whole file is checked against it before anything is read from it, in code and on a file data source: required fields and attributes are there, an element the schema allows once is there once, and each value is of its type (an integer, a decimal within its digits, a date, a date and time, `true` or `false`), one of an enumeration's values, or a text no longer than its longest. What breaks it stops the read with where it is, such as `shop/order[2]: 'quantity' is "x", which the schema says is an integer`. The schema is built into the program, so after changing it, build the program again.
- The format's options are written as lines too: `indent true`, `separator ";"`.
- Paths look the same in every format: steps with slashes between, `@name` for an attribute (XML only), `..` for the record around. In CSV a path is a column's name.
- `using OrderFeed` reads and writes through it; the format comes from the mapping. A file data source can name one as well: `datasource feed file "imports/orders.xml" external using OrderFeed`.
- In the IDE a mapping has a page of its own, under **Mappings** in the project tree (**+ New mapping** asks for the entity, the format and the name). It shows the format, the entity, where the records are, a row per field with its path (empty: found by its name), the records inside with the field that names the record around, the schema and the format's options. Every change is written back into the mapping's module at once. **Sample...** lays a file over the mapping and shows its first records as the entity's fields hold them (a yes or no as true or false, a decimal in its shortest form), or why the file does not fit. **Delete** (on the page, or a right click in the tree) asks first, and is refused, naming where, while a data source or code still uses the mapping.

## How what is read fits the record

- Names match by themselves, else ignoring case, underscores, hyphens and spaces: `creditLimit`, `credit_limit` and `Credit Limit` all fill `creditLimit`. When several match, the first written counts.
- Numbers and strings of numbers stand for each other, and a string may have spaces around it; `true` and `false` fill a yes/no field from text too. A decimal keeps every digit, a whole number is any 64-bit integer, a date and time fills a date with its date, and an enum value matches in any case.
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

See [Calling web APIs](code.web-apis.md) for where the text comes from, and [The language reference](../docs/LANGUAGE.md#data-formats) for every rule.
