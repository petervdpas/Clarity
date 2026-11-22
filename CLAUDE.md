# Clarity: rules for Claude Code

Clarity is one language and a designer-first IDE whose projects are transpiled to real programs on five stacks (Rust, Go, C, C++, C#), drawn by Clarity.UI on every stack, on Linux, Windows and macOS. These rules hold for every piece of work in this repository.

## Principles

- **Modular.** Small parts with one job; shared code in one place; parallel things in parallel places with parallel names.
- **Test first.** Write the test, see it fail for the right reason, then write the code. The workspace compiles between edits: a new enum variant covers every match in the same step.
- **One source of truth.** When two places decide the same thing (two stacks, the generator and the spec), move it to one place and let both use it. Shared behaviour is pinned by `conformance/*.json`, which every stack runs.
- **The backend drives the frontend.** The spec, the generator and the runtimes decide; screens and the IDE show what they are given.
- **Generic, as a builder of apps.** A feature comes with everything a developer building any app expects: a part option where an app may not want it, the IDE designer's property, the checker's rules, the docs and manual, translations of every word people read, every stack, every example.
- **Fail safe.** A failure gets a guard for its whole kind, not one test for one case: a part that fails shows an error in its own place, every area of the IDE and every runtime keeps working, tests sweep everything of a kind (every window of every example, every part kind).
- **Determinism.** No heuristics and no guessing of intent: a rename is asked for, never inferred; layouts and outputs are the same every time.

## Every stack, every system, every language

- A feature works on all five stacks, in each stack's own idiom, and on Linux, Windows and macOS alike.
- Every text a person reads comes from the translations, with named slots (`{customer.name}`); keys are written out whole, never built by interpolation.
- No dialog of the system or the browser: questions and messages go through Clarity's own dialogs module, in the IDE and in generated apps.
- Lower-case names are camelCase.
- Clarity's own agent never executes anything; anything that runs asks first.

## Asking and recording

- Never work from half-remembered notes. Before acting on a tool, workflow, format or API, read its real source (its README, code, docs or workflow file); when a note and the source disagree, the source wins.
- Before building something new, restate the goal in a line or two and ask; a remark is not a request to build.
- A design is written once, in its note's Design section (`docs/ledger/<piece>.md`); it quotes the user's words, and anything added goes under open questions. There are no plan documents beside the notes.
- **Use the user's words for every name.** An entity, field, enum value, window, part, button or text named in a design comes from what the user said. A name made up (a field nobody asked for, another word for one they gave) is listed under open questions, with what it is for, before it is used, in a diagram above all, and used only once the user agrees. Never use a word Clarity already means something else by (`page` is a window or view). A structure the user names (Project -> Dossier -> Review -> Items) is kept exactly. Every agent of Clarity is told the same (`clarity_tools::instructions`).

## Tests and builds

- After a change, run the tests of the module changed (`--test <file> <filter>`), not whole crate suites. When a check fails, run only what failed.
- Whole suites, packing NuGets, release builds of the libraries and installers are what GitHub Actions does: don't redo them locally.
- A slice is done only when every example builds on its own stack (`crates/clarity-gen/tests/gen/examples_build.rs`, one at a time).
- An example joins `EXAMPLES` in `examples_build.rs` and the checks' example list only once it checks clean and builds on its own stack; a half-built example stays out of both.
- A test known to fail is named in the reply to the user when the work stops, with why. A note in the ledger or in memory alone is not enough.
- Windows and views (`.cxaml`) are written through Clarity's tools; one written by hand passes `cargo test -p clarity-spec --test spec cxaml::` before it counts as done.
- A test that reads an example sees it as a fresh checkout does: what git ignores (an example's `data/` and `out/`) is not there in the checks, so a test passing only because a local database exists does not count.
- Say a thing is ready only when it is built and checked.
- The machine has 7.5 GB of memory: `CARGO_BUILD_JOBS=2`, one heavy build at a time, no builds in `/tmp`, check free memory first.
- Never run `cargo fmt`; format by hand in the surrounding style.

## Generated-file guards

Clarity checks generated files against their source in tests that fail when a file is behind; a release's checks fail on any of them. After changing what one is written from (a part kind, keyword, field, format, model type, diagram or theme), write the file again and run its test; before writing a release plan, write and test every guard.

| Writes | Command |
| --- | --- |
| `editors/vscode/syntaxes/clarity.tmLanguage.json` | `CLARITY_WRITE_GRAMMAR=1 cargo test -p clarity-spec --test spec highlight::` |
| `runtimes/rust-ui/catalog/*.json` | `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-gen --test gen web_windows::` |
| `runtimes/web/src/generated/*.ts` | `cargo test -p clarity-spec --lib` (ts-rs writes them) |
| the examples' diagrams | `CLARITY_WRITE_DIAGRAMS=1 cargo test -p clarity-spec --test spec diagram_views::` |
| `conformance/migrations.json` | `CLARITY_WRITE_CONFORMANCE=1 cargo test -p clarity-gen --test gen servers::` |
| `runtimes/rust-ui/themes/` | `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-spec --test spec clarity_ui_theme::` |
| the picture cases | `CLARITY_WRITE_PICTURES=1 cargo test -p clarity-image --test pictures` |
| the parts pages' catalog tables (`manual/parts.*.md`) | `CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test spec manual_catalog::` |
| `runtimes/carried/` (what the installer carries for builds with no internet) | `CLARITY_WRITE_CARRIED=1 cargo test -p clarity-gen --test gen carried::` |
| the IDE's TypeScript types (`runtimes/web/src/generated/*.ts`) | `cargo test -p clarity-preview --lib` (ts-rs writes them) |
| the project's TypeScript types | `cargo test -p clarity-project --lib` (ts-rs writes them) |
| the agent's TypeScript types | `cargo test -p clarity-agent --lib` (ts-rs writes them) |
| the engine the renderers are tested with (`conformance/engines/sky*.wasm`) | `CLARITY_WRITE_SKY=1 cargo test -p clarity-wasm --test sky` |
| the Go runtime's modules (`crates/clarity-gen-go/src/modules.txt`; needs the internet) | `CLARITY_WRITE_GO_MODULES=1 cargo test -p clarity-gen-go --test go -- --ignored --exact build::the_modules_of_each_package_group_are_recorded` |

Every guard is in `release::GUARDS` and in this table: `crates/clarity-devtools/tests/devtools/devtools.rs` fails on one missing from either.

## Releasing

- Before writing or changing a release plan or its progress file, read the Release Helper's own README (`~/Projects/ReleaseHelper/README.md`): its plan format, step kinds and options. Never weaken a step's check to let the next step go; `"next_after": "workflow"` keeps the check and frees the next step.
- The user commits, pushes and tags; Claude never does. Releases go through the Release Helper: a plan in `releases/<YYYY-MM-DD-HHMM>.json` (not committed), with the commit message, the checks and the tags in their order (a crate before the crate that needs it, a library before what fetches it, Go sums after the Go tag). Write and change a plan only with `write_release_plan` of `clarity-cdev`, never by hand (a hook refuses it): it refuses a new plan while a release check failed, did not run or ran on another tree; a plan's commit message (`fix`) it changes at any time.
- Before writing a plan: run the generated-file guards, make every version bump, and name agreed features that are not built yet.
- Before writing a plan, also: run the test targets of every crate changed since the last tag, one at a time (and svelte-check and the app's tests when `app/` or a model type changed); run `clarity spec check` on every example in the checks' list and the build test of each example added or changed since the last tag; look in `docs/ledger/` for entries marked partial and tests noted as failing, and name each to the user. A failing one blocks the plan until it is fixed or the user decides.
- While the user releases, leave the tree alone: the helper does not tag with uncommitted changes. When a check fails, find which workflow and step failed before changing anything.
- A new NuGet package needs nothing on nuget.org: the trusted publishing policy covers the repository and workflow.
- The C# runtime step gets `"next_after": "workflow"`: nuget.org indexes slowly and nothing after it needs the packages listed (Go, the IDE). The Go sums step is a manual step with its own `commit_message`.

## The IDE while it runs

**Build Clarity's own pieces yourself, with the command DevTools names.** Builds and the preview use the release; a Clarity run from the repository has Clarity settings > DevTools, the Disk and Release planning tabs, then a tab per piece (clarityUi, clarityUiBrowser, rustCrates, nugetPackages, goModule, clarityAudio, clarityVideo, clarityImage, clarityFormats, clarityAuth, clarityAppMcp) shown by itself while its own build is behind its source, else added by the cDdev choosing the release or the cDdev's own build, each with a Build button for the person (`crates/clarity-gen/src/tools/devtools/`). After changing a piece, run its build yourself (`CARGO_BUILD_JOBS=2 cargo build --release -p clarity-ui-ffi` for clarityUi, without `--release` for its debug build), one at a time, telling the person first; not through `build_piece`, which waits for a person to press Build. Never copy libraries. DevTools > Release planning is the person's review (the checks with one button, what a release would start, what changed); before writing a release plan, read how the releases and their checks stand with `release_state` of `clarity-cdev`, and write the plan from that, never from memory.

The user runs the IDE with `./dev.sh`. Editing `app/src/*` reloads its page; editing `app/src-tauri/*` or a crate it uses restarts it and clears the agent chat. While it runs, make no Rust edits and run no cargo builds without asking; gather IDE changes and apply them in one go.
