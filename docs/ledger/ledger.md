# The ledger

Status: partial. This folder: one entry per piece, read through the index; it replaces the plan documents, DESIGN.md and the status documents. Written; the move off the old documents is not finished.

## Code
- Docs: `docs/ledger/README.md` (index and how to keep it), `docs/ledger/<piece>.md`
- Old documents still in the tree: `docs/DESIGN.md`, `docs/*-PLAN.md`, `docs/WINDOWS-TESTS.md`, `docs/MODULAR-PLAN.md`, `docs/IMPLEMENTED.md`, `docs/UNIMPLEMENTED.md`, `docs/CONFLICTING.md`
- `crates/clarity-tools/src/lib.rs:33` embeds `docs/DESIGN.md` for the agents' `design` tool (see [mcp-tools](mcp-tools.md))

## Tests
- none yet; a test that every index line has its entry and every entry its index line, and that each entry keeps the fixed headings, would guard it

## Open
- Phase 2, old references: NOT BUILT. About 2000 code comments cite `(DESIGN N.N)` and about 430 cite a `*-PLAN.md`; each becomes `docs/ledger/<piece>.md`, found by the entries' Was lists. The generated TS in `runtimes/web/src/generated` changes with its Rust source. Waits for the other session to commit its sign-in work.
- Phase 3, the `design` tool reads the ledger instead of DESIGN.md: NOT BUILT (Rust, ask first).
- Phase 4, delete the plans, WINDOWS-TESTS.md, MODULAR-PLAN.md, the three status documents and DESIGN.md: NOT BUILT (after 2 and 3).
- Phase 5, Claude's memory notes point at entries instead of holding status: NOT BUILT.
- Reviewing the entries: through the maintenance example (see [examples](examples.md)), reviews kept as `docs/ledger/reviews/<piece>.md`.
- The repository README's first line still states the struck goal (see [vision](vision.md)).

## Conflicts
- Line numbers in entries were taken on 2026-11-12 while another session changed sign-in code; some have drifted (see each entry).

## Decisions
- 2026-11-12: three surveys first, from code only. User: "check against code, not the design... I think the design doc is confusing!", "write IMPLEMENTED.md for every part in clarity!!!!", "from code only!", then "now make an unimplemented.md", "now make CONFLICTING.md".
- 2026-11-12: their goal. User: "the goal with these documents (implemented, unimplemented and conflicting) is to clean up the plan documents that are done and make a better guide for the modularity plan!"
- 2026-11-12: fully built plans deleted (VERSION, PROJECT-BROWSER, APP-ICON-SPLASH, CXAML-SOURCE), their citations removed from code comments.
- 2026-11-12: a ledger instead. User: "this is too big, even for a LARGE language model!! It's not working as a development ledger, so we need to make that... a development ledger!" Shape: a folder of small files; it replaces the plans, the three documents, DESIGN.md and the status in Claude's memory notes.
- 2026-11-12: the first design is struck (see [vision](vision.md)). User: "strike first-design"
- 2026-11-12: the old references wait. User: "wait for the other session to commit"
- 2026-11-12: the manual moved from `docs/manual/` to `manual/` at the root (see [docs](docs.md)).
- 2026-11-12: the work on the ledger is itself in the ledger. User: "the things you doing right now should go in the ledger as well"

## Was
- docs/IMPLEMENTED.md, docs/UNIMPLEMENTED.md, docs/CONFLICTING.md (their structure; their facts are in the entries)
- memory: development-ledger.md
