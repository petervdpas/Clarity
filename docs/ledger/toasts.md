# Toasts

Status: built. A toast is a declaration of a module (`toast saved` with `text @key`, `lifespan` in milliseconds, `level`, `icon`, `corner`); code shows one by name with its text's slots filled (`toast saved(name = x)`). Each toast has its own lifespan, level, icon and corner, so one app shows `saved` for 3 seconds and `imported` for 8. Plain info messages and agents' changes show as toasts too.

Progress: 100%

## Code
- Spec: `ToastDecl`, `ToastLevel`, `ToastCorner`, `DEFAULT_TOAST_LIFESPAN` (5000), `TOAST_LIFESPANS` (500 to 120000) and `StmtKind::Toast` in `crates/clarity-spec/src/code/mod.rs`; `parse_toast` and the statement in `code/parse.rs`; `SpecFile.toasts`, `Project::toast` (`model.rs`); checks `check_toasts` (`check.rs`: name once, a text, the lifespan, the icon) and the statement (`code/check.rs`: a toast by that name, its slots' values); its text key and slots in `translations.rs` (a use where code shows it, `declared` where it is declared); icons in `icons::used`; lines set by key (`decl_lines.rs`: text, lifespan, level, icon, corner); `Kind::Toast` for Remove (`edit.rs`); words in `highlight.rs`
- Generators: one place, `crates/clarity-gen/src/toasts.rs` (the look and the text); each stack's call: Rust `ui.toast(clarity_runtime::ui::Toast { .. }, text)`, Go `ui.Toast(clarity.Toast{..}, text)`, C `clarity_ui_toast(ui, ...)`, C++ `ui.toast(clarity::Toast{..}, text)`, C# `ShowToast(new ToastLook(..), text)`; in a command, the text on standard error
- Runtimes: a message's `toast` on the wire (`{"name", "level", "lifespan", "icon", "corner"}`): Rust `ui::Toast` (`runtimes/rust/src/ui.rs`), Go `clarity.Toast` (`ui.go`), C `ClarityToast` (`ui.h`, `ui.c`), C++ `clarity::Toast` (`ui.hpp`, `ui.cpp`), C# `ToastLook` (`Messages.cs`, `WireJson.cs`, `WindowViewModel.ShowToast`)
- Clarity.UI: `runtimes/rust-ui/src/window/messages.rs` `ToastList` (each corner stacks its toasts, the level's theme color on the edge, the icon before the text, one wake at the first to go; `places`, `colors`, `icons` for tests); `window/mod.rs` `take` (a declared toast keeps its own lifespan whatever its level); info toasts 5000 ms (`INFO_DURATION`), agents' 10000 ms (`AGENT_TOAST_DURATION`)
- IDE: the tree's Toasts with + New toast (`app/src/lib/ProjectTree.svelte`, `App.svelte` `startNewToast`, `deleteToast`); `lib/ToastView.svelte` (text key and words, lifespan, level, icon, corner, a sample, Delete); `lib/toastLines.ts`
- Docs: manual `code.toasts.md`, `code.statements.md` (`toast`), `code.handlers.md`

## Tests
- spec `tests/toasts.rs`, `tests/decl_lines.rs` (a toast's lines; added and taken away)
- runtimes: Rust `ui::tests::a_toast_travels_with_its_level_lifespan_icon_and_corner`; Go `clarity/toast_test.go`; C `a_toast_travels_with_its_look` (`tests/values_test.c`); C++ `a_toast_travels_with_its_look` (`tests/ui_test.cpp`); C# `ToastTests` (`Clarity.Runtime.Tests/ToastTests.cs`)
- rust-ui `tests/windows/window_stack.rs` `a_declared_toast_shows_in_its_corner_for_its_lifespan`, `tests/hosts/agents.rs` `an_agent_s_toasts_go_by_themselves`
- app `src/lib/toastLines.test.ts`
- gen: the every-part fixture shows two toasts (`tests/fixtures/everything/everything.clarity`, a handler and a command); `everything_build` on all five stacks, without a warning
- examples: maintenance declares `entrySaved` (success, 3 s, bottom right), `reviewSaved` and `pickAnEntry` (warning, 4 s) and shows no plain message any more

## Open
- An app-wide default lifespan for plain messages: NOT BUILT (not asked; plain messages keep 5000 ms).

## Conflicts
- none

## Decisions
- 2026-11-13: a toast is a definition. User: "toasts must get a lifetime span!", "toasts should get a lifespan in milliseconds", "withing an app it might differ!", "NO, NOT EVERY TOAST!!!! A TOAST IS A DEFINITION, AND YOU CAN HAVE TOAST A AND TOAST B... AND A gets 3s AND B gets 8s", then "yes, and also level and icon!!! AND CORNER POSITION!". So a module declares each toast with its text, lifespan (milliseconds), level, icon and corner, and code shows one by name.
- 2026-11-13: they are toasts, never notes. User: "calling toasts notes is a bad, bad, bad idea!!!!" (Clarity.UI's `NoteList` became `ToastList`; see [app-mcp](app-mcp.md)).

## Was
- none (new piece)
