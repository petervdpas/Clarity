# The ledger

Status: partial. This folder: one entry per piece, read through the index; it replaces the plan documents, DESIGN.md (removed 2026-11-13) and the status documents. Written; the move off the plans and status documents is not finished.

Progress: 25%

## Code
- Docs: `docs/ledger/README.md` (index and how to keep it), `docs/ledger/<piece>.md`
- Release checks read it: `CLAUDE.md` "Releasing" names every entry marked partial and every test noted as failing before a release plan
- Old documents still in the tree: `docs/*-PLAN.md`, `docs/WINDOWS-TESTS.md`, `docs/MODULAR-PLAN.md`, `docs/IMPLEMENTED.md`, `docs/UNIMPLEMENTED.md`, `docs/CONFLICTING.md`
- `docs/DESIGN.md` and `docs/LANGUAGE.md` are gone (2026-11-13), and with them the agents' `design` and `reference` tools; their content is in the manual (see [docs](docs.md))

## Tests
- none yet; a test that every index line has its entry and every entry its index line, and that each entry keeps the fixed headings, would guard it

## Open
- Phase 2, old references: NOT BUILT; unblocked (the sign-in work was committed in 3eb6ef0c). About 1,470 code comments cite `DESIGN N.N` and about 430 cite a `*-PLAN.md`; each becomes `docs/ledger/<piece>.md`, found by the entries' Was lists. The generated TS in `runtimes/web/src/generated` changes with its Rust source. Code written since 2026-11-12 already cites `docs/ledger/<piece>.md` (about 120 lines, e.g. `crates/clarity-spec/src/fit.rs:2`).
- Phase 4, delete the plans, WINDOWS-TESTS.md, MODULAR-PLAN.md and the three status documents: NOT BUILT (after 2). DESIGN.md was deleted on 2026-11-13 (see [docs](docs.md)).
- Phase 5, Claude's memory notes point at entries instead of holding status: NOT BUILT.
- Reviewing the entries: UNTRIED. The maintenance example is built (it checks the index against the entry files and reads Progress lines); not yet run on this ledger (see [examples](examples.md)). Reviews are kept in that app's database, not in this folder.
- The repository README's first line still states the struck goal (see [vision](vision.md)).

## Conflicts
- none

## Decisions
- 2026-11-12: three surveys first, from code only. User: "check against code, not the design... I think the design doc is confusing!", "write IMPLEMENTED.md for every part in clarity!!!!", "from code only!", then "now make an unimplemented.md", "now make CONFLICTING.md".
- 2026-11-12: their goal. User: "the goal with these documents (implemented, unimplemented and conflicting) is to clean up the plan documents that are done and make a better guide for the modularity plan!"
- 2026-11-12: fully built plans deleted (VERSION, PROJECT-BROWSER, APP-ICON-SPLASH, CXAML-SOURCE), their citations removed from code comments.
- 2026-11-12: a ledger instead. User: "this is too big, even for a LARGE language model!! It's not working as a development ledger, so we need to make that... a development ledger!" Shape: a folder of small files; it replaces the plans, the three documents, DESIGN.md and the status in Claude's memory notes.
- 2026-11-12: the first design is struck (see [vision](vision.md)). User: "strike first-design"
- 2026-11-12: the old references wait. User: "wait for the other session to commit"
- 2026-11-12: the manual moved from `docs/manual/` to `manual/` at the root (see [docs](docs.md)).
- 2026-11-12: the work on the ledger is itself in the ledger. User: "the things you doing right now should go in the ledger as well"
- 2026-11-13: DESIGN.md and LANGUAGE.md removed; agents read only the manual (see [docs](docs.md)). The `design` tool went with DESIGN.md, so phase 3 (the tool reads the ledger) is gone.

## Was
- docs/IMPLEMENTED.md, docs/UNIMPLEMENTED.md, docs/CONFLICTING.md (their structure; their facts are in the entries)
- memory: development-ledger.md
