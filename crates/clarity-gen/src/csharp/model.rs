//! Enums and entities as C# enums and record classes.

use clarity_spec::Project;
use clarity_spec::code::{DataEvent, record_name};
use clarity_spec::model::{EnumDecl, Entity, Field, FieldType, Literal, SpecFile};

use super::names::{field_type, pascal, property, string};

pub fn emit(project: &Project, file: &SpecFile) -> String {
    let mut out = String::new();
    for decl in &file.enums {
        out.push_str(&enum_decl(decl));
    }
    for entity in &file.entities {
        out.push_str(&record(project, entity));
    }
    out
}

/// An enum, and its labels for text: `Labels.Of(status)`.
fn enum_decl(decl: &EnumDecl) -> String {
    let name = &decl.name.name;
    let values: String = decl.variants.iter().map(|v| format!("    {},\n", v.name.name)).collect();
    let labels: String = decl
        .variants
        .iter()
        .map(|v| format!("        {name}.{} => {},\n", v.name.name, string(&v.label)))
        .collect();
    format!(
        "public enum {name}
{{
{values}}}

public static partial class Labels
{{
    public static string Of({name} value) => value switch
    {{
{labels}        _ => \"\",
    }};
}}

"
    )
}

/// Fields that are stored in the entity's own table.
fn stored(entity: &Entity) -> impl Iterator<Item = &Field> {
    entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. }))
}

/// The initial value of a field, when it has one beyond C#'s own default.
fn initial(field: &Field) -> Option<String> {
    match (&field.ty, field.modifiers.default.as_ref()) {
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url, Some(Literal::Str(s))) => Some(string(s)),
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url, _) => Some("\"\"".into()),
        (FieldType::Integer | FieldType::Id, Some(Literal::Number(n))) => Some(format!("{n}L")),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => Some(format!("{n}m")),
        (FieldType::Bool, Some(Literal::Bool(true))) => Some("true".into()),
        (FieldType::Enum { name }, Some(Literal::Ident(v))) => Some(format!("{}.{v}", name.name)),
        (FieldType::List { .. }, _) => Some("[]".into()),
        _ => None,
    }
}

/// How a field is stored: most values go as they are, enums by name.
fn to_sql(entity: &str, field: &Field) -> String {
    let value = property(entity, &field.name.name);
    match &field.ty {
        FieldType::Enum { .. } => format!("Sql.Name({value})"),
        _ => value,
    }
}

fn from_sql(field: &Field) -> String {
    match &field.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url => "GetText".into(),
        FieldType::Integer | FieldType::Id => "GetInteger".into(),
        FieldType::Decimal { .. } | FieldType::Money => "GetDecimal".into(),
        FieldType::Bool => "GetBoolean".into(),
        FieldType::Date => "GetDate".into(),
        FieldType::DateTime => "GetDateTime".into(),
        FieldType::Enum { name } => format!("GetName<{}>", name.name),
        FieldType::Ref { .. } => "GetReference".into(),
        FieldType::List { .. } => unreachable!("list fields are not stored in the table"),
    }
}

fn record(project: &Project, entity: &Entity) -> String {
    let name = &entity.name.name;
    let columns = stored(entity).map(|f| string(entity.column_of(&f.name.name))).collect::<Vec<_>>().join(", ");
    let properties: String = entity
        .fields
        .iter()
        .map(|f| {
            let init = initial(f).map(|v| format!(" = {v};")).unwrap_or_default();
            format!("    public {} {} {{ get; set; }}{init}\n", field_type(&f.ty), property(name, &f.name.name))
        })
        .collect();
    let to_values = stored(entity).map(|f| to_sql(name, f)).collect::<Vec<_>>().join(", ");
    let from_values: String = stored(entity)
        .enumerate()
        .map(|(i, f)| {
            format!(
                "        {} = Sql.{}({}, values[{i}]),\n",
                property(name, &f.name.name),
                from_sql(f),
                string(&f.name.name)
            )
        })
        .collect();
    // The data events written in the entity's code block; each runs the
    // method generated from it.
    let events: Vec<DataEvent> = project
        .files
        .iter()
        .flat_map(|f| &f.code)
        .filter(|b| b.name.name == *name)
        .flat_map(|b| b.events.iter().map(|e| e.event))
        .collect();
    let mut hooks = String::new();
    if !events.is_empty() {
        hooks.push_str("\n    public static bool HasEvents => true;\n");
        for event in events {
            let method = pascal(&format!("{}_{}", event.word(), record_name(name)));
            hooks.push_str(&format!("\n    public void {}(Db db) => Logic.{method}(db, this);\n", pascal(event.word())));
        }
    }
    format!(
        "public sealed partial class {name} : IRecord<{name}>
{{
    public static string SourceName => {source};
    public static string TableName => {table};
    public static string KeyColumn => {key};
    public static IReadOnlyList<string> ColumnNames {{ get; }} = [{columns}];

    public long Id {{ get; set; }}
{properties}
    public object?[] ToValues() => [{to_values}];

    public static {name} FromValues(long id, object?[] values) => new()
    {{
        Id = id,
{from_values}    }};
{hooks}}}

",
        source = string(project.source_of(name).unwrap_or_default()),
        table = string(&entity.table_name()),
        key = string(entity.key_column()),
    )
}
