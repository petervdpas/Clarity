# Text, input and picture parts

Status: built. The parts that show text (label, help, heading, text, link, badge, progress, busy, status item, icon, separator), take input (textbox, password, memo, number, spinbox, slider, checkbox, switch, toggle, dropdown, radio, date, color, file, folder, tags, button, buttons) and show a picture or number (image, display).

Progress: 90%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (catalog entries, events per kind :162-220, `CODE_LANGUAGES` :596), `colors.rs`, `icons.rs` (Lucide 1.50.0 vendored in `crates/clarity-spec/icons`)
- Generators: parts written into window markup (`crates/clarity-gen/src/web_windows.rs`), form labels and undo scopes filled in; part properties code reads in each stack's logic.rs
- Runtimes: ranges R `runtimes/rust/src/ranges.rs`, Go `clarity/ranges.go`, C `clarity/ranges.c`, C++ `clarity/ranges.cpp`, C# `Ranges.cs`; dropdown options `dropdown_options.*` in R, Go, C, C++, C# `DropdownOptions.cs`; list fields `list_fields.*` in all five (C# `ListFields.cs`); spin range from code (`spin_min`, `SpinMin`, `clarity_ui_spin_min`, `PartState.SpinMin`)
- Clarity.UI: `runtimes/rust-ui/src/parts/` `label.rs`, `heading.rs`, `button.rs`, `text_box.rs` (text, password with eye and copy), `text_editor.rs` (memo), `code_colors.rs` (19 languages), `check_box.rs`, `inputs/` (`number.rs`, `slider.rs`, `toggle.rs`, `dropdown.rs`, `typed_dropdown.rs`, `choice_list.rs`, `radio.rs`, `date.rs`, `calendar.rs`, `color.rs`, `file.rs`, `folder.rs`, `tags.rs`, `link.rs`, `picture_field.rs`, `formats.rs`), `containers/` (`small.rs` help, badge, progress, busy; `text.rs`, `icon.rs`, `separator.rs`, `buttons.rs`, `image.rs`, `display.rs`); system pickers `desktop/files/` (portal, osascript, Windows COM)
- Status item (2026-11-13): spec `widgets.rs` (`StatusItem`), `check.rs` (max), `code/check.rs` (caption, checked, error set by code; a bound one keeps its own check mark); Clarity.UI `parts/containers/small.rs` (`StatusItemPart`), `access/mod.rs`; C# window state `csharp/windows.rs`; the every-part fixture's `copiesItem`
- IDE: `app/src/lib/PageProperties.svelte` (Placeholder, Code, Copy button, Undo, ranges), `PartTile.svelte`, `IconField.svelte`
- Docs: manual `design.parts.md` (Text, Basic input, Graphics); LANGUAGE.md section 6 (The catalog, Binding, Labels help texts and forms, Input and button properties), section 8 (Part properties)

## Tests
- `conformance/forms.json`, `small-parts.json`: run by rust-ui only
- `conformance/ranges.json`, `dropdown-options.json`: run by R, Go, C, C++, rust-ui (ranges also `runtimes/web`); skipped by C# (`Ranges.cs`, `DropdownOptions.cs` name them in a comment, no test loads them)
- `conformance/list-fields.json`: run by all five, rust-ui, `runtimes/web`
- `conformance/colors.json`, `code-languages.json`: run by rust-ui and the spec; `display.json`: rust-ui and `runtimes/web`
- spec `status_item.rs`; rust-ui `tests/parts/parts_status_item.rs` (caption, gauge, check mark, warning and tooltip, click)
- spec `small_parts.rs`, `button_kinds.rs`, `ranges.rs`, `spin_range.rs`, `dropdown_options.rs`, `list_fields.rs`, `value_lists.rs`, `pickers.rs`, `form_labels.rs`, `file_fields.rs`, `folders.rs`, `colors.rs`, `clipboard.rs`, `display.rs`, `icons.rs`
- gen `small_parts.rs`, `button_kinds.rs`, `ranges.rs`, `spin_range.rs` (all five), `dropdown_options.rs`, `list_fields.rs`, `colors.rs`, `file_fields.rs`, `folders.rs`, `clipboard.rs`, `icons.rs`, `text_align.rs`
- rust-ui `tests/inputs/*` (22 files), `tests/parts/text_box.rs`, `text_editor.rs`, `code_colors.rs`, `check_box.rs`, `parts_button.rs`, `parts_label.rs`, `core_help.rs`, `core_radio_label.rs`, `icon.rs`, `parts_display*.rs`, `parts_small_*.rs`

