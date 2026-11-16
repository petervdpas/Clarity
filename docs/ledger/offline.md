# Building offline

Status: not built. Building an app with no internet: a warmed cache (Prepare for offline) or a vendored project, tested with the network off.

Progress: 5%

## Code
- Spec: none
- Generators: none for offline as such. What is carried today: C and C++ copy their runtime into the output (cJSON, wasm3 vendored); in a dev IDE, a piece swapped to the cDdev's own build in Clarity settings > DevTools replaces downloads of it (`environment::use_libraries`; see [devtools](devtools.md)); the Clarity.UI library and browser files come from `CLARITY_UI`, the installed copy, or a fetch into the cache (`crates/clarity-gen/src/clarity_ui.rs`; see [build-tools](build-tools.md)); openh264 is carried beside video programs (see [video-camera](video-camera.md))
- Checked when fetched, as the design says: the Clarity.UI archive against the release's SHA256SUMS (`clarity_ui.rs:225`), openh264 against a pinned hash (`clarity_ui.rs:161-174`, fetched by `app/src-tauri/src/build.rs:481`); the cache folder per system (`clarity_ui.rs:106`)
- Runtimes: Rust from crates.io, C# from nuget.org, Go modules through the proxy: all download on first build
- Clarity.UI: none
- IDE: none (no offline sign beside Build)
- Docs: none for offline in the manual; `project.run-build.md:45` says the installer carries the Clarity.UI library and a Clarity without it fetches it once, checked

Where the network is needed today:

| Stack | First build fetches |
|---|---|
| Rust | Clarity's crates and their dependencies from crates.io |
| Go | the runtime module and its modules through the Go proxy |
| C# | Clarity's NuGets and theirs |
| Rust, Go, C, C++ (desktop) | the Clarity.UI archive, unless the installed Clarity carries it or `CLARITY_UI` names a folder |
| every stack | browser files for a web target, a video program's openh264, packages (see [parts-library](parts-library.md)) |

Rust links the ready-made Clarity.UI library (`runtimes/rust/Cargo.toml:41-45`, feature `native-ui`); nothing downloads or builds Skia.

## Tests
- none

## Open
- The offline test per stack (Design, "Testing"; step 1): NOT BUILT.
- Prepare for offline and the sign beside Build (Design, "Prepare for offline"; step 2): NOT BUILT.
- Vendored projects (Design, "Vendored projects"; step 3): NOT BUILT.
- The installer carrying the runtimes (Design, "The installer"; step 4): PARTIAL. It carries the Clarity.UI library and browser files only.
- A build lacking a piece names it (Design, "Testing"): NOT BUILT.
- To settle (Design, "To settle"): needs decision.

## Design
Clarity itself works offline (the IDE, checking, the designer, the preview, generating). This makes building offline a promise Clarity keeps and tests. After the first build of a stack, the next builds should come from the stacks' caches; a new Clarity version needs new runtimes again.

Prepare for offline
- Project, Prepare for offline in the IDE and `clarity prepare` on the command line, run while there is a network. For the project's stack and the Clarity version in use it fetches what a build needs:
  - Rust: `cargo fetch` for the generated program.
  - Go: `go mod download`.
  - C#: `dotnet restore`.
  - C and C++: the Clarity.UI archive for the system, in Clarity's cache folder.
  - Every stack: the project's packages, the Clarity.UI browser files for a web target, a video project's codec.
- It ends by saying it is ready, or which piece it could not fetch. The IDE shows beside Build whether the project can build offline now.

Vendored projects
- `clarity vendor` and Project, Vendor copy everything into the project, which keeps it with its files:
  - Rust: `cargo vendor`, the generated Cargo configuration pointing at it.
  - Go: `go mod vendor`.
  - C#: a local NuGet folder named in the generated NuGet configuration.
  - C and C++: the Clarity.UI archive, which the generated CMake finds before it would download.
  - Packages, browser files and codecs in the project's folders for them (packages: `clarity package vendor`, see [parts-library](parts-library.md)).
- A vendored project builds with the stacks' offline switches (`--offline`, `-mod=vendor`, a local source only), so a build never tries the network behind the person's back.

The installer
- The installed Clarity carries, for its own system, the runtimes of each stack in the forms the stacks read offline, beside the Clarity.UI library and browser files it carries now, so a fresh machine builds its first app with no network. Whether it carries all stacks or offers them as downloads is settled by its size.

Testing
- A CI job per stack generates an example, prepares it, builds it with networking off (a network namespace on Linux), and runs it headless.
- A vendored example built with no caches at all, in a clean container with no network.
- A build that lacks a piece offline fails with a message naming the piece and Prepare for offline, never with a stack tool's network error alone.

Order
1. The offline test per stack, to see what breaks today.
2. Prepare for offline and the IDE's sign.
3. Vendoring.
4. The installer carrying the runtimes.

To settle
- How a new Clarity version shows that a project needs Prepare again.
- How big a vendored project gets per stack, and what its `.gitignore` says about it.

## Conflicts
- none

## Decisions
- 2026-11-05: building an app must work without the internet. User: "because it has to"
- 2026-11-05: building never needs the network once a project has what it needs; checksums and signatures are checked when fetched, never when building.
- 2026-11-05: two ways: a warmed cache, or a vendored project; the stacks' own offline means, no tool of Clarity's own.
- 2026-11-05: tested with the network off; every OS alike, each keeping its caches in its own places.
- 2026-11-07: offline means no download at run time either: video programs carry openh264 (see [video-camera](video-camera.md)).

## Was
- OFFLINE-PLAN.md (removed 2026-11-16): goal, "Where the network is needed today", "Decisions", "Prepare for offline", "Vendored projects", "The installer", "Testing", "Order" steps 1-4, "To settle when building" (the skia-safe question dropped: Rust no longer builds Skia)
- memory: offline-builds.md
