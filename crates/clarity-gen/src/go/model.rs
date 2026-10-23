//! Enums and entities as Go types: an enum is a string type with a constant
//! per value and its labels; an entity is a struct with JSON names as the
//! renderer reads them, its defaults in `NewX`, and the methods the
//! runtime's `Record` asks for, on its pointer.

use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::{DataEvent, record_name};
use clarity_spec::model::{EnumDecl, Entity, Field, FieldType, Literal, SpecFile};

use super::names::{field, field_type, local, pascal, string};

/// The enums and records of `file`.
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

/// Lines of `columns` with each column padded to the widest, as gofmt
/// aligns struct fields and constants; the last column is not padded.
pub fn aligned(rows: &[Vec<String>], indent: &str) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns).map(|c| rows.iter().filter_map(|r| r.get(c)).map(|s| s.chars().count()).max().unwrap_or(0)).collect();
    let mut out = String::new();
    for row in rows {
        out.push_str(indent);
        for (c, cell) in row.iter().enumerate() {
            out.push_str(cell);
            if c + 1 < row.len() {
                out.push_str(&" ".repeat(widths[c] - cell.chars().count() + 1));
            }
        }
        out.push('\n');
    }
    out
}

/// The Go constant of an enum value: `OrderStatusApproved`.
pub fn constant(enum_name: &str, value: &str) -> String {
    format!("{enum_name}{}", pascal(value))
}

fn enum_decl(decl: &EnumDecl) -> String {
    let name = &decl.name.name;
    let rows: Vec<Vec<String>> = decl
        .variants
        .iter()
        .map(|v| vec![constant(name, &v.name.name), name.clone(), "=".into(), string(&v.name.name)])
        .collect();
    let mut labels = String::new();
    for v in &decl.variants {
        let _ = write!(labels, "\tcase {}:\n\t\treturn {}\n", constant(name, &v.name.name), string(&v.label));
    }
    format!(
        "type {name} string\n\nconst (\n{}\n)\n\n// Label is the value as people read it.\nfunc (v {name}) Label() string {{\n\tswitch v {{\n{labels}\t}}\n\treturn string(v)\n}}\n\n",
        aligned(&rows, "\t").trim_end()
    )
}

/// Fields that are stored in the entity's own table.
fn stored(entity: &Entity) -> impl Iterator<Item = &Field> {
    entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. }))
}

/// A literal decimal: `decimal.NewFromInt(100)`, `decimal.RequireFromString("9.95")`.
pub fn decimal_literal(text: &str) -> String {
    match text.parse::<i64>() {
        Ok(0) => "decimal.Zero".into(),
        Ok(n) => format!("decimal.NewFromInt({n})"),
        Err(_) => format!("decimal.RequireFromString({})", string(text)),
    }
}

/// The value a new record starts with, when it differs from Go's zero value.
fn initial(project: &Project, f: &Field) -> Option<String> {
    match (&f.ty, f.modifiers.default.as_ref()) {
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme, Some(Literal::Str(s))) if !s.is_empty() => Some(string(s)),
        (FieldType::Integer | FieldType::Id | FieldType::Duration, Some(Literal::Number(n))) if n != "0" => Some(n.clone()),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) if n.parse::<f64>().is_ok_and(|v| v != 0.0) => {
            Some(decimal_literal(n))
        }
        (FieldType::Bool, Some(Literal::Bool(true))) => Some("true".into()),
        (FieldType::Enum { name }, Some(Literal::Ident(v))) => Some(constant(&name.name, v)),
        // An enum without a default starts with its first value, as on every stack.
        (FieldType::Enum { name }, _) => project.enum_decl(&name.name).and_then(|d| d.variants.first()).map(|v| constant(&name.name, &v.name.name)),
        _ => None,
    }
}

fn to_value(f: &Field) -> String {
    let value = format!("r.{}", field(&f.name.name));
    match &f.ty {
        FieldType::Enum { .. } => format!("string({value})"),
        FieldType::Image => format!("clarity.PictureValue({value})"),
        _ => value,
    }
}

