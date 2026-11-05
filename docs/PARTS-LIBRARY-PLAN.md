# The shared parts library

A plan, agreed 2026-11-05; nothing is built yet.

## Goal

A developer packages parts and other pieces of Clarity once, publishes them, and any Clarity developer adds them to a project from the palette, as C++ Builder's component packages and its package manager did. A package works on every stack its contents allow, and most need no code per language at all.

## Decisions

- **A package holds anything Clarity declares**, not only parts: views, parts drawn in Clarity, themes, modules of functions and actions, devices, and native parts.
- **Clarity first, native only where it must.** Views, themes and modules are Clarity, and a part drawn on a canvas by an engine written in Clarity compiles to WebAssembly, which runs on every stack. Such a package works in Rust, Go, C#, C and C++ alike. A native part (a library per language, as dice's 3D dice on Bevy) is the exception, and its package says which stacks it brings.
- **Checked before it is published.** A package is checked as a project is, so a package with errors cannot be published, and its parts arrive with their designer properties, docs and translations.
- **Pinned and reproducible.** A project names the packages it uses and a lock file pins each exact version and its checksum, so every build fetches the same.
- **Open, as Clarity is.** No server of Clarity's own, no keys Clarity holds, no gatekeeper: the index is a public Git repository anyone can fork, and the archives live on GitHub releases, as many package systems began. A company or a school runs an index of its own and points Clarity at it.
- **Trust in layers.** Clarity and WebAssembly run in Clarity's own limits (an engine's time, memory and the services it may use); native code runs on the developer's machine, so adding it asks first; a signature, when a package has one, says who published it and where it was built.

## What a package can hold

| Kind | Example | Stacks |
|---|---|---|
| Views | an address form, a login panel, a settings page | all |
| Parts drawn in Clarity | a gauge, a calendar strip, a game board: a canvas and an engine written in Clarity | all |
| Themes | a light and a dark look, with their code colors | all |
| Modules | VAT rules, IBAN and postcode checks, date helpers | all |
| Devices (the hardware plan) | a Zigbee2MQTT light, a Firmata board | all |
| Native parts | a 3D view, a map, a PDF viewer | those it brings code for |

## The language

```clarity
// The package's own file: what it is and what it brings.
package Gauges
    version 1.2.0
    title "Gauges"
    author "Peter van de Pas"
    license MIT
    stacks all
```

```clarity
// A project that uses it.
project Greenhouse
    stack go desktop
    use package gauges 1.2
```

| What | Meaning |
|---|---|
| `package Name` | A package: its modules, engines, resources and translations in its folder, as a module folder has them. |
| `version`, `title`, `author`, `license` | What the index and the IDE show; the version is semver. |
| `stacks all`, `stacks rust go` | Where it works: `all` for a package of Clarity only; a native part's package names the stacks it brings code for. |
| `use package name 1.2` | In a project file: the package and the versions it takes (1.2 is 1.2.0 and later 1.x). |
| `use package name path "../gauges"`, `git "https://..."` | A package from a folder or a Git address, for making one or a team's own. |

A package's parts, views and functions are used by name, as a project's own are; a name both have is an error that says to write the package's name before it (`Gauges.Dial`).

## How it works

- **Fetching**: Clarity reads the index (the public one, or one the project or the person names instead), fetches the archive of the version the lock file pins (or the newest that fits, writing the lock file), checks its checksum and keeps it in the person's cache, one copy per version for every project.
- **Building**: the generators take a package's Clarity as the project's own, so it builds on the project's stack with nothing more; a native part joins the build as the project's own custom parts do (a crate, a Go module, a NuGet package, a CMake target).
- **The palette**: a package's parts stand under its title, with their icons and docs; a native part a project's stack lacks shows greyed out, saying which stacks it has.
- **Updates**: the IDE says when a newer version fits, with its changes, and updates the lock file on the developer's word; a new major version is never taken by itself.

## Publishing

- `clarity package check` checks a package as Build checks a project, its translations and its docs included; `clarity package publish` makes the archive with its checksum, signs it with Sigstore when the publisher wants (a workflow does it in CI with no secret to keep), puts it on a GitHub release and opens a pull request to the index.
- The index holds one file per package: its versions, checksums, stacks and where each archive is.
- Removing a published version is not possible; a broken one is marked so the IDE warns and offers the next.

## Trust

- A package of Clarity and WebAssembly cannot reach the machine beyond what Clarity's code may: engines keep their limits, and actions declare their effects (`db`, `files`, `net`), which the IDE shows before adding the package.
- A native part runs as code on the developer's machine and in their app, so adding one asks first, saying what it brings.
- **Signatures, open and optional.** A publisher signs with Sigstore (free and open source, a Linux Foundation project), with the identity they already have: their GitHub account, or the GitHub Actions run that built the archive (GitHub's artifact attestations are built on it). The signature goes into Sigstore's public log, which anyone can check; there are no keys to buy or keep, and Clarity holds none and runs nothing for it.
- **Never a gate.** An unsigned package installs as a signed one does; the IDE shows "unsigned", or "signed by github.com/someone, built from this commit". A signature that does not match its archive is refused.
- **Integrity without signing.** The lock file's checksums already make every build fetch exactly the archive it pinned; a signature only adds who made it.

## Offline

Building never needs the network: a project that has its packages builds on a machine with no internet.

- **Checked when fetched, never when built.** A package's checksum and signature are checked once, when it is fetched, while there is a network anyway. A build after that reads the archive from the cache and checks only the lock file's checksum, on the machine itself.
- **Signatures check offline too.** A Sigstore signature comes as a bundle with its proof from the public log, checked against Sigstore's root of trust kept on disk, so even a first check needs no network beyond fetching the archive.
- **Vendored packages.** `clarity package vendor` copies a project's packages into its `packages` folder, which the project then keeps with its files: it builds with no network and no cache at all, as a stack's own vendored libraries do (see OFFLINE-PLAN.md).
- **No network, no surprise.** Without a network the IDE uses what the cache and the project hold, and says which package it cannot fetch, never failing a build that has everything.

## The IDE

- **Get parts**: a browser of the index to search, read a package's page (what it holds, its stacks, its effects, its licence), see its parts drawn live (the designer draws with the same library, and a part drawn by an engine runs in the browser build), add it, update it and remove it.
- The project tree shows the packages a project uses, read only, with their contents.
- A **New package** project to make one, with Check and Publish.

## Testing

- Conformance cases for resolving versions (what 1.2 takes, a conflict between two packages, a lock file kept), checksums and name clashes.
- A test package of each kind built on every stack; a project that uses a package builds alike with and without network once it is cached.

## Order

1. **Packages of Clarity only** (views, parts drawn with engines, themes, modules) from a folder or a Git address, with the lock file.
2. **The index, publishing and Get parts** in the IDE.
3. **Native parts** per stack.
4. **Signatures** with Sigstore, optional.

## To settle when building

- The index's place and name (a public Git repository under the Clarity project), who reviews additions, and how another index is named (in the project file, or in the person's settings).
- The Sigstore library on the Rust side for checking signatures: a new dependency, asked for first.
- How a package's translations join a project's (a project may translate a package's keys itself).
- Whether packages can depend on other packages in the first version, or only later.
- A first set of packages to prove it: gauges drawn by an engine, an address form view, and the modelling app's Studio and Midnight themes.
