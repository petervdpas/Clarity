# Themes

Status: built. A theme is a set of named values (colors per light and dark, fonts, sizes, radii, part looks per state, styles) lying over Clarity's own theme; people switch between the themes a project offers.

Progress: 100%

## Code
- Spec: `crates/clarity-spec/src/themes.rs` (`ROLES`, 29 color roles :33; `built_in` :92; `resolve` :541 for light or dark; `use_themes` :444; `starter_from` :644 with System, Light, Dark, Cream and green), Clarity's own theme `crates/clarity-spec/themes/clarity.clarity`, `colors.rs` (palette)
- Project file: `theme A B` (offered, first starts; `crates/clarity-project/src/file.rs:393`)
- Generators: resolved themes per mode in `files.json` (`crates/clarity-gen/src/web_browser.rs`; also `rust/mod.rs`, `go/mod.rs`, `cpp/mod.rs`); C# `crates/clarity-gen/src/csharp/themes.rs` (`Themes/<theme>.light.json`, `.dark.json`); the designer's CSS `web::theme_css`
- Runtimes: C# `Clarity.Runtime/Themes.cs`, `OfferedThemes.cs`; `settings.theme` in each runtime's settings (see [settings-secrets](settings-secrets.md))
- Clarity.UI: `runtimes/rust-ui/src/theme.rs` (a project theme over the base :91), `themes/clarity.light.json`, `clarity.dark.json`, `system_theme.rs` (light or dark from GTK, the portal, GNOME, the Windows registry, macOS), `desktop/theme_events.rs`, `choices.rs` (theme kept)
- IDE: `app/src/lib/ThemeView.svelte` (text beside a sample window), `ThemeSample.svelte`, `ThemeStyles.svelte`
- Docs: manual `themes.md`; LANGUAGE.md section 7 (Themes)

## Tests
- `conformance/themes.json`: run by rust-ui, `runtimes/web` and the spec
- spec `themes.rs`, `clarity_ui_theme.rs`, `colors.rs`, `section_colors.rs`; gen `themes.rs`
- rust-ui `tests/screen/theme_look.rs`, `looks_contrast.rs`; `tests/windows/core_system_theme.rs`, `looks_system_theme_follow.rs`, `looks_system_theme_watch.rs`; `tests/data/looks_gallery.rs`
- Guard: `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-spec --test clarity_ui_theme` writes `runtimes/rust-ui/themes/`

## Open
- none

## Conflicts
- Color palette written three times: `crates/clarity-spec/src/colors.rs:8`, `runtimes/rust-ui/src/parts/inputs/color.rs:44`, `runtimes/csharp/Clarity.Runtime/Palette.cs:13` (no callers outside tests); equal today.
- DESIGN.md:240 says 21 color roles; there are 29 (`themes.rs:33`). DESIGN.md:243, 256, 1739 say theme JSON is for C# only; every stack ships it.
- DESIGN 1.5 table describes the web renderer's CSS per theme for generated apps; generated apps use Clarity.UI's resolved JSON, CSS is the designer's only.
- Manual themes.md:215, 246 use page blocks; :253-256 say themes go through the web renderer and CSS on Rust, Go, C, C++.
- `conformance/themes.json` "about" names Avalonia.

## Decisions
- 2026-10-17: a theme is named values, not a style sheet; parts pick a named `style`, never raw colors.
- 2026-10-17: Clarity's own look is the built-in theme Clarity, which sets every value; every theme starts from it and sets what it changes.
- 2026-10-17: the project file names the themes offered, the first one starts; the `theme` click target and `settings.theme` switch them; the choice is kept.
- 2026-11-01: every look is defined in Clarity (theme values, part looks per state), editable in the IDE; generated code only carries it out. User: "that is a base rule.... otherwise round tripping of the generated code and clarity will never be established!"
- 2026-11-04: Clarity's own theme takes the IDE's look, so a running app looks like the IDE.

## Was
- DESIGN 1.5 (Themes, T1 rows of the stack table)
- Roadmap: T1, G2c (themes part), G3c (themes part)
- memory: looks-in-clarity.md
