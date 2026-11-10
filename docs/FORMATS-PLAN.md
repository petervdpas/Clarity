# Formats plan

One catalog of data formats, written once in the shared Rust library `clarity-formats`, used by code (`read ... from <format>`, `<format> of`) and by data sources that are files in a format. Step 1 of APP-CAPABILITIES-PLAN.md, split out on 2026-11-10 because it grew. Step 1 built 2026-11-10 (unreleased: clarity-formats 0.1.0, clarity-runtime 0.54.0, Clarity.UI library 0.18.0, C# 0.90.0 with Clarity.Formats, Go v0.57.0); step 2 not built.

## The user's words

- "I want you to make a plan for it, to add them in the most generic way possible so it improves the sort of apps clarity can create!"
- "csv is just one format of data... making it specific isnt very generic, is it... you need to keep thinking c++ builder and what you could make with that back in the days"
- "shared Rust library, go with that"
- "be sure we are not going to do half implementations scattered all over!!!!!"
- "the both of you, work out the formats crate throught clarity!!"

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

Later entries (each a full step, see "Order"): `xml` (which elements are rows, attributes: to design in its own step), `yaml`, `xlsx` and `ods` (bytes, several tables: the sheets; cell values only, see open questions).

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

### Later steps, each with the same list

- XML (its row and attribute design first), YAML.
- `xlsx` and `ods`: formats like csv, but bytes and several tables (the sheets).
- Formats from libraries, when a project needs one: a library declares `format name`, its adapter (in the project's own language) turns text or bytes into rows and back, and the runtime fits them as for a built-in entry.

## Decided

Answered by the user, relayed by the other session on 2026-11-10:

- Go: cgo is accepted. A Go program that reads or writes a format links the library, through cgo generated only when it uses one. One implementation everywhere.
- `json of` goes through the library, normalised, so every stack writes the same bytes.
- C#: its own NuGet, `Clarity.Formats`, carried only when a program reads or writes a format.
- JSON moves into the library (the later of the two answers under "The user's words"); the three answers above build on it.

## Open questions

- Spreadsheet styling (widths, colors, formulas) on writing: an option of the `xlsx` entry, or a report output (REPORT-PLAN.md). Reading takes cell values only.
- A file source picked while the program runs (`"setting:name"`): reloaded when the setting changes, or at the next start.
