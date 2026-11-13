# Examples

Status: built. Example projects in `examples/`, each a project file with a folder of its modules and a README, built on the stack it names; every example is also a test.

## Code
- Examples: `examples/<name>.clarity` and `examples/<name>/` (modules, `design/*.cxaml`, translations, resources, README.md)
- Rust desktop: dashboard, maintenance (with cli), modelling, network, shop
- Go desktop: calculator, dial, pills, records, videoplayer, waves; Go desktop cli: stock; Go web: invaders
- C# desktop: mp3player, noughts, todo; C# web: walker; C# cli: ledger
- C desktop: kitchen; C++ desktop: novel, orbs
- Docs: `README.md` (examples list, one line each)

## Tests
- `crates/clarity-gen/tests/examples_build.rs`: `every_example_is_here` (the folder and the list agree, 21) and one ignored build test per example on its own stack, run one at a time
- `.github/workflows/checks.yml:286`: every example built on its stack in CI
- Examples are swept by spec tests: `cxaml.rs`, `design_edit.rs`, `keying.rs`, `placed_places.rs` (every example's windows); `diagram_views.rs` (a fixed list of ten)

## Open
- maintenance: built (2026-11-12). Main: toolbar (Projects, Reload, Review); left sidebar (list `projectList`, tree table `entryTree` by `key` and `parent`: label, status, reviewed); content MainView (one `markdown` part bound to `entryBody`); slide-out `reviewPanel` with tabs Review (reviewed, by, on, notes as a markdown memo), References (own references, related entries), Code and tests, Old references (both markdown), and Save. Projects with ProjectsView (table: name, ledger folder). Reviews are records of entity `Review` (project, piece, reviewed, reviewer, reviewedOn, notes, ownReferences), migration 0002. Builds without warnings. Tried by hand on Clarity's ledger 2026-11-12: the first pick of a project showed no entries until Reload (runtime bug, see [settings-secrets](settings-secrets.md)).
- maintenance, command line and MCP: built (2026-11-12). User: "also add mcp server to the maintenance app". The project file names `rust desktop cli`; commands `project list`, `project add`, `review list`, `review show`, `review set`, each with an `agent` line, so `maintenance mcp` serves them (see [app-mcp](app-mcp.md)); `agent/guidance.md` in the four sections, checked clean. The commands keep to the app's own data (projects, reviews): reading the ledger in a command would set the person's `ledgerFolder` setting, so agents read ledger files themselves. Built with the example's build test and tried live over MCP in a scratch folder. Building it found that the Rust and C++ command-line programs did not compile an enum option (now by its module's path; the commands fixture has one).
- maintenance, ReviewView: NOT BUILT. The slide-out's content becomes a view `ReviewView` (review, related, code, old references, the tabs, Save) that tells Main `saved`; Main marks the tree. Waits for clarity-b5's runtime fix and a reconnected MCP server (tools `add_tell`, wire_event `view`).
- maintenance, generating the documents: NOT BUILT, needs decision. User: "it should be able to generate the documents from the app"; chosen: ledger entries, the index, a review report, and the whole ledger from data. Sketch: entities Group (project, name, position), Piece (project, group, name, title, status enum Built/Partial/NotBuilt/NeedsDecision, summary, position), Item (piece, section enum Code/Tests/Open/Conflicts/Decisions/Was, marker enum none/NotBuilt/Partial/Untried, text, position); an import reads today's Markdown into them (the parser exists); writing gives `<piece>.md` with the fixed headings, `README.md` (the keeping rules from a resource, the index table with each piece's first Open item) and a review report, the same bytes every time. Decided: the database, after one import. Decided: Claude changes entries through the running app as an MCP server, each change granted by the person (see [app-mcp](app-mcp.md)), and through a command line of the app when it is closed (the project file names `rust desktop cli`, like the stock example's `desktop cli`), which also regenerates the files. Open questions: the commands (set an entry's status, add or close an open item, add a decision, generate); where the review report goes (the ledger folder or a folder of its own); the README's keeping rules then say "generated, do not edit".
- maintenance, what it is to do: pick a project (Clarity is one of them); its ledger as an orderly list in a left sidebar (a tree grouped as the ledger's index, title, status, reviewed); the entry on the right as Markdown (`markdown` part bound to its text); a slideout with the review: reviewed, by (Peter or Claude), when, notes (a `memo` in markdown mode), the person's own references, and read-only related entries, code paths and old references (the entry's Was). Projects are stored (name, ledger folder); a folder setting that picking a project sets points one file store at that ledger; reviews are kept in the app's database.
- No example uses rust web, c web or cpp web; `everything_build.rs` and the `the_web_target_serves_in_*` tests build those.
- No example calls the picture functions (see [picture-functions](picture-functions.md)); the modelling example does not use the graph (see [graph](graph.md)).

## Conflicts
- DESIGN.md:1363 says "All examples is eight"; there are 21. The removed dice example is still cited in DESIGN.md and LANGUAGE.md (see [docs](docs.md)).

## Decisions
- 2026-11-03: build an example only for the stack its project file names; "all examples" means every one. (memory: examples-own-stack.md)
- 2026-11-06: a slice is done only when every example builds on its own stack. (CLAUDE.md)
- 2026-11-12: maintenance, a Clarity app to review ledgers. User: "make a clarity app so we can review everything in there! call it \"maintenance\"... and clarity is 'just' one project in the app"; the list "as an orderly list (left sidebar)", on the right "a review page with slideout for properties like reviewed and references"; the page a "markdown text widget".
- 2026-11-12 (replaced below): reviews are files beside the ledger, so Claude can review too; references are related entries, code paths, old references and the person's own links; rust desktop.
- 2026-11-12: reviews move out of the ledger into the app's database (entity Review). User: "we are not going to put this in docs/ledger/reviews!", then chose "App's database".
- 2026-11-12: the app generates the ledger's documents (entries, index, review report) from data in its database, filled by one import of the Markdown. User: "it should be able to generate the documents from the app"; chose all four kinds, "Database, import once" and "App command line" for Claude's changes.
- 2026-11-12: maintenance is an example, not a project elsewhere. User: "you better put it in examples so it's another test as well!"
- 2026-11-12: maintenance went into the 0.41.0 commit half-built and failed the release's checks. User: "apparently an ai doesnt know how to make an app, eh", and "to me its prove that the directives an ai get arent nearly enough atm". Finished the same day; a release plan now spec checks every example in the checks list first.

## Was
- memory: examples-own-stack.md, examples-before-done.md, second-example.md, full-width-layouts.md
- IMPLEMENTED.md section 1 (examples table)
