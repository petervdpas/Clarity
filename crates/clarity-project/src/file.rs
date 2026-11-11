//! The project file in Clarity syntax: one `project` block with its
//! settings, one per line.
//!
//! ```text
//! project Shop
//!     title "The shop"
//!     stack rust desktop
//!     output "out"
//!     output keeps "vendor"
//!     datasource main sqlite "data/shop.db"
//!     datasource payments rest "https://api.example.com/v2"
//!     datasource erp sqlite "legacy/erp.db" external readonly
//!         auth bearer "setting:payments"
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
    AUTH_KINDS, Auth, BACKENDS, DEFAULT_OUT, DataSource, FileStore, Paging, PagingKind, SOURCE_KINDS, Settings, Stack, TARGETS, Target, UIS, Ui, auth_word, currency_problem,
    language_problem,
    backend_uis, backend_word, source_word, target_word, ui_word, usual_ui,
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

/// The words of the paging kinds, as the project file writes them.
const PAGING_KINDS: [(PagingKind, &str); 4] =
    [(PagingKind::Page, "page"), (PagingKind::Offset, "offset"), (PagingKind::Cursor, "cursor"), (PagingKind::Link, "link")];

fn paging_word(kind: PagingKind) -> &'static str {
    PAGING_KINDS.iter().find(|(k, _)| *k == kind).map(|(_, w)| *w).unwrap_or("page")
}

/// A `paging` line under a web API (DESIGN 5.23): `page "p" size "s" N`,
/// `offset "o" size "s" N`, `cursor "c" next "path"` or `link`, then
/// `items "path"` when a page's items are inside its JSON.
fn paging(line: u32, a: &[&Token]) -> Result<Paging, String> {
    let kind_word = a.first().and_then(|t| word(t)).unwrap_or("");
    let Some(kind) = PAGING_KINDS.iter().find(|(_, w)| *w == kind_word).map(|(k, _)| *k) else {
        return Err(format!("line {line}: the paging kinds are page, offset, cursor and link, not '{kind_word}'"));
    };
    let mut rest = &a[1..];
    // `items "data"` at the end, on every kind.
    let mut items = String::new();
    if let Some(at) = rest.iter().position(|t| word(t) == Some("items")) {
        match &rest[at + 1..] {
            [t] if t.kind == TokenKind::Str => items = t.text.clone(),
            _ => return Err(format!("line {line}: write items and where a page's items are, like: items \"data\"")),
        }
        rest = &rest[..at];
    }
    let text = |t: &Token| (t.kind == TokenKind::Str).then(|| t.text.clone());
    let none = String::new;
    let paging = match (kind, rest) {
        (PagingKind::Page | PagingKind::Offset, [param, size_word, size_param, size])
            if word(size_word) == Some("size") && text(param).is_some() && text(size_param).is_some() && size.kind == TokenKind::Number =>
        {
            let size: u32 = size.text.parse().unwrap_or(0);
            (size > 0).then(|| Paging { kind, param: param.text.clone(), size_param: size_param.text.clone(), size, next: none(), items: items.clone() })
        }
        (PagingKind::Cursor, [param, next_word, next]) if word(next_word) == Some("next") && text(param).is_some() && text(next).is_some() => {
            Some(Paging { kind, param: param.text.clone(), size_param: none(), size: 0, next: next.text.clone(), items: items.clone() })
        }
        (PagingKind::Link, []) => Some(Paging { kind, param: none(), size_param: none(), size: 0, next: none(), items: items.clone() }),
        _ => None,
    };
    paging.ok_or_else(|| {
        let example = match kind {
            PagingKind::Page => "paging page \"page\" size \"per_page\" 100",
            PagingKind::Offset => "paging offset \"offset\" size \"limit\" 100",
            PagingKind::Cursor => "paging cursor \"after\" next \"meta.next\"",
            PagingKind::Link => "paging link",
        };
        format!("line {line}: write {example}")
    })
}

