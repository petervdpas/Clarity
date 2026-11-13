# Entities

Status: partial. Entities and enums: the data dictionary of stored fields, their types and modifiers, and data events written next to the entity.

Progress: 65%

## Code
- Spec: `crates/clarity-spec/src/parser.rs` (`entity` :492-620, field types :725-784, modifiers :786 and :816-858, `enum` :426, `record` :462); model `model.rs` (`FieldType`, modifiers :464); checks `check.rs:2411-2626`; data events `code/mod.rs:43-124`; `dictionary.rs`, `field_rename.rs`
- Generators: columns and DDL `crates/clarity-gen/src/ddl.rs` (`column_type` :147); required fields `crates/clarity-gen/src/lib.rs:268-272`; records per stack in `crates/clarity-gen/src/{rust,go,c,cpp,csharp}/model.rs`
- Runtimes: R `runtimes/rust/src/required.rs`, `list_fields.rs`, `lib.rs` (save checks required only, :826); Go `runtimes/go/clarity/required.go`, `list_fields.go`, `record.go`; C `runtimes/c/clarity/list_fields.c`, `db.c`; C++ `runtimes/cpp/clarity/list_fields.cpp`, `db.cpp`; C# `runtimes/csharp/Clarity.Runtime/Record.cs`, `ListFields.cs`, `FieldFacts.cs`
- Clarity.UI: inputs read `min`/`max` for range only (`runtimes/rust-ui/src/parts/inputs/number.rs:147`, `slider.rs:48`)
- IDE: `app/src/lib/EntityEditor.svelte` (type list from `fieldType.ts`, `fieldTypes.svelte.ts` over `dictionary.rs` SIMPLE_TYPES, NAMED_TYPES; Search and Hashed columns; details row Min, Max, Pattern, Computed, Owned, Cascade, Column; rename from the name cell; version line :357-359), `app/src/lib/EnumEditor.svelte`; agent tools `crates/clarity-tools/src/structural/model.rs` (add_entity, add_field, change_field, add_enum, ...)
- Docs: manual `data.entities.md` (Field types, Modifiers, Lists in records), `code.handlers.md` (Data events)

## Tests
- `conformance/required.json`, `conformance/list-fields.json`: run by R, Go, C, C++, C#
- spec `crates/clarity-spec/tests/` parse.rs, defaults.rs, cascade.rs, file_fields.rs, records.rs, list_fields.rs, value_lists.rs, data_events.rs, names.rs, rename_field.rs, dictionary_edit.rs
- gen `crates/clarity-gen/tests/` required_fields.rs, cascade.rs, list_fields.rs, date_defaults.rs, ref_copy.rs

## Open
- `list<Entity>` on a stored entity: NOT BUILT on all five. No column or table (`crates/clarity-gen/src/ddl.rs`); reading one is refused, "list fields like ... are not loaded yet" (`rust/logic.rs:974`, `go/logic.rs:1546`, `c/logic.rs:1743`, `cpp/logic.rs:1104`, `csharp/logic.rs:1017`). Only web-API entities hold such lists; see [web-apis](web-apis.md).
- `owned`: NOT BUILT. Parsed and checked (`parser.rs:818`, `check.rs:2471`); no generator or runtime reads it.
- `pattern "regex"`: NOT BUILT. Parsed and checked (`parser.rs:826`, `check.rs:2517`), never enforced.
- `min`, `max`: PARTIAL. Only the range of sliders and spin boxes; `save` checks required fields only.
- `text(n)`: PARTIAL. VARCHAR(n) on server databases; nothing enforces it on SQLite.
- `computed` with a formula: PARTIAL. The word only leaves the field out of the required check (`crates/clarity-gen/src/lib.rs:270`); no syntax for the value.
- `time` field and part: NOT BUILT (DESIGN.md:2474, roadmap U1a).
- Unbound input parts reported as warnings: NOT BUILT (DESIGN 5.6, :489; flag in `widgets.rs:544`, read by no check).
- `steps` between enum values: NOT BUILT; see [diagrams](diagrams.md) (DG2).
- Editing data events in the designer; one transaction across databases: NOT BUILT / PARTIAL (D1f; one transaction per source, `step`, `runtimes/rust/src/lib.rs:1002`).

## Conflicts
- manual `data.entities.md:45` and `:57` promise `list<Entity>`, `min`, `max` and `pattern` as working.
- manual `data.entities.md:53-54` documents quoted `label "text"` and `help "text"`; the checker wants keys (`crates/clarity-spec/src/translations.rs:701`, :1089).
- The entity editor offers Owned on a `ref` (`app/src/lib/EntityEditor.svelte:338`) and manual `data.entities.md:87` says "Owned (for a ref or a list)"; the checker refuses it: "'owned' only applies to lists" (`check.rs:2471`).
- manual `data.entities.md:93` says `min`, `max` and `pattern` are written in the module's text; :87 gives them fields in the editor's details row.

## Decisions
- 2026-10-17: a field's types, validation, label, help and format live once in the entity; windows, schema and wire are made from it (DESIGN 5.2).
- 2026-10-25: a record held inside a record only in entities of a web API; a stored entity refers with `ref` (DESIGN 5.25).
- 2026-10-29: `duration` in whole seconds; `none` is the empty value of a date, datetime or reference (roadmap X2).
- 2026-11-06: every field name is camelCase; a column defaults to the snake_case of the field, so a rename keeps the database. User: "camelCase everywhere"
- 2026-11-12: `hashed` text fields, `readable when`, `writable when`; see [signin](signin.md). `version`, `savedSince`; see [row-versions](row-versions.md).

## Was
- DESIGN 5.2, 5.6 (unbound inputs, :489), roadmap D1f, X2 (duration, none), U1a (time)
- LANGUAGE.md section 4 "Enums", "Entities", "Lists in records"
- IMPLEMENTED.md 3 "Declarations", "Fields", "What the generators still refuse" (list fields)
- UNIMPLEMENTED.md 1 (`computed`), 2 "Fields"
- CONFLICTING.md 6 (grammar, store), 8 (data.entities.md)
- memory: camel-case-names.md
