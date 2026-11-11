# App capabilities plan: data formats, encrypted storage, mail, sign-in, version check, background jobs

Six things Clarity applications cannot do today, each added as a general concept on every stack rather than as a library one project writes for itself. Written 2026-11-10. Data formats (FORMATS-PLAN.md) step 1 released in Clarity 0.38.0 (JSON through Clarity.Formats); everything else not built.

## Goal

The user, after an outside review of how a tool for clearing personal data from websites (for the user and family members) would be built in Clarity, which named the missing pieces: "I want you to make a plan for it, to add them in the most generic way possible so it improves the sort of apps clarity can create!"

On a first draft that added a CSV reader: "csv is just one format of data... making it specific isnt very generic, is it... you need to keep thinking c++ builder and what you could make with that back in the days"

The review named these gaps:

- "I didn't see a built-in CSV reader, so you either parse it yourself or add a small library for it."
- "IMAP/SMTP: there's nothing built in for email."
- "Encryption at rest: the docs only describe plain SQLite."
- "I didn't find anything about login or user accounts for the web target in the docs."

What it found already covered stays as it is: `http get`/`post` (also for a local model such as Ollama), `read ... from json`, `rest` data sources with `auth bearer "setting:..."`, secret settings, the `cli` target for jobs run from cron, the `timer` part, `mailto:` links.

## The idea

In C++ Builder a grid did not care where its rows came from. A TDataSet was a TDataSet: a database table, a query, or a TClientDataSet loaded from a file. A TDBGrid and a TDBEdit showed and edited any of them, and the component palette had a tab per kind of connection (data access, Internet, dialogs), each dropped on a form and set in the Object Inspector. That is why a person could make a working program over a file, a database or a mail server in an afternoon.

Clarity already has half of that: entities are the datasets, data sources say where they live (sqlite, postgres, mysql, mssql, rest, external), and parts bind to them without code. This plan fills in the other half:

- A **format** is one pluggable thing that turns text or bytes into records and back. JSON is the first; CSV, XML and the rest are more of the same, and a project can add its own through a library.
- A **file** is a data source, in any format, so a table part edits a CSV or XML file as it edits a database table.
- **Encryption** is a property of storage, of any kind the application owns.
- **Mail** servers are data sources too, the first of an Internet tab of connections.
- **Who is using the program** is part of the app, for every target.

## What exists to build on

- `read T from json text` and `json of value` (LANGUAGE.md, code.web-apis.md): the name matching, defaults and errors that name the place become the rules of every format.
- `datasource ... external` with `table`, `key` and `column` mapping, and the Import panel that writes entities from tables (data.sources.md): a file source maps and imports the same way.
- `files f in store where ...` and `read text of f` (data.pictures-files.md): the mail step reads a mailbox the same way.
- `"setting:name"` secret settings and `"env:NAME"`: keys and passwords for every step.
- Each runtime's SQLite: rusqlite `bundled` (Rust), modernc.org/sqlite (Go, pure Go), Clarity.Runtime.Sqlite (C#), the bundled amalgamation in `runtimes/c/clarity/sqlite.c` and `runtimes/cpp/clarity/sqlite.cpp`.
- The web target already tells browsers apart (one id per tab, per-browser settings; DESIGN.md B2). Nothing knows who a person is.

## Step 1: Formats, and files as data sources

Split out into [FORMATS-PLAN.md](FORMATS-PLAN.md): one catalog of formats in the shared Rust library `clarity-formats` (JSON moved in as its first entry, CSV next), used by `read ... from <format>` and `<format> of` in code and by data sources that are files in a format, each step a complete slice. The two text functions the review needed stay here: `encoded(text)` (percent-encoding for a URL part, so `"mailto:{to}?subject={encoded(subject)}"` works) and `plainText(html)` (the readable text of an HTML page, for a model or a memo); they go into the language with the next step that touches text functions.

## Step 2: Encrypted storage

