//! Clarity.Formats (docs/FORMATS-PLAN.md): the data formats of the
//! programs Clarity generates, one catalog and one implementation of every
//! format, used by every stack. `read` parses what a program reads and fits
//! it to what it wants (`fit`); `write` writes a value, given as the JSON
//! its stack's codec writes, in the format; `catalog` lists the formats
//! and their options, which the checker, the designer and the agent read.
//!
//! One document model (`doc`) and one engine on it: a format is its syntax
//! only (`format::Format`: parse bytes into a document, write a document
//! as bytes, read its schemas into rules); paths (`path`), mappings
//! (`mapping`), reading (`shape`, then `fit`), writing (`build`), the
//! tables of file sources (`table`), the schema check (`check`) and
//! describe for Import (`describe`) are written once for every format. The catalog is the list of the
//! formats' implementations.

pub mod build;
pub mod check;
pub mod csv;
pub mod describe;
pub mod doc;
pub mod encoding;
pub mod fit;
pub mod format;
pub mod json;
pub mod kind;
pub mod mapping;
pub mod markup;
pub mod normal;
pub mod numbers;
pub mod path;
pub mod shape;
pub mod table;
pub mod tree;
pub mod xml;
pub mod xsd;
pub mod yaml;

pub use format::Format;
pub use kind::{Field, Kind, Shape};
pub use mapping::Mapping;
pub use tree::Tree;

/// Why reading or writing did not work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub kind: ProblemKind,
    /// The message, word for word the same on every stack.
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    /// What was read does not fit what the program wants, or is not in the format.
    NotFitting,
    /// No format of that name is in the catalog.
    UnknownFormat,
    /// An option the format does not have, or of the wrong kind.
    BadOptions,
    /// The kind or value given does not describe what it should (a fault of the program, not of what was read).
    Failed,
}

impl Problem {
    pub(crate) fn new(kind: ProblemKind, text: impl Into<String>) -> Problem {
        Problem { kind, text: text.into() }
    }
}

/// What a format's option takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    Text,
    Bool,
}

/// An option of a format: its name as code writes it, what it takes, and the key of its label for people.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionInfo {
    pub name: &'static str,
    pub kind: OptionKind,
    pub label: &'static str,
    /// Whether reading takes it (`read Order from json text at "data"`).
    pub reads: bool,
    /// Whether writing takes it (`csv of rows separator ";"`).
    pub writes: bool,
    /// When writing always takes it, what it says, for the checker's message
    /// (`'xml of' needs 'at': ...`).
    pub required_writing: Option<&'static str>,
}

/// A format of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatInfo {
    /// The word code names it by: `read Order from json text`.
    pub name: &'static str,
    /// How a message names it: `the JSON has no 'data'`.
    pub title: &'static str,
    /// Whether it is text (else bytes).
    pub text: bool,
    /// Whether records nest in it (else an entity with records inside is refused).
    pub nests: bool,
    /// The nodes it has, which the engine reads and writes by.
    pub nodes: Nodes,
    /// Whether a path in it may name an attribute (`@number`).
    pub attributes: bool,
    /// How many tables one of its files holds.
    pub tables: Tables,
    /// What separates the steps of a path in it (`at`, a field's `column`):
    /// `/` where it nests (an `at` without a slash is read with dots too);
    /// none for a flat format.
    pub path_separator: Option<char>,
    /// The file kinds of its schemas, which Import takes tables from, typed
    /// (`xsd` for XML); none for a format without schemas.
    pub schema_kinds: &'static [&'static str],
    pub options: Vec<OptionInfo>,
}

/// The nodes a format has (doc.rs), by which the engine knows where its
/// records are without `at` and how a value is written in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nodes {
    /// Objects, lists and typed values, the file one value (JSON, YAML).
    Values,
    /// Named elements with attributes and text, the file one root element (XML).
    Elements,
    /// Rows of text cells under a header (CSV, later spreadsheets).
    Rows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tables {
    One,
    Several,
}

/// Every format, in the order the catalog lists them: one implementation each.
pub fn formats() -> &'static [&'static dyn Format] {
    &[&json::Json, &csv::Csv, &xml::Xml, &yaml::Yaml]
}

/// The catalog: each format's entry.
pub fn catalog() -> Vec<FormatInfo> {
    formats().iter().map(|f| f.info()).collect()
}

