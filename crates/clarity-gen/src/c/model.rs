//! Enums and entities as C types, for the whole project in `records.h` and
//! `records.c`: modules that use each other need no includes between them,
//! since a record refers to another by its id. An enum is an `enum` with
//! its names and labels; an entity is a struct, its description for the
//! runtime (`customer_entity`), typed calls such as `customer_save` and
//! `customer_find`, and its JSON as the renderer reads it.

use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::{DataEvent, record_name};
use clarity_spec::model::{EnumDecl, Entity, Field, FieldType, Literal};

use super::names::{constant, decimal, field_type, ident, list_type, prefix, string, string_or_null};

/// The declarations and the definitions of every enum and record.
pub fn emit(project: &Project) -> (String, String) {
    let (mut header, mut source) = (String::new(), String::new());
    for decl in project.files.iter().flat_map(|f| &f.enums) {
        let (h, s) = enum_decl(decl);
        header.push_str(&h);
        source.push_str(&s);
    }
    // Every record named first, with its list, for the lists of records some hold.
    for entity in project.entities() {
        let name = &entity.name.name;
        let _ = writeln!(header, "typedef struct {name} {name};");
    }
    for entity in project.entities() {
        let name = &entity.name.name;
        let _ = writeln!(header, "/// Records of {name}, in the scratch arena.\ntypedef struct {{\n    {name}* items;\n    size_t count;\n}} {};", list_type(name));
    }
    header.push('\n');
    for entity in project.entities() {
        let (h, s) = record(project, entity);
        header.push_str(&h);
        source.push_str(&s);
    }
    (header, source)
}

fn enum_decl(decl: &EnumDecl) -> (String, String) {
    let name = &decl.name.name;
    let p = prefix(name);
    let values: Vec<String> = decl.variants.iter().map(|v| constant(name, &v.name.name)).collect();
    let header = format!(
        "typedef enum {{ {} }} {name};\n/// The value's name, as it is stored and travels.\nconst char* {p}_name({name} value);\n/// The value as people read it.\nconst char* {p}_label({name} value);\n/// The value of a name; the first value for a name that is none.\n{name} {p}_from(const char* name);\n\n",
        values.join(", ")
    );
    let mut names = String::new();
    let mut labels = String::new();
    let mut reads = String::new();
    for v in &decl.variants {
        let c = constant(name, &v.name.name);
        let _ = writeln!(names, "    case {c}: return {};", string(&v.name.name));
        let _ = writeln!(labels, "    case {c}: return {};", string(&v.label));
        let _ = writeln!(reads, "    if (strcmp(name, {}) == 0) return {c};", string(&v.name.name));
    }
    let first = values.first().cloned().unwrap_or_else(|| "0".into());
    let source = format!(
        "const char* {p}_name({name} value) {{\n    switch (value) {{\n{names}    }}\n    return \"\";\n}}\n\nconst char* {p}_label({name} value) {{\n    switch (value) {{\n{labels}    }}\n    return \"\";\n}}\n\n{name} {p}_from(const char* name) {{\n    if (!name) return {first};\n{reads}    return {first};\n}}\n\n"
    );
    (header, source)
}

/// Fields that are stored in the entity's own table.
pub fn stored(entity: &Entity) -> impl Iterator<Item = &Field> {
    entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. }))
}

/// The value a new record starts with, when it is not zero.
fn initial(project: &Project, f: &Field) -> Option<String> {
    match (&f.ty, f.modifiers.default.as_ref()) {
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme, Some(Literal::Str(s))) => Some(string(s)),
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme, _) => Some("\"\"".into()),
        (FieldType::Integer | FieldType::Id | FieldType::Duration, Some(Literal::Number(n))) => Some(n.clone()),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => Some(decimal(n)),
        (FieldType::Bool, Some(Literal::Bool(true))) => Some("true".into()),
        (FieldType::Enum { name }, Some(Literal::Ident(v))) => Some(constant(&name.name, v)),
        // An enum without a default starts with its first value, as on every stack.
        (FieldType::Enum { name }, _) => project.enum_decl(&name.name).and_then(|d| d.variants.first()).map(|v| constant(&name.name, &v.name.name)),
        _ => None,
    }
}

