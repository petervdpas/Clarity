//! Enums and entities as C++ types, for the whole project in `records.hpp`
//! and `records.cpp`: modules that use each other need no includes between
//! them, since a record refers to another by its id. An enum is an `enum
//! class` with its names and labels; an entity is a struct with its
//! defaults, what the runtime's `Db` asks of a record, and its JSON as the
//! renderer reads it.

use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::{DataEvent, record_name};
use clarity_spec::model::{EnumDecl, Entity, Field, FieldType, Literal};

use super::names::{decimal, field_type, ident, snake, string};

/// The declarations and the definitions of every enum and record.
pub fn emit(project: &Project) -> (String, String) {
    let (mut header, mut source) = (String::new(), String::new());
    for decl in project.files.iter().flat_map(|f| &f.enums) {
        let (h, s) = enum_decl(decl);
        header.push_str(&h);
        source.push_str(&s);
    }
    // Every record named first, for the lists of records some hold.
    for entity in project.entities() {
        let _ = writeln!(header, "struct {};", entity.name.name);
    }
    header.push('\n');
    // A record held inside another is declared first (DESIGN 5.25).
    for entity in crate::holding_order(project) {
        let (h, s) = record(project, entity);
        header.push_str(&h);
        source.push_str(&s);
    }
    (header, source)
}

/// The C++ name of an enum value: `OrderStatus::Approved`.
pub fn constant(enum_name: &str, value: &str) -> String {
    format!("{enum_name}::{}", ident(value))
}

/// The function that reads an enum value back from its name.
pub fn from_name(enum_name: &str) -> String {
    format!("{}_from", snake(enum_name))
}

fn enum_decl(decl: &EnumDecl) -> (String, String) {
    let name = &decl.name.name;
    let values: Vec<String> = decl.variants.iter().map(|v| ident(&v.name.name)).collect();
    let header = format!(
        "enum class {name} {{ {} }};\n/// The value's name, as it is stored and travels.\nstd::string name_of({name} value);\n/// The value as people read it.\nstd::string label_of({name} value);\n/// The value of a name; the first value for a name that is none.\n{name} {from}(const std::string& name);\nvoid to_json(nlohmann::json& out, {name} value);\nvoid from_json(const nlohmann::json& in, {name}& value);\n\n",
        values.join(", "),
        from = from_name(name)
    );
    let mut names = String::new();
    let mut labels = String::new();
    let mut reads = String::new();
    for v in &decl.variants {
        let c = constant(name, &v.name.name);
        let _ = writeln!(names, "    case {c}: return {};", string(&v.name.name));
        let _ = writeln!(labels, "    case {c}: return {};", string(&v.label));
        let _ = writeln!(reads, "    if (name == {}) return {c};", string(&v.name.name));
    }
    let first = decl.variants.first().map(|v| constant(name, &v.name.name)).unwrap_or_else(|| format!("{name}{{}}"));
    let source = format!(
        "std::string name_of({name} value) {{\n    switch (value) {{\n{names}    }}\n    return \"\";\n}}\n\nstd::string label_of({name} value) {{\n    switch (value) {{\n{labels}    }}\n    return \"\";\n}}\n\n{name} {from}(const std::string& name) {{\n{reads}    return {first};\n}}\n\nvoid to_json(nlohmann::json& out, {name} value) {{ out = name_of(value); }}\n\nvoid from_json(const nlohmann::json& in, {name}& value) {{ value = {from}(in.is_string() ? in.get<std::string>() : std::string()); }}\n\n",
        from = from_name(name)
    );
    (header, source)
}

/// Fields that are stored in the entity's own table.
pub fn stored(entity: &Entity) -> impl Iterator<Item = &Field> {
    entity.fields.iter().filter(|f| !f.ty.is_held())
}

/// The value a new record starts with, when it is not the type's own.
fn initial(project: &Project, f: &Field) -> Option<String> {
    match (&f.ty, f.modifiers.default.as_ref()) {
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret, Some(Literal::Str(s))) => Some(string(s)),
        (FieldType::Integer | FieldType::Id | FieldType::Duration, Some(Literal::Number(n))) => Some(n.clone()),
        (FieldType::Integer | FieldType::Id | FieldType::Duration, _) => Some("0".into()),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => Some(decimal(n)),
        (FieldType::Bool, Some(Literal::Bool(b))) => Some(b.to_string()),
        (FieldType::Bool, _) => Some("false".into()),
        (FieldType::Enum { name }, Some(Literal::Ident(v))) => Some(constant(&name.name, v)),
        // An enum without a default starts with its first value, as on every stack.
        (FieldType::Enum { name }, _) => project.enum_decl(&name.name).and_then(|d| d.variants.first()).map(|v| constant(&name.name, &v.name.name)),
        _ => None,
    }
}

