//! Enums and entities as Go types: an enum is a string type with a constant
//! per value and its labels; an entity is a struct with JSON names as the
//! renderer reads them, its defaults in `NewX`, and the methods the
//! runtime's `Record` asks for, on its pointer.

use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::{DataEvent, record_name};
use clarity_spec::model::{EnumDecl, Entity, Field, FieldType, Literal, SpecFile};

use super::names::{field, field_type, local, pascal, string, value_type};
use clarity_spec::code::check::ValueItem;

/// The enums and records of `file`.
pub fn emit(project: &Project, file: &SpecFile) -> String {
    let mut out = String::new();
    for decl in &file.enums {
        out.push_str(&enum_decl(decl));
    }
    for entity in &file.entities {
        out.push_str(&record(project, entity));
    }
    for decl in &file.records {
        out.push_str(&plain_record(project, decl));
    }
    out
}

/// A record (DESIGN 5.12): its fields with their JSON names and `NewX` with
/// their defaults; never stored, so it has no id and no table.
fn plain_record(project: &Project, record: &Entity) -> String {
    let name = &record.name.name;
    let rows: Vec<Vec<String>> = record.fields.iter().map(|f| vec![field(&f.name.name), field_type(&f.ty), format!("`json:\"{}\"`", f.name.name)]).collect();
    let defaults: Vec<String> = record.fields.iter().filter_map(|f| initial(project, f).map(|v| format!("{}: {v}", field(&f.name.name)))).collect();
    let body = if rows.is_empty() { String::new() } else { aligned(&rows, "\t") };
    format!(
        "type {name} struct {{\n{body}}}\n\n// New{name} is a new {} with its defaults.\nfunc New{name}() {name} {{\n\treturn {name}{{{}}}\n}}\n\n",
        record_name(name).replace('_', " "),
        defaults.join(", ")
    )
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
    entity.fields.iter().filter(|f| !f.ty.is_held())
}

/// A literal decimal: `decimal.NewFromInt(100)`, `decimal.RequireFromString("9.95")`.
pub fn decimal_literal(text: &str) -> String {
    match text.parse::<i64>() {
        Ok(0) => "decimal.Zero".into(),
        Ok(n) => format!("decimal.NewFromInt({n})"),
        Err(_) => format!("decimal.RequireFromString({})", string(text)),
    }
}

/// A date default written out (clarity_spec::dates).
fn go_date(d: &clarity_spec::dates::Moment) -> String {
    format!("clarity.Date{{Year: {}, Month: {}, Day: {}}}", d.year, d.month, d.day)
}

/// The value a new record starts with, when it differs from Go's zero value.
fn initial(project: &Project, f: &Field) -> Option<String> {
    match (&f.ty, f.modifiers.default.as_ref()) {
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret, Some(Literal::Str(s))) if !s.is_empty() => Some(string(s)),
        (FieldType::Integer | FieldType::Id | FieldType::Duration, Some(Literal::Number(n))) if n != "0" => Some(n.clone()),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) if n.parse::<f64>().is_ok_and(|v| v != 0.0) => {
            Some(decimal_literal(n))
        }
        (FieldType::Bool, Some(Literal::Bool(true))) => Some("true".into()),
        (FieldType::Date, Some(Literal::Str(s))) => clarity_spec::dates::date(s).map(|d| format!("&{}", go_date(&d))),
        (FieldType::DateTime, Some(Literal::Str(s))) => clarity_spec::dates::date_time(s)
            .map(|d| format!("&clarity.DateTime{{Date: {}, Hour: {}, Minute: {}, Second: {}}}", go_date(&d), d.hour, d.minute, d.second)),
        // An empty list, which travels as [] rather than null.
        (FieldType::Values { .. }, _) => Some(format!("{}{{}}", field_type(&f.ty))),
        (FieldType::Enum { name }, Some(Literal::Ident(v))) => Some(constant(&name.name, v)),
        // An enum without a default starts with its first value, as on every stack.
        (FieldType::Enum { name }, _) => project.enum_decl(&name.name).and_then(|d| d.variants.first()).map(|v| constant(&name.name, &v.name.name)),
        // Held records (DESIGN 5.25): a new one, and an empty list rather than nil.
        (FieldType::Record { entity }, _) => Some(format!("New{}()", entity.name)),
        (FieldType::List { .. }, _) => Some(format!("{}{{}}", field_type(&f.ty))),
        _ => None,
    }
}

