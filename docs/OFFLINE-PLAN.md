# Building without the internet

A plan, agreed 2026-11-05; nothing is built yet.

## Goal

Clarity builds an app on a machine with no internet: on a train, in a classroom with no network, behind a company's firewall. Today Clarity itself works offline (the IDE, checking, the designer, the preview, generating the code), but the first build of each stack needs the network. This plan makes building offline a promise Clarity keeps and tests.

## Where the network is needed today

| Stack | What the first build fetches | Where it is kept after |
|---|---|---|
| Rust | Clarity's crates and their dependencies from crates.io; skia-safe also downloads its prebuilt Skia while it builds | Cargo's cache (`~/.cargo`), the build folder |
| Go | Clarity's Go runtime and its modules through the Go proxy | Go's module cache |
| C# | Clarity's NuGet packages and theirs | NuGet's cache (`~/.nuget`) |
| C, C++ | the Clarity.UI archive from GitHub, unless the installed Clarity carries it or `CLARITY_UI` names a folder | Clarity's cache folder, the build folder |
| Every stack | the Clarity.UI browser files for a web target, packages (PARTS-LIBRARY-PLAN.md), Cisco's openh264 (VIDEO-PLAN.md) | Clarity's cache folder |

After the first build of a stack, the next builds should come from these caches; nothing holds that today, and a new Clarity version needs new runtimes again.

## Decisions

- **Building never needs the network once a project has what it needs.** Checks of checksums and signatures happen when something is fetched, never when building.
- **Two ways to have what it needs**: a warmed cache on the machine (quick, for one developer), or the project carrying everything itself (vendored, for a project that must build anywhere).
- **The stacks' own tools do the work.** Cargo, Go, NuGet and CMake each have an offline way of their own; Clarity uses those, adding no tool of its own.
- **Tested with the network off**, so offline cannot break without a test saying so.
- **Every system alike**: Linux, Windows and macOS keep their caches in their own places, and Clarity finds them there.

## Prepare for offline

A command in the IDE (Project, Prepare for offline) and on the command line (`clarity prepare`), run while there is a network. For the project's stack and the Clarity version in use it fetches everything a build will need:

- Rust: `cargo fetch` for the generated program, and Skia's prebuilt binaries kept where skia-safe finds them again.
- Go: `go mod download` for the generated program.
- C#: `dotnet restore` for the generated program.
- C and C++: the Clarity.UI archive for the system, in Clarity's cache folder.
- The project's packages, the Clarity.UI browser files for a web target, and a video project's codec.

It ends by saying it is ready, or which piece it could not fetch. The IDE shows, beside Build, whether the project can build offline now.

## Vendored projects

For a project that must build on any machine with no network at all, `clarity vendor` (and Project, Vendor in the IDE) copies everything into the project, which keeps it with its files:

- Rust: `cargo vendor`, with the generated program's Cargo configuration pointing at it.
- Go: `go mod vendor`.
- C#: a local NuGet folder, named in the generated program's NuGet configuration.
- C and C++: the Clarity.UI archive, which the generated CMake finds before it would download.
- Packages, browser files and codecs, in the project's folders for them.

A vendored project builds with the stacks' offline switches (`--offline`, `-mod=vendor`, a local source only), so a build never tries the network behind the person's back.

## The installer

The installed Clarity can carry, for its own system, the Clarity.UI library and browser files (as it already carries the library for the IDE) and the runtimes of each stack in the forms the stacks read offline, so a fresh machine builds its first app with no network. Whether the installer carries all stacks or offers them as downloads is settled by its size.

## Testing

- A CI job per stack generates an example, prepares it, then builds it with networking turned off (a network namespace on Linux), and runs it headless.
- A vendored example built with no caches at all, in a clean container with no network.
- A build that lacks a piece offline fails with a message naming the piece and Prepare for offline, never with a stack tool's network error alone.

## Order

1. **The offline test** for each stack, to see what breaks today.
2. **Prepare for offline**, and the IDE's sign of whether a project can build offline.
3. **Vendoring.**
4. **The installer** carrying the runtimes.

## To settle when building

- skia-safe's prebuilt Skia offline: where it looks for a downloaded copy (its own setting for the binaries' location) and how Clarity keeps it.
- How a new Clarity version shows that a project needs Prepare again.
- How big a vendored project gets per stack, and what the project's `.gitignore` says about it.
