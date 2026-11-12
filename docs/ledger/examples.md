# Examples

Status: built. Example projects in `examples/`, each a project file with a folder of its modules and a README, built on the stack it names; every example is also a test.

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
- maintenance: built (2026-11-12). `maintenance.clarity` (entity Project, enum Reviewer, record Entry, settings `ledgerFolder` and `lastProject`, the parsing functions, the handlers of Main and Projects, action `entryOf`), the project file's `filestore ledger "setting:ledgerFolder"`; Main with its own toolbar (Projects, Reload, Review), a left sidebar (list `projectList`, tree table `entryTree` by `rowKey` and `parentKey`: label, status, reviewed), content MainView (title, status, `markdown` part bound to `entryBody`) and the slide-out `reviewPanel` (reviewed, Reviewer, reviewed on, notes as a markdown memo, own references, read-only related, code and tests, old references, Save); Projects with ProjectsView (table with add, remove and editing of name and ledger folder); translations; migration 0001 for Project; README. Builds without warnings (`the_maintenance_example_builds_without_warnings`). Not yet tried by hand on a real ledger.
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
- 2026-11-12: maintenance went into the 0.41.0 commit half-built and failed the release's checks. User: "apparently an ai doesnt know how to make an app, eh", and "to me its prove that the directives an ai get arent nearly enough atm". Finished the same day; a release plan now spec checks every example in the checks list first.

## Was
- memory: examples-own-stack.md, examples-before-done.md, second-example.md, full-width-layouts.md
- IMPLEMENTED.md section 1 (examples table)
