//! The database schema from the data dictionary, as the first migration
//! step. Later steps come from the migration support (D1).

use clarity_spec::Project;
use clarity_spec::model::{Field, FieldType, Literal};

use super::HEADER;
use super::model::stored;
use super::names::table;

fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn sql_text(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

fn column(project: &Project, field: &Field) -> String {
    let default = field.modifiers.default.as_ref();
    let name = quoted(&field.name.name);
    let mut column = match (&field.ty, default) {
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url, Some(Literal::Str(s))) => {
            format!("{name} TEXT NOT NULL DEFAULT {}", sql_text(s))
        }
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url, _) => format!("{name} TEXT NOT NULL DEFAULT ''"),
        (FieldType::Integer | FieldType::Id, Some(Literal::Number(n))) => format!("{name} INTEGER NOT NULL DEFAULT {n}"),
        (FieldType::Integer | FieldType::Id, _) => format!("{name} INTEGER NOT NULL DEFAULT 0"),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => {
            format!("{name} TEXT NOT NULL DEFAULT {}", sql_text(n))
        }
        (FieldType::Decimal { .. } | FieldType::Money, _) => format!("{name} TEXT NOT NULL DEFAULT '0'"),
        (FieldType::Bool, Some(Literal::Bool(b))) => format!("{name} INTEGER NOT NULL DEFAULT {}", u8::from(*b)),
        (FieldType::Bool, _) => format!("{name} INTEGER NOT NULL DEFAULT 0"),
        (FieldType::Enum { name: e }, default) => {
            let first = project.enum_decl(&e.name).and_then(|d| d.variants.first()).map(|v| v.name.name.clone());
            let value = match default {
                Some(Literal::Ident(v)) => Some(v.clone()),
                _ => first,
            };
            format!("{name} TEXT NOT NULL DEFAULT {}", sql_text(&value.unwrap_or_default()))
        }
        (FieldType::Date | FieldType::DateTime, _) => format!("{name} TEXT"),
        (FieldType::Ref { entity }, _) => format!("{name} INTEGER REFERENCES {} (\"id\")", quoted(&table(&entity.name))),
        (FieldType::List { .. }, _) => unreachable!("list fields are not stored in the table"),
    };
    if field.modifiers.unique {
        column.push_str(" UNIQUE");
    }
    column
}

pub fn emit(project: &Project) -> String {
    let mut step = String::new();
    for entity in project.entities() {
        let mut columns = vec!["\"id\" INTEGER PRIMARY KEY".to_string()];
        columns.extend(stored(entity).map(|f| column(project, f)));
        step.push_str(&format!(
            "CREATE TABLE {} (\n    {}\n);\n",
            quoted(&table(&entity.name.name)),
            columns.join(",\n    ")
        ));
    }
    format!(
        "{HEADER}
/// The migration steps, in order. The database remembers how many it has
/// applied, and applies the rest when the application starts.
pub const STEPS: &[&str] = &[
    // 1: the schema from the data dictionary.
    r#\"{step}\"#,
];
"
    )
}
