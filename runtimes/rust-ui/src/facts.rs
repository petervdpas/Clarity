//! What the program's model says beyond the window's values (DESIGN 7.9):
//! which windows are dialogs and the size each opens at, the facts of every
//! field of its entities (label, format, required, range, what it refers
//! to), the values of its enums with their labels, and the person's formats.
//! A generated program carries its model as JSON (`AppFiles::model`), the
//! same the web screen reads, so nothing more is generated for it.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::parts::{Choice, FieldFacts, Formats};

/// What the model says of a window: whether it is a dialog, its title as written, and the size it opens at.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WindowFacts {
    pub dialog: bool,
    pub title: Option<String>,
    pub size: Option<(f64, f64)>,
    pub columns: Option<i64>,
}

/// A field as the model declares it: its kind (`text`, `ref`, `enum`...), what it names (an enum, an entity), and its facts.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FieldOf {
    pub kind: String,
    pub names: Option<String>,
    pub facts: FieldFacts,
    /// The file store its files are kept in (`store photos`); none in the database.
    pub store: Option<String>,
    /// What a list of plain values holds: `text`, `integer`, an enum's name; none for another field.
    pub item: Option<String>,
}

/// What a window's data holds by its name: a list of records, one record, or
/// a plain value, with the name of its type when it is a named one (an enum).
#[derive(Debug, Clone, PartialEq)]
enum Data {
    List(String),
    Record(String),
    Value(Option<String>),
}

/// The facts of a program's model, read once.
#[derive(Debug, Default)]
pub struct ModelFacts {
    windows: BTreeMap<String, WindowFacts>,
    data: BTreeMap<String, BTreeMap<String, Data>>,
    app_data: BTreeMap<String, Data>,
    entities: BTreeMap<String, BTreeMap<String, FieldOf>>,
    enums: BTreeMap<String, Vec<(String, String)>>,
    formats: Formats,
    keys_editable: bool,
    /// The icons the program carries: an SVG path in a 24 by 24 box, by the icon's name.
    icons: BTreeMap<String, String>,
}

/// A literal of the model as Clarity writes it: `{ "kind": "int", "value": 3 }` is 3.
fn literal(value: &Value) -> Option<String> {
    match value.get("value") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Null) | None => None,
        Some(other) => Some(other.to_string()),
    }
}

fn data_of(decls: &Value) -> BTreeMap<String, Data> {
    let mut out = BTreeMap::new();
    for decl in decls.as_array().into_iter().flatten() {
        let Some(name) = decl["name"]["name"].as_str() else { continue };
        let entity = decl["of"]["entity"]["name"].as_str().unwrap_or_default().to_string();
        let data = match decl["of"]["kind"].as_str() {
            Some("list") => Data::List(entity),
            Some("record") => Data::Record(entity),
            _ => Data::Value(decl["of"]["ty"]["kind"]["name"].as_str().map(str::to_string)),
        };
        out.insert(name.to_string(), data);
    }
    out
}

impl ModelFacts {
    /// The facts of the model written as JSON; an empty or broken model has none.
    pub fn read(model: &str) -> ModelFacts {
        let model: Value = serde_json::from_str(model).unwrap_or_default();
        let project = &model["project"];
        let mut facts = ModelFacts {
            formats: Formats { language: project["locale"]["language"].as_str().map(str::to_string), currency: project["locale"]["currency"].as_str().map(str::to_string) },
            keys_editable: project["keysEditable"].as_bool().unwrap_or(false),
            icons: model["icons"].as_object().into_iter().flatten().filter_map(|(name, path)| Some((name.clone(), path.as_str()?.to_string()))).collect(),
            ..ModelFacts::default()
        };
        for file in project["files"].as_array().into_iter().flatten() {
            for page in file["pages"].as_array().into_iter().flatten() {
                let Some(name) = page["name"]["name"].as_str() else { continue };
                let size = page["size"].as_object().and_then(|s| Some((s.get("width")?.as_f64()?, s.get("height")?.as_f64()?)));
                facts.windows.insert(
                    name.to_string(),
                    WindowFacts { dialog: page["kind"].as_str() == Some("dialog"), title: page["title"].as_str().map(str::to_string), size, columns: page["columns"].as_i64() },
                );
                facts.data.insert(name.to_string(), data_of(&page["data"]));
            }
            for app in file["apps"].as_array().into_iter().flatten() {
                facts.app_data.extend(data_of(&app["data"]));
            }
            for decl in file["enums"].as_array().into_iter().flatten() {
                let Some(name) = decl["name"]["name"].as_str() else { continue };
                let variants = decl["variants"].as_array().into_iter().flatten().filter_map(|v| Some((v["name"]["name"].as_str()?.to_string(), v["label"].as_str().unwrap_or_default().to_string()))).collect();
                facts.enums.insert(name.to_string(), variants);
            }
            for entity in file["entities"].as_array().into_iter().flatten().chain(file["records"].as_array().into_iter().flatten()) {
                let Some(name) = entity["name"]["name"].as_str() else { continue };
                facts.entities.insert(name.to_string(), Self::fields_of(name, entity));
            }
        }
        facts
    }