Any storage the application owns can be encrypted on disk, with its key never in the project.

```
datasource main sqlite "data/people.db"
    encrypted "setting:databaseKey"

filestore scans "data/scans"
    encrypted "setting:databaseKey"
```

- `encrypted "setting:name"` takes the key from a secret setting, and nothing else: no new way of asking. While the secret is not kept, the storage does not open and the application shows the window where the secret is entered, as it already does for a secret database connection (`unconnectedNotKept`, `on unconnected`).
- For a computer several people share, the secret setting itself gets one option, not the storage: `databaseKey: secret, forget` keeps it in memory for the run only, never in the system's store, so the window asks at every start. That option then works for every secret (an API token too).
- Where it applies: a `sqlite` source, a file source (FORMATS-PLAN.md), a file store, and a backup of any of them (BACKUP-PLAN.md). Server databases encrypt on their server; the checker says so.
- A SQLite database uses one file format on every stack and in the IDE, because the IDE reads the same database (data browser, migrations, import, backups). The candidate is SQLCipher 4, through SQLite3 Multiple Ciphers (MIT, an amalgamation that reads and writes SQLCipher files) where a stack bundles its own SQLite. Files and file stores use what Clarity's vault already uses (crates/clarity-vault: Argon2id for a password, AES-256-GCM per file under a fresh nonce), not a second scheme, the same in every runtime and readable by the IDE.
- Changing the key, and encrypting or decrypting what exists: `clarity encrypt project main`, and in the IDE beside the source in Settings. In the application, changing the secret setting in its settings window re-encrypts.
- A forgotten password loses the data, as with the vault; the dialog says it once when the password is set.

Opens: anything holding personal data (health, finance, family, customer lists) on a laptop.

## Step 3: Mail, the first Internet connections

C++ Builder's Internet tab (Indy) gave SMTP, POP3, IMAP, FTP and more as components of one family. In Clarity they are data sources of one family, with one auth line and one TLS rule. This step builds mail; the same shape takes more kinds later.

```
datasource outbox smtp "smtp://127.0.0.1:1025"
    auth basic "setting:mailLogin"
    from "setting:mailAddress"

datasource inbox imap "imap://127.0.0.1:1143"
    auth basic "setting:mailLogin"
```

### 3a: Sending

```clarity
let sent = send mail to site.privacyEmail subject @erasureSubject body letter from outbox
if not sent.ok
    message "Not sent: {sent.text}"
```

- `send mail to address subject text body text from source`, with optional `cc`, `bcc`, `replyTo`, `html body`, `attach f` (a file of a file store, or bytes with a name, so `attach csv of requests named "requests.csv"` sends any format of step 1) and `header "Name": value`.
- The reply is like an HTTP reply: `ok`, `text` (why not), and `messageId`, so the app can store it and match replies later. It never stops the code.

### 3b: Reading

```clarity
for m in mails in inbox folder "INBOX" where m.inReplyTo == request.messageId
    request.status = Answered
    request.reply = m.text
    save request
```

- `mails m in source` with `folder`, `where`, `order by` and `limit`, the shape of `files f in store`. A mail has `id`, `from`, `to`, `cc`, `subject`, `date`, `text`, `html`, `messageId`, `inReplyTo`, `seen` and `attachments` (each with `name`, `size`, and `read bytes of`, so `read list<Order> from csv read text of a` reads an attached file in any format).
- `where` on `from`, `subject`, `date`, `seen` and `messageId` goes to the server as an IMAP search; the rest is filtered after fetching. Bodies and attachments are fetched only when code reads them.
- `mark m seen`, `move m to "Archive"`, `delete m`.

### Both

