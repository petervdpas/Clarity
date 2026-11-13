# Translations

Status: partial. Texts by key with named slots, one file per language, resources per language, and values shown in the person's locale (display formats).

Progress: 85%

## Code
- Spec: `crates/clarity-spec/src/translations.rs` (files, checks, Clarity's own words in English :35-320), `keyed.rs`, `design_keys.rs`, `resources.rs`, `formats.rs` (named formats)
- Generators: languages go into each program's model and `files.json`; C# also writes `.resx` (`crates/clarity-gen/src/csharp/texts.rs`)
- Runtimes: R `runtimes/rust/src/i18n.rs`; Go `runtimes/go/clarity/i18n.go`, `text.go`; C `runtimes/c/clarity/text.c`; C++ `runtimes/cpp/clarity/text.cpp`; C# `runtimes/csharp/Clarity.Runtime/Text.cs`, `Languages.cs`, `Locale.cs`, `ShownText.cs`, `Formats.cs`
- Clarity.UI: `runtimes/rust-ui/src/locale.rs`, `parts/inputs/formats.rs`, `choices.rs` (language kept)
- IDE: `app/src/lib/TranslationsPanel.svelte`, `NewLanguageDialog.svelte`, `ResourceView.svelte`; `app/src-tauri/src/translations.rs`, `resources.rs`
- CLI: `clarity spec keys` (make texts keys), `clarity spec texts` (keys without a value)
- Docs: `manual/languages.md`; LANGUAGE.md section 2 (Translations, Resources), section 4 (Formats), section 10 (Texts by key, Text with slots)

## Tests
- `conformance/translations.json`: run by R, Go, C, C++, C#, `runtimes/web/src/screen`
- `conformance/formats.json` (how values show and read): run by Clarity.UI (`runtimes/rust-ui/tests/inputs/inputs_formats.rs`), C#, `runtimes/web`; skipped by R, Go, C, C++ runtime tests
- `crates/clarity-spec/tests/`: translations.rs, keyed.rs, keying.rs, slots.rs, own_words_everywhere.rs, design_keys.rs, resources.rs, formats.rs
- `crates/clarity-gen/tests/`: texts_build.rs, resources.rs

## Open
- `context` for one text with two meanings: NOT BUILT (DESIGN 1.5, roadmap I1c).
- Dates and numbers in the system's locale, with a choice apart from the language: PARTIAL. Formats follow the shown language; 8 locales, others fall back to en-US (`runtimes/rust-ui/src/parts/inputs/formats.rs:303-318`).
- The IDE's own words: see [ide-shell](ide-shell.md).

## Conflicts
- DESIGN :203, :214, :255 and 6.7 (:1719) describe key maps in a UI bundle; programs carry texts in `files.json` and the model (C# also `.resx`).
- DESIGN 1.5 "Every stack" table describes the web renderer (Intl, CSS) for R, Go, C, C++; every program uses Clarity.UI.
- `manual/languages.md:154` says translations go through the web renderer; :132 says the locale follows the system, it follows the language shown.
- Display formats are written three times: `runtimes/rust-ui/src/parts/inputs/formats.rs`, `runtimes/web/src/screen/format.ts`, `runtimes/csharp/Clarity.Runtime/Formats.cs`. Equal on the cases.
- R and Go cli duration readers say "as formats.json reads them" (`runtimes/rust/src/cli.rs:308`, `runtimes/go/clarity/cli.go:387`) but their tests do not load formats.json.
- Grammar summary (LANGUAGE.md:3028-3149): translations have no `resource`.

## Decisions
- 2026-10-17: formats follow the person's locale; `language` and `currency` in the project file; named formats, not patterns (DESIGN 1.5, I1a).
- 2026-10-17: one translation file per language for the whole project, `translations/<lang>.clarity`.
- 2026-10-21: texts by key (`@saveCaption`), named slots given by name; a literal text people read is an error. Replaces texts found by what they say.
- 2026-10-21: every text people read comes from the translations, in design, code and engines. User: "labels, texts, textareas(blocks), panel headings etc should always come from translates!"
- 2026-10-22: a key may point at a resource file per language (`gameRules resource "rules-nl.md"`) (R1).

## Was
- DESIGN 1.5 (Formats I1a, Text with slots I1b, Translations I1c, Texts by key, the formats and translations rows of "Every stack"); DESIGN 6.7
- Roadmap I1a, I1b, I1c; R1 (resource keys; the markdown part is [markdown](markdown.md))
- IMPLEMENTED.md section 3 "Translations and resources"; UNIMPLEMENTED.md section 1 (`context`), section 4 (locale)
- memory: i18n-everywhere.md