/// The implementation of the format called `name`.
pub fn implementation(name: &str) -> Option<&'static dyn Format> {
    formats().iter().copied().find(|f| f.info().name == name)
}

/// The implementation and its entry, or why there is none.
fn found(name: &str) -> Result<(&'static dyn Format, FormatInfo), Problem> {
    let format = implementation(name).ok_or_else(|| Problem::new(ProblemKind::UnknownFormat, format!("there is no format '{name}'")))?;
    Ok((format, format.info()))
}

/// The format of the catalog called `name`.
pub fn format(name: &str) -> Option<FormatInfo> {
    catalog().into_iter().find(|f| f.name == name)
}

/// The options given, checked against the format's and the generic
/// mapping keys (`mapping::keys`): none it does not have, each of its kind.
fn options_of(format: &FormatInfo, options: &Tree) -> Result<Vec<(String, Tree)>, Problem> {
    let pairs = match options {
        Tree::Null => return Ok(Vec::new()),
        Tree::Object(pairs) => pairs.clone(),
        _ => return Err(Problem::new(ProblemKind::BadOptions, "the options are an object of names and values")),
    };
    let generic = mapping::keys(format);
    for (name, value) in &pairs {
        // `check` is refused by a format without schemas with its own message (check.rs).
        if generic.contains(&name.as_str()) || name == "check" {
            continue;
        }
        let Some(info) = format.options.iter().find(|o| o.name == name) else {
            let names: Vec<&str> = format.options.iter().map(|o| o.name).chain(generic.iter().copied()).collect();
            let has = if names.is_empty() { "no options".to_string() } else { format!("the options {}", names.join(", ")) };
            return Err(Problem::new(ProblemKind::BadOptions, format!("{} has no option '{name}'; it has {has}", format.name)));
        };
        let fits = matches!((info.kind, value), (OptionKind::Text, Tree::Text(_)) | (OptionKind::Bool, Tree::Bool(_)));
        if !fits {
            let wanted = if info.kind == OptionKind::Text { "text" } else { "true or false" };
            return Err(Problem::new(ProblemKind::BadOptions, format!("the option '{name}' of {} is {wanted}", format.name)));
        }
    }
    Ok(pairs)
}

/// Reads `input` in `format` with its `options` (an object, or null for
/// none: the format's own and a mapping) and fits it to `kind`; the fitted
/// value as JSON, in the form every stack's codec reads.
pub fn read(format: &str, input: &[u8], options: &Tree, kind: &Kind) -> Result<String, Problem> {
    let (format, info) = found(format)?;
    let options = options_of(&info, options)?;
    let map = Mapping::of(&options, &info)?;
    let rules = check::rules(format, &info, &options)?;
    let doc = format.parse(input, &options, &shape::names(kind, &map))?;
    if let Some(rules) = &rules {
        check::document(&doc, &info, rules)?;
    }
    let tree = shape::read(&doc, &info, &map, kind)?;
    let fitted = fit::fit(tree, kind, info.title).map_err(|text| Problem::new(ProblemKind::NotFitting, text))?;
    Ok(tree::write(&fitted))
}

/// Reads `input`, a file in `format`, as its table, or its tables where the
/// mapping nests records (table.rs), in JSON; `fields` is the JSON of the
/// columns an entity maps with their kinds.
pub fn read_table(format: &str, input: &[u8], options: &str, fields: &str) -> Result<String, Problem> {
    let options = options_tree(options)?;
    let (format, info) = found(format)?;
    let options = options_of(&info, &options)?;
    let map = Mapping::of(&options, &info)?;
    let fields = table::fields_from_json(fields).map_err(|t| Problem::new(ProblemKind::Failed, t))?;
    table::read(format, &info, input, &options, &map, &fields)
}

/// The tables `input`, a file in `format`, holds and their columns, as JSON
/// (describe.rs), for Import: every table, whatever `at` says.
pub fn describe(format: &str, input: &[u8], options: &str) -> Result<String, Problem> {
    let options = options_tree(options)?;
    let (format, info) = found(format)?;
    let options = options_of(&info, &options)?;
    let doc = format.parse(input, &options, &[])?;
    Ok(describe::to_json(&describe::describe(&doc, info.nodes)))
}

