# Runtime data: one data library for what a running program holds

Status: partial. One library of the runtime data every stack holds while a program runs (records, their lists, their values), built once in Rust; step 1 in progress.

Progress: 30%

## Code
- Library: `runtimes/rust-data` (crate `clarity-data` 0.1.0: `tree`, `kind`, `numbers`, `fit`, `normal`, moved from Clarity.Formats, which gives them on with `pub use`); release `.github/workflows/release-rust-data.yml`, unit `clarityData` before `clarityFormats` (`crates/clarity-gen/src/tools/devtools/release.rs`)
- Held lists: spec `Project::held_list_key`, `held_lists` (`crates/clarity-spec/src/model.rs`), checker `check_held_lists`; shapes `crates/clarity-gen/src/shape.rs` (a held list as records, one level); runtimes R `Record::load_lists`, `Db::find_alone` (`runtimes/rust/src/lib.rs`), Go `HoldsLists`, `FindAlone` (`runtimes/go/clarity/db.go`), C# `IRecord.LoadLists`, `Db.FindAlone`, C++ `detail::holds_lists`, `Db::find_alone`; C through the generated `<entity>_load_lists` around `clarity_find`; generators `held_lists` in each stack's `model.rs`, records holding their stored lists (Go `record`)

## Tests
- `runtimes/rust-data` (moved unit tests), `runtimes/rust-formats` (all targets, through the re-export)
- Held lists: `crates/clarity-spec/tests/held_lists.rs`; runtimes `runtimes/rust/tests/held_lists.rs`, `runtimes/go/clarity/held_lists_test.go`, `runtimes/csharp/Clarity.Runtime.Tests/HeldListTests.cs`, `runtimes/cpp/tests/held_lists_test.cpp`; generators `crates/clarity-gen/tests/held_lists.rs` (every stack; whole builds per stack, ignored), `src/shape.rs` tests; the sweep `crates/clarity-gen/tests/checks_generate.rs`

## Open
- Step 1, the core moved out of Clarity.Formats: BUILT 2026-11-20 (needs a release of clarity-data before clarity-formats).
- Step 2, held lists on every stack: BUILT 2026-11-20 on all five stacks (whole builds on Rust, Go, C, C++; C# waits on Clarity.UI's library); `conformance/held-lists.json` NOT BUILT (each runtime has its own test of the same case); one read per record found, see Open questions.
- Step 3, part values and report data through the library: NOT BUILT.
- Step 4, values the same on every stack: NOT BUILT; needs decision.
- Open questions: needs decision.

## Design

The user, 2026-11-20, after held lists turned out to need the same work written five times: "dont we need a generic data library... not database, runtime data?", and on the design: "yes write the design and implement".

What it is
- The data library is the shared runtime core ([modularity](modularity.md), Step 2's rule) for runtime data: the decisions about records and their values that every stack makes while a program runs, made once in a Rust crate. Each stack's runtime keeps its own types (a Go struct, a C# class) and reads and writes its own database, files and screen; the library decides the shape.
- It is not a database: where records come from (a database, a web API, a file, code) stays with the stack and its sources.

What was found (2026-11-20)
- Clarity.Formats already holds the start of it: one value tree (`tree.rs`), the kinds of what a program reads and writes, as the generator describes them once for every stack (`kind.rs`, written by `crates/clarity-gen/src/shape.rs`), exact numbers (`numbers.rs`), fitting a value to its kind (`fit.rs`) and normalizing (`normal.rs`). `json of` goes through it on all five stacks.
- These five modules use nothing else of Clarity.Formats; the formats (JSON, CSV, XML, YAML syntax, mappings, paths) are built on them.
- What does not go through it: part values (`invoicePdf.order = current`) and report data, each written by the stack's own JSON code (Go `JSONOf`, C# `PartValues.Write`, C `<entity>_json`, C++ `nlohmann::json`, Rust serde); and a stored record's list (`order.lines`), which no stack loads (`Kind::Other`, "a list of stored records").

Step 1: the core moved out of Clarity.Formats
- A crate `runtimes/rust-data` holds `tree`, `kind`, `numbers`, `fit` and `normal`, moved as they are, with their tests.
- Clarity.Formats uses it and gives the same modules on (`pub use`), so no stack, binding or C interface changes.
- Settles: one value model, named for what it is, under the formats.

Step 2: held lists on every stack
- A stored record's list of records (`order.lines`) loads with the record by the one `ref Order` of Line (`Project::held_list_key`), one level deep (the lines do not load their own lists), in the order they were stored; code reads it; setting it is refused (each listed record is saved by itself). Built 2026-11-20: the checker (`check_held_lists`, `crates/clarity-spec/tests/held_lists.rs`) and the Go runtime (`HoldsLists`, `FindAlone`, `runtimes/go/clarity/held_lists_test.go`).
- A kind carries a held list as records (`Kind::Records`), not `Other`, so `json of`, part values and report data hold the lines.
- Each stack's generator writes a record's list loading beside its other record code (the loading reads the stack's own database); `conformance/held-lists.json` pins what loads, in which order, on every stack.

Step 3: part values and report data through the library
- A record given to a part (`invoicePdf.order = current`), to `pdf of` and to a report is written as JSON by the library from its kind, as `json of` is: one shape on every stack (decimals as exact text, dates as written, enums by name, held lists as lists).

Step 4: values the same on every stack (needs decision)
- The open question of [modularity](modularity.md) ("No step settles value semantics across stacks": ordered comparisons, decimal division, overflow, empty values in order) taken into the library, with a conformance file.

Later
- Records made in code for a report (R4, [reports-pdf](reports-pdf.md)), and lists in memory that windows and reports share, as datasets did in the builders of old.

Open questions
- The library's name: "Clarity.Data" (crate `clarity-data`, `runtimes/rust-data`), after the user's "data library", in the line of Clarity.Formats and Clarity.Pdf. Made up; used in step 1 until the user names it.
- Step 4: whether value semantics join this library or get a step of their own.
- The user, 2026-11-20, naming C#'s ExpandoObject (an object whose fields are added while the program runs, read by name): whether the library gives records shaped at runtime, for data put together in code (R4 of [reports-pdf](reports-pdf.md), `new Report`, `r.add(...)`). Not designed.
- The user, 2026-11-20: "is the data crate giving solutions for (concurrent)collections and such?" Not yet: each stack's code holds lists in its own collections, on one thread per window. Whether the library holds collections (lists in memory that windows, parts and reports share, with filtering, sorting and lookups) and safe sharing between threads (once background jobs, [background-jobs](background-jobs.md), are built).
- Loading every order's lines whenever orders are found reads the database once per order: whether a list loads for a whole find at once (one read per list) before step 2 is called done.

## Conflicts
- none

## Decisions
- 2026-11-20: held lists load with the record (option A: "do A, load lists on all stacks"), over a report taking a list as its own data.
- 2026-11-20: a data library for runtime data, not a database ("not database, runtime data"); design and build ("yes write the design and implement").

## Was
- none
