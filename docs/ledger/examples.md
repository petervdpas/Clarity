# Examples

Status: partial. Example projects in `examples/`, each a project file with a folder of its modules and a README, built on the stack it names; every example is also a test. maintenance is being built.

## Code
- Examples: `examples/<name>.clarity` and `examples/<name>/` (modules, `design/*.cxaml`, translations, resources, README.md)
- Rust desktop: dashboard, maintenance, modelling, network, shop
- Go desktop: calculator, dial, pills, records, videoplayer, waves; Go desktop cli: stock; Go web: invaders
- C# desktop: mp3player, noughts, todo; C# web: walker; C# cli: ledger
- C desktop: kitchen; C++ desktop: novel, orbs
- Docs: `README.md` (examples list, one line each)

## Tests
- `crates/clarity-gen/tests/examples_build.rs`: `every_example_is_here` (the folder and the list agree, 21) and one ignored build test per example on its own stack, run one at a time
- `.github/workflows/checks.yml:286`: every example built on its stack in CI
- Examples are swept by spec tests: `cxaml.rs`, `design_edit.rs`, `keying.rs`, `placed_places.rs` (every example's windows); `diagram_views.rs` (a fixed list of ten)

## Open
- maintenance: PARTIAL. Written (2026-11-12): `maintenance.clarity` (entity Project, enum Reviewer, record Entry, settings with folder `ledgerFolder` and `lastProject`, the parsing functions, every handler of Main and Projects, action `entryOf`), the project file's `filestore ledger "setting:ledgerFolder"`, data `projects` on Main. Next: the rest of Main's data (`entries: list<Entry>`, `projectName`, `piece`, `related`, `codePaths`, `oldReferences`, `reviewedOn`, `notes`, `ownReferences`: text; `reviewed`: bool; `reviewer`: Reviewer), MainView's data (`entryTitle`, `entryStatus`, `entryBody`: text) and parts (heading, status text, `markdown` bound to `entryBody`); on Main a toolbar (Projects opens Projects, Reload, Review), a left sidebar (list `projectList` of projects, treetable `entryTree` by key and parent: label, status, reviewed) and the slideout `reviewPanel` (checkbox, Reviewer dropdown, reviewed on, notes as a markdown `memo`, own references, read-only related, code paths and old references, Save); a Projects window (table with add, remove and edit of name and ledger folder); the events wired; translations; a migration for Project; README; then build. Needs the rebuilt `clarity mcp` (the fixed `add_data`, see [mcp-tools](mcp-tools.md)); its build test fails until then.
- maintenance, what it is to do: pick a project (Clarity is one of them); its ledger as an orderly list in a left sidebar (a tree grouped as the ledger's index, title, status, reviewed); the entry on the right as Markdown (`markdown` part bound to its text); a slideout with the review: reviewed, by (Peter or Claude), when, notes (a `memo` in markdown mode), the person's own references, and read-only related entries, code paths and old references (the entry's Was). Projects are stored (name, ledger folder); a folder setting that picking a project sets points one file store at that ledger; reviews are files `<ledger>/reviews/<piece>.md`, which Claude writes too.
- No example uses rust web, c web or cpp web; `everything_build.rs` and the `the_web_target_serves_in_*` tests build those.
- No example calls the picture functions (see [picture-functions](picture-functions.md)); the modelling example does not use the graph (see [graph](graph.md)).

## Conflicts
- DESIGN.md:1363 says "All examples is eight"; there are 21. The removed dice example is still cited in DESIGN.md and LANGUAGE.md (see [docs](docs.md)).

## Decisions
- 2026-11-03: build an example only for the stack its project file names; "all examples" means every one. (memory: examples-own-stack.md)
- 2026-11-06: a slice is done only when every example builds on its own stack. (CLAUDE.md)
- 2026-11-12: maintenance, a Clarity app to review ledgers. User: "make a clarity app so we can review everything in there! call it \"maintenance\"... and clarity is 'just' one project in the app"; the list "as an orderly list (left sidebar)", on the right "a review page with slideout for properties like reviewed and references"; the page a "markdown text widget".
- 2026-11-12: reviews are files beside the ledger, so Claude can review too; references are related entries, code paths, old references and the person's own links; rust desktop.
- 2026-11-12: maintenance is an example, not a project elsewhere. User: "you better put it in examples so it's another test as well!"

## Was
- memory: examples-own-stack.md, examples-before-done.md, second-example.md, full-width-layouts.md
- IMPLEMENTED.md section 1 (examples table)