fn to_value(f: &Field) -> String {
    let value = format!("r.{}", field(&f.name.name));
    match &f.ty {
        FieldType::Enum { .. } => format!("string({value})"),
        FieldType::Image => format!("clarity.PictureValue({value})"),
        // Its content, then its name in the column beside it.
        FieldType::File => format!("clarity.FileContent({value}), clarity.FileNameValue({value})"),
        FieldType::Values { .. } => format!("clarity.ListValue({value})"),
        _ => value,
    }
}

fn read(i: usize, f: &Field, project: &Project) -> String {
    let column = string(&f.name.name);
    match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => format!("read.Text({i}, {column})"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => format!("read.Integer({i}, {column})"),
        FieldType::Decimal { .. } | FieldType::Money => format!("read.Decimal({i}, {column})"),
        FieldType::Bool => format!("read.Bool({i}, {column})"),
        FieldType::Date => format!("read.Date({i}, {column})"),
        FieldType::DateTime => format!("read.DateTime({i}, {column})"),
        FieldType::Ref { .. } => format!("read.Reference({i}, {column})"),
        FieldType::Image => format!("read.Picture({i}, {column})"),
        FieldType::File => format!("read.File({i}, {column})"),
        FieldType::Enum { name } => {
            let names: String =
                project.enum_decl(&name.name).map(|d| d.variants.iter().map(|v| format!(", {}", string(&v.name.name))).collect()).unwrap_or_default();
            format!("{}(read.Name({i}, {column}{names}))", name.name)
        }
        FieldType::Values { item } => format!("clarity.ReadList[{}](read, {i}, {column})", value_type(&ValueItem::of_list(item).info())),
        FieldType::List { .. } | FieldType::Record { .. } => unreachable!("held records are not stored in the table"),
    }
}

