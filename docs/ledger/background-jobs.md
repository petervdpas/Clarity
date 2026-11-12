# Background jobs

Status: not built. An action started in the background, `at most N` at a time, its results coming back to the window as ordinary handlers; the runtime does the counting, no thread or lock appears in Clarity code.

## Code
- none. Designed in APP-CAPABILITIES-PLAN.md step 6.
- What it builds on: actions with declared effects and no screen, handlers running one at a time, row versions ([row-versions](row-versions.md)), `me` ([signin](signin.md)), the progress part.
- Docs: none. Planned: manual `code.jobs.md` registered in `crates/clarity-tools/src/manual.rs`; LANGUAGE.md; agent reference.

## Tests
- none. Planned: `conformance/jobs.json` (the limit holds, `on done` per call, `on failed` without stopping the rest, `on finished` once, `stop`).

## Open
- `at most N` on an action (default `at most 1` for background runs); `start action(args)`, `start action(x) for x in list`; a direct call stays as today and does not count: NOT BUILT.
- `on done: handler` (arguments and result, in the order calls end), `on failed: handler` (arguments and problem text), `on finished: handler` (once per `start`): NOT BUILT.
- `start ... as sending` with `sending.total`, `.done`, `.failed`, `.running`; `stop sending`; a run belongs to the window that started it and closing it stops the run: NOT BUILT.
- A run runs as the person who started it: `me` is that person, `allow when` checked at `start`, signing out stops the person's runs: NOT BUILT.
- Per runtime: Rust threads with a counting gate, Go goroutines with a buffered channel, C# `Task` with `SemaphoreSlim`, C and C++ a small thread pool; one gate for data, files, settings and libraries; served programs count the limit across the server; a CLI command waits for its runs: NOT BUILT.
- Designer: `at most` in the action's properties; wiring `on done`, `on failed`, `on finished`: NOT BUILT.
- Scheduled work (`every day at 08:00`) and runs that outlive their window or start from `on start`: NOT BUILT, open questions (APP-CAPABILITIES-PLAN.md:397, :414).
- The `examples/erasure` example (`stack go desktop cli`), acceptance test of all six steps: NOT BUILT (APP-CAPABILITIES-PLAN.md "The example").
- Open questions: `on done` in the order calls end (proposed) or list order; `me` in a run with no person; later, database work and safe library functions outside the gate.

## Conflicts
- none.

## Decisions
- 2026-11-10: a background job with a declared limit. User: "add a background job with at most N to the plan" (after "could clarity apps use semaphores?")
- 2026-11-10: no new declaration: a job is an action; one line `at most N` and one statement `start`. Results come back as ordinary handlers, one at a time (C++ Builder's `Synchronize`, built in).
- 2026-11-10: the smaller version: one runtime gate for the database, files, settings and libraries; only http, mail, formats and computation run in parallel. User: "put the smaller version in the plan"
- 2026-11-10: scheduling is each runtime's own, not a shared library; the shared libraries called outside the gate keep their problem text per thread.
- 2026-11-12: steps 4, 5 and 6 as one slice, in that order. User: "after the sums and release I want to do 5 and 6", then "think it kinda combines with multi user app... so 4 too then?"
- 2026-11-12: a background save refused by the version check is a failed call of its run.

## Was
- APP-CAPABILITIES-PLAN.md step 6 (all), "Steps 4, 5 and 6 as one slice" (jobs run as their starter), "The example", "Order" 6-7, open questions (outliving runs, `on done` order, `me` without a person, scheduled work, parallel database)
- UNIMPLEMENTED.md 6 "Encrypted storage, mail, jobs" (jobs, scheduled work)
- memory: app-capabilities-plan.md (step 6)
