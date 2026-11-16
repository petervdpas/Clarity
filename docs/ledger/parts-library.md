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
- Today's nearest: per-project custom parts and libraries ([custom-parts](custom-parts.md)); effects on actions (`EFFECTS`, `crates/clarity-spec/src/code/mod.rs:1189`)

## Tests
- none (planned in Design, "Testing")

## Open
- Step 1, packages of Clarity only from a path or a Git address, with the lock file (Design, "The language", "How it works"): NOT BUILT.
- Step 2, the index, publishing and Get parts (Design, "Publishing", "The IDE"): NOT BUILT.
- Step 3, native parts per stack (Design, "What a package can hold", "How it works"): NOT BUILT.
- Step 4, optional Sigstore signatures (Design, "Trust"): NOT BUILT.
- Packages offline and `clarity package vendor` (Design, "Offline"): NOT BUILT.
- To settle (Design, "To settle"): needs decision.

## Design
A developer packages parts and other pieces once, publishes them, and any Clarity developer adds them to a project from the palette. A package works on every stack its contents allow; most need no code per language.

What a package can hold

| Kind | Example | Stacks |
|---|---|---|
| Views | an address form, a login panel, a settings view | all |
| Parts drawn in Clarity | a gauge, a calendar strip, a game board: a canvas and an engine written in Clarity (WebAssembly) | all |
| Themes | a light and a dark look, with their code colors | all |
| Modules | VAT rules, IBAN and postcode checks, date helpers | all |
| Devices (see [hardware](hardware.md)) | a Zigbee2MQTT light, a Firmata board | all |
| Native parts | a 3D view, a map, a PDF viewer | those it brings code for |

The language

```clarity
// The package's own file: what it is and what it brings.
package Gauges
    version "1.2.0"
    title "Gauges"
    author "Peter van de Pas"
    license MIT
    stacks all
```

```clarity
// A project file that uses it.
project Greenhouse
    stack go desktop
    use package gauges 1.2
```

- `package Name`: a package; its modules, engines, resources and translations in its folder, as a project's folder has them.
- `version`, `title`, `author`, `license`: what the index and the IDE show; the version is semver.
- `stacks all`, `stacks rust go`: where it works; `all` for a package of Clarity only, a native part's package names the stacks it brings code for.
- `use package name 1.2` in a project file: the package and the versions it takes (1.2 is 1.2.0 and later 1.x).
- `use package name path "../gauges"`, `git "https://..."`: a package from a folder or a Git address.
- A package's parts, views and functions are used by name, as a project's own are; a name both have is an error that says to write the package's name before it (`Gauges.Dial`).

How it works
- Fetching: Clarity reads the index (the public one, or one the project or the person names), fetches the archive of the version the lock file pins (or the newest that fits, writing the lock file), checks its checksum and keeps it in the person's cache, one copy per version for every project.
- Building: the generators take a package's Clarity as the project's own; a native part joins the build as a project's custom parts do (a crate, a Go module, a NuGet package, a CMake target).
- The palette: a package's parts stand under its title, with their icons and docs; a native part the project's stack lacks shows greyed out, saying which stacks it has.
- Updates: the IDE says when a newer version fits, with its changes, and updates the lock file on the developer's word; a new major version is never taken by itself.

Publishing
- `clarity package check` checks a package as Build checks a project, translations and docs included.
- `clarity package publish` makes the archive with its checksum, signs it with Sigstore when the publisher wants (a CI workflow, no secret to keep), puts it on a GitHub release and opens a pull request to the index.
- The index holds one file per package: its versions, checksums, stacks and where each archive is.
- A published version cannot be removed; a broken one is marked, so the IDE warns and offers the next.

Trust
- A package of Clarity and WebAssembly reaches the machine only as Clarity's code may: engines keep their limits, and actions declare their effects (`db.read`, `db.write`, `files.read`, `files.write`, `net`), which the IDE shows before adding the package.
- A native part runs as code on the developer's machine and in their app, so adding one asks first, saying what it brings.
- Signatures with Sigstore, with the identity a publisher already has (a GitHub account, or the GitHub Actions run that built the archive); the signature goes into Sigstore's public log. Clarity holds no keys and runs nothing for it.
- Never a gate: an unsigned package installs as a signed one does; the IDE shows "unsigned" or "signed by github.com/someone, built from this commit". A signature that does not match its archive is refused.
- The lock file's checksums already make every build fetch exactly the archive it pinned; a signature only adds who made it.

Offline (see [offline](offline.md))
- A package's checksum and signature are checked once, when fetched. A build after that reads the archive from the cache and checks only the lock file's checksum.
- A Sigstore signature comes as a bundle with its proof from the public log, checked against Sigstore's root of trust kept on disk.
- `clarity package vendor` copies a project's packages into its `packages` folder, kept with the project's files, so it builds with no network and no cache.
- Without a network the IDE uses what the cache and the project hold and says which package it cannot fetch, never failing a build that has everything.

The IDE
- Get parts: a browser of the index to search, read a package's description (what it holds, its stacks, its effects, its licence), see its parts drawn live (the browser build of Clarity.UI), add, update and remove it.
- The project tree shows the packages a project uses, read only, with their contents.
- A New package project, with Check and Publish.

Testing
- Conformance cases for resolving versions (what 1.2 takes, a conflict between two packages, a lock file kept), checksums and name clashes.
- A test package of each kind built on every stack; a project that uses a package builds alike with and without network once it is cached.

Order
1. Packages of Clarity only (views, parts drawn with engines, themes, modules) from a folder or a Git address, with the lock file.
2. The index, publishing and Get parts.
3. Native parts per stack.
4. Signatures with Sigstore, optional.

To settle
- The index's place and name, who reviews additions, and how another index is named (project file or the person's settings).
- The Sigstore crate for checking signatures: a new dependency, asked for first.
- How a package's translations join a project's (a project may translate a package's keys itself).
- Whether packages depend on other packages in the first version.
- A first set to prove it: gauges drawn by an engine, an address form view, the modelling example's Studio and Midnight themes.

## Conflicts
- none

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
- PARTS-LIBRARY-PLAN.md (removed 2026-11-16): goal, decisions, what a package can hold, the language, how it works, publishing, trust, offline, the IDE, testing, order steps 1-4, to settle. Its native example (dice's 3D dice on Bevy) dropped: the dice example was removed. Its effects `db`, `files`, `net` written as the code names them.
- memory: parts-library-plan.md