/// A field as the runtime stores it.
fn to_value(f: &Field) -> String {
    let value = format!("r->{}", ident(&f.name.name));
    match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => format!("clarity_text_value({value})"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => format!("clarity_integer_value({value})"),
        FieldType::Decimal { .. } | FieldType::Money => format!("clarity_decimal_value({value})"),
        FieldType::Bool => format!("clarity_bool_value({value})"),
        FieldType::Date => format!("clarity_date_value({value})"),
        FieldType::DateTime => format!("clarity_date_time_value({value})"),
        FieldType::Enum { name } => format!("clarity_text_value({}_name({value}))", prefix(&name.name)),
        FieldType::Ref { .. } => format!("clarity_reference_value({value})"),
        FieldType::Image => format!("clarity_picture_value({value})"),
        FieldType::List { .. } => unreachable!("list fields are not stored in the table"),
    }
}

/// The lines that read stored value `i` back as the field.
fn read(i: usize, f: &Field) -> String {
    let column = string(&f.name.name);
    let target = format!("r->{}", ident(&f.name.name));
    let get = |what: &str| format!("    if (!clarity_get_{what}({column}, values[{i}], &{target})) return false;\n");
    match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => get("text"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => get("integer"),
        FieldType::Decimal { .. } | FieldType::Money => get("decimal"),
        FieldType::Bool => get("bool"),
        FieldType::Date => get("date"),
        FieldType::DateTime => get("date_time"),
        FieldType::Ref { .. } => get("reference"),
        FieldType::Image => get("picture"),
        FieldType::Enum { name } => format!(
            "    const char* {field}_name = NULL;\n    if (!clarity_get_text({column}, values[{i}], &{field}_name)) return false;\n    {target} = {}_from({field}_name);\n",
            prefix(&name.name),
            field = ident(&f.name.name)
        ),
        FieldType::List { .. } => unreachable!("list fields are not stored in the table"),
    }
}

/// A field in the record's JSON.
fn json_of(f: &Field) -> String {
    let value = format!("r->{}", ident(&f.name.name));
    match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => format!("clarity_json_text({value})"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => format!("clarity_json_integer({value})"),
        FieldType::Decimal { .. } | FieldType::Money => format!("clarity_json_decimal({value})"),
        FieldType::Bool => format!("clarity_json_bool({value})"),
        FieldType::Date => format!("clarity_json_date({value})"),
        FieldType::DateTime => format!("clarity_json_date_time({value})"),
        FieldType::Enum { name } => format!("clarity_json_text({}_name({value}))", prefix(&name.name)),
        FieldType::Ref { .. } => format!("clarity_json_reference({value})"),
        FieldType::Image => format!("clarity_picture_json({value})"),
        FieldType::List { item } => format!("{}_list_json(&{value})", prefix(&item.name)),
    }
}

/// The lines that read a field from the record's JSON; a missing field keeps its default.
fn json_read(f: &Field) -> String {
    let key = string(&f.name.name);
    let target = format!("r->{}", ident(&f.name.name));
    let get = |what: &str| format!("    if (!clarity_json_get_{what}(in, {key}, &{target})) return false;\n");
    match &f.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => get("text"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => get("integer"),
        FieldType::Decimal { .. } | FieldType::Money => get("decimal"),
        FieldType::Bool => get("bool"),
        FieldType::Date => get("date"),
        FieldType::DateTime => get("date_time"),
        FieldType::Ref { .. } => get("reference"),
        FieldType::Image => get("picture"),
        FieldType::Enum { name } => format!(
            "    const char* {field}_name = NULL;\n    if (!clarity_json_get_text(in, {key}, &{field}_name)) return false;\n    if ({field}_name) {target} = {}_from({field}_name);\n",
            prefix(&name.name),
            field = ident(&f.name.name)
        ),
        FieldType::List { item } => format!(
            "    const cJSON* {field}_items = cJSON_GetObjectItemCaseSensitive(in, {key});\n    if ({field}_items && !cJSON_IsNull({field}_items) && !{}_list_from_json({field}_items, &{target})) return false;\n",
            prefix(&item.name),
            field = ident(&f.name.name)
        ),
    }
}

