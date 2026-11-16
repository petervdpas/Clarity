# Data formats

Status: partial. One catalog of data formats (json, csv, xml, yaml) in Clarity.Formats, used by code (`read ... from <format>`, `<format> of`), by mappings, and by data sources that are files in a format.

Progress: 55%

## Code
- Library: `runtimes/rust-formats` (crate `clarity-formats` 0.2.0; one document model `doc.rs`, formats as syntax only behind `Format` in `format.rs`: `json.rs`, `csv.rs`, `xml.rs` with `markup.rs` and `xsd.rs`, `yaml.rs`; one engine `path.rs`, `mapping.rs`, `shape.rs`, `fit.rs`, `build.rs`, `table.rs`, `describe.rs`, `check.rs`; own JSON tree `tree.rs`); C table `runtimes/rust-formats-ffi` (`include/clarity_formats.h`, `clarity_formats_api()`)
- Spec: `ReadFormat`, `FormatOf` (`crates/clarity-spec/src/code/mod.rs`), `mapping M of E from fmt` (`code/parse.rs:1096`, `code/mapping.rs`), `formats.rs`, `xml.rs`; the catalog read by parser, checker and highlighter
- Project file: file sources `datasource x csv|json|xml|yaml|file "path" external [using M]` with options (`crates/clarity-project/src/file.rs:504-570`); Import of files `file_import.rs`; `mappings.rs` (`preview_mapping`)
- Generators: kinds as JSON `crates/clarity-gen/src/shape.rs`; file sources `file_sources` (`crates/clarity-gen/src/lib.rs:99-135`); `carries_formats` (lib.rs:201) decides when a program carries the library
- Runtimes: R `runtimes/rust/src/formats.rs`, `file_source.rs` (feature `formats`); Go `runtimes/go/clarity/formats.go`, `formats_library.go`, `file_source.go` (generated cgo `formats.go`); C `runtimes/c/formats/{formats,file_source}.c`; C++ `runtimes/cpp/formats/{formats,file_source}.cpp`; C# NuGet `Clarity.Formats` (`FormatFunctions.cs`, `FileSource.cs`). A file source is one in-memory SQLite table per table, written back to `<path>.new` then replaced.
- IDE: `app/src/lib/MappingPage.svelte` (mapping form, sample preview), `FormatOptions.svelte` (options from the catalog), `ImportPanel.svelte`; `app/src-tauri/src/mappings.rs`
- CLI: `clarity import <project> <source>` for CSV, JSON, XML (and XSD) files
- Docs: manual `code.formats.md` (Mappings), `data.sources.md` (A file as a data source), `project.file.md`

## Tests
- `conformance/formats-json-read.json`, `formats-json-write.json`, `formats-csv.json`, `formats-xml.json`, `formats-xml-table.json`, `formats-yaml.json`, `formats-mapping.json`, `formats-table.json`, `formats-check.json`: run by `runtimes/rust-formats/tests/conformance.rs` (once, in the library)
- `conformance/formats-runtime.json`: run by R (`tests/formats.rs`), Go (`formats_test.go`), C (`formats_test.c`), C++ (`formats_test.cpp`), C# (`FormatFunctionsTests.cs`, `JsonTests.cs`)
- `runtimes/rust-formats/tests/` mapping.rs, xml.rs, xml_table.rs, yaml.rs, check.rs; spec formats.rs, mapping.rs, xml.rs, csv.rs, json.rs; `crates/clarity-project/tests/mappings.rs`; gen `file_sources.rs`, `mapping_check.rs`, `http_build.rs` (identical bytes on all five)
- Examples: shop (CSV price list), stock (XML deliveries), todo (YAML ideas through a mapping)

## Open
- The data browser shows file sources: NOT BUILT (no data browser); see Design "In the IDE".
- An agent tool to add a file data source: NOT BUILT; see Design "In the IDE".
- Reading formats from bytes: NOT BUILT; see Design "Bytes".
- Choosing one table of a file that holds several: NOT BUILT; see Design "Several tables".
- xlsx and ods: NOT BUILT; see Design "Spreadsheets".
- Formats from libraries: NOT BUILT; see Design "Library formats".
- Text functions `encoded(text)` and `plainText(html)`: NOT BUILT; see Design "Text functions".
- Encrypted file sources: see [encrypted-storage](encrypted-storage.md). Shared core for file sources: NOT BUILT ([modularity](modularity.md) step 2).

