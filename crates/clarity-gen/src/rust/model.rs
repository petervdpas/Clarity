//! Enums and entities as Rust enums and records.

use clarity_spec::Project;
use clarity_spec::code::{DataEvent, record_name};
use clarity_spec::model::{EnumDecl, Entity, Field, FieldType, Literal, SpecFile};

use super::names::{decimal, field_type, ident};

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

fn enum_decl(decl: &EnumDecl) -> String {
    let name = &decl.name.name;
    let mut out = format!(
        "#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]\npub enum {name} {{\n"
    );
    for (i, v) in decl.variants.iter().enumerate() {
        if i == 0 {
            out.push_str("    #[default]\n");
        }
        out.push_str(&format!("    {},\n", v.name.name));
    }
    out.push_str("}\n\n");
    let arms = |f: &dyn Fn(&str, &str) -> String| decl.variants.iter().map(|v| f(&v.name.name, &v.label)).collect::<String>();
    out.push_str(&format!(
        "impl std::fmt::Display for {name} {{\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        f.write_str(match self {{\n{}        }})\n    }}\n}}\n\n",
        arms(&|v, _| format!("            {name}::{v} => {v:?},\n"))
    ));
    out.push_str(&format!(
        "impl std::str::FromStr for {name} {{\n    type Err = ();\n    fn from_str(s: &str) -> std::result::Result<Self, ()> {{\n        match s {{\n{}            _ => Err(()),\n        }}\n    }}\n}}\n\n",
        arms(&|v, _| format!("            {v:?} => Ok({name}::{v}),\n"))
    ));
    out.push_str(&format!(
        "impl Show for {name} {{\n    fn show(&self) -> String {{\n        match self {{\n{}        }}\n        .to_string()\n    }}\n}}\n\n",
        arms(&|v, label| format!("            {name}::{v} => {label:?},\n"))
    ));
    out
}

/// Fields that are stored in the entity's own table.
pub fn stored(entity: &Entity) -> impl Iterator<Item = &Field> {
    entity.fields.iter().filter(|f| !f.ty.is_held())
}

fn default_value(project: &Project, field: &Field) -> String {
    let literal = field.modifiers.default.as_ref();
    match (&field.ty, literal) {
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret, Some(Literal::Str(s))) => format!("String::from({s:?})"),
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret, _) => "String::new()".into(),
        (FieldType::Integer | FieldType::Id | FieldType::Duration, Some(Literal::Number(n))) => format!("{n}i64"),
        (FieldType::Integer | FieldType::Id | FieldType::Duration, _) => "0".into(),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => decimal(n),
        (FieldType::Decimal { .. } | FieldType::Money, _) => "Decimal::ZERO".into(),
        (FieldType::Bool, Some(Literal::Bool(b))) => b.to_string(),
        (FieldType::Bool, _) => "false".into(),
        (FieldType::Enum { name }, Some(Literal::Ident(v))) => format!("{}::{v}", name.name),
        (FieldType::Enum { name }, _) => match project.enum_decl(&name.name) {
            Some(_) => format!("{}::default()", name.name),
            None => "Default::default()".into(),
        },
        (FieldType::List { .. } | FieldType::Values { .. }, _) => "Vec::new()".into(),
        // A record held inside starts as a new one (DESIGN 5.25).
        (FieldType::Record { entity }, _) => format!("{}::default()", entity.name),
        _ => "None".into(),
    }
}

fn to_sql(field: &Field) -> String {
    let value = format!("self.{}", ident(&field.name.name));
    match &field.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => format!("sql::text(&{value})"),
        FieldType::Integer | FieldType::Id | FieldType::Duration => format!("sql::integer({value})"),
        FieldType::Decimal { .. } | FieldType::Money => format!("sql::decimal({value})"),
        FieldType::Bool => format!("sql::boolean({value})"),
        FieldType::Date => format!("sql::date({value})"),
        FieldType::DateTime => format!("sql::date_time({value})"),
        FieldType::Enum { .. } => format!("sql::name(&{value})"),
        FieldType::Ref { .. } => format!("sql::reference({value})"),
        FieldType::Image => format!("sql::picture(&{value})"),
        // Its content, then its name in the column beside it.
        FieldType::File => format!("sql::file_content(&{value}),\n            sql::file_name(&{value})"),
        FieldType::Values { .. } => format!("sql::list(&{value})"),
        FieldType::List { .. } | FieldType::Record { .. } => unreachable!("held records are not stored in the table"),
    }
}

fn from_sql(field: &Field) -> &'static str {
    match &field.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => "get_text",
        FieldType::Integer | FieldType::Id | FieldType::Duration => "get_integer",
        FieldType::Decimal { .. } | FieldType::Money => "get_decimal",
        FieldType::Bool => "get_boolean",
        FieldType::Date => "get_date",
        FieldType::DateTime => "get_date_time",
        FieldType::Enum { .. } => "get_name",
        FieldType::Ref { .. } => "get_reference",
        FieldType::Image => "get_picture",
        FieldType::File => "get_file",
        FieldType::Values { .. } => "get_list",
        FieldType::List { .. } | FieldType::Record { .. } => unreachable!("held records are not stored in the table"),
    }
}