fn record(project: &Project, entity: &Entity) -> String {
    let name = &entity.name.name;
    let mut rows = vec![vec!["ID".to_string(), "int64".into(), "`json:\"id\"`".into()]];
    // An entity of a web API holds its records inside (DESIGN 5.25); a stored one keeps them in their own table.
    let held: Vec<&Field> = if project.in_web_api(name) { entity.fields.iter().filter(|f| f.ty.is_held()).collect() } else { Vec::new() };
    let fields: Vec<&Field> = stored(entity).chain(held.iter().copied()).collect();
    for f in &fields {
        rows.push(vec![field(&f.name.name), field_type(&f.ty), format!("`json:\"{}\"`", f.name.name)]);
    }
    // A versioned entity (docs/APP-CAPABILITIES-PLAN.md step 5) carries the
    // version it was read with, which travels with the record to the screen and back.
    let version = project.version_column(name);
    if version.is_some() {
        rows.push(vec!["RowVersion".into(), "int64".into(), "`json:\"rowVersion\"`".into()]);
    }
    let defaults: Vec<String> = fields.iter().filter_map(|f| initial(project, f).map(|v| format!("{}: {v}", field(&f.name.name)))).collect();
    // A file has the column of its name beside its own.
    let columns = stored(entity)
        .flat_map(|f| {
            let column = entity.column_of(&f.name.name).to_string();
            let name = (f.ty == FieldType::File).then(|| format!("{column}_name"));
            std::iter::once(column).chain(name)
        })
        .map(|c| string(&c))
        .collect::<Vec<_>>()
        .join(", ");
    let values = stored(entity).map(to_value).collect::<Vec<_>>().join(", ");
    let mut load = String::new();
    let mut at = 0;
    for f in stored(entity) {
        let _ = writeln!(load, "\tr.{} = {}", field(&f.name.name), read(at, f, project));
        at += if f.ty == FieldType::File { 2 } else { 1 };
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
        .filter(|f| matches!(f.ty, FieldType::Image | FieldType::File))
        .map(|f| {
            let column = entity.column_of(&f.name.name);
            let store = f.modifiers.store.as_deref().map(|s| format!(", Store: {}", string(s))).unwrap_or_default();
            let name = if f.ty == FieldType::File { format!(", Name: {}", string(&format!("{column}_name"))) } else { String::new() };
            format!("{{Column: {}{store}{name}}}", string(&column))
        })
        .collect();
    if !pictures.is_empty() {
        let _ = write!(out, "func (*{name}) Pictures() []clarity.PictureColumn {{\n\treturn []clarity.PictureColumn{{{}}}\n}}\n\n", pictures.join(", "));
    }
    // The required fields a save checks, each by its column and its label as written.
    let required: Vec<String> =
        crate::required_fields(project, entity).iter().map(|(column, label)| format!("{{Column: {}, Label: {}}}", string(column), string(label))).collect();
    if !required.is_empty() {
        let _ = write!(out, "func (*{name}) Required() []clarity.RequiredField {{\n\treturn []clarity.RequiredField{{{}}}\n}}\n\n", required.join(", "));
    }
    if let Some(column) = &version {
        let _ = write!(out, "func (*{name}) VersionColumn() string {{\n\treturn {}\n}}\n\n", string(column));
        let _ = write!(out, "func (r *{name}) GetVersion() int64 {{\n\treturn r.RowVersion\n}}\n\n");
        let _ = write!(out, "func (r *{name}) SetVersion(version int64) {{\n\tr.RowVersion = version\n}}\n\n");
        if let Some(key) = &entity.saved_since {
            out.push_str(&saved_since(project, entity, key));
        }
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
    // `writable when` (docs/APP-CAPABILITIES-PLAN.md step 4) is checked before a
    // save and a delete, by code or by a window, then the event written for it runs.
    let writable = entity.writable.is_some() && !entity.record && !entity.settings;
    let unwritten = [DataEvent::BeforeSave, DataEvent::BeforeDelete].into_iter().filter(|e| writable && !events.contains(e));
    for event in events.iter().copied().chain(unwritten) {
        let written = events.contains(&event);
        let checked = writable && matches!(event, DataEvent::BeforeSave | DataEvent::BeforeDelete);
        let function = local(&format!("{}_{}", event.word(), record_name(name)));
        let check = super::logic::access_function("writable", name);
        let body = match (checked, written) {
            (true, true) => format!("\tif err := {check}(db, r); err != nil {{\n\t\treturn err\n\t}}\n\treturn {function}(db, r)\n"),
            (true, false) => format!("\treturn {check}(db, r)\n"),
            _ => format!("\treturn {function}(db, r)\n"),
        };
        let _ = write!(out, "func (r *{name}) {}(db *clarity.Db) error {{\n{body}}}\n\n", pascal(event.word()));
    }
    // What cascades from it is deleted first, each child as any record is.
    let cascades = crate::cascades(project, name);
    if !cascades.is_empty() {
        let _ = writeln!(out, "// Cascade deletes the records that refer to this one with `cascade`, first.\nfunc (r *{name}) Cascade(db *clarity.Db) error {{");
        for (child, _, column) in &cascades {
            let _ = writeln!(
                out,
                "\t{{\n\t\tchildren, err := clarity.Find[{}](db, clarity.Query{{}}.Filter(clarity.Compare({column:?}, clarity.Eq, r.ID)))\n\t\tif err != nil {{\n\t\t\treturn err\n\t\t}}\n\t\tfor i := range children {{\n\t\t\tif err := clarity.Delete(db, &children[i]); err != nil {{\n\t\t\t\treturn err\n\t\t\t}}\n\t\t}}\n\t}}",
                child.name.name
            );
        }
        let _ = write!(out, "\treturn nil\n}}\n\n");
    }
    out
}

/// `savedSince @key` of a versioned entity: the refusal of a save someone
/// saved since, in the entity's words, its slots filled from the record as it
/// is stored now (`clarity_spec::versions::saved_since_slots`): its id, its
/// fields, and for each reference the id and the fields of the record it
/// refers to, read through db. A reference to none, or to a record that is
/// gone or not readable, leaves those slots empty; it never fails the refusal.
/// A field whose value no slot shows (a picture, a file, a list) fills none.
fn saved_since(project: &Project, entity: &Entity, key: &str) -> String {
    let mut slots = vec![("current.id".to_string(), "clarity.SlotInteger(r.ID, \"\")".to_string())];
    let mut refs = String::new();
    for f in &entity.fields {
        let path = format!("current.{}", f.name.name);
        let value = format!("r.{}", field(&f.name.name));
        if let FieldType::Ref { entity: target } = &f.ty {
            let mut inner = format!("text = text.\n\tWith({}, clarity.SlotInteger(*{value}, \"\")).\n\tWith({}, clarity.SlotInteger(*{value}, \"\"))\n", string(&path), string(&format!("{path}.id")));
            if let Some(target) = project.entity(&target.name) {
                // Named apart from the receiver r, db, text and the package clarity.
                let found = format!("{}Record", local(&f.name.name));
                let readable = if target.readable.is_some() {
                    format!(" && {}(db, &{found}) == nil", super::logic::access_function("readable", &target.name.name))
                } else {
                    String::new()
                };
                let mut fields = String::new();
                for t in &target.fields {
                    let _ = write!(fields, "{}", field_slot(t, &format!("{found}.{}", field(&t.name.name)), &format!("{path}.{}", t.name.name)));
                }
                if !fields.is_empty() {
                    let _ = write!(inner, "if {found}, err := clarity.Get[{}](db, *{value}); err == nil{readable} {{\n{}}}\n", target.name.name, indent(&fields));
                }
            }
            let _ = write!(refs, "\tif {value} != nil {{\n{}\t}}\n", indent(&indent(&inner)));
            continue;
        }
        if let Some(slot) = slot_of(f, &value) {
            slots.push((path, slot));
        }
    }
    let with: String = slots.iter().map(|(path, slot)| format!(".\n\t\tWith({}, {slot})", string(path))).collect();
    let body = if refs.is_empty() {
        format!("\treturn clarity.KeyText({}){with}\n", string(key))
    } else {
        format!("\ttext := clarity.KeyText({}){with}\n{refs}\treturn text\n", string(key))
    };
    format!(
        "// SavedSince is why a save of a {} someone saved since is refused, in its entity's words about it as stored now.\nfunc (r *{}) SavedSince(db *clarity.Db) clarity.Text {{\n{body}}}\n\n",
        record_name(&entity.name.name).replace('_', " "),
        entity.name.name,
    )
}

/// One tab more on every line of `code`.
fn indent(code: &str) -> String {
    code.lines().map(|l| if l.is_empty() { "\n".to_string() } else { format!("\t{l}\n") }).collect()
}

/// The slot of field `f`, whose value is `value`; none for a field no slot shows.
fn slot_of(f: &Field, value: &str) -> Option<String> {
    let format = string(f.modifiers.format.as_deref().unwrap_or_default());
    Some(match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => format!("clarity.SlotText({value})"),
        FieldType::Integer | FieldType::Id => format!("clarity.SlotInteger({value}, {format})"),
        FieldType::Duration => format!("clarity.SlotDuration({value})"),
        FieldType::Decimal { scale, .. } => format!("clarity.SlotDecimal({value}, {scale}, {format})"),
        FieldType::Money => format!("clarity.SlotMoney({value}, {format})"),
        FieldType::Date => format!("clarity.SlotDate({value}, {format})"),
        FieldType::DateTime => format!("clarity.SlotDateTime({value}, {format})"),
        FieldType::Bool => format!("clarity.SlotBool({value})"),
        FieldType::Enum { name } => format!("clarity.SlotEnum(string({value}), {})", string(&name.name)),
        FieldType::Ref { .. } | FieldType::Image | FieldType::File | FieldType::Values { .. } | FieldType::List { .. } | FieldType::Record { .. } => return None,
    })
}

/// The line that gives slot `path` the value of field `f` of a referred
/// record: a reference of it shows its id, and none when it refers to none.
fn field_slot(f: &Field, value: &str, path: &str) -> String {
    match &f.ty {
        FieldType::Ref { .. } => format!("if {value} != nil {{\n\ttext = text.With({}, clarity.SlotInteger(*{value}, \"\"))\n}}\n", string(path)),
        _ => slot_of(f, value).map(|slot| format!("text = text.With({}, {slot})\n", string(path))).unwrap_or_default(),
    }
}
