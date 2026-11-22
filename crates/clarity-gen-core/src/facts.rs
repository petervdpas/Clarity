//! What every stack's program on Clarity.UI carries beside its code: the
//! facts Clarity.UI asks for (DESIGN 7.9) and the files of its windows,
//! themes and resources.

use clarity_spec::Project;
use clarity_spec::model::FieldType;

use crate::OutFile;

/// The facts of [`facts_rs`] as the JSON the library reads (`facts`, DESIGN
/// 7.9, `clarity_runtime::fields::Facts::to_json`), for the languages that
/// answer it through the C interface.
pub fn facts_json(project: &Project) -> String {
    use clarity_spec::model::{DataKind, Literal};
    use serde_json::{Map, Value, json};
    let literal = |value: &Option<Literal>| match value {
        Some(Literal::Number(v) | Literal::Str(v) | Literal::Ident(v)) => Value::from(v.as_str()),
        Some(Literal::Bool(b)) => Value::from(b.to_string()),
        None => Value::Null,
    };
    let mut windows = Map::new();
    let pages = project.files.iter().flat_map(|f| f.pages.iter().map(|p| (&p.name.name, &p.data)));
    let apps = project.files.iter().flat_map(|f| f.apps.iter().map(|a| (&a.name.name, &a.data)));
    for (name, data) in pages.chain(apps) {
        let values: Map<String, Value> = data
            .iter()
            .map(|d| {
                let ty = match &d.of {
                    DataKind::Record { entity } => entity.name.clone(),
                    DataKind::List { entity } => format!("list<{}>", entity.name),
                    DataKind::Value { ty, .. } => code_fact_type(ty),
                };
                (d.name.name.clone(), Value::from(ty))
            })
            .collect();
        windows.insert(name.clone(), Value::Object(values));
    }
    let mut records = Map::new();
    for file in &project.files {
        for entity in file.entities.iter().chain(&file.records) {
            let types: Map<String, Value> = entity.fields.iter().map(|f| (f.name.name.clone(), Value::from(fact_type(&f.ty)))).collect();
            records.insert(entity.name.name.clone(), Value::Object(types));
        }
    }
    let mut fields = Map::new();
    for entity in project.entities().chain(project.records()) {
        let parent = crate::tree_parent(entity);
        for field in &entity.fields {
            let refers = match &field.ty {
                FieldType::Ref { entity } => Some(entity.name.as_str()),
                _ => None,
            };
            let format = fact_format_parts(field).map(|(kind, scale)| json!({"kind": kind, "scale": scale, "format": field.modifiers.format}));
            fields.insert(
                format!("{}.{}", entity.name.name, field.name.name),
                json!({
                    "label": field.label,
                    "format": format,
                    "required": field.modifiers.required,
                    "min": literal(&field.modifiers.min),
                    "max": literal(&field.modifiers.max),
                    "refers": refers,
                    "parent": parent,
                    "color": matches!(field.ty, FieldType::Color),
                    "folder": matches!(field.ty, FieldType::Folder),
                }),
            );
        }
    }
    let enums: Map<String, Value> = project.enums().map(|e| (e.name.name.clone(), Value::from(e.variants.iter().map(|v| v.name.name.clone()).collect::<Vec<_>>()))).collect();
    json!({"windows": windows, "records": records, "fields": fields, "enums": enums}).to_string()
}

/// The files of a program on Clarity.UI (manual/start.how-it-works.md), beside its code: the
/// windows and app frames as `.cxaml` (the same markup the shared screen and
/// C# read) and the themes the project offers as Clarity resolves them. The
/// program carries them, with the model, whose translations Clarity.UI loads.
pub fn clarity_files(project: &Project) -> Vec<OutFile> {
    let mut files: Vec<OutFile> = crate::web_windows::window_files(project)
        .into_iter()
        .chain(crate::web_windows::app_files(project))
        .map(|file| OutFile { path: file.path.replace("ui/windows/", "windows/").replace("ui/apps/", "apps/"), ..file })
        .collect();
    files.extend(crate::themes::resolved_files(project, "themes"));
    files.extend(crate::web::resource_files(project, "resources"));
    files
}

/// A type as the runtime's facts write it (`clarity_runtime::fields`): a
/// record, entity or enum by its name, a list as `list<Order>`, a reference
/// as `ref<Customer>`, a plain value by its keyword.
pub fn fact_type(ty: &FieldType) -> String {
    match ty {
        FieldType::Text { .. } => "text".into(),
        FieldType::Integer => "integer".into(),
        FieldType::Decimal { .. } => "decimal".into(),
        FieldType::Money => "money".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Date => "date".into(),
        FieldType::DateTime => "datetime".into(),
        FieldType::Duration => "duration".into(),
        FieldType::Image => "image".into(),
        FieldType::Blob => "blob".into(),
        FieldType::Email => "email".into(),
        FieldType::Url => "url".into(),
        FieldType::Color => "color".into(),
        FieldType::Secret => "secret".into(),
        FieldType::Folder => "folder".into(),
        FieldType::File => "file".into(),
        FieldType::Language => "language".into(),
        FieldType::Theme => "theme".into(),
        FieldType::Id => "id".into(),
        FieldType::Enum { name } => name.name.clone(),
        FieldType::Ref { entity } => format!("ref<{}>", entity.name),
        FieldType::Record { entity } => entity.name.clone(),
        FieldType::List { item } => format!("list<{}>", item.name),
        FieldType::Values { item } => format!("list<{}>", item.word()),
    }
}

/// A type written in code, as [`fact_type`] writes it.
pub fn code_fact_type(ty: &clarity_spec::code::TypeRef) -> String {
    use clarity_spec::code::TypeRefKind as K;
    match &ty.kind {
        K::Text => "text".into(),
        K::Integer => "integer".into(),
        K::Decimal => "decimal".into(),
        K::Money => "money".into(),
        K::Bool => "bool".into(),
        K::Date => "date".into(),
        K::DateTime => "datetime".into(),
        K::Duration => "duration".into(),
        K::File => "file".into(),
        K::Bytes => "bytes".into(),
        K::Named { name } => name.clone(),
        K::List { item } => format!("list<{item}>"),
    }
}

/// A field's format kind as conformance/formats.json names it, and its scale; none for a type without formats.
pub fn fact_format_parts(field: &clarity_spec::model::Field) -> Option<(&'static str, Option<u32>)> {
    match &field.ty {
        FieldType::Integer => Some(("integer", None)),
        FieldType::Decimal { .. } => Some(("decimal", field.ty.shown_scale())),
        FieldType::Money => Some(("money", None)),
        FieldType::Date => Some(("date", None)),
        FieldType::DateTime => Some(("dateTime", None)),
        FieldType::Duration => Some(("duration", None)),
        _ => None,
    }
}