    fn fields_of(entity: &str, decl: &Value) -> BTreeMap<String, FieldOf> {
        let fields = decl["fields"].as_array().cloned().unwrap_or_default();
        // A record whose field refers to its own entity names its parent: its records make a tree.
        let parent = fields.iter().find(|f| f["type"]["kind"].as_str() == Some("ref") && f["type"]["entity"]["name"].as_str() == Some(entity)).and_then(|f| f["name"]["name"].as_str()).map(str::to_string);
        let mut out = BTreeMap::new();
        for field in &fields {
            let Some(name) = field["name"]["name"].as_str() else { continue };
            let ty = &field["type"];
            let kind = ty["kind"].as_str().unwrap_or("text").to_string();
            let names = ty["name"]["name"].as_str().or_else(|| ty["entity"]["name"].as_str()).or_else(|| ty["item"]["name"].as_str()).map(str::to_string);
            let modifiers = &field["modifiers"];
            let facts = FieldFacts {
                label: field["label"].as_str().or_else(|| modifiers["label"].as_str()).unwrap_or(name).to_string(),
                // A record's decimal without precision and scale (28, 0) shows the places it has.
                format: format_written(&kind, ty["scale"].as_u64().filter(|s| *s != 0 || ty["precision"].as_u64() != Some(28)), modifiers["format"].as_str()),
                required: modifiers["required"].as_bool().unwrap_or(false),
                min: literal(&modifiers["min"]),
                max: literal(&modifiers["max"]),
                refers: if kind == "ref" { names.clone() } else { None },
                parent: parent.clone(),
                color: kind == "color",
                help: modifiers["help"].as_str().filter(|h| !h.is_empty()).map(str::to_string),
            };
            let store = modifiers["store"].as_str().map(str::to_string);
            let item = if kind == "values" { ty["item"]["name"]["name"].as_str().or_else(|| ty["item"]["kind"].as_str()).map(str::to_string) } else { None };
            out.insert(name.to_string(), FieldOf { kind, names, facts, store, item });
        }
        out
    }