## Design

Each item is a complete slice: all five stacks, conformance, checker, designer, Settings rows, agent tools, manual, i18n, an example, every example building.

### Bytes

- The same shapes with `bytes` for formats that are not text: `read list<Order> from xlsx bytes`, and a mapping used on bytes.

### Several tables

- A catalog entry already says how many tables a file holds. Where it holds several (a spreadsheet's sheets), code and a file source name the one meant. Whether that is the mapping's `at` or a separate `table "Orders"` line is open.

### Spreadsheets

- `xlsx` and `ods` as catalog entries: bytes, several tables (the sheets), cell values only.
- Built into `clarity-formats` (Rust `calamine` to read, `rust_xlsxwriter` to write) or left to a library format: open.

### Library formats

- A library declares `format name`; its adapter, in the project's own language, implements the two calls a built-in format has (parse to the document model, write from it), and the engine does the rest.

### Text functions

- `encoded(text)`: percent-encoding for a URL part, so `"mailto:{to}?subject={encoded(subject)}"` works.
- `plainText(html)`: the readable text of an HTML page, for a model or a memo.
- They join the language with the next change that touches text functions.

### In the IDE

- When the IDE gets a data browser, it shows file sources as it shows databases.
- An agent tool adds a file data source with its format's options from the catalog (there is none for data sources today).

### Open questions

- `check` taking JSON Schema for JSON and YAML, beside XSD for XML.
- Built in beyond json, csv, xml and yaml: toml and ini, or library formats.
- Spreadsheet styling (widths, colors, formulas) on writing: an option of `xlsx`, or a report output ([reports-pdf](reports-pdf.md)).
- A file source at a `"setting:name"` path: reloaded when the setting changes, or at the next start.

## Conflicts
- Go's file-source tests carry the `clarityformats` tag but CI runs `-run Formats`, which does not match their names (`.github/workflows/checks.yml:207`, `runtimes/go/clarity/file_source_test.go`).

## Decisions
- 2026-11-10: formats are one generic catalog, never one format. User: "csv is just one format of data... making it specific isnt very generic, is it... you need to keep thinking c++ builder and what you could make with that back in the days"
- 2026-11-10: written once in a shared Rust crate with a C interface. User: "shared Rust library, go with that"
- 2026-11-10: each step a complete slice on every stack. User: "be sure we are not going to do half implementations scattered all over!!!!!"
- 2026-11-10: JSON moves into the library and `json of` writes the same bytes on every stack (relayed: keeping JSON per runtime "leaves a formats crate with CSV and JSON somewhere else, which removes the strength of the idea").
- 2026-11-10: Go accepts cgo when a program uses a format; C# gets its own NuGet `Clarity.Formats`; the library joins a program only when used.
- 2026-11-12: a file source is always `external`, find/save/delete work, write-back keeps unmapped columns, types are never guessed on Import.
- 2026-11-12: XML. User: "xpath slashes, element first, require at, start step 3"
- 2026-11-12: mappings per case, one path style for every format. User: "yaml! and the still points should be configured per case not carved in stone beforehand!", "Yes, build mappings", "Slashes, dots still read"

## Was
- docs/FORMATS-PLAN.md (all: steps 1-5, "In code" (bytes), "The catalog" (`table`), step 2 items 3-4, "Later steps", "Decided", open questions)
- docs/APP-CAPABILITIES-PLAN.md step 1 (`encoded`, `plainText`), "Where the code lives" (formats column), open questions (toml, ini, spreadsheets, write-back)
- DESIGN 5.23 (JSON second slice, "Moved into Clarity.Formats"), 5.25 (JSON part)
- MODULAR-PLAN.md order 2 (file sources)
- IMPLEMENTED.md 6 "Files and formats" (formats, file sources)
- UNIMPLEMENTED.md 1 (`encoded`, `plainText`), 6 "Formats"
- CONFLICTING.md 3 (Go file-source tests), 9 (released or not, mapping example)
- memory: formats-plan.md