fn read(i: usize, f: &Field, project: &Project) -> String {
    let column = string(&f.name.name);
    match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => format!("read.Text({i}, {column})"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => format!("read.Integer({i}, {column})"),
        FieldType::Decimal { .. } | FieldType::Money => format!("read.Decimal({i}, {column})"),
        FieldType::Bool => format!("read.Bool({i}, {column})"),
        FieldType::Date => format!("read.Date({i}, {column})"),
        FieldType::DateTime => format!("read.DateTime({i}, {column})"),
        FieldType::Ref { .. } => format!("read.Reference({i}, {column})"),
        FieldType::Image => format!("read.Picture({i}, {column})"),
        FieldType::Enum { name } => {
            let names: String =
                project.enum_decl(&name.name).map(|d| d.variants.iter().map(|v| format!(", {}", string(&v.name.name))).collect()).unwrap_or_default();
            format!("{}(read.Name({i}, {column}{names}))", name.name)
        }
        FieldType::List { .. } => unreachable!("list fields are not stored in the table"),
    }
}

fn record(project: &Project, entity: &Entity) -> String {
    let name = &entity.name.name;
    let mut rows = vec![vec!["ID".to_string(), "int64".into(), "`json:\"id\"`".into()]];
    for f in stored(entity) {
        rows.push(vec![field(&f.name.name), field_type(&f.ty), format!("`json:\"{}\"`", f.name.name)]);
    }
    let defaults: Vec<String> =
        stored(entity).filter_map(|f| initial(project, f).map(|v| format!("{}: {v}", field(&f.name.name)))).collect();
    let columns = stored(entity).map(|f| string(entity.column_of(&f.name.name))).collect::<Vec<_>>().join(", ");
    let values = stored(entity).map(to_value).collect::<Vec<_>>().join(", ");
    let mut load = String::new();
    for (i, f) in stored(entity).enumerate() {
        let _ = writeln!(load, "\tr.{} = {}", field(&f.name.name), read(i, f, project));
    }
    let mut out = format!(
        "type {name} struct {{\n{}}}\n\n// New{name} is a new {} with its defaults.\nfunc New{name}() {name} {{\n\treturn {name}{{{}}}\n}}\n\n",
        aligned(&rows, "\t"),
        record_name(name).replace('_', " "),
        defaults.join(", ")
    );
    // Every method on lines of its own: gofmt leaves those as they are.
    let _ = write!(out, "func (*{name}) Table() string {{\n\treturn {}\n}}\n\n", string(&entity.table_name()));
    let _ = write!(out, "func (*{name}) Columns() []string {{\n\treturn []string{{{columns}}}\n}}\n\n");
    let source = project.source_of(name).unwrap_or_default();
    if !source.is_empty() && source != project.default_source() {
        let _ = write!(out, "func (*{name}) Source() string {{\n\treturn {}\n}}\n\n", string(source));
    }
    if entity.key_column() != "id" {
        let _ = write!(out, "func (*{name}) Key() string {{\n\treturn {}\n}}\n\n", string(entity.key_column()));
    }
    let pictures: Vec<String> = entity
        .fields
        .iter()
        .filter(|f| f.ty == FieldType::Image)
        .map(|f| {
            let store = f.modifiers.store.as_deref().map(|s| format!(", Store: {}", string(s))).unwrap_or_default();
            format!("{{Column: {}{store}}}", string(entity.column_of(&f.name.name)))
        })
        .collect();
    if !pictures.is_empty() {
        let _ = write!(out, "func (*{name}) Pictures() []clarity.PictureColumn {{\n\treturn []clarity.PictureColumn{{{}}}\n}}\n\n", pictures.join(", "));
    }
    let _ = write!(out, "func (r *{name}) GetID() int64 {{\n\treturn r.ID\n}}\n\n");
    let _ = write!(out, "func (r *{name}) SetID(id int64) {{\n\tr.ID = id\n}}\n\n");
    let _ = write!(out, "func (r *{name}) Values() []any {{\n\treturn []any{{{values}}}\n}}\n\n");
    let _ = write!(
        out,
        "func (r *{name}) Load(id int64, values []any) error {{\n\tread := clarity.NewReader(values)\n\tr.ID = id\n{load}\treturn read.Err()\n}}\n\n"
    );
    // The data events written in the entity's code block; each runs the
    // function generated from it.
    let events: Vec<DataEvent> =
        project.files.iter().flat_map(|f| &f.code).filter(|b| b.name.name == *name).flat_map(|b| b.events.iter().map(|e| e.event)).collect();
    for event in &events {
        let function = local(&format!("{}_{}", event.word(), record_name(name)));
        let _ = write!(out, "func (r *{name}) {}(db *clarity.Db) error {{\n\treturn {function}(db, r)\n}}\n\n", pascal(event.word()));
    }
    out
}