fn record(project: &Project, entity: &Entity) -> String {
    let name = &entity.name.name;
    let mut out = format!(
        "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\npub struct {name} {{\n    pub id: Id,\n"
    );
    for field in &entity.fields {
        out.push_str(&format!("    pub {}: {},\n", ident(&field.name.name), field_type(&field.ty)));
    }
    out.push_str("}\n\n");

    out.push_str(&format!("impl Default for {name} {{\n    fn default() -> Self {{\n        {name} {{\n            id: 0,\n"));
    for field in &entity.fields {
        out.push_str(&format!("            {}: {},\n", ident(&field.name.name), default_value(project, field)));
    }
    out.push_str("        }\n    }\n}\n\n");
    // The settings are kept per person, never in a database.
    if entity.settings {
        return out;
    }

    // A file has the column of its name beside its own.
    let columns = stored(entity)
        .flat_map(|f| {
            let column = entity.column_of(&f.name.name).to_string();
            let name = (f.ty == FieldType::File).then(|| format!("{column}_name"));
            std::iter::once(column).chain(name)
        })
        .map(|c| format!("{c:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let to_values: String = stored(entity).map(|f| format!("            {},\n", to_sql(f))).collect();
    let mut at = 0;
    let mut from_values = String::new();
    for f in stored(entity) {
        let values = if f.ty == FieldType::File { format!("&values[{at}], &values[{}]", at + 1) } else { format!("&values[{at}]") };
        from_values.push_str(&format!("            {}: sql::{}({:?}, {values})?,\n", ident(&f.name.name), from_sql(f), f.name.name));
        at += if f.ty == FieldType::File { 2 } else { 1 };
    }
    let lists: String = entity
        .fields
        .iter()
        .filter(|f| f.ty.is_held())
        .map(|f| match &f.ty {
            // A record held inside starts as a new one (DESIGN 5.25).
            FieldType::Record { entity } => format!("            {}: {}::default(),\n", ident(&f.name.name), entity.name),
            _ => format!("            {}: Vec::new(),\n", ident(&f.name.name)),
        })
        .collect();
    // The data events written in the entity's code block; each runs the
    // action generated from it.
    let events: Vec<DataEvent> = project
        .files
        .iter()
        .flat_map(|f| &f.code)
        .filter(|b| b.name.name == *name)
        .flat_map(|b| b.events.iter().map(|e| e.event))
        .collect();
    let mut hooks = String::new();
    let pictures: Vec<String> = entity
        .fields
        .iter()
        .filter(|f| matches!(f.ty, FieldType::Image | FieldType::File))
        .map(|f| {
            let column = entity.column_of(&f.name.name);
            let name = (f.ty == FieldType::File).then(|| format!("{column}_name"));
            format!("PictureColumn {{ column: {column:?}, store: {:?}, name: {name:?} }}", f.modifiers.store.as_deref())
        })
        .collect();
    let pictures = if pictures.is_empty() {
        String::new()
    } else {
        format!("    const PICTURES: &'static [PictureColumn] = &[{}];\n", pictures.join(", "))
    };
    // What cascades from it is deleted first, each child as any record is.
    let cascades = crate::cascades(project, name);
    if !cascades.is_empty() {
        hooks.push_str("\n    const CASCADES: bool = true;\n\n    fn cascade(db: &Db, id: Id) -> Result<()> {\n");
        for (child, file, column) in &cascades {
            let module = file.module.as_ref().map(|m| super::names::snake(&m.name)).unwrap_or_default();
            hooks.push_str(&format!(
                "        for child in db.find::<crate::modules::{module}::{}>(&clarity_runtime::query::Query::new().filter(clarity_runtime::query::Cond::compare({column:?}, clarity_runtime::query::Op::Eq, SqlValue::Integer(id))))? {{\n            db.delete(&child)?;\n        }}\n",
                child.name.name
            ));
        }
        hooks.push_str("        Ok(())\n    }\n");
    }
    if !events.is_empty() {
        hooks.push_str("\n    const EVENTS: bool = true;\n");
        for event in events {
            let function = ident(&format!("{}_{}", event.word(), record_name(name)));
            hooks.push_str(&format!(
                "\n    fn {}(&mut self, db: &Db) -> Result<()> {{\n        {function}(db, self)\n    }}\n",
                event.word()
            ));
        }
    }
    out.push_str(&format!(
        "impl Record for {name} {{
    const SOURCE: &'static str = {source:?};
    const TABLE: &'static str = {table:?};
    const KEY: &'static str = {key:?};
    const COLUMNS: &'static [&'static str] = &[{columns}];
{pictures}
    fn id(&self) -> Id {{
        self.id
    }}

    fn set_id(&mut self, id: Id) {{
        self.id = id;
    }}

    fn to_values(&self) -> Vec<SqlValue> {{
        vec![
{to_values}        ]
    }}

    fn from_values(id: Id, values: &[SqlValue]) -> Result<Self> {{
        Ok({name} {{
            id,
{from_values}{lists}        }})
    }}
{hooks}}}

",
        table = entity.table_name(),
        key = entity.key_column(),
        source = project.source_of(name).unwrap_or_default()
    ));
    out
}
