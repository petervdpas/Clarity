# Parts library (packages)

Status: not built. Packages of views, parts drawn in Clarity, themes, modules, devices and native parts, published to an open index and added from the palette, as C++ Builder's component packages.

Progress: 0%

## Code
- Spec: none (`package` files and `use package` are not parsed)
- Generators: none
- Runtimes: none
- Clarity.UI: none
- IDE: none
- CLI: none (`clarity package check/publish/vendor` do not exist)
- Docs: none
- Today's nearest: per-project custom parts and libraries ([custom-parts](custom-parts.md))

## Tests
- none (planned: conformance for resolving versions, lock files, checksums, name clashes; a test package of each kind on every stack; builds alike with and without network once cached)

## Open
- Step 1, packages of Clarity only (views, engine-drawn parts, themes, modules) from a path or a Git address, with a lock file of versions and checksums: NOT BUILT.
- Step 2, the index (a public Git repository, one file per package, archives on GitHub releases), `clarity package check` and `publish`, the Get parts browser with live previews, packages read-only in the tree, a New package project: NOT BUILT.
- Step 3, native parts per stack (`stacks rust go`; a crate, Go module, NuGet or CMake target; greyed out on stacks they lack; adding one asks first): NOT BUILT.
- Step 4, optional Sigstore signatures, checked offline from the bundle: NOT BUILT.
- `clarity package vendor` into the project's `packages` folder; checks on fetch, never on build: NOT BUILT (see [offline](offline.md)).
- To settle when building (needs decision): the index's place and name and who reviews additions; how another index is named (project file or person's settings); the Sigstore crate (asked for first); how a package's translations join a project's; whether packages depend on packages in the first version; a first set (gauges drawn by an engine, an address form, the modelling app's Studio and Midnight themes).

## Conflicts
- PARTS-LIBRARY-PLAN.md:12 names dice's 3D dice on Bevy as the native part example; the dice example was removed.

## Decisions
- 2026-11-05: a package holds anything Clarity declares, not only parts.
- 2026-11-05: Clarity first, native only where it must: Clarity and engine-drawn parts (WebAssembly) work on every stack.
- 2026-11-05: checked before publishing; a package with errors cannot be published.
- 2026-11-05: pinned and reproducible: `use package name 1.2` plus a lock file with checksums; a new major version is never taken by itself.
- 2026-11-05: open: no Clarity server or keys; a forkable Git index; a company or school can run its own.
- 2026-11-05: trust in layers: Clarity and WebAssembly within Clarity's limits; native asks first; signatures optional and never a gate; a mismatched signature is refused.
- 2026-11-05: a published version cannot be removed; a broken one is marked.
- No verbatim user words were recorded for these; memory: "the user's C++ Builder goal (component packages / GetIt)".

## Was
- PARTS-LIBRARY-PLAN.md (all: decisions, language, how it works, publishing, trust, offline, IDE, order steps 1-4, to settle)
- memory: parts-library-plan.md
