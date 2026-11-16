# The ledger

Status: partial. This folder: one entry per piece, read through the index; it replaces the plan documents, DESIGN.md (removed 2026-11-13) and the status documents. Written; the move off the plans and status documents is not finished.

Progress: 60%

## Code
- Docs: `docs/ledger/README.md` (index and how to keep it), `docs/ledger/<piece>.md`
- Release checks read it: `CLAUDE.md` "Releasing" names every entry marked partial and every test noted as failing before a release plan
- Each note holds everything about its piece: what is built (Code, Tests), what is open (Open, one line per item), and the design of what is not built yet, written once (Design). The manual describes what is built.
- Folded into the notes and removed 2026-11-16: the plan documents, the Windows test list, the modular plan and the three status documents; each note's Was says where their sections went
- `docs/DESIGN.md` and `docs/LANGUAGE.md` are gone (2026-11-13), and with them the agents' `design` and `reference` tools; their content is in the manual (see [docs](docs.md))

## Tests
- none yet; a test that every index line has its entry and every entry its index line, and that each entry keeps the fixed headings, would guard it

## Open
- Phase 2, old references: built 2026-11-16. The plan citations (about 350) point at the note that holds the piece; the `DESIGN N.N` citations (about 1,480, all on built code) point at the manual page that describes it, with the old roadmap codes beside them dropped (user: "fix the DESIGN N.N comments too"). The notes' Was lists still trace the old numbers.
- Phase 4, fold in and delete the plans, the Windows test list, the modular plan and the three status documents: BUILT 2026-11-16 (Decisions). DESIGN.md was deleted on 2026-11-13 (see [docs](docs.md)).
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
- 2026-11-16: one piece, one file. The plans, the Windows test list, the modular plan and the three surveys are folded in and removed; the design of what is not built yet goes once into its note's Design section (heading order Status, Code, Tests, Open, Design, Conflicts, Decisions, Was), and Open points into it. The user wanted the docs "distinct and unambiguous", having "the idea that different docs concern about the same thing and too much about stuff that is decided different upon in the end or implementation", and agreed: "yes, go ahead... and remove the old docs"

## Was
- docs/IMPLEMENTED.md, docs/UNIMPLEMENTED.md, docs/CONFLICTING.md (removed 2026-11-16; their structure; their facts are in the entries; the last two items found nowhere else went to [vision](vision.md) and [language](language.md))
- memory: development-ledger.md