fn record(project: &Project, entity: &Entity) -> (String, String) {
    let name = &entity.name.name;
    let p = prefix(name);
    let list = list_type(name);
    let mut fields = String::new();
    for f in &entity.fields {
        let _ = writeln!(fields, "    {} {};", field_type(&f.ty), ident(&f.name.name));
    }
    let events: Vec<DataEvent> =
        project.files.iter().flat_map(|f| &f.code).filter(|b| b.name.name == *name).flat_map(|b| b.events.iter().map(|e| e.event)).collect();
    let header = format!(
        "struct {name} {{\n    int64_t id;\n{fields}}};\n\n/// How the runtime reads and writes records of {name}.\nextern const ClarityEntity {p}_entity;\n/// A new record, with the defaults of its fields.\n{name} {p}_new(void);\n/// Inserts a new record (and gives it its id) or updates a stored one.\nbool {p}_save(ClarityDb* db, {name}* record);\nbool {p}_delete(ClarityDb* db, const {name}* record);\nbool {p}_find(ClarityDb* db, const ClarityQuery* query, {list}* out);\nbool {p}_all(ClarityDb* db, {list}* out);\nbool {p}_get(ClarityDb* db, int64_t id, {name}* out);\n/// The record a reference field points at.\nbool {p}_load(ClarityDb* db, int64_t reference, {name}* out);\ncJSON* {p}_json(const {name}* record);\nbool {p}_from_json(const cJSON* in, {name}* out);\ncJSON* {p}_list_json(const {list}* list);\nbool {p}_list_from_json(const cJSON* in, {list}* out);\n\n"
    );

    let mut source = String::new();
    let columns: Vec<String> = stored(entity).map(|f| string(entity.column_of(&f.name.name))).collect();
    if !columns.is_empty() {
        let _ = write!(source, "static const char* const {p}_columns[] = {{{}}};\n\n", columns.join(", "));
    }
    let pictures: Vec<String> = entity
        .fields
        .iter()
        .filter(|f| f.ty == FieldType::Image)
        .map(|f| format!("{{{}, {}}}", string(entity.column_of(&f.name.name)), string_or_null(f.modifiers.store.as_deref())))
        .collect();
    if !pictures.is_empty() {
        let _ = write!(source, "static const ClarityPictureColumn {p}_pictures[] = {{{}}};\n\n", pictures.join(", "));
    }
    let mut values = String::new();
    let mut reads = String::new();
    for (i, f) in stored(entity).enumerate() {
        let _ = writeln!(values, "    out[{i}] = {};", to_value(f));
        reads.push_str(&read(i, f));
    }
    let used = if values.is_empty() { "    (void)record;\n    (void)out;\n" } else { "" };
    let r = if values.is_empty() { "" } else { "    const {name}* r = record;\n" }.replace("{name}", name);
    let _ = write!(source, "static void {p}_values(const void* record, ClarityValue* out) {{\n{used}{r}{values}}}\n\n");
    let unused_values = if reads.is_empty() { "    (void)values;\n" } else { "" };
    let _ = write!(
        source,
        "static bool {p}_from_values(int64_t id, const ClarityValue* values, void* record) {{\n{unused_values}    {name}* r = record;\n    *r = {p}_new();\n    r->id = id;\n{reads}    return true;\n}}\n\n"
    );
    // Each data event runs the action Clarity's code block wrote for it.
    let record_word = record_name(name);
    for event in &events {
        let word = event.word();
        let _ = write!(
            source,
            "static bool {p}_{word}(ClarityDb* db, void* record) {{\n    return {}(db, record);\n}}\n\n",
            ident(&format!("{word}_{record_word}"))
        );
    }
    let source_name = project.source_of(name).unwrap_or_default();
    let source_name = if source_name.is_empty() || source_name == project.default_source() { "" } else { source_name };
    let mut descriptor = format!(
        "const ClarityEntity {p}_entity = {{\n    .name = {},\n    .table = {},\n    .key = {},\n    .source = {},\n",
        string(name),
        string(&entity.table_name()),
        string(entity.key_column()),
        string(source_name)
    );
    if !columns.is_empty() {
        let _ = writeln!(descriptor, "    .columns = {p}_columns,\n    .column_count = {},", columns.len());
    }
    if !pictures.is_empty() {
        let _ = writeln!(descriptor, "    .pictures = {p}_pictures,\n    .picture_count = {},", pictures.len());
    }
    let _ = writeln!(descriptor, "    .size = sizeof({name}),\n    .id_at = offsetof({name}, id),\n    .values = {p}_values,\n    .from_values = {p}_from_values,");
    for event in &events {
        let _ = writeln!(descriptor, "    .{word} = {p}_{word},", word = event.word());
    }
    descriptor.push_str("};\n\n");
    source.push_str(&descriptor);

    let defaults: Vec<String> = entity.fields.iter().filter_map(|f| initial(project, f).map(|v| format!("        .{} = {v},", ident(&f.name.name)))).collect();
    let new = if defaults.is_empty() { format!("    return ({name}){{0}};\n") } else { format!("    return ({name}){{\n{}\n    }};\n", defaults.join("\n")) };
    let _ = write!(
        source,
        "{name} {p}_new(void) {{\n{new}}}\n\nbool {p}_save(ClarityDb* db, {name}* record) {{ return clarity_save(db, &{p}_entity, record); }}\n\nbool {p}_delete(ClarityDb* db, const {name}* record) {{ return clarity_delete(db, &{p}_entity, record); }}\n\nbool {p}_find(ClarityDb* db, const ClarityQuery* query, {list}* out) {{\n    void* items = NULL;\n    size_t count = 0;\n    if (!clarity_find(db, &{p}_entity, query, &items, &count)) return false;\n    *out = ({list}){{items, count}};\n    return true;\n}}\n\nbool {p}_all(ClarityDb* db, {list}* out) {{ return {p}_find(db, clarity_query(), out); }}\n\nbool {p}_get(ClarityDb* db, int64_t id, {name}* out) {{ return clarity_get(db, &{p}_entity, id, out); }}\n\nbool {p}_load(ClarityDb* db, int64_t reference, {name}* out) {{ return clarity_load(db, &{p}_entity, reference, out); }}\n\n"
    );
    let mut to = String::new();
    let mut from = String::new();
    for f in &entity.fields {
        let _ = writeln!(to, "    cJSON_AddItemToObject(out, {}, {});", string(&f.name.name), json_of(f));
        from.push_str(&json_read(f));
    }
    let _ = write!(
        source,
        "cJSON* {p}_json(const {name}* r) {{\n    cJSON* out = cJSON_CreateObject();\n    cJSON_AddItemToObject(out, \"id\", clarity_json_integer(r->id));\n{to}    return out;\n}}\n\nbool {p}_from_json(const cJSON* in, {name}* r) {{\n    *r = {p}_new();\n    if (!clarity_json_get_integer(in, \"id\", &r->id)) return false;\n{from}    return true;\n}}\n\ncJSON* {p}_list_json(const {list}* list) {{\n    cJSON* out = cJSON_CreateArray();\n    for (size_t i = 0; i < list->count; i++) cJSON_AddItemToArray(out, {p}_json(&list->items[i]));\n    return out;\n}}\n\nbool {p}_list_from_json(const cJSON* in, {list}* out) {{\n    size_t count = (size_t)cJSON_GetArraySize(in);\n    {name}* items = CLARITY_NEW({name}, count);\n    size_t i = 0;\n    const cJSON* item;\n    cJSON_ArrayForEach(item, in) {{\n        if (!{p}_from_json(item, &items[i++])) return false;\n    }}\n    *out = ({list}){{items, count}};\n    return true;\n}}\n\n"
    );
    (header, source)
}