    /// The facts of a program's model, read once for each model: known by
    /// its text, not where it lies, since a model the IDE's designer reloads
    /// is dropped and another may come at the same place. The last few are kept.
    pub fn of(model: &str) -> Arc<ModelFacts> {
        static READ: Mutex<Vec<(u64, usize, Arc<ModelFacts>)>> = Mutex::new(Vec::new());
        const KEPT: usize = 8;
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            model.hash(&mut hasher);
            hasher.finish()
        };
        let mut read = READ.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(at) = read.iter().position(|(h, len, _)| *h == hash && *len == model.len()) {
            let found = read.remove(at);
            let facts = found.2.clone();
            read.push(found);
            return facts;
        }
        let facts = Arc::new(ModelFacts::read(model));
        read.push((hash, model.len(), facts.clone()));
        if read.len() > KEPT {
            read.remove(0);
        }
        facts
    }

    /// What the model says of window `name`; none for one it does not have.
    pub fn window(&self, name: &str) -> Option<&WindowFacts> {
        self.windows.get(name)
    }

    /// Whether window `name` is a dialog.
    pub fn is_dialog(&self, name: &str) -> bool {
        self.window(name).is_some_and(|w| w.dialog)
    }

    /// The SVG path of icon `name` the program carries; none when it carries none of that name.
    pub fn icon(&self, name: &str) -> Option<&str> {
        self.icons.get(name).map(String::as_str)
    }

    /// Whether the person may set their own keys for the program's commands.
    pub fn keys_editable(&self) -> bool {
        self.keys_editable
    }

    /// The person's formats as the project writes them: its language and currency.
    pub fn formats(&self) -> Formats {
        self.formats.clone()
    }

    /// The entity of the records at `path` in window `window`'s data
    /// (`tasks`, `order`, `tasks[2]`, `order.customer` through a reference
    /// held inside); none for a plain value.
    fn entity_at(&self, window: &str, path: &str) -> Option<String> {
        let plain = without_places(path);
        let mut steps = plain.split('.');
        let first = steps.next()?;
        let data = self.data.get(window).and_then(|d| d.get(first)).or_else(|| self.app_data.get(first))?;
        let mut entity = match data {
            Data::List(e) | Data::Record(e) => e.clone(),
            Data::Value(_) => return None,
        };
        for step in steps {
            let field = self.entities.get(&entity)?.get(step)?;
            match field.kind.as_str() {
                "record" | "list" => entity = field.names.clone()?,
                _ => return None,
            }
        }
        Some(entity)
    }

    /// The field at `path` in window `window`'s data, with its entity: `order.total`,
    /// `orders[0].total` and `orders.total` are all `Order.total`.
    pub fn field_at(&self, window: &str, path: &str) -> Option<(String, &FieldOf)> {
        let plain = without_places(path);
        let (owner, field) = plain.rsplit_once('.')?;
        let entity = self.entity_at(window, owner)?;
        let of = self.entities.get(&entity)?.get(field)?;
        Some((entity, of))
    }

    /// The facts of the field at `path`; none for a value that is no entity's field.
    pub fn field(&self, window: &str, path: &str) -> Option<FieldFacts> {
        self.field_at(window, path).map(|(_, f)| f.facts.clone())
    }

    /// The facts of `field` of `entity` (a table's column of the records it shows).
    pub fn field_of(&self, entity: &str, field: &str) -> Option<&FieldOf> {
        self.entities.get(entity)?.get(field)
    }

    /// The type of the value at `path`, by its name: an enum's name, an
    /// entity's for a record or the records of a list, else the field's
    /// kind as the model writes it (`text`, `date`, `ref`, `language`); none
    /// for what the model does not know.
    pub fn type_of(&self, window: &str, path: &str) -> Option<String> {
        if let Some((_, field)) = self.field_at(window, path) {
            return Some(match field.kind.as_str() {
                "enum" | "record" | "list" => field.names.clone().unwrap_or_else(|| field.kind.clone()),
                // A list of plain values: `list<text>`, `list<Genre>`.
                "values" => format!("list<{}>", field.item.clone().unwrap_or_else(|| "text".to_string())),
                kind => kind.to_string(),
            });
        }
        if let Some(name) = self.value_enum(window, path) {
            return Some(name.to_string());
        }
        self.entity_at(window, path)
    }

    /// The values of enum `name` with their labels, in the language shown; empty for no enum.
    pub fn enum_choices(&self, name: &str) -> Vec<Choice> {
        self.enums
            .get(name)
            .map(|variants| variants.iter().map(|(value, label)| Choice { value: Value::from(value.clone()), label: shown(label, value), cells: None }).collect())
            .unwrap_or_default()
    }

    /// The file store `field` of `entity` keeps its files in; none for one kept in the database.
    pub fn store_of(&self, entity: &str, field: &str) -> Option<String> {
        self.field_of(entity, field).and_then(|f| f.store.clone())
    }

    /// The enum a window's own value `path` holds (`data kind: DiagramKind`); none for any other.
    pub fn value_enum(&self, window: &str, path: &str) -> Option<&str> {
        match self.data.get(window).and_then(|d| d.get(without_places(path).as_str())).or_else(|| self.app_data.get(path))? {
            Data::Value(Some(name)) if self.is_enum(name) => Some(name),
            _ => None,
        }
    }

    /// Whether `name` is an enum of the model.
    pub fn is_enum(&self, name: &str) -> bool {
        self.enums.contains_key(name)
    }
}

/// A field's format as `FieldFacts::format` writes it: its kind, then its
/// scale and its named format when it has them (`decimal 2 percent`,
/// `date long`); none for a field whose values have no format.
pub fn format_written(kind: &str, scale: Option<u64>, format: Option<&str>) -> Option<String> {
    let kind = match kind {
        "integer" | "decimal" | "money" | "date" | "dateTime" | "duration" => kind,
        _ => return format.map(str::to_string),
    };
    let mut written = kind.to_string();
    if let Some(scale) = scale.filter(|_| kind == "decimal") {
        written.push(' ');
        written.push_str(&scale.to_string());
    }
    if let Some(format) = format.filter(|f| !f.is_empty()) {
        written.push(' ');
        written.push_str(format);
    }
    Some(written)
}

/// A label as shown: a key (`@red`) by its value in the language shown, else as written; empty, the value itself.
pub fn shown(label: &str, value: &str) -> String {
    if label.is_empty() {
        return value.to_string();
    }
    match label.strip_prefix('@') {
        Some(key) => clarity_runtime::i18n::value(key).unwrap_or_else(|| key.to_string()),
        None => clarity_runtime::i18n::text(label),
    }
}

/// A path without the places of rows in it: `tasks[1].done` is `tasks.done`.
pub fn without_places(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut inside = false;
    for c in path.chars() {
        match c {
            '[' => inside = true,
            ']' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    out
}