/// The tables a schema of `format` declares, typed (an XSD for XML), as
/// `describe` gives a file's.
pub fn describe_schema(format: &str, schema: &str) -> Result<String, Problem> {
    let (format, _) = found(format)?;
    format.schema(schema).map(|t| describe::to_json(&t))
}

/// Writes `table` (JSON, as `read_table` gives it) back into a file in
/// `format`, inside `original` where the format keeps more than the table.
pub fn write_table(format: &str, original: &[u8], table: &str, options: &str, fields: &str) -> Result<Vec<u8>, Problem> {
    let options = options_tree(options)?;
    let (format, info) = found(format)?;
    let options = options_of(&info, &options)?;
    let map = Mapping::of(&options, &info)?;
    let fields = table::fields_from_json(fields).map_err(|t| Problem::new(ProblemKind::Failed, t))?;
    table::write(format, &info, original, table, &options, &map, &fields)
}

/// Writes `value`, the JSON its stack's codec wrote, in `format` with its
/// `options`, as `kind` says it is: for JSON, in Clarity's one form.
pub fn write(format: &str, value: &str, options: &Tree, kind: &Kind) -> Result<Vec<u8>, Problem> {
    let (format, info) = found(format)?;
    let options = options_of(&info, options)?;
    let map = Mapping::of(&options, &info)?;
    let value = tree::parse(value).ok_or_else(|| Problem::new(ProblemKind::Failed, "the value to write is no JSON"))?;
    let doc = build::write(&info, &map, &normal::normal(&value, kind), kind)?;
    format.write(&doc, &options)
}

/// `read` with the options and kind given as JSON, as the C table and every stack pass them.
pub fn read_json(format: &str, input: &[u8], options: &str, kind: &str) -> Result<String, Problem> {
    let (options, kind) = given(options, kind)?;
    read(format, input, &options, &kind)
}

/// `write` with the options and kind given as JSON.
pub fn write_json(format: &str, value: &str, options: &str, kind: &str) -> Result<Vec<u8>, Problem> {
    let (options, kind) = given(options, kind)?;
    write(format, value, &options, &kind)
}

fn options_tree(options: &str) -> Result<Tree, Problem> {
    if options.trim().is_empty() { Ok(Tree::Null) } else { tree::parse(options).ok_or_else(|| Problem::new(ProblemKind::BadOptions, "the options are no JSON")) }
}

fn given(options: &str, kind: &str) -> Result<(Tree, Kind), Problem> {
    let options = options_tree(options)?;
    let kind = Kind::from_json(kind).map_err(|why| Problem::new(ProblemKind::Failed, why))?;
    Ok((options, kind))
}

/// The catalog as JSON, for stacks and tools that read it through the C table.
pub fn catalog_json() -> String {
    let formats: Vec<Tree> = catalog()
        .into_iter()
        .map(|f| {
            let options = f
                .options
                .iter()
                .map(|o| {
                    Tree::Object(vec![
                        ("name".into(), Tree::Text(o.name.into())),
                        ("kind".into(), Tree::Text(if o.kind == OptionKind::Text { "text" } else { "bool" }.into())),
                        ("label".into(), Tree::Text(o.label.into())),
                        ("reads".into(), Tree::Bool(o.reads)),
                        ("writes".into(), Tree::Bool(o.writes)),
                        ("requiredWriting".into(), Tree::Bool(o.required_writing.is_some())),
                    ])
                })
                .collect();
            Tree::Object(vec![
                ("name".into(), Tree::Text(f.name.into())),
                ("title".into(), Tree::Text(f.title.into())),
                ("text".into(), Tree::Bool(f.text)),
                ("nests".into(), Tree::Bool(f.nests)),
                ("tables".into(), Tree::Text(if f.tables == Tables::One { "one" } else { "several" }.into())),
                ("options".into(), Tree::List(options)),
                ("nodes".into(), Tree::Text(match f.nodes { Nodes::Values => "values", Nodes::Elements => "elements", Nodes::Rows => "rows" }.into())),
                ("attributes".into(), Tree::Bool(f.attributes)),
                ("mapping".into(), Tree::List(mapping::keys(&f).iter().map(|k| Tree::Text((*k).into())).collect())),
            ])
        })
        .collect();
    tree::write(&Tree::List(formats))
}
