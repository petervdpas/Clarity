# Formats plan

One catalog of data formats, written once in the shared Rust library `clarity-formats`, used by code (`read ... from <format>`, `<format> of`) and by data sources that are files in a format. Step 1 of APP-CAPABILITIES-PLAN.md, split out on 2026-11-10 because it grew. Step 1 built 2026-11-10 and released in Clarity 0.38.0 on 2026-11-11 (clarity-formats 0.1.0, clarity-runtime 0.54.0, Clarity.UI library 0.18.0, C# 0.90.0 with Clarity.Formats, Go v0.57.0); steps 2 (CSV, file data sources), 3 (XML), 4 (mappings on one model and engine) and 5 (YAML) built 2026-11-12, unreleased.

## The user's words

- "I want you to make a plan for it, to add them in the most generic way possible so it improves the sort of apps clarity can create!"
- "csv is just one format of data... making it specific isnt very generic, is it... you need to keep thinking c++ builder and what you could make with that back in the days"
- "shared Rust library, go with that"
- "be sure we are not going to do half implementations scattered all over!!!!!"
- "the both of you, work out the formats crate throught clarity!!"
- On XML (2026-11-12): "design xml in the plan... and remember it needs to be generic! No fixed format!!! What C++ builder could Clarity should be able to do but then in this day and age"
- After XML (2026-11-12): "yaml! and the still points should be configured per case not carved in stone beforehand!"
- On the first per-format design of those points: "you're not taking this generic!!!!!!!!", and "Clarity language and implementation needs to general all purpose!!!!"

On JSON: answered in this session "Keep per runtime" (JSON stays in each runtime). Relayed afterwards from the other session, the user pointed out that this "leaves a formats crate with CSV and JSON somewhere else, which removes the strength of the idea", and JSON moves into the library. This plan follows the later decision; see "Decided".

## The idea

In C++ Builder a grid did not care where its rows came from: a table, a query, or a TClientDataSet loaded from a file were all a TDataSet. Clarity's entities are the datasets and its data sources say where they live. This plan makes a format one pluggable thing that turns text or bytes into records and back, and lets a file in any format be a data source, so a table part edits a CSV file as it edits a database table.

## Rules for the whole plan

1. **The catalog is the design from day one.** One statement shape (`read T from <format> text`, `read list<T> from <format> text`, `<format> of value`), one fitting of rows to records, one file-source mechanism, and each format's options declared in the catalog the way part properties are. A format is an entry, never a special path. Adding XML later is one more entry; nothing else changes.
2. **One implementation of every format.** JSON moves into `clarity-formats` as the first entry. Every runtime's JSON reading and writing goes through it, and the old per-runtime JSON code is removed in the same step. There are never two.
3. **Each step is a complete slice before the next starts.** All five stacks, conformance passing on every stack, checker, designer, Settings rows, inspector, agent tools and instructions, language reference, manual, i18n keys, an example that uses it, every example building green. The done list per step is under "Order".
4. **Nothing merged half-wired.** No format the parser accepts that a runtime cannot run; no designer row, Settings row or agent tool for something the runtimes lack. The catalog is the single source the parser, checker, designer and agent read, so they cannot drift from it.

## In code

```clarity
let logins = read list<Login> from csv text separator ";"
let order = read Order from json reply.text at "data"
write csv of requests to exports "requests.csv"
let body = json of order
```

- `read T from <format> text`, `read list<T> from <format> text`, `<format> of value`. `read ... from json` and `json of` keep their meaning and syntax exactly; they become the catalog's `json` entry.
- Every format follows the rules JSON has today: names match ignoring case, underscores, hyphens and spaces (`Login URL` fills `loginUrl`); missing values keep their default; unknown ones are left aside; numbers and strings of numbers stand for each other; an enum value matches in any case; a value that does not fit stops the handler with a message that says where (`field 'lines' of Order, item 2: ...`, `row 12, column 'date': "31-02" is no date`). Reading has no effect, so a function may do it.
- Records inside records and lists of them work where a format nests (json); a flat format (csv) refuses an entity that nests, with a check error at the line.
- Bytes as well as text, for formats that are not text: the same shape with `bytes` (spreadsheets, later).

## The catalog

Each entry declares, in one place the parser, checker, designer, generator and agent read:

| Entry | Text or bytes | Tables | Nests | Options |
|---|---|---|---|---|
| `json` | text | one, or several by path | yes | `at "path"` |
| `csv` | text | one | no | `separator ";"` (`"\t"` for tab-separated), `noHeader`, `quote "'"`, `encoding "windows-1252"` |

- **Tables** is part of every entry: a CSV file holds one table, a spreadsheet holds several (its sheets), a JSON file one or several by path. Where a file holds several, `table "Orders"` names the one meant, in code (`read list<Order> from xlsx bytes table "Orders"`) and on a file source, as for an external database.

- An option has a name, a type and a default, like a part property, with an i18n label and help for the designer and Settings, and an entry in the language reference written from the catalog.
- `csv` reads RFC 4180: quoted fields, doubled quotes, commas and line breaks inside quotes, CRLF or LF, a UTF-8 BOM dropped. The first row names the columns unless `noHeader`, which fills fields in declared order. Writing quotes where needed and writes a header row.

Later entries (each a full step, see "Order"): `xml` (designed under "XML" below), `yaml`, `xlsx` and `ods` (bytes, several tables: the sheets; cell values only, see open questions).

## XML

### What C++ Builder had

- **TXMLDocument**: any XML document as a tree, read, changed and written.
- **XML Data Binding wizard**: read an XML file or its XSD schema and generate typed classes for its elements, so code says `Order->Number` instead of walking nodes.
- **XML Mapper with TXMLTransformProvider**: map any XML document's elements and attributes onto the rows and fields of a dataset (a transformation file per mapping), so a TDBGrid edits an XML file that was never designed for it, and writes the changes back into the same document.

So no XML shape was fixed: the developer said where the rows are and where each field is, and the grid did not care. Clarity does the same with the words it already has, without a separate mapping file or generated classes: the entity is the typed class, its fields are the mapping, and the catalog's options say where the records are.

### Where the records are, where each field is

```clarity
// <shop><orders><order number="A-1" placed="2026-11-01"><customer><name>Ada</name></customer>
//   <lines><line product="Coffee" quantity="2"/></lines><note>leave at door</note></order>...</orders></shop>
let orders = read list<Order> from xml text at "shop/orders/order"
let feed = xml of orders at "shop/orders/order"
```

- `at` says where the records are, in every format that nests: for JSON the list, for XML the repeated element. An XML path is written as XPath writes one, names with slashes between (`"shop/orders/order"`); the last name is the record's element: every `<order>` inside `<shop><orders>` is one record. Without `at`, reading takes the root element's children as the records.
- A field finds its value inside the record's element by name, matched as every format matches names (case, underscores, hyphens and spaces ignored), in this order: a child element of that name, else an attribute of that name. `number` above comes from the attribute, `note` from the child element. The order is fixed, never guessed per document.
- Where a field's value is somewhere else, its `column` says where, the same word that maps a field onto a database column or a CSV column: `column "@number"` is the attribute only, `column "customer/name"` the text of an element inside, `column "."` the record element's own text. A path is relative to the record's element.
- An element's text is its value: numbers, dates, yes and no as every format fits them. The entity says what each element is, since XML has no lists of its own: a field that is a record takes the element of its name (`customer: Customer` from `<customer>`). A field that is a list takes the elements of its own name, repeated (`<tags>a</tags><tags>b</tags>`), or one element of its name holding the items, when that element's children all have one name that is not a field of the item's entity (`<lines><line/><line/></lines>`; `<lines><product>Tea</product></lines>` is one line). No other name is guessed: `<line>` elements directly inside the record fill `lines` only with `column "line"`.
- Namespaces: names match by their local name, so `<o:order>` is an `order`, whatever its prefix. Writing puts no prefix; the `namespace "urn:..."` option writes that namespace as the document's default.
- The XML declaration's encoding is honoured on reading (UTF-8, UTF-16 and the encodings CSV reads); writing is UTF-8 unless `encoding` says otherwise.

### Writing

- `xml of value` writes a record, a list of records or a value, and always takes `at`: it names the elements written around the records and the record element itself, `at "shop/orders/order"` writing `<shop><orders><order>...`, and `at "order"` one record as `<order>`. Without it the checker asks for it; no element name is guessed.
- Each field is written as a child element of its name; a list of values as repeated elements of the field's name, and a list of records inside an element of the field's name holding one element per record, named after its entity in camelCase (`<lines><line>...</line></lines>`). So what Clarity writes it reads back the same. A null value is left out. `indent true` writes one element per line, indented, for files people read. The document starts with the XML declaration.

### A file as a data source

```
datasource feed xml "imports/orders.xml" external
    at "shop/orders/order"
```

- One entity's records, at `at`, kept in the in-memory table as every file source is. The fields map by name or `column` path as above.
- Writing back keeps the whole document: everything outside the records as it was (the declaration, comments, other elements), and inside each record element the attributes and elements no field maps, in their place. A new record gets an element with the mapped fields only; a deleted record's element goes, with what it held.
- A file source is flat, one table: a field that is a record or a list of records inside the record element is kept with the element, unmapped. Nested documents are read and written whole in code (`read list<Order> from xml text`), where records inside records work as in JSON.

### Import, and the XML Data Binding wizard made generic

- **Import...** next to an XML file source lists the repeated elements found in the file, by their path (`shop/orders/order`); ticking one writes an entity with a text field per attribute and child element of its records, each with its `column` path where its name alone would not find it. Types are not guessed from values (determinism).
- **From a schema**: Import also takes an XSD file. Its types become the entities and fields with their real types: `xs:int` and the other whole numbers an `integer`, `xs:decimal` a `decimal`, `xs:date` a `date`, `xs:dateTime` a `datetime`, `xs:boolean` a `bool`, an enumeration an `enum`, `maxLength` a text's size, `minOccurs="1"` `required`, a complex type inside another a record field, `maxOccurs="unbounded"` a list. This is what the Data Binding wizard did, written as Clarity's own entities instead of generated classes, so every stack and every part uses them.

### The catalog entry

| Entry | Text or bytes | Tables | Nests | Options |
|---|---|---|---|---|
| `xml` | text | one per `at` | yes | `at "a/b/record"` (reading, writing, file source), `namespace "urn:..."` (writing), `indent true` (writing), `encoding "..."` (writing) |

The parser is a small pure Rust XML reader in `clarity-formats` (quick-xml, without features), so every stack reads XML alike; XML is turned into the same tree the fit already reads (an element into an object of its attributes and children), and the fit, the table calls and the write-back stay one mechanism for every format.

## A file as a data source

```
datasource main sqlite "data/erasure.db"
datasource logins csv "imports/logins.csv" external readonly
    separator ","
```

- `sqlite` is already a data source that is a file in a format. Each catalog entry is a kind beside it: `csv`, `json`, later `xml` and the rest. No new word `file` (a field type, and the name of `files f in store`, already).
- The path follows SQLite's rules (relative to the project, the program's data folder, `"env:NAME"`, `"setting:name"` for a file the person picks while the program runs). The entry's options are indented lines under it, as `auth` and `paging` are under a `rest` source, and the Settings row shows them from the catalog.
- A file source is always `external`: Clarity does not own its shape and writes no migrations for it. It reuses the external-database words: `table` names the part of the file that holds an entity where a format has several (a JSON path, later a sheet), `column "Login URL"` maps a field, `readonly` refuses saves; a column the entity wants that the file lacks is an error on load.
- `find`, `save` and `delete` work. The runtime reads the whole file into rows in memory and writes it back on save to a new file that replaces the old one, so a crash never leaves half a file.
- Writing back keeps the file's own column order and the columns no entity maps, so saving one record never drops data the person cannot see.
- Every part that binds to records works unchanged: a table part over a CSV file is a CSV editor with no code.
- The existing Import panel and `clarity import` take file sources too: one entity per table, one field per column, every field `text`. Types are not guessed from the values (determinism); the person changes them in the entity editor.
- An encrypted file source is step 2 of APP-CAPABILITIES-PLAN.md.