- TLS by default (`smtps://`, `imaps://`, or STARTTLS on `smtp://`/`imap://`); plain only to localhost. A local bridge with its own certificate (Proton Mail Bridge) names it: `trust "certs/bridge.pem"`.
- Login is `auth basic` from a secret setting; OAuth is an open question.
- Written once in the shared Rust library `clarity-mail` (see "Where the code lives"), vendored like every other dependency (OFFLINE-PLAN.md).
- Tests against a small fake SMTP and IMAP server in the suites, offline; `conformance/mail.json`.
- The agent never sends mail itself (agent-never-executes rule); sending is the built application's.

### Later on the same line

Not in this plan, noted so the shape fits them: `pop3`; a file store on a server (`filestore shared sftp "sftp://..."`, so `files f in shared` and the file sources of FORMATS-PLAN.md work remotely). Message streams (`tcp`, `mqtt`, `websocket`) are not a second concept here: they are the transports of HARDWARE-PLAN.md, which a later step opens to more than devices.

Opens: help desks, CRM follow-up, notifications from any app, mail-in forms, the review's reply tracking.

## Step 4: Who is using the program

The app knows who is using it, on the web and, when asked, on the desktop.

```clarity
app Erasure
    main Requests
    signin Member by email password passwordHash
    roles Role
```

```clarity
entity Member
    email: email, unique, required
    passwordHash: text, hashed
    role: Role
```

