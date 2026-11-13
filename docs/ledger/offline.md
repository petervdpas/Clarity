# Building offline

Status: not built. Building an app with no internet: a warmed cache (Prepare for offline) or a vendored project, tested with the network off.

Progress: 5%

## Code
- Spec: none
- Generators: none for offline as such. What is carried today: C and C++ copy their runtime into the output (cJSON, wasm3 vendored); the Clarity.UI library and browser files can come from the installed copy or `CLARITY_UI` (`crates/clarity-gen/src/clarity_ui.rs`; see [build-tools](build-tools.md)); openh264 is carried beside video programs (see [video-camera](video-camera.md))
- Runtimes: R from crates.io, C# from nuget.org, Go modules through the proxy: all download on first build
- Clarity.UI: none
- IDE: none (no offline sign beside Build)
- Docs: none in the manual

Where the network is needed today (OFFLINE-PLAN.md):

| Stack | First build fetches |
|---|---|
| Rust | Clarity's crates and their dependencies from crates.io |
| Go | the runtime module and its modules through the Go proxy |
| C# | Clarity's NuGets and theirs |
| C, C++ | the Clarity.UI archive, unless the installed Clarity carries it or `CLARITY_UI` names a folder |
| every stack | packages (see [parts-library](parts-library.md)) |

## Tests
- none

## Open
- `clarity prepare` and Project, Prepare for offline (`cargo fetch`, `go mod download`, `dotnet restore`, the Clarity.UI archive, packages, browser files, codecs): NOT BUILT (OFFLINE-PLAN.md:31-45, step 2).
- The IDE's sign beside Build whether a project can build offline: NOT BUILT (step 2).
- `clarity vendor` and Project, Vendor (`cargo vendor`, `go mod vendor`, a local NuGet folder, the archive), built with the stacks' offline switches: NOT BUILT (OFFLINE-PLAN.md:47-57, step 3).
- The installer carrying the runtimes in offline form: NOT BUILT (step 4). It carries only the Clarity.UI library and browser files today.
- A CI job per stack building with the network off; a vendored example in a clean container: NOT BUILT (step 1).
- A build lacking a piece offline fails naming the piece and Prepare for offline: NOT BUILT.
- To settle when building: how a new Clarity version shows a project needs Prepare again; vendored size per stack and its `.gitignore`.

## Conflicts
- OFFLINE-PLAN.md:13 says skia-safe downloads Skia while building; Rust links the ready-made Clarity.UI library (`runtimes/rust/Cargo.toml:45`). OFFLINE-PLAN.md:14 leaves Go out of the stacks that take the Clarity.UI archive.

## Decisions
- 2026-11-05: building an app must work without the internet. User: "because it has to"
- 2026-11-05: building never needs the network once a project has what it needs; checksums and signatures are checked when fetched, never when building.
- 2026-11-05: two ways: a warmed cache, or a vendored project; the stacks' own offline means, no tool of Clarity's own.
- 2026-11-05: tested with the network off; every OS alike.
- 2026-11-07: offline means no download at run time either: video programs carry openh264 (see [video-camera](video-camera.md)).

## Was
- OFFLINE-PLAN.md steps 1-4, "Decisions", "Prepare for offline", "Vendored projects", "The installer", "Testing", "To settle when building"
- memory: offline-builds.md