## Where the code lives

The pattern of Clarity.Image (not clarity-graph, which is a plain crate inside Clarity.UI):

- `runtimes/rust-formats` (crate `clarity-formats`, released on `runtimes/rust-formats/v*` to crates.io), used directly by the Rust stack through a clarity-runtime feature, so a Rust program carries it only when it uses a format.
- `runtimes/rust-formats-ffi` (`clarity-formats-ffi`, `include/clarity_formats.h`, table function `clarity_formats_api()`), `libclarity_formats` prebuilt per system beside Clarity.UI's library, for C#, Go, C and C++.
- C# NuGet `Clarity.Formats` (`runtimes/csharp/Clarity.Formats`, in `Clarity.slnx`), released with the C# runtime.
- The library parses and writes formats: text or bytes to rows of named values and back, with the place of every value for error messages. Rows cross the C interface as JSON.
- An app that reads or writes any format carries the library, only when it uses one (accepted by the user's decision).

### What exists today (survey, 2026-11-10)

From the other session's survey of the tree; file:line references are from that day.

- **Language.** AST `ExprKind::ReadJson { ty, text, at }` and `JsonOf` (crates/clarity-spec/src/code/mod.rs:421-430). The parser knows only the word `json` (parse.rs:1380-1406). The checker wants a record or a list of a non-settings entity and refuses it in engines (check.rs:3092-3122); `json of` gives text (3870-3876).
- **Two stages on every stack.** "Fit": parse the text, walk `at`, lay each field of a shape descriptor over a base (the new record's encoding), matching names and coercing values, with errors that say where. "Decode": the stack's own record codec reads the fitted tree; that codec is also the window wire's encoding. `json of` is the codec's encoder alone.
- **The descriptor** is built once in crates/clarity-gen/src/json_fields.rs (Kind: Text, Integer, Decimal, Bool, Date, DateTime, Enum, Values, Record, Records), then written as five different literals by `rust_json_kind`, `go_json_kind`, `cs_json_kind`, `cpp_json_kind`, `c_json_kind`.
- **The fit is written five times:** runtimes/rust/src/json.rs, runtimes/go/clarity/json.go, runtimes/c/clarity/json_read.c and .h, runtimes/cpp/clarity/json_read.cpp and .hpp, runtimes/csharp/Clarity.Runtime/Json.cs.
- **Tests.** conformance/json.json covers the fit only. Nothing covers `json of` output across stacks except crates/clarity-gen/tests/http_build.rs, which sorts keys first; only C# tests written output (JsonTests.cs).
- **Where the five differ today**, uncovered by conformance:
  - a decimal from a JSON number is exact in Go and C#, but goes through a double in Rust, C and C++ (digits lost);
  - an integer from a float is capped below 9e15 in Rust, C# and C, with no cap in Go;
  - whitespace in number strings is trimmed differently per stack;
  - the plain-name match picks in document order (C#, C), alphabetically (C++, Rust), or at random (Go map iteration, so not deterministic);
  - decode errors are worded differently in C;
  - `json of` key order and number format differ per codec.

Moving JSON into one library fixes all six at once, which is the strongest argument for the user's decision.

### The design

**The library owns parsing and the fit, once, for every format.** Every format parses to one neutral tree (the JSON data model, in the library's own tree type) and is fitted by the same rules, so CSV and every later entry get name matching, defaults, coercion and errors that say where without writing them again.

**What is read or written: a kind.** The generator describes each read and write as a JSON "kind", the same text on every stack: `{"kind":"record","entity":..,"fields":[..],"base":{..}}`, `{"kind":"records",..}` for a list of records, or a plain value kind (`{"kind":"decimal"}`, `{"kind":"values","item":{..}}`). One kind covers a record, a list of records, `json of 5` and lists of values, so there is no separate "many" flag. A record kind lists every field of the entity, those never read as kind `"other"`, so a write knows the full order. The conformance cases use the same `kind` field.

**Reading.** The stack calls the library with the format, the input, the options and the kind; the fitted result comes back as JSON text and the stack decodes it with its existing codec, which stays because the window wire needs it. After a correct fit the decode cannot fail, so C's different wording stops mattering.

**Writing.** The stack encodes the value with its codec, as `json of` does today, and the library writes the format from that JSON and the kind. JSON itself included, normalised, so all five stacks give the same bytes: the record's id first, then the entity's field order; decimals exact; whole numbers plain; date-times normalised; enums as declared.

**A record's base** (its defaults) comes from each stack's codec at run time, because only the codec knows how its own new record decodes. The generator writes the kind in pieces and each stack fills in the base; the kind's format is the same everywhere.

**Messages** name the format by its catalog title: "the text is no JSON", "the JSON has no 'data'". JSON's texts stay word for word as today; CSV's say "the CSV ...".

**Its own JSON tree, no serde_json features.** serde_json's `arbitrary_precision` and `preserve_order` would apply to the whole Rust build and change the IDE's and the runtime's JSON everywhere. So clarity-formats has a small JSON tree and parser of its own (`runtimes/rust-formats/src/tree.rs`): numbers kept as written, names in their order, no dependencies.

**Rust API:**

```rust
pub fn read(format: &str, input: &[u8], options: &Options, kind: &Kind) -> Result<Tree, Problem>;
pub fn write(format: &str, value: &Tree, options: &Options, kind: &Kind) -> Result<Vec<u8>, Problem>;
pub fn catalog() -> Vec<FormatInfo>; // name, title, text or bytes, tables, nests, options with types and defaults

pub struct Problem { pub kind: ProblemKind, pub text: String }
pub enum ProblemKind { NotFitting, UnknownFormat, BadOptions, Failed }
```

`catalog()` is the one list of formats and options; the checker, designer, Settings and agent read it, so nobody keeps a second one. `table` is one of its options for entries that hold several tables. The crate's source is the truth for exact names; this is the shape.

**C table** (`runtimes/rust-formats-ffi/include/clarity_formats.h`), in Clarity.Image's style: versioned, it only grows at the end, `catch_unwind` around every call, the problem text kept per thread. Everything crosses as JSON text or bytes.

```c
typedef struct {
    uint32_t interface;
    int32_t (*read)(format, format_len, input, input_len, options, options_len, kind, kind_len, out, out_len);
    int32_t (*write)(format, format_len, value, value_len, options, options_len, kind, kind_len, out, out_len);
    int32_t (*catalog)(out, out_len);
    void (*free)(bytes, len);
    /* problem(into, size): the last problem's text on this thread */
} ClarityFormatsApi;

const ClarityFormatsApi* clarity_formats_api(void);
```

Return codes: 0 OK, 1 NOT_FITTING, 2 UNKNOWN_FORMAT, 3 BAD_OPTIONS, 4 FAILED. The header is the truth for exact types.

**Generator.** json_fields.rs becomes the writer of kinds, the same JSON on every stack; the five per-stack kind emitters go. Each call site becomes: read (format, text or bytes, options literal, kind with the stack's base), then decode with the codec; or encode with the codec, then write. The library joins a program only when it reads or writes a format, as pictures do.

**Language.** `read T from <format> <expr> [options]` and `<format> of <expr> [options]`, the format a name from the catalog; `at` becomes one of the options. AST `ReadFormat { format, ty, input, options }` and `FormatOf`; `ReadJson` and `JsonOf` are removed. The checker takes formats and option types from the catalog, refuses nested records or lists of records for a flat format (csv) at compile time naming the field, and keeps today's rules on T. Today's `read ... from json` and `json of` stay valid syntax, so the OpenAPI importer (crates/clarity-project/src/openapi.rs) needs no change.

**Per runtime.**

| | Kept | Removed | How it calls the library |
|---|---|---|---|
| All | the codec and wire, settings.json, engines' value passing, HTTP paging's merging of pages (a transport, not a format), database list columns | the fit implementation and the JsonKind/JsonField types | a thin read and write plus the codec |
| Rust | | runtimes/rust/src/json.rs fit | clarity-runtime feature `formats = ["dep:clarity-formats"]`, called directly |
| Go | | clarity/json.go fit | cgo in generated code, as pictures do (crates/clarity-gen/src/go/mod.rs), only when used |
| C, C++ | | json_read.c/.h, json_read.cpp/.hpp | runtimes/c/formats and runtimes/cpp/formats like pictures/, `clarity_use_formats()` in clarity.cmake |
| C# | | Json.cs fit | `delegate* unmanaged` table like Clarity.Image's PictureFunctions.cs |

**Conformance.**

- conformance/json.json moves into the library's own tests (the fit, once, exhaustively) as `formats-json-read.json`.
- New `formats-csv.json` (step 2): read and write, unmapped columns kept on write-back.
- The divergences, decided in `formats-json-read.json` (29 cases: today's 23 unchanged plus 6):
  - a decimal from a number keeps every digit, and an exponent is written out (`1e3` is `"1000"`);
  - a whole number is any 64-bit integer, also when written as `3.0` or `2e3`;
  - spaces around number strings are trimmed;
  - of names that match loosely, the first written counts;
  - a number read as text keeps how it was written (`1.50` is `"1.50"`).
- `formats-json-write.json`: 5 byte-for-byte cases (id first then field order, exact decimals, plain whole numbers, normalised date-times, enums as declared).
- Each runtime runs a small `formats-runtime.json` end to end through its wrapper and codec: a nested record read, an error passed through word for word, a write. The generator's http_build test stops sorting keys.

**Docs and agents.** LANGUAGE.md's JSON section becomes Formats (documenting that spaces are stripped in name matching, which happens today but is not written down); docs/manual/code.web-apis.md; a manual page for formats; DESIGN.md 5.23 and 5.25. Agents learn the construct from the reference embedded in clarity-tools, so the docs are the agent change; the agent tools for file data sources come with step 2.

## Order

Each step is done only when every item of its list is done. Nothing of a later step is started or merged before.

### Step 1: the catalog, with JSON moved in

1. `clarity-formats` and `clarity-formats-ffi` with the `json` entry and `catalog()`; the C table; the NuGet; the release workflows and the native library staged beside Clarity.UI's.
2. Conformance: `formats-json-read.json` (29 cases: today's 23 plus 6 deciding the divergences), `formats-json-write.json` (5 byte-for-byte cases), `formats-runtime.json` passing on all five stacks.
3. Every runtime's reading and `json of` routed through the library; the five fits and the JsonKind/JsonField types removed in the same step.
4. Spec: `ReadFormat` and `FormatOf` replace `ReadJson` and `JsonOf`; parser, checker and typing read the catalog.
5. Generator: kinds written as JSON replace the five per-stack kind emitters; the library added per stack only when used; http_build no longer sorts keys.
6. Manuals, every page that step 1 changes or makes wrong:
   - a new page `docs/manual/code.formats.md` (formats, the catalog, options, name matching including stripped spaces, errors), listed in `docs/manual/README.md` and registered in the IDE's manual (`crates/clarity-tools/src/manual.rs`, the `include_str!` topic list the IDE's manual window, F1 and the agent's `manual` tool all read); without that entry the page exists on disk but not in Clarity;
   - `docs/manual/code.web-apis.md`: its JSON part shortened to how a reply is read, linking to the formats page;
   - `docs/manual/stacks.go.md`: "A web program alone needs no cgo and no native library" is no longer true for a program that reads or writes a format; say when cgo and `libclarity_formats` come in;
   - `docs/manual/stacks.rust.md`, `stacks.c.md`, `stacks.cpp.md`, `stacks.csharp.md`: the formats library carried when used (Rust feature, native library beside Clarity.UI's, the `Clarity.Formats` NuGet);
   - `docs/manual/project.run-build.md` and `docs/manual/setup.tools.md`: the tools each stack needs (nlohmann-json for C++ stays or goes, depending on whether the codec still uses it; a C compiler for Go programs that use a format);
   - LANGUAGE.md's JSON section becomes Formats; DESIGN.md 5.23 and 5.25;
   - agent instructions and the reference embedded in clarity-tools. (The catalog's option label keys, such as `formatOptionAt`, have no screen in step 1, since formats are a code construct there; they come with step 2.)
7. Every example builds on its own stack and the examples that read JSON run as before.

   Built 2026-11-10. Decided while building: writing gives a decimal in its shortest exact form (`12.50` is written `12.5`), because the stacks' decimal types keep or drop trailing zeros differently; Go reads one record as `*Order` and a list as `[]Order`, as its generated code holds them; a Go program's cgo calls are a generated `formats.go` (its runtime module stays pure Go, its own test of the library behind the build tag `clarityformats`); Go, C and C++ programs of every target (command line and web too) get Clarity.UI's folder when their code uses a format (`clarity_ui_folder` and `clarity_use_formats` in clarity.cmake, `needs_library_folder` in the generator, the IDE's build). The generated programs of `tests/http_build.rs` read and write JSON identically on all five stacks.

### Step 2: CSV, in code and as a data source

1. The `csv` entry with its options; conformance cases (quoted line breaks, BOM, empty last field, `noHeader`, separators, write-back) passing on all five stacks.
2. File data sources for every catalog entry (`csv`, `json`): parsing, checker, external mapping words, find/save/delete, write-back keeping unmapped columns, atomic replace.
3. Settings rows for file sources with the entry's options; the Import panel and `clarity import` for file sources; the data browser shows them.
4. Agent tools (add a file data source, its options) and instructions; language reference; i18n keys, including the catalog's option label keys (`formatOptionAt` and the rest) for the Settings rows of file sources.
5. The IDE's manual, for what the person does in Clarity: `data.sources.md` (file sources, their Settings row with the format's options, Import of a file), `project.file.md` (the new data source kinds and the Settings section), `code.formats.md` (csv), each topic in `crates/clarity-tools/src/manual.rs`.
6. An example that reads a CSV in code and edits a CSV data source in a table part (the erasure example's import, or one of its own).
7. Every example building green.

   Built 2026-11-12. Decided while building: a file source is one table of an in-memory SQLite database in each runtime, filled through Clarity.Formats (`read_table`, `write_table`, C interface 2, `conformance/formats-table.json`), so the generated find, save and delete code is the SQLite code; each change kept (outside a transaction, or at its commit) writes the file again to `<path>.new`, which then replaces it. A file not there yet starts empty with the entity's columns and is made at the first save (a readonly source needs its file). Without an `id` column, a record's id is its row's place, never written. The generators write `add_file` per file source after connecting (`crates/clarity-gen/tests/file_sources.rs`); `carries_formats` decides in one place whether a program carries the library. Settings offers **CSV file** and **JSON file** with the format's options from the catalog; Import takes a file source as one table of text fields. A yes/no option is written with its value, `noHeader true`. The IDE has no data browser yet, so item 3's "the data browser shows them" waits for one. The example is the shop's price list (`examples/shop/prices.clarity`).

### Step 3: XML, in code and as a data source

1. The `xml` entry with its options; the record path (`at`), field paths (`column "@a"`, `"b/c"`, `"."`), records and lists inside records, namespaces by local name; conformance cases (attributes and elements, a path, namespaces, entities and CDATA, an encoding declaration, writing with and without `at`, indent) passing on all five stacks.
2. XML file sources: the table calls for `xml`, writing back the whole document with unmapped attributes and elements kept in their place; runtime tests on all five stacks.
3. The checker: `column` paths on fields of entities read from XML or held in an XML file source; a path that cannot be (`"@a/b"`) is an error at the field; `xml of` without `at` is an error at the line.
4. Settings: **XML file** with its options; Import of an XML file (its repeated elements) and of an XSD (typed entities); `clarity import` for both.
5. Language reference, manual (`code.formats.md`, `data.sources.md`, `project.file.md`), agent instructions, i18n keys of the new options.
6. An example: an existing example gains an XML feed or settings file, reading it in code and editing it as a data source.
7. Every example building green.

   Built 2026-11-12; its implementation was then moved onto the one document model and engine of step 4 (see there), which replaced its per-format code (`xml_doc.rs`, `xml_table.rs`, `xml_describe.rs`). What stays from it: XML's reading rules (a child element before an attribute, the wrapper rule for lists, `xml of` needing `at` through the catalog's `required_writing`), write-back keeping a whole document, Import listing every table a file holds and writing typed fields from an XSD (into a file source the top entity only), and the examples (stock's deliveries in `data/deliveries.xml`, shop's editable CSV price list).

### Step 4: mappings, one per case, for every format

The user: "the still points should be configured per case not carved in stone beforehand!", and on the step 5 first written here per format: "you're not taking this generic!!!!!!!!".

What C++ Builder's XML Mapper did for XML (a transformation file per case, saying where the rows and each field are), Clarity does for every format with one declaration, the **mapping**: how one kind of file is laid over an entity, configured per case, in the Clarity project, used in code and on data sources alike. Nothing in it is particular to a format; each format only reads and writes its own syntax behind the `Format` interface.

```clarity
mapping OrderFeed of Order from xml
    at "shop/orders/order"
    number at "@number"
    customer at "customer/name"
    lines of Line at "lines/line"
        order is ".."
    check "orders.xsd"
    indent true

let orders = read list<Order> from text using OrderFeed
let feed = text of orders using OrderFeed
datasource feed file "imports/orders.xml" external using OrderFeed
```

- **One path style for every format**: names with slashes between, `@name` for an attribute where a format has them, `.` for a record's own text, `..` for the record around it. JSON, YAML and XML read the same paths; a format without attributes refuses `@` with a message.
- **What a mapping says**, every line optional, each one a choice for this case only:
  - `at`: where the records are (else a format's own default: the whole file for JSON and YAML, the root's children for XML, the rows for CSV);
  - `field at "path"`: where a field is (else found by its name, as today);
  - `field of Entity at "path"`: records inside, another entity from the same file, with `field is ".."` naming the record they sit in (several entities from one file, linked, per mapping);
  - `check "file"`: a schema the file is checked against while it is read (a value that breaks it stops the read with a message that says where); a format says which schema kinds it takes (`schema_kinds`);
  - the format's own options (`separator ";"`, `indent true`), from the catalog as today.
- **Without a mapping** everything works as it does now: `read list<Order> from xml text at "..."` is a mapping written inline, and a file source's indented options are one too.
- **The IDE**: a mapping is edited as a page of its own (the format, the entity, a row per field with its path, the entities inside), with a preview of a sample file laid over it; Import writes a mapping beside the entities it writes, so the paths it found are kept and can be changed.
- **The checker**: a path a format cannot have, a field the entity lacks, a schema file that is not there.

### How it is built: one document model, one engine

General purpose in the implementation too, not only in the language:

- **One document model** in `clarity-formats`: a node has a name, attributes (where the format has them), its text, and child nodes in their order, and keeps what a writer needs to write a file back as it was (comments, order, quoting, spacing). Every format parses into it and writes from it: XML elements are nodes; a JSON or YAML object is a node whose children are its members, a list is repeated children of one name; a CSV is a root of row nodes whose children are the columns.
- **One engine** on that model, written once for every format: paths (`at`, a field's path, `..`), what a record and a field are (a value, a record inside, a list), fitting to the entity, the table of a file source (several tables when a mapping has entities inside), writing records back into the document with what no field maps kept in place, and the schema check (a format turns its schema kind into the engine's one rule set).
- **A format is its syntax only**: `parse(bytes) -> document`, `write(document) -> bytes`, and its schema kinds. JSON, CSV and XML move onto it; YAML is added on it. Formats from libraries (a later step) implement the same two calls.
- **A mapping crosses to every stack as data**: the generator writes a mapping as the options object every stack already passes (`{"at": "shop/orders/order", "fields": {"number": "@number"}, "nested": {"lines": {"of": "Line", "at": "lines/line", "parent": "order"}}, "check": "orders.xsd", "indent": true}`), so the C interface and the five runtimes take mappings without a change of their own, except where a file source holds several entities (the table JSON then holds several tables).

Decided by the user on 2026-11-12: build mappings first ("Yes, build mappings"); one path style for every format, slashes, with JSON's released dots still read ("Slashes, dots still read").

Built so far (2026-11-12):

- **The library** (`runtimes/rust-formats/src`): one model (`doc.rs`: a node has a name, a form, attributes and children; a document keeps what a writer needs), formats as syntax only behind `Format` (`info`, `parse`, `write`, `schema`; `json.rs`, `csv.rs`, `xml.rs` with `markup.rs`, `xsd.rs`), and one engine (`path.rs`, `mapping.rs`, `shape.rs` then the one fit, `build.rs`, `table.rs`, `describe.rs`). A format's entry says how its records lie (`nodes`: values, elements or rows), whether paths may name attributes (`attributes`), its path separator and schema kinds. `conformance/formats-mapping.json` runs the same mapping cases against JSON, XML and CSV.
- **Paths**, as built: slashes; `@name` an attribute where the format has them; `.` the own text and `./name` a name holding a dot; `..` the record a record was found in; an `at` without a slash is split at dots, so released JSON paths read as before.
- **The language**: `mapping Name of Entity from <format>` with `at`, `field at "path"`, `field of Entity at "path"` and its lines, `field is ".."`, `check "file"` and the format's options; `read ... from text using M`, `text of x using M`; `datasource feed file "x" external using M`. The generator writes a mapping as the options object (`at`, `fields`, `nested` with `parent`, the format's options) for all five stacks; the checker reads the catalog for attributes, nesting and schema kinds.
- **`check`**: the project is loaded with the schema file's text (`MappingDecl.schema`), the generator writes it into the options (`"check": "<xs:schema ...>"`), so no stack reads files for the library; the engine (`check.rs`) reads it with the format's `schema` and checks the whole document before fitting, in `read` and a file source's `read_table` (`conformance/formats-check.json`). A format without schema kinds refuses it.
- **Several entities in one file source**: a mapping with records inside (`lines of Line`) makes the file source hold both entities on all five runtimes. The generator passes the records field among the fields with its kind and the table and key its entity is stored in (`{"name":"lines","kind":"records","entity":"Line","fields":[...],"base":{},"table":"line","key":"id"}`), and the mapping's field lines and `order is ".."` by column; the checker wants the inner entity in the same source. Each runtime keeps one SQLite table per table the library gives, turns a link (the place of the row it sits in) into that row's key on reading and back into its place on writing, and writes all tables back after any change.
- **The IDE**: a mapping page (`app/src/lib/MappingPage.svelte`), opened from **Mappings** in the project tree (**+ New mapping** asks for the entity, the format and the name with Clarity's dialogs) and from **Mapping...** next to a file source laid over one. It edits the mapping as a form (`clarity_spec::code::mapping::MappingForm`, `form_of`, `text`): the format, the entity, `at`, a row per field with its path, records inside as nested rows with their `is ".."` field, `check` through Clarity's file browser, and the format's options with the rows Settings shows (`FormatOptions.svelte`). Each change is written back into its module by the printer as one edit (`save_mapping`). A sample file picked with the file browser is read through the mapping's options object (`options_json`, the generator's too) by `clarity_formats::read_table`, the first records shown as the entity's fields hold them, or the read's message (`clarity_project::mappings::preview_mapping`).
- **Import writes a mapping**: beside the entity of a file source, `mapping <Entity>Feed of <Entity> from <format>` with the `at`, the paths that differ from the fields' names and the source's options, and the source becomes `datasource x file "path" external using <Entity>Feed`. A source laid over a mapping already keeps it.
- **Not yet**: examples using a mapping.

### Step 5: YAML

YAML is one more implementation of `Format` (reading YAML 1.2's core schema into the tree every format reads, writing block YAML, a file source that keeps the file as written outside what changed, Import by `describe`); the paths, mappings, checks and Settings it uses are those of step 4, so the step is its parser and writer, their conformance cases on all five stacks, and an example.


   Built 2026-11-12: `runtimes/rust-formats/src/yaml.rs`, parse and write only; its entry is JSON's (values, no attributes, `at`), so paths, mappings, fitting, file sources and Import are the engine's. The model gained one generic field, a document's text as read (`Kept.text`), so write-back keeps a YAML file as written (comments, quoting, untouched lines). Decided while building: numbers JSON cannot write (`007`, `.5`, `0x1F`) become the number they equal; `.inf` and `.nan` stay text; complex keys are refused; a flow list is written in block style once it changes. Conformance: `formats-yaml.json`, YAML inputs in `formats-mapping.json` and `formats-table.json`, a YAML case in `formats-runtime.json`.
### Later steps, each with the same list

- `xlsx` and `ods`: formats like csv, but bytes and several tables (the sheets).
- Formats from libraries, when a project needs one: a library declares `format name`, its adapter (in the project's own language) turns text or bytes into rows and back, and the runtime fits them as for a built-in entry.

## Decided

XML, answered by the user on 2026-11-12: "xpath slashes, element first, require at, start step 3". XML paths are written with slashes (`"shop/orders/order"`, `"customer/name"`); a field takes a child element of its name before an attribute of its name; `xml of` always takes `at`.

Answered by the user, relayed by the other session on 2026-11-10:

- Go: cgo is accepted. A Go program that reads or writes a format links the library, through cgo generated only when it uses one. One implementation everywhere.
- `json of` goes through the library, normalised, so every stack writes the same bytes.
- C#: its own NuGet, `Clarity.Formats`, carried only when a program reads or writes a format.
- JSON moves into the library (the later of the two answers under "The user's words"); the three answers above build on it.

## Open questions

Step 4, waiting for the user's word:

- Whether `check` also takes JSON Schema for JSON and YAML files, beside XSD for XML.

- Spreadsheet styling (widths, colors, formulas) on writing: an option of the `xlsx` entry, or a report output (REPORT-PLAN.md). Reading takes cell values only.
- A file source picked while the program runs (`"setting:name"`): reloaded when the setting changes, or at the next start.
