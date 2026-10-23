//! Enums and entities as C# enums and record classes.

use clarity_spec::Project;
use clarity_spec::code::{DataEvent, record_name};
use clarity_spec::model::{EnumDecl, Entity, Field, FieldType, Literal, SpecFile};

use super::names::{backing, field_type, pascal, property, string};
use super::texts::Texts;

/// The enums and records of `file`, in the application's `namespace`.
pub fn emit(project: &Project, file: &SpecFile, namespace: &str, texts: &Texts) -> String {
    let mut out = String::new();
    for decl in &file.enums {
        out.push_str(&enum_decl(decl, namespace, texts));
    }
    for entity in &file.entities {
        if entity.settings {
            out.push_str(&settings_class(project, entity, namespace));
        } else {
            out.push_str(&record(project, entity));
        }
    }
    out
}

/// An enum, and its labels for text: `Labels.Of(status)`, in the language
/// shown; the choices of a dropdown are made each time, so they follow it.
fn enum_decl(decl: &EnumDecl, namespace: &str, texts: &Texts) -> String {
    let name = &decl.name.name;
    let values: String = decl.variants.iter().map(|v| format!("    {},\n", v.name.name)).collect();
    let labels: String = decl
        .variants
        .iter()
        .map(|v| format!("        {name}.{} => {},\n", v.name.name, texts.code(&v.label)))
        .collect();
    let choices: String = decl
        .variants
        .iter()
        .map(|v| format!("        new(global::{namespace}.{name}.{}, {}),\n", v.name.name, texts.code(&v.label)))
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

public static partial class Choices
{{
    public static Choice[] {name} =>
    [
{choices}    ];
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
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme, Some(Literal::Str(s))) => Some(string(s)),
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme, _) => Some("\"\"".into()),
        (FieldType::Integer | FieldType::Id | FieldType::Duration, Some(Literal::Number(n))) => Some(format!("{n}L")),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => Some(format!("{n}m")),
        (FieldType::Bool, Some(Literal::Bool(true))) => Some("true".into()),
        (FieldType::Enum { name }, Some(Literal::Ident(v))) => Some(format!("{}.{v}", name.name)),
        (FieldType::List { .. }, _) => Some("[]".into()),
        _ => None,
    }
}

