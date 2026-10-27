//! The project file in Clarity syntax: one `project` block with its
//! settings, one per line.
//!
//! ```text
//! project Shop
//!     title "The shop"
//!     stack rust web desktop
//!     output "out"
//!     output keeps "vendor"
//!     datasource main sqlite "data/shop.db"
//!     datasource payments rest "https://api.example.com/v2"
//!     datasource erp sqlite "legacy/erp.db" external readonly
//!         auth bearer "keychain:payments"
//!     filestore photos "env:PHOTO_FOLDER"
//! ```
//!
//! Reading is strict: unknown settings and values are errors, so a typo
//! never silently picks a default. Writing keeps every line whose value did
//! not change exactly as written, with the comments and blank lines before
//! it, so what a developer writes by hand survives the Project tab.

use std::collections::HashMap;

use clarity_spec::lexer::{Token, TokenKind, lex, quote};

use crate::{
    AUTH_KINDS, Auth, BACKENDS, DEFAULT_OUT, DataSource, FileStore, SOURCE_KINDS, Settings, Stack, TARGETS, Target, UIS, auth_word, currency_problem,
    language_problem,
    backend_word, source_word, target_word, ui_word, usual_ui,
};

/// One line of the project file.
struct Entry {
    /// The line number, from 1.
    line: u32,
    indent: usize,
    tokens: Vec<Token>,
}

fn word(token: &Token) -> Option<&str> {
    (token.kind == TokenKind::Ident).then_some(token.text.as_str())
}

/// A list of words as a sentence: "a, b and c".
fn listed(words: &[&str]) -> String {
    match words.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// The value of `words` that `text` names, or an error listing the words.
fn pick<T: Copy>(line: u32, what: &str, plural: &str, all: &[T], name: fn(T) -> &'static str, text: &str) -> Result<T, String> {
    all.iter().copied().find(|v| name(*v) == text).ok_or_else(|| {
        let words: Vec<&str> = all.iter().map(|v| name(*v)).collect();
        format!("line {line}: unknown {what} '{text}'; the {plural} are {}", listed(&words))
    })
}

fn entries(text: &str) -> Result<Vec<Entry>, String> {
    let (lines, diagnostics) = lex(text);
    if let Some(d) = diagnostics.first() {
        return Err(format!("line {}: {}", d.span.line, d.message));
    }
    Ok(lines
        .into_iter()
        .map(|l| Entry {
            line: l.span.line,
            indent: l.indent,
            tokens: l.tokens,
        })
        .collect())
}

/// The tokens after the keyword: identifiers and strings, with what kind
/// each is.
fn args(entry: &Entry) -> Vec<&Token> {
    entry.tokens.iter().skip(1).collect()
}

/// Why a folder the output keeps cannot be one. It is the name of a folder
/// beside the generated files, not a path, so Clarity can tell it from what
/// it writes itself.
pub fn kept_problem(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return Some("a folder the output keeps needs a name".to_string());
    }
    if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Some(format!("'{name}' is a path; name one folder in the output, like \"vendor\""));
    }
    None
}

fn string(line: u32, token: Option<&&Token>, what: &str) -> Result<String, String> {
    match token {
        Some(t) if t.kind == TokenKind::Str => Ok(t.text.clone()),
        _ => Err(format!("line {line}: {what} is a text in quotes")),
    }
}

