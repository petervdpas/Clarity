# Row versions and undo of saves

Status: partial. A save never overwrites someone else's change: each record carries a version, a stale save or delete is refused with `savedSince`; undo of saves uses the same version, one transaction per handler.

## Code
- Spec: `crates/clarity-spec/src/versions.rs` (which entities carry a version, the checks); entity lines `version "Col"`, `version none`, `savedSince @key` (`parser.rs:508-596`); `save x anyway` (`code/parse.rs:649-651`), `reload x`; migration op `AddVersion` (`migrate.rs`); Clarity's words `savedSince`, `undoTooOld` (`translations.rs:218-219`)
- Generators: version per stored entity in each stack's `model.rs`; tests `crates/clarity-gen/tests/versions_{rust,go,c,cpp,csharp}.rs`
- Runtimes: R `runtimes/rust/src/lib.rs` (checked UPDATE/DELETE, `UNDO_STEPS` :224), `edits.rs`, `file_source.rs` (file changed on disk); Go `runtimes/go/clarity/db.go`, `record.go`, `undo.go`, `edits.go`; C `runtimes/c/clarity/db.c`, `edits.c`; C++ `runtimes/cpp/clarity/db.cpp`, `edits.cpp`; C# `Clarity.Runtime/Db.cs`, `Record.cs`, `Undo.cs`, `EditBus.cs`
- Clarity.UI: `rowVersion` carried on the wire with the record (`runtimes/rust-ui/tests` `inputs_record_keys`)
- IDE: version line in `app/src/lib/EntityEditor.svelte:229-231` (`entity.keptVersion`, not editable)
- Docs: manual `data.entities.md`, `code.handlers.md`, `data.migrations.md`, `data.sources.md`; LANGUAGE.md section 4 "External databases" (`version`), "Migrations" (`add version`), section 8 "Undo" (:1904)

## Tests
- `conformance/versions.json`: run by R (`runtimes/rust/tests/versions.rs`), Go (`versions_test.go`), C (`versions_test.c`), C++ (`versions_test.cpp`), C# (`VersionsTests.cs`, `FileSourceTests.cs`)
- `conformance/edit-bus.json`: run by R, Go, C, C++, C#
- `crates/clarity-spec/tests/versions.rs`, `undo.rs`; per-runtime undo tests (`runtimes/rust/tests/edits.rs`, Go `undo_test.go`, C# `UndoTests.cs`)

## Open
- The IDE's data browser saves through the version check: NOT BUILT (no data browser; APP-CAPABILITIES-PLAN.md:228, :247).
- ETag and `If-Match` on web API writes: NOT BUILT (APP-CAPABILITIES-PLAN.md:240, open question :399).
- Other windows reload a record saved elsewhere: NOT BUILT (APP-CAPABILITIES-PLAN.md "Later: other screens follow").
- Whether an owned entity may turn the check off: open question (APP-CAPABILITIES-PLAN.md:400).
- Shared Rust core for row versions and undo: NOT BUILT; five copies (MODULAR-PLAN.md order 2).
- A background save refused by the version is a failed call: waits on [background-jobs](background-jobs.md).

## Conflicts
- Edit bus size: R, Go, C, C++ keep 500 edits per window (`runtimes/rust/src/edits.rs:37`, `go/clarity/edits.go:40`, `c/clarity/edits.c:44`, `cpp/clarity/edits.cpp:15`); C# drops none (`runtimes/csharp/Clarity.Runtime/EditBus.cs:29-35`). `conformance/edit-bus.json` has no limit case.
- The 100-step undo cap is written in each runtime (CONFLICTING.md 4; equal today).
- DESIGN 5.29 describes the screen side as the web renderer and Avalonia view models; both are gone, Clarity.UI draws every stack.
- APP-CAPABILITIES-PLAN.md:3 says steps 4 and 5 are unreleased; they are before tag v0.40.0.

## Decisions
- 2026-10-30: undo is part of the language on every stack; every part the person edits has undo unless it says `no undo`; history lasts until the program closes (DESIGN 5.29).
- 2026-10-31: the edit record is kept by each runtime, never by the screen. User: "ctrl+z over a messagebus and processed stack ... should message back the previous state ... a track record of the thing that's changed and what's changed"
- 2026-11-10: a version column and checked UPDATE/DELETE as step 5. User: "yes, add the version check as a step"
- 2026-11-10: one record is one unit, no field merging; overwriting on purpose is `save x anyway`, never guessed (APP-CAPABILITIES-PLAN.md step 5).
- 2026-11-10: external databases name their column (`version "Col"`) or say `version none`; without either a warning. File sources: the file is the unit.
- 2026-11-10: who changed is the project's own `before save`, no built-in `changed_by`; `savedSince @key` may use `{current.field}` slots.
- 2026-11-11: a handler is one transaction per source; Ctrl+Z on a multi-step edit is all or none; 100 steps kept, older refused with `undoTooOld`. User chose "all 3" after "check how stable ctrl+z / undo is ... (transaction log and rollback?)"
- Undo compares the version where an entity has one, the whole row where it has none (APP-CAPABILITIES-PLAN.md "Undo, made one with the version check").

## Was
- APP-CAPABILITIES-PLAN.md step 5 (all subsections, "Undo, made one with the version check", "Later: other screens follow"), "Steps 4, 5 and 6 as one slice" ("One notion of changed since", "Who changed")
- DESIGN 5.29 (database side and edit bus; the parts' undo scope belongs with [clarity-ui](clarity-ui.md))
- MODULAR-PLAN.md order 2 (row versions and undo)
- IMPLEMENTED.md 6 "Row versions, undo, backups" (versions, undo)
- UNIMPLEMENTED.md 2 "Row versions"
- CONFLICTING.md 2 (edit bus size), 4 (undo 100 steps), 9 (released or not)
- memory: row-versions-runtime.md, runtime-holds-it.md