/// The person's settings (DESIGN 5.17) as one shared object: every
/// window's `Settings` is `Settings.Current`. `Start` takes back what was
/// kept and still fits (Clarity.Avalonia's KeptSettings), shows its language
/// and theme, keeps every change, and follows the pickers.
fn settings_class(project: &Project, entity: &Entity, program: &str) -> String {
    let name = &entity.name.name;
    let mut properties = String::new();
    let mut described = String::new();
    let mut taken = String::new();
    let mut json = String::new();
    let mut follow = String::new();
    let mut shown = String::new();
    for f in &entity.fields {
        let ty = field_type(&f.ty);
        let prop = property(name, &f.name.name);
        let field = backing(&f.name.name);
        let key = string(&f.name.name);
        let init = initial(f).map(|v| format!(" = {v}")).unwrap_or_default();
        properties.push_str(&format!("    public {ty} {prop} {{ get => {field}; set => SetProperty(ref {field}, value); }}
    {ty} {field}{init};
"));
        let bound = |l: &Option<Literal>| match l {
            Some(Literal::Number(n)) => string(n),
            _ => "null".into(),
        };
        let (tag, values) = match &f.ty {
            FieldType::Text { .. } => ("text", "null".to_string()),
            FieldType::Enum { name } => ("enum", format!("Enum.GetNames<{}>()", name.name)),
            FieldType::Language => ("language", "Locale.Offered".into()),
            FieldType::Theme => ("theme", "[.. Themes.Offered.Select(t => t.Name)]".into()),
            FieldType::DateTime => ("datetime", "null".into()),
            other => (clarity_spec::widgets::field_type_tag(other), "null".into()),
        };
        described.push_str(&format!(
            "        [{key}] = new({}, {}, {}, {values}),
",
            string(tag),
            bound(&f.modifiers.min),
            bound(&f.modifiers.max)
        ));
        let read = match &f.ty {
            FieldType::Integer | FieldType::Duration => "v.GetValue<long>()".to_string(),
            FieldType::Decimal { .. } | FieldType::Money => "decimal.Parse(v.GetValue<string>(), CultureInfo.InvariantCulture)".into(),
            FieldType::Bool => "v.GetValue<bool>()".into(),
            FieldType::Date => "DateOnly.ParseExact(v.GetValue<string>(), \"yyyy-MM-dd\", CultureInfo.InvariantCulture)".into(),
            FieldType::DateTime => "DateTime.Parse(v.GetValue<string>(), CultureInfo.InvariantCulture)".into(),
            FieldType::Enum { name } => format!("Enum.Parse<{}>(v.GetValue<string>())", name.name),
            _ => "v.GetValue<string>()".into(),
        };
        taken.push_str(&format!("        if (kept[{key}] is {{ }} {field}Kept) settings.{prop} = {};
", read.replace("v.", &format!("{field}Kept."))));
        let written = match &f.ty {
            FieldType::Decimal { .. } | FieldType::Money => format!("{prop}.ToString(CultureInfo.InvariantCulture)"),
            FieldType::Date => format!("{prop}?.ToString(\"yyyy-MM-dd\", CultureInfo.InvariantCulture)"),
            FieldType::DateTime => format!("{prop}?.ToString(\"yyyy-MM-ddTHH:mm:ss\", CultureInfo.InvariantCulture)"),
            FieldType::Enum { .. } => format!("{prop}.ToString()"),
            _ => prop.clone(),
        };
        json.push_str(&format!("        [{key}] = {written},
"));
        match &f.ty {
            FieldType::Language => {
                shown.push_str(&format!("        if (!kept.ContainsKey({key})) settings.{prop} = Locale.Current;
        else if (settings.{prop} != Locale.Current) Locale.Choose(settings.{prop});
"));
                follow.push_str(&format!(
                    "            // The views are rebuilt for a language, so the input that changed it is done first.
            if (e.PropertyName == nameof({prop}) && settings.{prop} != Locale.Current) Dispatcher.UIThread.Post(() => Locale.Choose(settings.{prop}));
"
                ));
                shown.push_str(&format!("        Locale.Changed += () => settings.{prop} = Locale.Current;
"));
            }
            FieldType::Theme => {
                shown.push_str(&format!("        if (!kept.ContainsKey({key})) settings.{prop} = Themes.Current ?? \"\";
        else if (settings.{prop} != Themes.Current) Themes.Choose(settings.{prop});
"));
                follow.push_str(&format!("            if (e.PropertyName == nameof({prop}) && settings.{prop} != Themes.Current) Themes.Choose(settings.{prop});
"));
                shown.push_str(&format!("        Themes.Changed += () => settings.{prop} = Themes.Current ?? \"\";
"));
            }
            _ => {}
        }
    }
    let _ = project;
    format!(
        "/// <summary>The person's settings: one for the whole application, shared by every window, kept on their machine as they change.</summary>
public sealed partial class {name} : ObservableObject
{{
    const string ProgramName = {program};

    /// <summary>The settings shown, the same in every window.</summary>
    public static {name} Current {{ get; private set; }} = new();

{properties}
    /// <summary>Takes back what was kept and still fits, shows its language and theme, and keeps every change.</summary>
    public static void Start()
    {{
        var settings = new {name}();
        var kept = KeptSettings.Taken(Described(), KeptSettings.Read(ProgramName));
{taken}        Current = settings;
{shown}        settings.PropertyChanged += (_, e) =>
        {{
            KeptSettings.Keep(ProgramName, settings.ToJson());
{follow}        }};
    }}

    /// <summary>The settings as a kept value is checked against them.</summary>
    static Dictionary<string, Setting> Described() => new()
    {{
{described}    }};

    /// <summary>The settings as they are kept.</summary>
    public JsonObject ToJson() => new()
    {{
{json}    }};
}}

",
        program = string(program)
    )
}

/// How a field is stored: most values go as they are, enums by name.
fn to_sql(entity: &str, field: &Field) -> String {
    let value = property(entity, &field.name.name);
    match &field.ty {
        FieldType::Enum { .. } => format!("Sql.Name({value})"),
        FieldType::Image => format!("Sql.Picture({value})"),
        _ => value,
    }
}

fn from_sql(field: &Field) -> String {
    match &field.ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => "GetText".into(),
        FieldType::Integer | FieldType::Id | FieldType::Duration => "GetInteger".into(),
        FieldType::Decimal { .. } | FieldType::Money => "GetDecimal".into(),
        FieldType::Bool => "GetBoolean".into(),
        FieldType::Date => "GetDate".into(),
        FieldType::DateTime => "GetDateTime".into(),
        FieldType::Enum { name } => format!("GetName<{}>", name.name),
        FieldType::Ref { .. } => "GetReference".into(),
        FieldType::Image => "GetPicture".into(),
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
            let ty = field_type(&f.ty);
            let prop = property(name, &f.name.name);
            let field = backing(&f.name.name);
            let init = initial(f).map(|v| format!(" = {v}")).unwrap_or_default();
            format!("    public {ty} {prop} {{ get => {field}; set => SetProperty(ref {field}, value); }}\n    {ty} {field}{init};\n")
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
        "public sealed partial class {name} : ObservableObject, IRecord<{name}>
{{
    public static string SourceName => {source};
    public static string TableName => {table};
    public static string KeyColumn => {key};
    public static IReadOnlyList<string> ColumnNames {{ get; }} = [{columns}];
{pictures}
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
        pictures = pictures(entity),
        table = string(&entity.table_name()),
        key = string(entity.key_column()),
    )
}

/// The columns of an entity's image fields, with the file store of each.
fn pictures(entity: &Entity) -> String {
    let columns: Vec<String> = entity
        .fields
        .iter()
        .filter(|f| f.ty == FieldType::Image)
        .map(|f| {
            let store = f.modifiers.store.as_deref().map_or("null".to_string(), string);
            format!("new({}, {store})", string(entity.column_of(&f.name.name)))
        })
        .collect();
    if columns.is_empty() {
        return String::new();
    }
    format!("    public static IReadOnlyList<PictureColumn> Pictures {{ get; }} = [{}];\n", columns.join(", "))
}