/// A field as the runtime stores it.
fn to_value(f: &Field) -> String {
    let value = ident(&f.name.name);
    match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => format!("clarity::sql::text({value})"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => format!("clarity::sql::integer({value})"),
        FieldType::Decimal { .. } | FieldType::Money => format!("clarity::sql::decimal({value})"),
        FieldType::Bool => format!("clarity::sql::boolean({value})"),
        FieldType::Date => format!("clarity::sql::date({value})"),
        FieldType::DateTime => format!("clarity::sql::date_time({value})"),
        FieldType::Enum { .. } => format!("clarity::sql::text(name_of({value}))"),
        FieldType::Ref { .. } => format!("clarity::sql::reference({value})"),
        FieldType::Image => format!("clarity::sql::picture({value})"),
        // Its content, then its name in the column beside it.
        FieldType::File => format!("clarity::file_content({value}), clarity::file_name_value({value})"),
        FieldType::Values { .. } => format!("clarity::sql::list({value})"),
        FieldType::List { .. } | FieldType::Record { .. } => unreachable!("held records are not stored in the table"),
    }
}

/// A stored value read back as the field.
fn read(i: usize, f: &Field) -> String {
    let column = string(&f.name.name);
    let get = |what: &str| format!("clarity::sql::get_{what}({column}, values[{i}])");
    match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => get("text"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => get("integer"),
        FieldType::Decimal { .. } | FieldType::Money => get("decimal"),
        FieldType::Bool => get("boolean"),
        FieldType::Date => get("date"),
        FieldType::DateTime => get("date_time"),
        FieldType::Ref { .. } => get("reference"),
        FieldType::Image => get("picture"),
        FieldType::File => format!("clarity::get_file({column}, values[{i}], values[{}])", i + 1),
        FieldType::Enum { name } => format!("{}({})", from_name(&name.name), get("text")),
        FieldType::Values { .. } => format!("{}.get<{}>()", get("list"), field_type(&f.ty)),
        FieldType::List { .. } | FieldType::Record { .. } => unreachable!("held records are not stored in the table"),
    }
}

/// A field in the record's JSON.
fn json_of(f: &Field) -> String {
    let value = format!("r.{}", ident(&f.name.name));
    match &f.ty {
        // An empty date or reference is null.
        FieldType::Date | FieldType::DateTime | FieldType::Ref { .. } => format!("{value} ? nlohmann::json(*{value}) : nlohmann::json(nullptr)"),
        _ => format!("nlohmann::json({value})"),
    }
}

/// A field read from the record's JSON.
fn json_read(f: &Field) -> String {
    let inner = match &f.ty {
        FieldType::Date => "clarity::Date".to_string(),
        FieldType::DateTime => "clarity::DateTime".to_string(),
        FieldType::Ref { .. } => "int64_t".to_string(),
        other => field_type(other),
    };
    format!("in[{}].get<{inner}>()", string(&f.name.name))
}