pub fn parse(text: &str) -> Result<Settings, String> {
    let entries = entries(text)?;
    let Some(first) = entries.first() else {
        return Err("the project file is empty; it starts with 'project Name'".to_string());
    };
    let header = first.tokens.first().and_then(word);
    let name = match (header, first.tokens.get(1), first.tokens.len(), first.indent) {
        (Some("project"), Some(name), 2, 0) if name.kind == TokenKind::Ident => name.text.clone(),
        _ => return Err(format!("line {}: a project file starts with 'project Name'", first.line)),
    };

    let mut title = None;
    let mut stack = None;
    let mut out = None;
    let mut keeps: Option<Vec<String>> = None;
    let mut language = None;
    let mut currency = None;
    let mut keys_editable = None;
    let mut engines_interpreted = None;
    let mut themes: Option<Vec<String>> = None;
    let mut datasources: Vec<DataSource> = Vec::new();
    let mut filestores: Vec<FileStore> = Vec::new();
    let setting_indent = entries.get(1).map(|e| e.indent).unwrap_or(4);
    for entry in &entries[1..] {
        let line = entry.line;
        let keyword = entry.tokens.first().and_then(word).unwrap_or("");
        if entry.indent == 0 {
            return Err(format!("line {line}: a project file holds one project; indent its settings"));
        }
        // `openapi "forms.yaml"`, `pick "listTemplates", ...` and `pick schema
        // "Entry"` under a web API data source (DESIGN 5.23).
        if entry.indent > setting_indent && (keyword == "openapi" || keyword == "pick") {
            let Some(source) = datasources.last_mut() else {
                return Err(format!("line {line}: '{keyword}' belongs under a datasource line, indented"));
            };
            let a = args(entry);
            if keyword == "openapi" {
                if a.len() != 1 || source.openapi.is_some() {
                    return Err(format!("line {line}: write 'openapi \"file.yaml\"' once per data source"));
                }
                source.openapi = Some(string(line, a.first(), "the description's file")?);
                continue;
            }
            let schemas = a.first().and_then(|t| word(t)) == Some("schema");
            let named: Vec<&&Token> = a[usize::from(schemas)..].iter().filter(|t| t.kind != TokenKind::Comma).collect();
            if named.is_empty() || named.iter().any(|t| t.kind != TokenKind::Str) {
                return Err(format!("line {line}: write what is picked as texts in quotes, like: pick \"listTemplates\", \"getTemplate\""));
            }
            let into = if schemas { &mut source.pick_schemas } else { &mut source.picks };
            into.extend(named.iter().map(|t| t.text.clone()));
            continue;
        }
        if entry.indent > setting_indent {
            if keyword != "auth" {
                return Err(format!("line {line}: unexpected indented line"));
            }
            let Some(source) = datasources.last_mut() else {
                return Err(format!("line {line}: 'auth' belongs under a datasource line, indented"));
            };
            let a = args(entry);
            let kind_word = a.first().and_then(|t| word(t)).unwrap_or("");
            let kind = pick(line, "auth kind", "auth kinds", &AUTH_KINDS, auth_word, kind_word)?;
            let secret = string(line, a.get(1), "the secret")?;
            if a.len() > 2 || source.auth.is_some() {
                return Err(format!("line {line}: write 'auth kind \"env:NAME\"' once per data source"));
            }
            source.auth = Some(Auth { kind, secret });
            continue;
        }
        let once = |slot_is_set: bool| {
            if slot_is_set {
                Err(format!("line {line}: '{keyword}' is set twice"))
            } else {
                Ok(())
            }
        };
        let a = args(entry);
        match keyword {
            "title" => {
                once(title.is_some())?;
                title = Some(string(line, a.first(), "the title")?);
            }
            // `output keeps "vendor"` names folders in the output that are
            // the project's own; `output "folder"` says where it is.
            "output" if a.first().and_then(|t| word(t)) == Some("keeps") => {
                once(keeps.is_some())?;
                let named: Vec<&&Token> = a[1..].iter().filter(|t| t.kind != TokenKind::Comma).collect();
                if named.is_empty() || named.iter().any(|t| t.kind != TokenKind::Str) {
                    return Err(format!(
                        "line {line}: write the folders the output keeps as texts in quotes, like: output keeps \"vendor\", \"notes\""
                    ));
                }
                let folders: Vec<String> = named.iter().map(|t| t.text.clone()).collect();
                if let Some(problem) = folders.iter().find_map(|f| kept_problem(f)) {
                    return Err(format!("line {line}: {problem}"));
                }
                keeps = Some(folders);
            }
            "output" => {
                once(out.is_some())?;
                out = Some(string(line, a.first(), "the output folder")?);
            }
            "language" => {
                once(language.is_some())?;
                let tag = string(line, a.first(), "the language")?;
                if let Some(problem) = language_problem(&tag) {
                    return Err(format!("line {line}: {problem}"));
                }
                language = Some(tag);
            }
            "keys" => {
                once(keys_editable.is_some())?;
                if a.len() != 1 || a.first().and_then(|t| word(t)) != Some("editable") {
                    return Err(format!("line {line}: write 'keys editable' to let the people using the application set the keys"));
                }
                keys_editable = Some(true);
            }
            "engines" => {
                once(engines_interpreted.is_some())?;
                engines_interpreted = match (a.len(), a.first().and_then(|t| word(t))) {
                    (1, Some("interpreted")) => Some(true),
                    (1, Some("compiled")) => Some(false),
                    _ => return Err(format!("line {line}: write 'engines compiled' or 'engines interpreted': how the engines written in Clarity run")),
                };
            }
            "currency" => {
                once(currency.is_some())?;
                let code = a.first().and_then(|t| word(t)).unwrap_or("");
                if let Some(problem) = currency_problem(code) {
                    return Err(format!("line {line}: {problem}"));
                }
                currency = Some(code.to_string());
            }
            "theme" => {
                once(themes.is_some())?;
                let names: Vec<String> = a.iter().filter_map(|t| word(t)).map(str::to_string).collect();
                if names.is_empty() || names.len() != a.len() {
                    return Err(format!("line {line}: write the names of the themes, the first one the application starts with, like: theme Shop Night"));
                }
                themes = Some(names);
            }
            "stack" => {
                once(stack.is_some())?;
                let words: Vec<&str> = a.iter().map(|t| word(t).unwrap_or("")).collect();
                // `stack go cli`: a program without a window names no UI.
                if words.get(1) == Some(&target_word(Target::Cli)) {
                    let backend = pick(line, "backend", "backends", &BACKENDS, backend_word, words[0])?;
                    let targets = words[1..]
                        .iter()
                        .map(|t| pick(line, "target", "targets", &TARGETS, target_word, t))
                        .collect::<Result<Vec<_>, _>>()?;
                    if targets.iter().any(|t| *t != Target::Cli) {
                        return Err(format!(
                            "line {line}: a program with a window names its UI before the targets, like: stack go web desktop cli"
                        ));
                    }
                    stack = Some(Stack { backend, ui: usual_ui(backend), targets });
                } else {
                    let (Some(backend), Some(ui), targets) = (words.first(), words.get(1), words.get(2..).unwrap_or(&[]))
                    else {
                        return Err(format!(
                            "line {line}: write the backend, the UI and at least one target, like: stack rust web desktop"
                        ));
                    };
                    if targets.is_empty() {
                        return Err(format!(
                            "line {line}: the stack needs at least one target, like: stack rust web desktop"
                        ));
                    }
                    stack = Some(Stack {
                        backend: pick(line, "backend", "backends", &BACKENDS, backend_word, backend)?,
                        ui: pick(line, "UI", "UIs", &UIS, ui_word, ui)?,
                        targets: targets
                            .iter()
                            .map(|t| pick(line, "target", "targets", &TARGETS, target_word, t))
                            .collect::<Result<_, _>>()?,
                    });
                }
            }
            "datasource" => {
                let name = a.first().and_then(|t| word(t));
                let kind = a.get(1).and_then(|t| word(t));
                let (Some(name), Some(kind)) = (name, kind) else {
                    return Err(format!(
                        "line {line}: write a name, a kind and where it is, like: datasource main sqlite \"data/app.db\""
                    ));
                };
                let kind = pick(line, "data source kind", "kinds", &SOURCE_KINDS, source_word, kind)?;
                let url = string(line, a.get(2), "where the data source is")?;
                let (mut external, mut readonly) = (false, false);
                for flag in &a[3.min(a.len())..] {
                    let slot = match word(flag) {
                        Some("external") => &mut external,
                        Some("readonly") => &mut readonly,
                        _ => {
                            return Err(format!(
                                "line {line}: unexpected '{}'; a data source can be external or readonly",
                                flag.text
                            ));
                        }
                    };
                    if *slot {
                        return Err(format!("line {line}: '{}' is written twice", flag.text));
                    }
                    *slot = true;
                }
                datasources.push(DataSource {
                    name: name.to_string(),
                    kind,
                    url,
                    auth: None,
                    external,
                    readonly,
                    openapi: None,
                    picks: Vec::new(),
                    pick_schemas: Vec::new(),
                });
            }
            "filestore" => {
                let (Some(name), Some(location), None) = (a.first().and_then(|t| word(t)), a.get(1), a.get(2)) else {
                    return Err(format!(
                        "line {line}: write a name and its folder, like: filestore photos \"pictures\""
                    ));
                };
                let location = string(line, Some(location), "the folder of the file store")?;
                filestores.push(FileStore { name: name.to_string(), location });
            }
            "auth" => return Err(format!("line {line}: 'auth' belongs under a datasource line, indented")),
            other => {
                return Err(format!(
                    "line {line}: unknown setting '{other}'; the settings are title, stack, output, language, currency, theme, datasource and filestore"
                ));
            }
        }
    }
    let Some(stack) = stack else {
        return Err("the project needs a 'stack' line, like: stack rust web desktop".to_string());
    };
    Ok(Settings {
        name,
        title,
        stack,
        datasources,
        language,
        currency,
        themes: themes.unwrap_or_default(),
        filestores,
        out: out.unwrap_or_else(|| DEFAULT_OUT.to_string()),
        keeps: keeps.unwrap_or_default(),
        keys_editable: keys_editable.unwrap_or(false),
        engines_interpreted: engines_interpreted.unwrap_or(false),
    })
}