/// The `paging` line as the project file writes it.
pub fn paging_line(p: &Paging) -> String {
    let mut out = format!("paging {}", paging_word(p.kind));
    match p.kind {
        PagingKind::Page | PagingKind::Offset => out.push_str(&format!(" {} size {} {}", quote(&p.param), quote(&p.size_param), p.size)),
        PagingKind::Cursor => out.push_str(&format!(" {} next {}", quote(&p.param), quote(&p.next))),
        PagingKind::Link => {}
    }
    if !p.items.is_empty() {
        out.push_str(&format!(" items {}", quote(&p.items)));
    }
    out
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
    let mut icon = None;
    let mut version = None;
    let mut stack = None;
    let mut out = None;
    let mut keeps: Option<Vec<String>> = None;
    let mut language = None;
    let mut currency = None;
    let mut keys_editable = None;
    let mut signin: Option<clarity_spec::signin::Signin> = None;
    // Whether the indented lines that follow belong to the signin block.
    let mut in_signin = false;
    let mut strategy_indent: Option<usize> = None;
    let mut engines_interpreted = None;
    let mut port = None;
    let mut row_height = None;
    let mut themes: Option<Vec<String>> = None;
    let mut datasources: Vec<DataSource> = Vec::new();
    let mut filestores: Vec<FileStore> = Vec::new();
    let mut codecs: Vec<crate::CodecLibrary> = Vec::new();
    let setting_indent = entries.get(1).map(|e| e.indent).unwrap_or(4);
    for entry in &entries[1..] {
        let line = entry.line;
        let keyword = entry.tokens.first().and_then(word).unwrap_or("");
        if entry.indent == 0 {
            return Err(format!("line {line}: a project file holds one project; indent its settings"));
        }
        // A strategy or an option under `signin`.
        if entry.indent > setting_indent && in_signin {
            let block = signin.as_mut().expect("in a signin block");
            crate::signin_file::line(block, line, entry.indent, &mut strategy_indent, keyword, &args(entry))?;
            continue;
        }
        if entry.indent <= setting_indent {
            in_signin = false;
        }
        // `openapi "forms.yaml"`, `pick "listTemplates", ...` and `pick schema
        // "Entry"` under a web API data source (DESIGN 5.23).
        if entry.indent > setting_indent && keyword == "paging" {
            let Some(source) = datasources.last_mut() else {
                return Err(format!("line {line}: 'paging' belongs under a datasource line, indented"));
            };
            if source.paging.is_some() {
                return Err(format!("line {line}: write 'paging' once per data source"));
            }
            source.paging = Some(paging(line, &args(entry))?);
            continue;
        }
        // `separator ";"` under a data source that is a file: an option of its format.
        if entry.indent > setting_indent
            && let Some(source) = datasources.last_mut()
            && let Some(file) = source.file.as_mut()
            && let Some(info) = clarity_formats::format(&file.format)
            && let Some(option) = info.options.iter().find(|o| o.name == keyword)
        {
            let a = args(entry);
            let value = match (option.kind, a.first()) {
                (clarity_formats::OptionKind::Text, Some(t)) if t.kind == TokenKind::Str && a.len() == 1 => t.text.clone(),
                (clarity_formats::OptionKind::Bool, Some(t)) if a.len() == 1 && matches!(word(t), Some("true" | "false")) => t.text.clone(),
                (clarity_formats::OptionKind::Text, _) => return Err(format!("line {line}: write the option and its text in quotes, like: {keyword} \";\"")),
                (clarity_formats::OptionKind::Bool, _) => return Err(format!("line {line}: write the option and true or false, like: {keyword} true")),
            };
            if file.options.iter().any(|o| o.name == keyword) {
                return Err(format!("line {line}: '{keyword}' is written twice"));
            }
            file.options.push(crate::SourceOption { name: keyword.to_string(), value });
            continue;
        }
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
                let written = string(line, a.first(), "the title")?;
                // The program's name (its settings folder, its package): no slots.
                if written.contains('{') || written.contains('}') {
                    return Err(format!("line {line}: the project's title is plain text, the program's name; show the version in a window's or the app's title with {{app.version}}, like \"Todo v{{app.version}}\""));
                }
                title = Some(written);
            }
            "version" => {
                once(version.is_some())?;
                match a.first() {
                    Some(t) if t.kind == TokenKind::Str && a.len() == 1 && is_version(&t.text) => version = Some(t.text.clone()),
                    _ => return Err(format!("line {line}: write 'version' and the version as major.minor.patch in quotes, like: version \"1.2.0\" (or \"2.0.0-beta.1\")")),
                }
            }
            "icon" => {
                once(icon.is_some())?;
                match a.first() {
                    Some(t) if t.kind == TokenKind::Str && a.len() == 1 => icon = Some(t.text.clone()),
                    _ => return Err(format!("line {line}: write 'icon' and the picture among the resources in quotes, like: icon \"app.png\"")),
                }
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
            "signin" => {
                once(signin.is_some())?;
                signin = Some(crate::signin_file::header(line, &a)?);
                in_signin = true;
                strategy_indent = None;
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
            // `ui library` and `ui crate` chose how a Rust desktop got Clarity.UI (v0.30.0): every one
            // links the ready-made library now. The line is read and left out when the file is written.
            "ui" if matches!((a.len(), a.first().and_then(|t| word(t))), (1, Some("library" | "crate"))) => {}
            "port" => {
                once(port.is_some())?;
                let number = match a.as_slice() {
                    [t] if t.kind == TokenKind::Number => t.text.parse::<u16>().ok().filter(|p| *p > 0),
                    _ => None,
                };
                port = Some(number.ok_or_else(|| format!("line {line}: write 'port' and a number from 1 to 65535: where the web application listens"))?);
            }
            "row" => {
                once(row_height.is_some())?;
                let number = match a.as_slice() {
                    [h, t] if word(h) == Some("height") && t.kind == TokenKind::Number => t.text.parse::<u32>().ok().filter(|n| clarity_spec::layout::row_height_fits(*n)),
                    _ => None,
                };
                row_height = Some(number.ok_or_else(|| format!("line {line}: {}", clarity_spec::layout::row_height_problem()))?);
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
                // A backend that draws with Clarity.UI takes it unnamed: `stack csharp desktop`,
                // `stack rust desktop`. A UI it also offers is still named.
                let unnamed = match words.first().map(|b| pick(line, "backend", "backends", &BACKENDS, backend_word, b)).transpose()? {
                    Some(backend) => {
                        let uis = backend_uis(backend);
                        let named = words.get(1).is_some_and(|w| uis.iter().any(|ui| *ui != Ui::Clarity && ui_word(*ui) == *w));
                        (uis.contains(&Ui::Clarity) && !named).then_some(backend)
                    }
                    None => None,
                };
                if let Some(backend) = unnamed {
                    // A UI named where the language draws with Clarity.UI only, as Rust's webview desktop once was.
                    if words.len() > 1 && UIS.iter().any(|ui| ui_word(*ui) == words[1]) {
                        if words.len() == 2 {
                            return Err(format!("line {line}: the stack needs at least one target, like: stack {} desktop", backend_word(backend)));
                        }
                        let targets = Stack { backend, ui: Ui::Clarity, targets: Vec::new() };
                        return Err(format!(
                            "line {line}: {} draws its windows with Clarity.UI and names no UI; write {} {}",
                            crate::backend_info(backend).label,
                            crate::stack_line(&targets),
                            words[2..].join(" ")
                        ));
                    }
                    // Clarity.UI is not named: the targets follow the language.
                    let targets = words[1..]
                        .iter()
                        .map(|t| pick(line, "target", "targets", &TARGETS, target_word, t))
                        .collect::<Result<Vec<_>, _>>()?;
                    if targets.is_empty() {
                        return Err(format!("line {line}: the stack needs at least one target, like: stack {} desktop", backend_word(backend)));
                    }
                    // A program without a window keeps the backend's usual UI, as `stack rust cli` always did.
                    let ui = if targets.iter().all(|t| *t == Target::Cli) { usual_ui(backend) } else { Ui::Clarity };
                    stack = Some(Stack { backend, ui, targets });
                } else if words.get(1) == Some(&target_word(Target::Cli)) {
                    // `stack go cli`: a program without a window names no UI.
                    let backend = pick(line, "backend", "backends", &BACKENDS, backend_word, words[0])?;
                    let targets = words[1..]
                        .iter()
                        .map(|t| pick(line, "target", "targets", &TARGETS, target_word, t))
                        .collect::<Result<Vec<_>, _>>()?;
                    if targets.iter().any(|t| *t != Target::Cli) {
                        return Err(format!(
                            "line {line}: a program with a window names its targets after the language, like: stack go desktop cli"
                        ));
                    }
                    stack = Some(Stack { backend, ui: usual_ui(backend), targets });
                } else {
                    let (Some(backend), Some(ui), targets) = (words.first(), words.get(1), words.get(2..).unwrap_or(&[]))
                    else {
                        return Err(format!(
                            "line {line}: write the backend and at least one target, like: stack go desktop"
                        ));
                    };
                    if targets.is_empty() {
                        return Err(format!(
                            "line {line}: the stack needs at least one target, like: stack go desktop"
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
                // A format of Clarity's catalog names a source that is a file in it.
                // `file ... using OrderFeed`: a file whose mapping says the format.
                let (kind, mut file) = match clarity_formats::format(kind) {
                    Some(info) => (crate::SourceKind::File, Some(crate::FileFormat { format: info.name.to_string(), options: Vec::new(), mapping: None })),
                    None if kind == source_word(crate::SourceKind::File) => (crate::SourceKind::File, Some(crate::FileFormat { format: String::new(), options: Vec::new(), mapping: None })),
                    None => (pick(line, "data source kind", "kinds", &SOURCE_KINDS, source_word, kind)?, None),
                };
                let url = string(line, a.get(2), "where the data source is")?;
                let (mut external, mut readonly, mut beside) = (false, false, false);
                let mut flags = a[3.min(a.len())..].iter();
                while let Some(flag) = flags.next() {
                    if word(flag) == Some("using") {
                        let Some(mapping) = flags.next().and_then(|t| word(t)) else {
                            return Err(format!("line {line}: write the mapping after 'using', like: datasource {name} file \"{url}\" external using OrderFeed"));
                        };
                        match file.as_mut() {
                            Some(f) if f.format.is_empty() && f.mapping.is_none() => f.mapping = Some(mapping.to_string()),
                            Some(f) if f.mapping.is_some() => return Err(format!("line {line}: 'using' is written twice")),
                            _ => return Err(format!("line {line}: a file laid over by a mapping is of kind file, since the mapping says its format: write it like: datasource {name} file \"{url}\" external using {mapping}")),
                        }
                        continue;
                    }
                    let slot = match word(flag) {
                        Some("external") => &mut external,
                        Some("readonly") => &mut readonly,
                        Some("beside") => &mut beside,
                        _ => {
                            return Err(format!(
                                "line {line}: unexpected '{}'; a data source can be external, readonly or beside, and a file can be laid over by a mapping: using OrderFeed",
                                flag.text
                            ));
                        }
                    };
                    if *slot {
                        return Err(format!("line {line}: '{}' is written twice", flag.text));
                    }
                    *slot = true;
                }
                if file.as_ref().is_some_and(|f| f.format.is_empty() && f.mapping.is_none()) {
                    return Err(format!("line {line}: a data source of kind file names the mapping that says its format: write it like: datasource {name} file \"{url}\" external using OrderFeed"));
                }
                if file.is_some() && !external {
                    return Err(format!("line {line}: a data source that is a file is external, since Clarity does not own its shape: write it like: datasource {name} {} \"{url}\" external{}", file.as_ref().map(|f| file_kind_word(f)).unwrap_or(""), file.as_ref().and_then(|f| f.mapping.as_ref()).map(|m| format!(" using {m}")).unwrap_or_default()));
                }
                datasources.push(DataSource {
                    name: name.to_string(),
                    kind,
                    url,
                    auth: None,
                    external,
                    readonly,
                    beside,
                    openapi: None,
                    picks: Vec::new(),
                    pick_schemas: Vec::new(),
                    paging: None,
                    file,
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
            "codec" => {
                let (Some(name), Some(from), Some(path), None) = (a.first().and_then(|t| word(t)), a.get(1), a.get(2), a.get(3)) else {
                    return Err(format!("line {line}: write the codec's name and its library, like: codec vp9 from \"codecs/libvp9\""));
                };
                if from.text != "from" {
                    return Err(format!("line {line}: write the codec's name and its library, like: codec vp9 from \"codecs/libvp9\""));
                }
                let from = string(line, Some(path), "the codec library's path")?;
                if codecs.iter().any(|c: &crate::CodecLibrary| c.name == name) {
                    return Err(format!("line {line}: codec {name} is named twice"));
                }
                codecs.push(crate::CodecLibrary { name: name.to_string(), from });
            }
            "auth" => return Err(format!("line {line}: 'auth' belongs under a datasource line, indented")),
            other => {
                return Err(format!(
                    "line {line}: unknown setting '{other}'; the settings are title, icon, stack, output, language, currency, theme, signin, keys, engines, port, row height, datasource, filestore and codec"
                ));
            }
        }
    }
    let Some(stack) = stack else {
        return Err("the project needs a 'stack' line, like: stack rust desktop".to_string());
    };
    if let Some(block) = &signin {
        crate::signin_file::finish(block, Some(&stack))?;
    }
    Ok(Settings {
        name,
        title,
        icon,
        version,
        stack,
        datasources,
        language,
        currency,
        themes: themes.unwrap_or_default(),
        filestores,
        codecs,
        out: out.unwrap_or_else(|| DEFAULT_OUT.to_string()),
        keeps: keeps.unwrap_or_default(),
        keys_editable: keys_editable.unwrap_or(false),
        signin,
        engines_interpreted: engines_interpreted.unwrap_or(false),
        port,
        row_height,
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

/// The kind a file source's line names: its format, or `file` when its mapping says the format.
fn file_kind_word(file: &crate::FileFormat) -> &str {
    if file.mapping.is_some() { source_word(crate::SourceKind::File) } else { &file.format }
}

/// Which setting a line holds, so a changed file can keep each line's
/// comments: `stack`, `datasource main`, `auth main`.
fn key(entry: &Entry, current_source: Option<&str>) -> String {
    let keyword = entry.tokens.first().map(|t| t.text.as_str()).unwrap_or("");
    match keyword {
        "datasource" => format!("datasource {}", entry.tokens.get(1).map(|t| t.text.as_str()).unwrap_or("")),
        "filestore" => format!("filestore {}", entry.tokens.get(1).map(|t| t.text.as_str()).unwrap_or("")),
        "codec" => format!("codec {}", entry.tokens.get(1).map(|t| t.text.as_str()).unwrap_or("")),
        "auth" => format!("auth {}", current_source.unwrap_or("")),
        "openapi" => format!("openapi {}", current_source.unwrap_or("")),
        "paging" => format!("paging {}", current_source.unwrap_or("")),
        "pick" if entry.tokens.get(1).map(|t| t.text.as_str()) == Some("schema") => format!("pick schema {}", current_source.unwrap_or("")),
        "pick" => format!("pick {}", current_source.unwrap_or("")),
        // An option of a file source's format, under its datasource line.
        option if clarity_formats::catalog().iter().any(|f| f.options.iter().any(|o| o.name == option)) => format!("{option} {}", current_source.unwrap_or("")),
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
        // The indent of the signin line while its block is being read.
        let mut signin_indent: Option<usize> = None;
        for entry in entries(text)? {
            let index = entry.line as usize - 1;
            if signin_indent.is_some_and(|i| entry.indent <= i) {
                signin_indent = None;
            }
            let key = match signin_indent {
                Some(_) => format!("signin {}", entry.tokens.first().map(|t| t.text.as_str()).unwrap_or("")),
                None => key(&entry, source.as_deref()),
            };
            if key.starts_with("datasource ") {
                source = entry.tokens.get(1).map(|t| t.text.clone());
            }
            if key == "signin" {
                signin_indent = Some(entry.indent);
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
    let stack_line = crate::stack_line(stack);
    let mut lines: Vec<(String, usize, String)> = vec![("project".into(), 0, format!("project {}", settings.name))];
    if let Some(title) = &settings.title {
        lines.push(("title".into(), 1, format!("title {}", quote(title))));
    }
    if let Some(icon) = &settings.icon {
        lines.push(("icon".into(), 1, format!("icon {}", quote(icon))));
    }
    if let Some(version) = &settings.version {
        lines.push(("version".into(), 1, format!("version {}", quote(version))));
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
    if let Some(signin) = &settings.signin {
        lines.extend(crate::signin_file::lines(signin));
    }
    if settings.keys_editable {
        lines.push(("keys".into(), 1, "keys editable".to_string()));
    }
    if settings.engines_interpreted {
        lines.push(("engines".into(), 1, "engines interpreted".to_string()));
    }
    if let Some(port) = settings.port {
        lines.push(("port".into(), 1, format!("port {port}")));
    }
    if let Some(height) = settings.row_height {
        if !clarity_spec::layout::row_height_fits(height) {
            return Err(clarity_spec::layout::row_height_problem());
        }
        lines.push(("row".into(), 1, format!("row height {height}")));
    }
    for source in &settings.datasources {
        lines.push((
            format!("datasource {}", source.name),
            1,
            format!(
                "datasource {} {} {}{}{}{}{}",
                source.name,
                source.file.as_ref().map(file_kind_word).unwrap_or(source_word(source.kind)),
                quote(&source.url),
                if source.external { " external" } else { "" },
                if source.readonly { " readonly" } else { "" },
                if source.beside { " beside" } else { "" },
                source.file.as_ref().and_then(|f| f.mapping.as_ref()).map(|m| format!(" using {m}")).unwrap_or_default()
            ),
        ));
        if let Some(auth) = &source.auth {
            lines.push((
                format!("auth {}", source.name),
                2,
                format!("auth {} {}", auth_word(auth.kind), quote(&auth.secret)),
            ));
        }
        if let Some(paging) = &source.paging {
            lines.push((format!("paging {}", source.name), 2, paging_line(paging)));
        }
        if let Some(file) = &source.file {
            let info = clarity_formats::format(&file.format);
            for option in &file.options {
                let bool_option = info.as_ref().and_then(|i| i.options.iter().find(|o| o.name == option.name)).is_some_and(|o| o.kind == clarity_formats::OptionKind::Bool);
                let value = if bool_option { option.value.clone() } else { quote(&option.value) };
                lines.push((format!("{} {}", option.name, source.name), 2, format!("{} {value}", option.name)));
            }
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
    for codec in &settings.codecs {
        lines.push((format!("codec {}", codec.name), 1, format!("codec {} from {}", codec.name, quote(&codec.from))));
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

/// Whether `text` is a version as `major.minor.patch`, with an optional
/// pre-release after a hyphen: `1.2.0`, `2.0.0-beta.1`.
pub fn is_version(text: &str) -> bool {
    let (core, pre) = match text.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (text, None),
    };
    let numbers: Vec<&str> = core.split('.').collect();
    let number = |n: &&str| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit());
    numbers.len() == 3 && numbers.iter().all(number) && pre.is_none_or(|p| !p.is_empty() && p.split('.').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric())))
}
