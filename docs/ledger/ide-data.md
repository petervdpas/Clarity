# IDE data editors

Status: partial. The IDE's pages for the data model and the project's settings: entity and enum editors, mappings, web APIs, translations, project settings, migrations, backups, import.

## Code
- Spec: `crates/clarity-spec/src/edit.rs` (module text edits: entities, fields, enums, variants, uses), `dictionary.rs`, `field_rename.rs`, `rename.rs`, `migrate.rs`; `code/mapping.rs` (`MappingForm`, `form_of`)
- IDE:
  - Entities and enums: `app/src/lib/EntityEditor.svelte` (a row per field: name, type, modifiers including Search and Hashed), `EnumEditor.svelte`, `RenameField.svelte`; see [entities](entities.md)
  - Mappings: `MappingPage.svelte` (form of the mapping, live sample read through `clarity_formats::read_table`), `FormatOptions.svelte`; Tauri `app/src-tauri/src/mappings.rs`; `crates/clarity-project/src/mappings.rs` (`preview_mapping`); see [formats](formats.md)
  - Web APIs: `ApiView.svelte`, `ApiPanel.svelte` (what the app uses of an OpenAPI file); Tauri `apis.rs`; `crates/clarity-project/src/openapi.rs`; see [web-apis](web-apis.md)
  - Translations: `TranslationsPanel.svelte` (a value per key per language, filter, progress, resource keys, "make texts keys"), `NewLanguageDialog.svelte`; Tauri `translations.rs`; see [translations](translations.md)
  - Project settings: `ProjectSettings.svelte` (865 lines: name, folders, title, stack with a tools check, language, formats, theme, data sources, file stores, port), `StackFields.svelte`; see [project-file](project-file.md)
  - Migrations: `MigrationPanel.svelte` (per database, in the Database section); Tauri `migrations.rs`; see [migrations](migrations.md)
  - Backups: `BackupPanel.svelte` (Back up now, list with Restore and Delete, confirmed through Clarity's dialogs); Tauri `backups.rs`; see [backups](backups.md)
  - Import and take ownership: `ImportPanel.svelte`; Tauri `external.rs`; `crates/clarity-project/src/file_import.rs`
  - Sign-in settings (`SigninSettings.svelte`, `SigninView.svelte`): see [signin](signin.md)
- Docs: manual/data.entities.md, data.sources.md, data.migrations.md, languages.md, project.file.md

## Tests
- `crates/clarity-spec/tests/dictionary_edit.rs`, rename_field.rs, rename.rs
- `crates/clarity-project/tests/mappings.rs`, openapi.rs, external.rs, backups.rs, settings.rs
- `app/src/lib/paging.test.ts`, stackTools.test.ts

## Open
- A data browser (rows of a source, file sources, saves through the version check): NOT BUILT (FORMATS-PLAN.md:262, APP-CAPABILITIES-PLAN.md:229)
- Testing a data source's connection from the Project tab: NOT BUILT (DESIGN 1.3, :131)
- Import, live-table checks and baselines for server databases: NOT BUILT ("importing is for SQLite and files only for now", `crates/clarity-project/src/lib.rs:1382`)
- "Back up before a migration" checkbox for `backups off`: NOT BUILT (BACKUP-PLAN.md:45); see [backups](backups.md)
- For a server database the IDE has no backup (BACKUP-PLAN.md:39)

## Conflicts
- Port 8080 written twice: `crates/clarity-project/src/lib.rs:285` and `app/src/lib/ProjectSettings.svelte:369` (equal today)
- manual/data.entities.md:87 leaves out the editor's Search and Hashed columns
- manual/project.file.md:66: settings hold Migrations and Backups per database; translations open from the tree, not settings
- manual/design.parts.md:24: sign-in is set in the tree's Sign-in item, not "Who is using it" in Project settings
- Project settings show the UI as a select (`StackFields.svelte:61-64`) while every backend offers only Clarity.UI; the plan says the settings keep it as text

## Decisions
- 2026-11-05: backups are taken, listed, restored and deleted from the Database section of the Project tab, each confirmed through Clarity's own dialogs (BACKUP-PLAN.md step 1)
- 2026-11-10: Project settings keeps showing the UI as text, so an older project still says what it uses (PROJECT-BROWSER-PLAN.md, Decided)

## Was
- FORMATS-PLAN.md step 4 (the mapping page, IDE half)
- BACKUP-PLAN.md step 1 (the IDE half)