/// A line's text as the writer produces it, to compare an existing line
/// with a new one regardless of spacing and trailing comments.
fn canonical(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|t| if t.kind == TokenKind::Str { quote(&t.text) } else { t.text.clone() })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Which setting a line holds, so a changed file can keep each line's
/// comments: `stack`, `datasource main`, `auth main`.
fn key(entry: &Entry, current_source: Option<&str>) -> String {
    let keyword = entry.tokens.first().map(|t| t.text.as_str()).unwrap_or("");
    match keyword {
        "datasource" => format!("datasource {}", entry.tokens.get(1).map(|t| t.text.as_str()).unwrap_or("")),
        "filestore" => format!("filestore {}", entry.tokens.get(1).map(|t| t.text.as_str()).unwrap_or("")),
        "auth" => format!("auth {}", current_source.unwrap_or("")),
        "openapi" => format!("openapi {}", current_source.unwrap_or("")),
        "pick" if entry.tokens.get(1).map(|t| t.text.as_str()) == Some("schema") => format!("pick schema {}", current_source.unwrap_or("")),
        "pick" => format!("pick {}", current_source.unwrap_or("")),
        // The two kinds of output line: where it is, and what it keeps.
        "output" if entry.tokens.get(1).map(|t| t.text.as_str()) == Some("keeps") => "output keeps".to_string(),
        other => other.to_string(),
    }
}

