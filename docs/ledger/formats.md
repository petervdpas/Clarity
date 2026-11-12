# Data formats

Status: partial. One catalog of data formats (json, csv, xml, yaml) in Clarity.Formats, used by code (`read ... from <format>`, `<format> of`), by mappings, and by data sources that are files in a format.

## Code
- Library: `runtimes/rust-formats` (crate `clarity-formats` 0.2.0; one document model `doc.rs`, formats as syntax only behind `Format` in `format.rs`: `json.rs`, `csv.rs`, `xml.rs` with `markup.rs` and `xsd.rs`, `yaml.rs`; one engine `path.rs`, `mapping.rs`, `shape.rs`, `fit.rs`, `build.rs`, `table.rs`, `describe.rs`, `check.rs`; own JSON tree `tree.rs`); C table `runtimes/rust-formats-ffi` (`include/clarity_formats.h`, `clarity_formats_api()`)
- Spec: `ReadFormat`, `FormatOf` (`crates/clarity-spec/src/code/mod.rs`), `mapping M of E from fmt` (`code/parse.rs:1091`, `code/mapping.rs`), `formats.rs`, `xml.rs`; the catalog read by parser, checker and highlighter
- Project file: file sources `datasource x csv|json|xml|yaml|file "path" external [using M]` with options (`crates/clarity-project/src/file.rs:474-540`); Import of files `file_import.rs`; `mappings.rs` (`preview_mapping`)
- Generators: kinds as JSON `crates/clarity-gen/src/shape.rs`; file sources `crates/clarity-gen/src/lib.rs:96-136`; `carries_formats` (lib.rs:200) decides when a program carries the library
- Runtimes: R `runtimes/rust/src/formats.rs`, `file_source.rs` (feature `formats`); Go `runtimes/go/clarity/formats.go`, `formats_library.go`, `file_source.go` (generated cgo `formats.go`); C `runtimes/c/formats/{formats,file_source}.c`; C++ `runtimes/cpp/formats/{formats,file_source}.cpp`; C# NuGet `Clarity.Formats` (`FormatFunctions.cs`, `FileSource.cs`). A file source is one in-memory SQLite table per table, written back to `<path>.new` then replaced.
- IDE: `app/src/lib/MappingPage.svelte` (mapping form, sample preview), `FormatOptions.svelte` (options from the catalog), `ImportPanel.svelte`; `app/src-tauri/src/mappings.rs`
- CLI: `clarity import <project> <source>` for CSV, JSON, XML (and XSD) files
- Docs: manual `code.formats.md`, `data.sources.md`, `project.file.md`; LANGUAGE.md section 10 "Data formats" (:2795), "Mappings" (:2828)

## Tests
- `conformance/formats-json-read.json`, `formats-json-write.json`, `formats-csv.json`, `formats-xml.json`, `formats-xml-table.json`, `formats-yaml.json`, `formats-mapping.json`, `formats-table.json`, `formats-check.json`: run by `runtimes/rust-formats/tests/conformance.rs` (once, in the library)
- `conformance/formats-runtime.json`: run by R (`tests/formats.rs`), Go (`formats_test.go`), C (`formats_test.c`), C++ (`formats_test.cpp`), C# (`FormatFunctionsTests.cs`, `JsonTests.cs`)
- `runtimes/rust-formats/tests/` mapping.rs, xml.rs, xml_table.rs, yaml.rs, check.rs; spec formats.rs, mapping.rs, xml.rs, csv.rs, json.rs; `crates/clarity-project/tests/mappings.rs`; gen `file_sources.rs`, `mapping_check.rs`, `http_build.rs` (identical bytes on all five)
- Examples: shop (CSV price list), stock (XML deliveries), todo (YAML ideas through a mapping)

## Open
- The data browser shows file sources: NOT BUILT (no data browser; FORMATS-PLAN.md step 2 item 3).
- An agent tool to add a file data source: NOT BUILT (FORMATS-PLAN.md step 2 item 4).
- Reading formats from bytes: NOT BUILT (FORMATS-PLAN.md "In code").
- `table "Orders"` for files with several tables: NOT BUILT (FORMATS-PLAN.md "The catalog").
- xlsx and ods; toml and ini: NOT BUILT (FORMATS-PLAN.md "Later steps"; APP-CAPABILITIES-PLAN.md open questions).
- Formats from libraries (`format name` in a library): NOT BUILT.
- JSON Schema in `check` for JSON and YAML: NOT BUILT, open question.
- A file source at a `setting:` path reloading when the setting changes: NOT BUILT, open question.
- Text functions `encoded(text)` and `plainText(html)`: NOT BUILT (APP-CAPABILITIES-PLAN.md step 1).
- Encrypted file sources: see [encrypted-storage](encrypted-storage.md). Shared core for file sources: NOT BUILT (MODULAR-PLAN.md order 2).

## Conflicts
- Go's file-source tests carry the `clarityformats` tag but CI runs `-run Formats`, which does not match their names (`.github/workflows/checks.yml:207`, `runtimes/go/clarity/file_source_test.go`).
- FORMATS-PLAN.md:3 says steps 2-5 unreleased; they are in tag v0.39.0. FORMATS-PLAN.md:334 says no example uses a mapping; todo does (`examples/todo/ideas.clarity:13`).

## Decisions
- 2026-11-10: formats are one generic catalog, never one format. User: "csv is just one format of data... making it specific isnt very generic, is it... you need to keep thinking c++ builder and what you could make with that back in the days"
- 2026-11-10: written once in a shared Rust crate with a C interface. User: "shared Rust library, go with that"
- 2026-11-10: each step a complete slice on every stack. User: "be sure we are not going to do half implementations scattered all over!!!!!"
- 2026-11-10: JSON moves into the library and `json of` writes the same bytes on every stack (relayed: keeping JSON per runtime "leaves a formats crate with CSV and JSON somewhere else, which removes the strength of the idea").
- 2026-11-10: Go accepts cgo when a program uses a format; C# gets its own NuGet `Clarity.Formats`; the library joins a program only when used.
- 2026-11-12: a file source is always `external`, find/save/delete work, write-back keeps unmapped columns, types are never guessed on Import (FORMATS-PLAN.md "A file as a data source").
- 2026-11-12: XML. User: "xpath slashes, element first, require at, start step 3"
- 2026-11-12: mappings per case, one path style for every format. User: "yaml! and the still points should be configured per case not carved in stone beforehand!", "Yes, build mappings", "Slashes, dots still read"

## Was
- FORMATS-PLAN.md (all: steps 1-5, "Later steps", "Decided", open questions)
- APP-CAPABILITIES-PLAN.md step 1, "Where the code lives" (formats column)
- DESIGN 5.23 (JSON second slice, "Moved into Clarity.Formats"), 5.25 (JSON part)
- MODULAR-PLAN.md order 2 (file sources)
- IMPLEMENTED.md 6 "Files and formats" (formats, file sources)
- UNIMPLEMENTED.md 1 (`encoded`, `plainText`), 6 "Formats"
- CONFLICTING.md 3 (Go file-source tests), 9 (released or not, mapping example)
- memory: formats-plan.md