fn record(project: &Project, entity: &Entity) -> (String, String) {
    let name = &entity.name.name;
    let mut fields = String::new();
    for f in &entity.fields {
        let init = initial(project, f).map(|v| format!(" = {v}")).unwrap_or_default();
        let _ = writeln!(fields, "    {} {}{init};", field_type(&f.ty), ident(&f.name.name));
    }
    // A file has the column of its name beside its own.
    let columns: Vec<String> = stored(entity)
        .flat_map(|f| {
            let column = entity.column_of(&f.name.name).to_string();
            let name = (f.ty == FieldType::File).then(|| format!("{column}_name"));
            std::iter::once(column).chain(name)
        })
        .map(|c| string(&c))
        .collect();
    let pictures: Vec<String> = entity
        .fields
        .iter()
        .filter(|f| matches!(f.ty, FieldType::Image | FieldType::File))
        .map(|f| {
            let column = entity.column_of(&f.name.name);
            let store = f.modifiers.store.as_deref().map_or("std::nullopt".to_string(), string);
            let name = if f.ty == FieldType::File { format!(", {}", string(&format!("{column}_name"))) } else { String::new() };
            format!("{{{}, {store}{name}}}", string(column))
        })
        .collect();
    let source_name = project.source_of(name).unwrap_or_default();
    let mut statics = format!("    static constexpr const char* table = {};\n", string(&entity.table_name()));
    if !source_name.is_empty() && source_name != project.default_source() {
        let _ = writeln!(statics, "    static constexpr const char* source = {};", string(source_name));
    }
    if entity.key_column() != "id" {
        let _ = writeln!(statics, "    static constexpr const char* key = {};", string(entity.key_column()));
    }
    let events: Vec<DataEvent> =
        project.files.iter().flat_map(|f| &f.code).filter(|b| b.name.name == *name).flat_map(|b| b.events.iter().map(|e| e.event)).collect();
    let mut methods = String::from(
        "    static const std::vector<std::string>& columns();\n    std::vector<clarity::SqlValue> values() const;\n    static ",
    );
    let _ = writeln!(methods, "{name} from_values(int64_t id, const clarity::Row& values);");
    if !pictures.is_empty() {
        methods.push_str("    static std::vector<clarity::PictureColumn> pictures();\n");
    }
    for event in &events {
        let _ = writeln!(methods, "    void {}(clarity::Db& db);", event.word());
    }
    // What cascades from it is deleted first, each child as any record is.
    let cascades = crate::cascades(project, name);
    if !cascades.is_empty() {
        methods.push_str("    void cascade(clarity::Db& db) const;\n");
    }
    let header = format!(
        "struct {name} {{\n    int64_t id = 0;\n{fields}\n{statics}{methods}}};\nvoid to_json(nlohmann::json& out, const {name}& r);\nvoid from_json(const nlohmann::json& in, {name}& r);\n\n"
    );

    let mut source = String::new();
    if !cascades.is_empty() {
        let _ = writeln!(source, "void {name}::cascade(clarity::Db& db) const {{");
        for (child, _, column) in &cascades {
            let _ = writeln!(
                source,
                "    for (const auto& child : db.find<{}>(clarity::Query().filter(clarity::Cond::compare({}, clarity::Op::Eq, clarity::sql::integer(id))))) db.remove(child);",
                child.name.name,
                string(column)
            );
        }
        source.push_str("}\n\n");
    }
    let _ = write!(
        source,
        "const std::vector<std::string>& {name}::columns() {{\n    static const std::vector<std::string> names{{{}}};\n    return names;\n}}\n\n",
        columns.join(", ")
    );
    let values: Vec<String> = stored(entity).map(to_value).collect();
    let _ = write!(source, "std::vector<clarity::SqlValue> {name}::values() const {{\n    return {{{}}};\n}}\n\n", values.join(", "));
    let mut reads = String::new();
    let mut i = 0;
    for f in stored(entity) {
        let _ = writeln!(reads, "    r.{} = {};", ident(&f.name.name), read(i, f));
        i += if f.ty == FieldType::File { 2 } else { 1 };
    }
    let _ = write!(source, "{name} {name}::from_values(int64_t id, const clarity::Row& values) {{\n    {name} r;\n    r.id = id;\n{reads}    return r;\n}}\n\n");
    if !pictures.is_empty() {
        let _ = write!(source, "std::vector<clarity::PictureColumn> {name}::pictures() {{\n    return {{{}}};\n}}\n\n", pictures.join(", "));
    }
    let mut to = String::new();
    let mut from = String::new();
    for f in &entity.fields {
        let key = string(&f.name.name);
        let _ = writeln!(to, "    out[{key}] = {};", json_of(f));
        let _ = writeln!(from, "    if (in.contains({key}) && !in[{key}].is_null()) r.{} = {};", ident(&f.name.name), json_read(f));
    }
    let _ = write!(
        source,
        "void to_json(nlohmann::json& out, const {name}& r) {{\n    out = nlohmann::json::object();\n    out[\"id\"] = r.id;\n{to}}}\n\nvoid from_json(const nlohmann::json& in, {name}& r) {{\n    r = {name}{{}};\n    if (in.contains(\"id\") && in[\"id\"].is_number()) r.id = in[\"id\"].get<int64_t>();\n{from}}}\n\n"
    );
    let _ = record_name;
    (header, source)
}
