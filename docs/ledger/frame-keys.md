# Frame and keys

Status: partial. The app's and a window's frame (menu, toolbar, ribbon, sidebars, footer), its commands, shortcut keys per layer, and the keys window where people set their own keys.

Progress: 80%

## Code
- Spec: `crates/clarity-spec/src/frame.rs` (regions :16, :54-96; `BUILT_IN_COMMANDS` quit, close, language, theme, keys, undo, redo, back, forward :187; `BUTTON_COMMANDS` :191; sign-in and sign-out targets :195), `shortcuts.rs` (`catalog`: commands per place), `check.rs` (`check_page_keys` :673, `frame_keys` :2789; a frame on a non-window page :467)
- Project file: `keys editable` (`crates/clarity-project/src/file.rs:351`)
- Generators: frames written into the app and window markup (`crates/clarity-gen/src/web_windows.rs`); catalog `runtimes/rust-ui/catalog/command-*.json`, `sidebar-*.json`
- Runtimes: key rules and the person's keys R `runtimes/rust/src/keys.rs` (used by Clarity.UI for every stack); C# `Clarity.Runtime/KeyRules.cs`, `PersonKeys.cs`; ribbon and sidebar state in each runtime's part state
- Clarity.UI: `runtimes/rust-ui/src/frame/` (`menu_bar.rs` with Alt letters and F10, `menus.rs`, `toolbar.rs`, `ribbon.rs`, `sidebar.rs`, `footer.rs`, `buttons.rs`, `rules.rs`, `spec.rs`), `shortcuts.rs`, `dialogs/keys.rs` (keys window), person's keys kept through the program's settings (`window/mod.rs:103-109`)
- Footer: its own part is the status item, see [parts-inputs](parts-inputs.md); a footer with parts is a row high and 8 more (`layout.rs` `FOOTER_PARTS_BEYOND_ROW`, `STATUS_HEIGHT` for a status line alone; see [windows-dialogs](windows-dialogs.md))
- IDE: `app/src/lib/FrameProperties.svelte` (also a part's context menu, Add item and Add separator; see [by-hand](by-hand.md)); the IDE's own keys are [ide-shell](ide-shell.md)
- Docs: manual `design.frame.md` (Regions, Commands, Built-in click targets, Keys), `app.keys.md` (In your application)

## Tests
- `conformance/shortcut-keys.json`: run by R runtime (`runtimes/rust/tests/keys.rs`), C# (`Clarity.Runtime.Tests/ShortcutKeyCaseTests.cs`), the IDE (`app/src/lib/shortcutKeys.test.ts`); skipped by Go, C, C++ and `runtimes/web/src/host/keys.ts`'s tests
- `conformance/ribbons.json`, `sidebars.json`: run by R, Go, C, C++, rust-ui; skipped by C#; `sidebars.json` also by `runtimes/web` (`src/frame/sidebars.test.ts`)
- spec `frame.rs`, `frame_edit.rs`, `ribbons.rs`, `sidebars.rs`, `menus.rs`, `commands.rs`, `new_windows.rs`, `shortcut_keys.rs`; gen `ribbons.rs`, `sidebars.rs`, `shortcut_keys.rs`
- rust-ui `tests/windows/core_frame.rs`, `core_frame_regions.rs`, `frame_keyboard.rs`, `frame_state.rs`, `dialogs_keys.rs`, `looks_menu_access.rs`; `tests/screen/shortcuts.rs`

## Open
- Ctrl means Cmd on macOS, in matching and in what is shown: NOT BUILT (DESIGN.md:1504; `runtimes/rust-ui/src/shortcuts.rs:135-138` matches Cmd only as Meta).
- A view that brings its own ribbon tab and menu items: NOT BUILT (DESIGN.md:1502, 1506). A frame on a non-window page is an error (`crates/clarity-spec/src/check.rs:467`).
- AltGr on a European layout not running commands: UNTRIED (WINDOWS-TESTS.md:51).

## Conflicts
- The person's keys in C# have two writers: Clarity.UI keeps them through the program's settings (`runtimes/rust-ui/src/window/mod.rs:103-109`), and the C# program writes `keys` from `PersonKeys`, read once at start (`crates/clarity-gen/src/csharp/model.rs:287`, `runtimes/csharp/Clarity.Runtime/PersonKeys.cs:25`). A save after a key change may write the old keys back.
- Key rules written three times: `runtimes/rust/src/keys.rs:99`, `runtimes/csharp/Clarity.Runtime/KeyRules.cs:79` (no callers outside tests), `runtimes/web/src/host/keys.ts:151`. C# counts UTF-16 units for one-letter keys, Rust chars (keys.rs:107).

## Decisions
- 2026-10-25: keys belong to layers, the innermost first: the focused part, the view, the window's frame, the app's frame; a key is unique within one layer, an inner layer may reuse an outer's.
- 2026-10-25: ribbon and toolbar buttons get `key`; a ribbon button's key works whichever tab is open.
- 2026-10-25: whether people set their keys is a project setting (`keys editable`); kept in the settings file, only what differs from the design. User: "shortcuts on/off is project setting"
- 2026-10-25: keys the system and inputs need (typing keys, Tab, Enter, Escape) are refused; a key taken in the same layer offers a swap.
- 2026-10-25: a view may bring its own ribbon tab and menu items (decided, not built).

## Was
- DESIGN 5.6 (Frame regions), 5.28
- Roadmap: S3c, U1a (menu item keys)
- WINDOWS-TESTS.md section 5; section 6