- `signin Entity by field password field` names the entity of people and the field they sign in with. No new type: `password` is already the input part, and `secret` is a setting type that an entity field may not have. A text field with the modifier `hashed` is stored as an Argon2id hash (the vault's crate): code assigns text to it (`member.passwordHash = typed`), which stores the hash, and never reads it back. The `password` part binds to it as to any text.
- Served, every browser gets the sign-in page first (drawn by Clarity.UI, texts by Clarity's keys), then a session cookie (HTTP-only, SameSite, expiry). Failed tries slow down per address and per account.
- `signin ... desktop` asks on the desktop too: one computer, several family members, each with their own records.
- Code reads `me`, the signed-in record, and `me.role`. A window, view or command says who may open it: `allow Admin, Parent`; the designer shows it as a property of every page and command. The program refuses what is not allowed, whatever a browser sends.
- Rows are not filtered by magic (determinism rule): code writes `where r.person.owner == me`. An entity can declare `readable by owner` so the program refuses records that are not the person's, as a check rather than a hidden filter.
- First start with no records of the entity: the page makes the first account, with the first role value.
- `clarity users project add|password|remove` from the command line, for a server without a browser at hand.

Opens: anything used by more than one person: family tools, small team apps, a club's records.

## Step 5: A save never overwrites someone else's change

The user, after asking "really, and same data manipulation by two or more client sides?": "yes, add the version check as a step".

Today a save writes every field of the record as its window holds it (`UPDATE ... SET <all columns> WHERE id = ?` in each runtime), with no check whether the record changed since the window read it. Two people (or two windows, or a window and a background job) open customer 12; Anna saves a new phone, then Ben saves a new address and writes Anna's old phone back with it. Anna's change is gone and nobody is told. The database stays sound because handlers take turns; the data does not.

### The check

- Every record of an owned entity carries a version, a whole number the runtime keeps in a column of its own (`row_version`, snake_case as columns are). Code never sets it; `find` reads it with the record.
- A save of an existing record writes only when the version is still the one it read: `UPDATE ... SET ..., row_version = row_version + 1 WHERE id = ? AND row_version = ?`. No row changed while the record exists means someone saved it since: the save is refused and changes nothing.
- A delete is checked the same way: deleting a record someone changed since is refused too.
- A refused save behaves as a refused save does today: the handler stops and the person sees a message, Clarity's word `savedSince` ("Someone else changed this record since you opened it. Reload it to see their change."). Code that wants to do more catches nothing new: `reload customer` reads the current record, and the window shows it.
- Overwriting on purpose is written out, never guessed (determinism): `save customer anyway` writes regardless of the version.
- One record is one unit. No merging of fields (Anna's phone with Ben's address): a merge guesses which change wins, and that is the person's call after a reload.

### One mechanism, used everywhere

- **Undo.** Undo already refuses when what it would take back was changed since (`undoChangedSince`). It uses the version too, so there is one notion of "changed since", not two.
- **Background jobs (step 6).** A job's save that is refused is a failed call: `on failed` runs with the `savedSince` text, the other calls go on.
- **Served programs.** Two browsers are exactly the case above; nothing extra.
- **The IDE's data browser** saves through the same check, so an edit in Clarity while the program runs is refused rather than lost.
- **Events on records** run inside the save, so a refused save runs none of its events' writes.

### Where the version lives

- **Owned SQL sources** (SQLite, PostgreSQL, MySQL, SQL Server): Clarity proposes one migration step per source that adds `row_version` to every table, starting at 1 for the records already there, as any step is proposed. New entities get it from their first step.
- **External databases** that already have such a column name it, as they name their key: `version "RowVersion"`. Without it there is no check on that entity, and the checker says so with a warning, so it is a choice and not an accident. An entity can also say `version none` to silence the warning.
- **File sources** (FORMATS-PLAN.md): the file is the unit. A save refuses when the file changed on disk since it was read (its size and modification time, then its hash), with the same `savedSince`.
- **Web API sources**: a service that hands out an ETag gets `If-Match` on its writes; open question whether that comes with this step.

### Every stack, every target

- Per runtime: the version column read with every record, the checked UPDATE and DELETE, `anyway`, the refusal. One conformance file, `conformance/versions.json`: two copies of one record, the second save refused; `anyway` writes; a delete of a changed record refused; undo refused through the version; a file source changed on disk refused.
- Language and checker: `save ... anyway`, `version "Column"` and `version none` on entities of external sources, the warning.
- Generator and migrations: the proposed step, the column in every stored entity's mapping.
- IDE: the data browser's saves; the entity editor shows the version column as Clarity's, not editable; Settings shows nothing new.
- Manual: `data.entities.md` and `code.handlers.md` (what a refused save looks like, `reload`, `anyway`), `data.migrations.md` (the proposed step), `data.sources.md` (`version` on external entities); all already registered in `crates/clarity-tools/src/manual.rs`. LANGUAGE.md, agent reference, i18n for `savedSince`.
- Every example: the proposed step written and applied; every example building green.

### Later: other screens follow

Not in this step: when a record is saved, the windows of other people (or other windows) that show it reload by themselves, so stale screens become rare and refusals rarer. On the desktop that is the windows of one program; served, the server tells the pages that show the record. The version check is what makes it safe either way, so it comes first.

Opens: every program used by more than one person or window: shared family tools, small team apps, anything served, and every program with background jobs.

## Step 6: Background jobs, at most N at a time

The user, after semaphores came up ("could clarity apps use semaphores?", "could c++ builder do semaphores?"): "add a background job with at most N to the plan".

Clarity code has no threads: handlers run one at a time and a long call makes its window wait. That avoids locks, but it cannot do "send these 200 requests, at most 4 at a time". C++ Builder did it with TThread and a TSemaphore, later TTask and TParallel.For. Clarity gets the second kind: the program says what runs in the background and how many at once, and the runtime does the counting. No semaphore, lock or thread appears in Clarity code.

### A job is an action started in the background

No new kind of declaration. An action already runs without the screen (no `message`, `open`, `close` or parts) and declares its effects, which is exactly what may run beside the window. One new line on the action and one new statement:

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
- `at most N` on the action is the limit for every background run of it in the program, however many `start`s there are. That is the semaphore, declared: the fifth call waits until one of the four ends. Without the line, background runs of that action go one at a time (`at most 1`), so a program never runs code in parallel by accident.
- A direct call of the action from a handler stays as today (the window waits) and does not count against the limit.

### Results come back to the window, one at a time

- `on done: handler` runs on the window after each call ends, with the call's arguments and its result (`requestSent(request, ok)`), in the order calls end. `on failed: handler` runs with the arguments and the problem text when a call fails or refuses; the others go on. `on finished: handler` runs once when every call of that `start` has ended.
- These handlers are ordinary handlers: they run one at a time between other handlers, so they may touch parts and window data. This is C++ Builder's `Synchronize`, built in: background code never touches the window, and the window never sees a half-done call.
- The checker refuses what a background run cannot do: an action that touches the window is already an error, and `start` of something that is not an action is an error.

### Watching and stopping

- `start ... as sending` names the run, so code and parts can read it: `sending.total`, `sending.done`, `sending.failed`, `sending.running` (bool). A progress part binds to it (`progress bind sending.done max sending.total`).
- `stop sending` lets running calls end and starts no new ones; `on finished` still runs.
- A run belongs to the window that started it. Closing the window stops it as `stop` does; see open questions for runs that outlive their window.

### What runs in parallel: the smaller version

The user's decision: "put the smaller version in the plan". Today every runtime assumes one piece of code at a time: its database connection, settings, file stores and, in C and C++, its global state. Making all of that safe for threads, five times, would be most of the step. So the first version keeps that assumption inside the runtime:

- **One gate.** Everything that reaches the runtime's shared state goes through one gate, one call at a time: `find`, `save`, `delete` and queries, files of file stores, settings, and library functions (a project's library may not be safe for threads). The database keeps its single connection, as today.
- **Outside the gate, in parallel:** the waiting and the work that make jobs worth having: `http` calls, sending and reading mail, reading and writing formats, and Clarity's own computation on the call's own values. Up to `at most N` of these run at once.
- So in `sendRequest` above, four mails are on their way at the same time, and the four `save request` lines take turns.
- A save is one transaction, as today. Two calls that change the same record: the second save is refused by the version check of step 5 and becomes a failed call.
- Nothing in the language changes when a later version lets the database work in parallel too: the gate is the runtime's, not the program's.

### Every stack, every target

- Per runtime, like handlers are (not a shared library: it is the runtime's own scheduling): Rust threads with a counting gate, Go goroutines with a buffered channel, C# `Task` with `SemaphoreSlim`, C and C++ a small thread pool; and in each, the one gate (a single lock) at the runtime's entry points for data, files, settings and libraries. The shared libraries called outside the gate (clarity-formats, clarity-mail) are safe for threads: they keep their problem text per thread. One conformance file, `conformance/jobs.json`: the limit holds, `on done` per call, `on failed` without stopping the rest, `on finished` once, `stop`.
- Desktop: the window keeps working while jobs run. Served: a run belongs to the browser's window that started it, and the limit counts across the whole server, since what it protects (a mail server, an API's rate) is shared. CLI: a command waits for its runs to end before it exits.
- Designer: the action's `at most` in its properties; the handler wiring of `on done`, `on failed`, `on finished` as events are wired. Manual: a page `code.jobs.md`, registered in `crates/clarity-tools/src/manual.rs`; LANGUAGE.md; agent reference; i18n for Clarity's own words (the Stop caption of a run shown in a progress part).

Opens: sending many mails or requests, importing and converting files, downloads, thumbnails, calling a slow API for many records, any batch that should not freeze the window.

## The example

`examples/erasure`, `stack go desktop cli` (the review's suggestion), built once the steps it uses are done, as the acceptance test of all six:

- Person, Site, Account, Request entities; Status enum (sent, acknowledged, done, refused, noReply).
- The password-manager export as a `csv` source (step 1), imported into Account; JustDeleteMe JSON through `http get` (exists); privacy contact found by a local model through `http post` with `plainText` (step 1).
- `encrypted "setting:databaseKey"` on the database with `databaseKey: secret, forget` (2), one family member per sign-in on the desktop (4).
- Article 17 requests sent from each person's own address (3a), at most 4 at a time in the background with a progress bar (6), a save refused when another family member changed the record meanwhile (5), replies matched by `inReplyTo` (3b); the `mailto:` route with `encoded` stays for those without SMTP.
- CLI command `requests overdue` for cron.

## Order

1. Step 1, in the order and with the done lists of FORMATS-PLAN.md.
2. Step 2 (format choice first: a spike that opens one SQLCipher file from Rust, Go, C#, C and the IDE).
3. Step 3a, then 3b.
4. Step 4.
5. Step 5 (before jobs, which make the same record saved from two places more common).
6. Step 6.
7. The example.

Each step: designer properties and Settings rows, checks, manual pages, i18n keys, agent tools and reference entries, all ten example builds green (think-generic-builder, examples-before-done).

## Where the code lives

The user's decision: "shared Rust library, go with that".

Formats and mail are written once, in Rust, and every stack uses that one implementation, as audio, video and image are (and as HARDWARE-PLAN.md plans for devices). The pattern is Clarity.Image's (or Audio's). Not graph's: clarity-graph is a plain Rust crate linked into Clarity.UI, with no C interface.

| | Formats | Mail |
|---|---|---|
| Rust crate (used directly by the Rust stack) | `runtimes/rust-formats`, `clarity-formats` | `runtimes/rust-mail`, `clarity-mail` |
| C interface for C#, Go, C, C++ | `runtimes/rust-formats-ffi`, `clarity_formats.h`, `libclarity_formats` | `runtimes/rust-mail-ffi`, `clarity_mail.h`, `libclarity_mail` |
| C# NuGet, released with the C# runtime | `Clarity.Formats` | `Clarity.Mail` |
| Release tag (crates.io) | `runtimes/rust-formats/v*` | `runtimes/rust-mail/v*` |
| Native library | prebuilt per system beside Clarity.UI's by the release workflow, offline once fetched | the same |

- The shared library does the parsing and the protocols: text or bytes to rows of named values and back (csv, xml, yaml, later xlsx; JSON moves in too, so every format has one reader), and SMTP, IMAP, MIME and TLS. Rows cross the C interface as JSON, as the other libraries' calls do.
- Each runtime keeps only what is its own: fitting rows to its records with the shared matching rules, a file source's find/save over the rows, and its secret settings. That part stays per runtime with a conformance file, as JSON reading is today.
- A library `format` (FORMATS-PLAN.md, later step) stays in the project's own language, as every library does (custom-parts-are-libraries); its adapter hands rows to the runtime the same way the shared library does.
- SQLite encryption (step 2) is not in a shared library: it lives in each runtime's own SQLite. File and file-store encryption uses the vault's scheme, so it can be one shared crate too (`clarity-crypt`, or part of `clarity-formats`); to decide in step 2.
- Sign-in (step 4) lives in each stack's web server, which is per runtime; only Argon2id hashing could come from the shared library.
- The generator adds a library to a program only when the project uses it; for Rust a feature on clarity-runtime (as `pictures = ["dep:clarity-image"]`).
- What a new C-interface library touches, from Clarity.Image and Clarity.Video:
  - `runtimes/rust-X` and `runtimes/rust-X-ffi` with `include/clarity_X.h` and a table function `clarity_X_api()`;
  - `release-rust-X.yml`; the -ffi build and header copy in `release-clarity-ui.yml`, `checks.yml` (library job), `.github/scripts/stage-clarity-ui.sh` and `release-csharp-runtime.yml`;
  - `crates/clarity-gen`: `BASE_LIBRARIES` in `src/lib.rs`, the header lookup in `src/clarity_ui.rs`, the wiring in `src/{go,c,cpp,csharp,rust}/mod.rs`;
  - `runtimes/c/clarity.cmake` and `runtimes/cpp/clarity.cmake`;
  - the NuGet project `runtimes/csharp/Clarity.X`, added to `Clarity.slnx`;
  - the `clarity libraries` command.
- Per-stack package lists in steps 1 and 3 above (Go `encoding/csv`, MailKit, libcurl and so on) are replaced by this: Rust crates only (`csv`, `quick-xml`, a YAML crate, `lettre`, `imap`, `mail-parser`, `rustls`), so the open questions about C and C++ libraries go away.

## Overlap with what exists

Checked 2026-11-10 against the language and the other plans:

| Proposed | Existing construct | How the plan avoids a duplicate |
|---|---|---|
| Formats catalog | `read ... from json`, `json of` | JSON becomes the catalog's first entry; one fitting code path |
| File data sources | `sqlite` kind, `external`, `table`/`column`/`readonly`, Import panel | Kinds per format beside `sqlite`; always external; same mapping words and importer |
| (dropped) kind `file` | `file` field type, `files f in store`, `filestore` | Not used |
| Encryption key | secret settings and the unconnected window | Key is a secret setting; one new option `forget` on secrets instead of `encrypted ask` |
| File encryption | the vault's Argon2id + AES-256-GCM | Same crate and scheme |
| Encrypted backups | BACKUP-PLAN.md (built) | Extends it |
| `tcp`/`mqtt`/`websocket` | HARDWARE-PLAN.md transports | Left to that plan |
| (dropped) `password` type | `password` part, `secret` setting type | `hashed` modifier on a text field |
| `mails m in inbox` | `files f in store` | Same shape on purpose |
| Library `format` | library functions, PARTS-LIBRARY-PLAN.md packages | An extra line in the same declaration; ships as any library |
| Version check | undo's `undoChangedSince`, refused saves, external `key` mapping, migrations | Undo uses the same version; a refusal is a refused save; `version "Column"` maps like `key`; the column comes by an ordinary proposed step |
| Background jobs | actions (no screen, declared effects), `uses progress` long calls, timers, engines beside the screen | A job is an action; one new line (`at most N`) and one statement (`start`); results come back as ordinary handlers |
| PDF (REPORT-PLAN.md) | not a format here | A report renders records; a format exchanges them. Kept apart |

## Open questions

- Background runs that outlive their window (a run started from a dialog that the person closes), or a run started by the app itself (`on start`) or on a schedule (`every day at 08:00`), which would also answer the scheduled-work question below.
- Whether `on done` handlers come in the order calls end (proposed) or in the order of the list.
- Web API sources with ETags: `If-Match` in step 5, or later.
- Whether an entity of an owned source may turn the version check off, or it is always on there.
- Later: database work in parallel too (a connection per call), and library functions outside the gate when a library declares it is safe for threads.
- Where this sits in the agreed roadmap (MCP 3-5, pdf, questionnaire, reports, debugger), and whether to start with step 1 only.
- Which formats are built in beyond json and csv: xml and yaml as proposed, and whether toml and ini join them, or stay library formats.
- Spreadsheets (xlsx, ods): built into `clarity-formats` (Rust `calamine` to read, `rust_xlsxwriter` to write) or left to a library format. A sheet is a `table` of a file source either way.
- Writing a file source back: keep the file's own column order and the columns no entity maps (proposed), or write only mapped columns.
- Go: modernc.org/sqlite is pure Go and has no encryption. Options: cgo with SQLite3 Multiple Ciphers (loses pure Go and complicates cross builds), or ncruces/go-sqlite3 (its own encryption formats, which the IDE would then need to read). To decide after the spike.
- C#: SQLitePCLRaw's `bundle_e_sqlcipher` is community-built; Zetetic's official SQLCipher for .NET is paid. Or ship SQLite3 Multiple Ciphers as the native SQLite of Clarity.Runtime.Sqlite.
- Mail login with OAuth (Gmail, Outlook) now or later; Proton through Bridge needs only `basic`.
- Sign-in methods beyond a password (passkeys, a link by mail once step 3 exists, an outside identity provider).
- Scheduled work inside a served application (`every day at 08:00 run overdueCheck`) versus the CLI target plus cron, which works today.
