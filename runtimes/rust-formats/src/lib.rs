//! Clarity.Formats (docs/FORMATS-PLAN.md): the data formats of the
//! programs Clarity generates, one catalog and one implementation of every
//! format, used by every stack. `read` parses what a program reads and fits
//! it to what it wants (`fit`); `write` writes a value, given as the JSON
//! its stack's codec writes, in the format; `catalog` lists the formats
//! and their options, which the checker, the designer and the agent read.
//! JSON is the first entry.

pub mod fit;
pub mod kind;
pub mod normal;
pub mod numbers;
pub mod tree;

pub use kind::{Field, Kind, Shape};
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
    fn new(kind: ProblemKind, text: impl Into<String>) -> Problem {
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
    /// How many tables one of its files holds.
    pub tables: Tables,
    pub options: Vec<OptionInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tables {
    One,
    Several,
}

/// Every format, in the order the catalog lists them.
pub fn catalog() -> Vec<FormatInfo> {
    vec![FormatInfo {
        name: "json",
        title: "JSON",
        text: true,
        nests: true,
        tables: Tables::One,
        options: vec![OptionInfo { name: "at", kind: OptionKind::Text, label: "formatOptionAt", reads: true, writes: false }],
    }]
}

/// The format of the catalog called `name`.
pub fn format(name: &str) -> Option<FormatInfo> {
    catalog().into_iter().find(|f| f.name == name)
}

/// The options given, checked against the format's: none it does not have, each of its kind.
fn options_of(format: &FormatInfo, options: &Tree) -> Result<Vec<(String, Tree)>, Problem> {
    let pairs = match options {
        Tree::Null => return Ok(Vec::new()),
        Tree::Object(pairs) => pairs.clone(),
        _ => return Err(Problem::new(ProblemKind::BadOptions, "the options are an object of names and values")),
    };
    for (name, value) in &pairs {
        let Some(info) = format.options.iter().find(|o| o.name == name) else {
            let names: Vec<&str> = format.options.iter().map(|o| o.name).collect();
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

fn option_text<'a>(options: &'a [(String, Tree)], name: &str) -> Option<&'a str> {
    options.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.as_text())
}

/// Reads `input` in `format` with its `options` (an object, or null for
/// none) and fits it to `kind`; the fitted value as JSON, in the form every
/// stack's codec reads.
pub fn read(format: &str, input: &[u8], options: &Tree, kind: &Kind) -> Result<String, Problem> {
    let info = self::format(format).ok_or_else(|| Problem::new(ProblemKind::UnknownFormat, format!("there is no format '{format}'")))?;
    let options = options_of(&info, options)?;
    let parsed = match info.name {
        "json" => std::str::from_utf8(input).ok().and_then(tree::parse),
        _ => None,
    };
    let parsed = parsed.ok_or_else(|| Problem::new(ProblemKind::NotFitting, format!("the text is no {}", info.title)))?;
    let fitted = fit::fit(parsed, option_text(&options, "at"), kind, info.title).map_err(|text| Problem::new(ProblemKind::NotFitting, text))?;
    Ok(tree::write(&fitted))
}

/// Writes `value`, the JSON its stack's codec wrote, in `format` with its
/// `options`, as `kind` says it is: for JSON, in Clarity's one form.
pub fn write(format: &str, value: &str, options: &Tree, kind: &Kind) -> Result<Vec<u8>, Problem> {
    let info = self::format(format).ok_or_else(|| Problem::new(ProblemKind::UnknownFormat, format!("there is no format '{format}'")))?;
    options_of(&info, options)?;
    let value = tree::parse(value).ok_or_else(|| Problem::new(ProblemKind::Failed, "the value to write is no JSON"))?;
    let normal = normal::normal(&value, kind);
    match info.name {
        "json" => Ok(tree::write(&normal).into_bytes()),
        _ => Err(Problem::new(ProblemKind::UnknownFormat, format!("there is no format '{format}'"))),
    }
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

fn given(options: &str, kind: &str) -> Result<(Tree, Kind), Problem> {
    let options = if options.trim().is_empty() { Tree::Null } else { tree::parse(options).ok_or_else(|| Problem::new(ProblemKind::BadOptions, "the options are no JSON"))? };
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
            ])
        })
        .collect();
    tree::write(&Tree::List(formats))
}