## Open
- Dropdown search on the labels people see; accent-blind sorting and search: NOT BUILT (DESIGN.md:300; `runtimes/rust/src/dropdown_options.rs:26-38` lowercases the stored value only).
- Dates and numbers in the system's locale, chosen apart from the language: PARTIAL. Formats follow the shown language, 8 locales, others en-US (`runtimes/rust-ui/src/parts/inputs/formats.rs:303-318`).
- The file part runs no change handler on open and save a copy: PARTIAL (`inputs/file.rs:1-9`).
- The folder part has no picker in a browser: PARTIAL; it says a server path is typed (`inputs/folder.rs:8`, word `typeServerFolder`).
- Unbound input parts as warnings: NOT BUILT (DESIGN.md:489; catalog flag `widgets.rs:540`, no check reads it).
- A `time` field and part: NOT BUILT (DESIGN.md:2474).
- Conformance cases for `enter`, `leave` and pictures in forms: NOT BUILT (DESIGN.md:277).
- The folder picker on Windows and macOS: UNTRIED (WINDOWS-TESTS.md:139).
- The status item's gauge stands at the item's right end, not right after its caption: the caption's width is not known when the places are laid out (`StatusItemPart::places`). PARTIAL by choice; settle if it looks wrong in a wide item.

## Conflicts
- Decision needed: CLAUDE.md:19 says no dialog of the system. Generated apps open the system's file and folder pickers (`runtimes/rust-ui/src/desktop/files/mod.rs`), and LANGUAGE.md:675, 994, DESIGN.md:1843, manual design.parts.md:26 describe that as intended.
- Code-language list twice: `crates/clarity-spec/src/widgets.rs:596`, `runtimes/rust-ui/src/parts/code_colors.rs:9`; equal.
- Display formats three times: `runtimes/rust-ui/src/parts/inputs/formats.rs`, `runtimes/web/src/screen/format.ts`, `runtimes/csharp/Clarity.Runtime/Formats.cs`; equal on the cases.
- C# `Ranges.cs` and `Palette.cs` have no callers outside tests.
- LANGUAGE.md catalog: memo has `link` and `toolbox` (:982); number takes `duration` (:986); password takes `secret` (:983).

## Decisions
- 2026-10-19: every input tells a change only when the person changed it to a value that differs, never for what code sets.
- 2026-10-23: parts that are an option of another become that option: `typed` and `search` on the dropdown, not a combo box or lookup part; a search bar is a text box and a handler.
- 2026-10-23: `typed` offers the values the field already has; `search` filters the records loaded, showing the `show` fields.
- 2026-10-23: the color input uses Clarity's own palette popup, never the system's color dialog; colors kept as `#rrggbb`.
- 2026-10-23: the file part's copy goes to Downloads without a dialog.
- 2026-10-25: a password input has an eye and a copy button. User: "passwords, secrets and alike should have an option (open eye/closed eye) to show it or copy it"
- 2026-11-05: every input that can be empty takes `placeholder`; `memo ... code <lang>` colors code in the theme's code roles; `copy` gives a text box or memo a copy button.
- 2026-11-08: a spin box's `min` and `max` are read and set by code, kept in part state.
- 2026-11-13: a part for footer widgets. User: "I think there needs to come a part for footer widgets?", then "yes, make the part". It is `statusItem`: icon, caption, a gauge of the number it binds, `checked` as a check mark and `error` as a warning with its text as tooltip, all set by code through what every stack already sets (caption, checked, error), so the generators need no new member.

## Was
- DESIGN 5.6 (Names and events, Forms labels and help texts), 5.27 (showing and copying a secret), 5.34, 5.36
- Roadmap: U1a, U1b (icons, small parts, toggle, button group, split button), U1c (slider, spinbox, color, file, tags, typed and search dropdowns)
