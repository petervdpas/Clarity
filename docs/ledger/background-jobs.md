# Background jobs

Status: not built. An action started in the background, `at most N` at a time, its results coming back to the window as ordinary handlers; the runtime does the counting, no thread or lock appears in Clarity code.

Progress: 0%

## Code
- none.
- What it builds on: actions with declared effects and no screen, handlers running one at a time, row versions ([row-versions](row-versions.md)), `me` ([signin](signin.md)), the progress part.
- Docs: none.

## Tests
- none.

## Open
- `at most N` and `start`: NOT BUILT; see Design "A job is an action".
- `on done`, `on failed`, `on finished`: NOT BUILT; see Design "Results".
- Naming, watching and stopping a run: NOT BUILT; see Design "Watching and stopping".
- A run as the person who started it: NOT BUILT; see Design "Watching and stopping".
- The runtime gate and per-runtime scheduling: NOT BUILT; see Design "What runs in parallel" and "Every stack".
- The erasure example: NOT BUILT; see Design "The example".

## Design

### A job is an action

No new declaration: an action already runs without the screen and declares its effects. One line on the action and one statement:

```clarity
action sendRequest(request: Request) -> bool
    effects net, db.write
    at most 4
    let sent = send mail to request.site.privacyEmail subject @erasureSubject body request.letter from outbox
    request.messageId = sent.messageId
    save request
    return sent.ok

code Requests
    handler sendAllClicked
        start sendRequest(r) for r in pending on done: requestSent on finished: allSent

    handler requestSent(request: Request, ok: bool)
        sentCount = sentCount + 1

    handler allSent
        message @allRequestsSent(count = sentCount)
```

- `start action(args)` runs one call in the background and the handler goes on at once; `start action(x) for x in list` runs one call per item.
- `at most N` is the limit for every background run of that action in the program. Without it, `at most 1`, so nothing runs in parallel by accident.
- A direct call stays as today (the window waits) and does not count against the limit.
- The checker refuses `start` of anything that is not an action; an action that touches the window is already an error.

### Results

- `on done: handler` runs on the window after each call ends, with the call's arguments and result, in the order calls end.
- `on failed: handler` runs with the arguments and the problem text when a call fails or is refused; the others go on. A save refused by the version check ([row-versions](row-versions.md)) is such a failure, with the `savedSince` text.
- `on finished: handler` runs once when every call of that `start` has ended.
- These are ordinary handlers, run one at a time between others, so they may touch parts and window data.

### Watching and stopping

- `start ... as sending` names the run: `sending.total`, `sending.done`, `sending.failed`, `sending.running` (bool). A progress part binds to it (`progress bind sending.done max sending.total`).
- `stop sending` lets running calls end and starts no new ones; `on finished` still runs.
- A run belongs to the window that started it; closing the window stops it as `stop` does.
- A run runs as the person who started it: `me` in its calls is that person, the action's `allow when` is checked at `start` for every call, `readable when` and `writable when` hold inside it. Signing out stops the person's runs.

### What runs in parallel

- One gate in the runtime, one call at a time: `find`, `save`, `delete` and queries, files of file stores, settings, library functions. The database keeps its single connection.
- Outside the gate, up to `at most N` at once: `http` calls, mail, formats, and computation on the call's own values.
- A save is one transaction; two calls that change the same record: the second is refused by the version check.
- A later version may let database work run in parallel; the language does not change, since the gate is the runtime's.

### Every stack

- Each runtime's own scheduling, not a shared library: Rust threads with a counting gate, Go goroutines with a buffered channel, C# `Task` with `SemaphoreSlim`, C and C++ a small thread pool; the gate a single lock at the runtime's entry points.
- Desktop: the window keeps working. Served: a run belongs to the browser's window that started it, and the limit counts across the server. CLI: a command waits for its runs before it exits.
- Designer: `at most` in the action's properties; `on done`, `on failed`, `on finished` wired as events are.
- Manual `code.jobs.md` registered in `crates/clarity-tools/src/manual.rs`; i18n for Clarity's own words (the Stop caption of a run in a progress part).
- `conformance/jobs.json`: the limit holds, `on done` per call, `on failed` without stopping the rest, `on finished` once, `stop`.

### The example

`examples/erasure`, `stack go desktop cli`, the acceptance test of formats, encrypted storage, mail, sign-in, row versions and jobs, built once each of them is done:

- Person, Site, Account, Request entities; Status enum (sent, acknowledged, done, refused, noReply).
- A password manager's export as a `csv` source, imported into Account; JustDeleteMe JSON through `http get`; a privacy contact found by a local model through `http post` with `plainText`.
- `encrypted "setting:databaseKey"` with `databaseKey: secret, forget`; one family member per sign-in on the desktop with `account` and `password`.
- Requests sent from each person's own address, at most 4 at a time with a progress bar; a save refused when another family member changed the record; replies matched by `inReplyTo`; the `mailto:` route with `encoded` for those without SMTP.
- CLI command `requests overdue` for cron.

### Open questions

- Runs that outlive their window (a dialog closed), runs started by the app (`on start`) or on a schedule (`every day at 08:00 run overdueCheck`), versus the CLI target plus cron, which works today.
- `on done` in the order calls end (proposed) or in list order.
- `me` in a run with no person: an optional record, or not readable.
- Later: database work in parallel (a connection per call), and library functions outside the gate when a library says it is safe for threads.

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
- docs/APP-CAPABILITIES-PLAN.md step 6 (all), "Steps 4, 5 and 6 as one slice" (jobs run as their starter), "The example", "Order" 6-7, open questions (outliving runs, `on done` order, `me` without a person, scheduled work, parallel database)
- UNIMPLEMENTED.md 6 "Encrypted storage, mail, jobs" (jobs, scheduled work)
- memory: app-capabilities-plan.md (step 6)
