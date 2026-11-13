# Windows, dialogs, splash, icon and version

Status: built. The window stack (one desktop window, windows opened in place), Clarity's own message and ask dialogs, the splash page, the app's icon and its version.

Progress: 90%

## Code
- Spec: `crates/clarity-spec/src/model.rs:512` (page kinds window, dialog, view, splash, toolbox, signin), `parser.rs:963` (page kind), app `main`, `splash ... for N seconds` (0.1 to 30, `parser.rs:1330-1355`), `layout.rs` (window 1100 x 760, min 200 x 150, max 3840 x 2160; splash 480 x 280); `code/mod.rs:429-437` (`app.version`, `{app.version}`); asks in `code/check.rs`
- Window room: a window holds the rows its height leaves under 12 px of padding above and below and its bars (menu 30, toolbar 38, ribbon 30 plus 92 or 30 folded, footer 28 or a row plus 8; a dialog its own bars only): `crates/clarity-spec/src/layout.rs` (`Bar`, `frame_bars` :135, `rows_held` :154); every design edit fits the design in the same write and Undo (`fit.rs`: `fit_window` cuts a part to end by the last row held and a slide-out to that many rows, `fit_view` a view's parts to the rows from its content area's top; the window keeps its size), run by `crates/clarity-tools/src/edits.rs:165` for the designer and every agent's tools; the checker (`slideout_problems`, `crates/clarity-project/src/lib.rs:214`) warns that a window or view reaching further scrolls, and refuses a slide-out taller than its window; Clarity.UI's slide-out takes no room in the window (`parts/containers/slideout.rs` `takes_room`)
- Project file: `icon "app.png"` (`crates/clarity-project/src/file.rs:312`), `version "1.2.0"` (:305); warning for an unnamed `icon.png` (`crates/clarity-project/src/lib.rs:1251`)
- Generators: splash on the app markup `<App splash=".." splashFor="..">` (`crates/clarity-gen/src/web_windows.rs:113-118`); version stamped into Cargo.toml, csproj `<Version>`, CMake `project(VERSION)` (`versioned`, `crates/clarity-gen/src/lib.rs:1698`); favicon (`web_browser.rs:43`); Windows icon and VERSIONINFO (`windows_resources.rs`): see [generation](generation.md)
- Runtimes: ask R `runtimes/rust/src/ui.rs:914` (`fn ask`; `Ask` :434); Go `runtimes/go/clarity/ask.go`; C `runtimes/c/clarity/ui.h`; C++ `runtimes/cpp/clarity/ui.hpp`; C# `Clarity.Runtime/Questions.cs`, `Messages.cs`, `OpenWindows.cs`. Splash status: R `splash.rs` (wire `splash_status`, `wire.rs:195`), Go `clarity/splash.go`, C `clarity/splash.h`, C++ `clarity/splash.hpp`, C# `Splash.cs`
- Clarity.UI: `runtimes/rust-ui/src/window/mod.rs` (`WindowStack` :223, `open` :713 with the own desktop window or modal overlay choice :716, `ended` :598 when the last window closes), `window/history.rs` (Back, Forward), `window/view.rs`, `window/messages.rs` (info note that closes itself; warning and error with OK), `dialogs/ask.rs`, `dialogs/long_call.rs` (bar with Stop), `dialogs/over.rs`, `dialogs/grant.rs` (an agent's change, lines that scroll: see [app-mcp](app-mcp.md)), `splash.rs`, `desktop/mod.rs:98-213` (app icon; Clarity's own `desktop/icon.png` when none)
- IDE: `app/src/lib/NewWindowDialog.svelte:91` (Splash screen template), `ProjectSettings.svelte:261` (Icon), version field with `app/src/lib/version.ts`; renaming a window or view is [ide-designer](ide-designer.md); the IDE's own dialogs are [ide-shell](ide-shell.md)
- Docs: manual `design.windows.md` (Kinds, Splash screen, Size; also what a window's height holds), `code.handlers.md` (Asking the person)

## Tests
- `conformance/windows.json`: run by rust-ui only
- `conformance/window-room.json`: run by the spec (`tests/window_room.rs`) and rust-ui (`tests/windows/window_room.rs`); spec `fit.rs`; rust-ui `tests/windows/slideouts.rs` (`a_tall_slide_out_gives_the_window_nothing_to_scroll`), `tests/parts/parts_content.rs` (`a_view_as_high_as_its_content_fits_in_it`)
- rust-ui `tests/windows/window_stack.rs`, `core_windows.rs`, `windows_life.rs`, `history.rs`, `core_return.rs`, `core_dialogs.rs`, `dialogs_ask.rs`, `dialogs_long_call.rs`, `dialogs_windows.rs`, `looks_escape_dialog.rs`, `splash.rs`
- spec `asks.rs`, `new_windows.rs`, `splash.rs`; gen `asks.rs`, `splash.rs`, `windows_resources.rs`, `icons.rs`; project `icon.rs`, `version.rs`; IDE `app/src/lib/version.test.ts`

## Open
- The app's icon in the designer and preview: NOT BUILT (from the removed icon and splash plan, step 1).
- macOS app bundle (Info.plist, .icns): NOT BUILT; see [os-checks](os-checks.md).
- `progress n of total` from handlers and actions: NOT BUILT (DESIGN.md:809, 830, 2528); see [language](language.md).
- `conformance/windows.json` run by the runtimes: NOT BUILT (only rust-ui loads it).

## Conflicts
- Manual project.file.md:26-38 leaves out `version` and `icon`.

## Decisions
- 2026-10-17: one desktop window per app on every stack; `open` shows a window in place, a new desktop window only for a dialog or `in new window`. User: "only open new windows if explicitly set!", "for all language/framework that the rule"
- 2026-10-22: never a dialog of the system or the browser; Clarity's own, in the IDE and in generated apps. User: "no js dialogs please (is rule!!!)", "that needs to land in all stacks as well"
- 2026-10-22: a handler asks and finishes; the answer runs another handler (`ask ... then h`); cancel runs nothing; only handlers ask.
- 2026-10-22: `message info` stays a note that fades; `warning` and `error` show a dialog with OK.
- 2026-10-22: a dialog window gives a value: `page X: dialog gives T`, `close with value`.
- 2026-11-06: the icon is declared (`icon "app.png"`), never found by a file's name; one picture for window, taskbar, favicon and Windows program file.
- 2026-11-06: the splash is a designed page shown while `on start` runs, closed when the main window shows but not before its time; the splash window file declares its window as Avalonia's markup does (`decorations="none"`, `startup="centerScreen"`).
- 2026-11-06: the version is optional; without it nothing shows one; `app.version` used without one is an error.
- 2026-11-13: window room. User: "slideouts should not be allowed to have more height than the content/view/window they are in!"; the checker counting the frame bars, user: "yes, in this release"; then "that warning points to design time... design must force automatic correction!!!", chose "Fit the parts (Recommended)", and for a window that still reaches further "Warning (Recommended)". Recorded first in [app-mcp](app-mcp.md).

## Was
- DESIGN 5.5, 5.6 (page kinds, one desktop window), 5.14
- Roadmap: U1d (dialogs that give a value), U1f
- APP-ICON-SPLASH-PLAN.md steps 1-3; VERSION-PLAN.md steps 1-4 (step 2 and 3 build side: [generation](generation.md))
- memory: one-window.md, no-native-dialogs.md, app-icon-splash-plan.md, version-plan.md