/// An existing line: the comments and blank lines before it, the line as
/// written, and its canonical text.
struct Kept {
    before: Vec<String>,
    raw: String,
    canonical: String,
}

/// The project file text for `settings`. With `existing`, lines that keep
/// their value stay exactly as written, with the comments before them.
pub fn write(existing: Option<&str>, settings: &Settings) -> Result<String, String> {
    let mut kept: HashMap<String, Kept> = HashMap::new();
    let mut trailing: Vec<String> = Vec::new();
    if let Some(text) = existing {
        let raw: Vec<&str> = text.lines().collect();
        let mut next = 0;
        let mut source: Option<String> = None;
        for entry in entries(text)? {
            let index = entry.line as usize - 1;
            let key = key(&entry, source.as_deref());
            if key.starts_with("datasource ") {
                source = entry.tokens.get(1).map(|t| t.text.clone());
            }
            kept.insert(
                key,
                Kept {
                    before: raw[next..index].iter().map(|s| s.to_string()).collect(),
                    raw: raw[index].to_string(),
                    canonical: canonical(&entry.tokens),
                },
            );
            next = index + 1;
        }
        trailing = raw[next.min(raw.len())..].iter().map(|s| s.to_string()).collect();
    }

    // The lines of the new file, in their canonical order.
    let stack = &settings.stack;
    let mut stack_line = format!("stack {}", backend_word(stack.backend));
    if stack.has_ui() {
        stack_line.push(' ');
        stack_line.push_str(ui_word(stack.ui));
    }
    for target in &stack.targets {
        stack_line.push(' ');
        stack_line.push_str(target_word(*target));
    }
    let mut lines: Vec<(String, usize, String)> = vec![("project".into(), 0, format!("project {}", settings.name))];
    if let Some(title) = &settings.title {
        lines.push(("title".into(), 1, format!("title {}", quote(title))));
    }
    lines.push(("stack".into(), 1, stack_line));
    // A file that never named an output folder keeps building into `out`
    // without growing an output line.
    if existing.is_none() || settings.out != DEFAULT_OUT || kept.contains_key("output") {
        lines.push(("output".into(), 1, format!("output {}", quote(&settings.out))));
    }
    if !settings.keeps.is_empty() {
        let folders: Vec<String> = settings.keeps.iter().map(|f| quote(f)).collect();
        lines.push(("output keeps".into(), 1, format!("output keeps {}", folders.join(", "))));
    }
    if let Some(language) = &settings.language {
        lines.push(("language".into(), 1, format!("language {}", quote(language))));
    }
    if let Some(currency) = &settings.currency {
        lines.push(("currency".into(), 1, format!("currency {currency}")));
    }
    if !settings.themes.is_empty() {
        lines.push(("theme".into(), 1, format!("theme {}", settings.themes.join(" "))));
    }
    if settings.keys_editable {
        lines.push(("keys".into(), 1, "keys editable".to_string()));
    }
    if settings.engines_interpreted {
        lines.push(("engines".into(), 1, "engines interpreted".to_string()));
    }
    for source in &settings.datasources {
        lines.push((
            format!("datasource {}", source.name),
            1,
            format!(
                "datasource {} {} {}{}{}",
                source.name,
                source_word(source.kind),
                quote(&source.url),
                if source.external { " external" } else { "" },
                if source.readonly { " readonly" } else { "" }
            ),
        ));
        if let Some(auth) = &source.auth {
            lines.push((
                format!("auth {}", source.name),
                2,
                format!("auth {} {}", auth_word(auth.kind), quote(&auth.secret)),
            ));
        }
        if let Some(openapi) = &source.openapi {
            lines.push((format!("openapi {}", source.name), 2, format!("openapi {}", quote(openapi))));
        }
        if !source.picks.is_empty() {
            let picks: Vec<String> = source.picks.iter().map(|p| quote(p)).collect();
            lines.push((format!("pick {}", source.name), 2, format!("pick {}", picks.join(", "))));
        }
        if !source.pick_schemas.is_empty() {
            let picks: Vec<String> = source.pick_schemas.iter().map(|p| quote(p)).collect();
            lines.push((format!("pick schema {}", source.name), 2, format!("pick schema {}", picks.join(", "))));
        }
    }
    for store in &settings.filestores {
        lines.push((format!("filestore {}", store.name), 1, format!("filestore {} {}", store.name, quote(&store.location))));
    }

    let mut out: Vec<String> = Vec::new();
    for (key, depth, text) in lines {
        match kept.remove(&key) {
            Some(old) => {
                out.extend(old.before);
                out.push(if old.canonical == text { old.raw } else { format!("{}{text}", "    ".repeat(depth)) });
            }
            None => out.push(format!("{}{text}", "    ".repeat(depth))),
        }
    }
    out.extend(trailing);
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    Ok(out.join("\n") + "\n")
}
