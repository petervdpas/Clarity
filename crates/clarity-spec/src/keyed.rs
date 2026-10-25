//! Make texts keys (I1c): a project written before texts had keys gets them.
//! Each text people read becomes a key, named after where it is (a part's
//! name and what the text is to it: `saveCaption`, `customerNameLabel`,
//! `cardTitle`) or, in code, after what it says (`orderIsApproved`), with the
//! text as the default language's value. A text with slots names its values
//! by their last word, which code gives by name:
//! `"Saved {customer.name}"` becomes `@savedName(name = customer.name)` with
//! the value `Saved {name}`. Translation files that translated texts by what
//! they said give the same keys their values, and every language gets every
//! key, empty where nothing translated it.

use std::collections::BTreeMap;

use crate::lexer::{Line, Token, TokenKind, lex, quote};
use crate::translations::{CLI_KEYS, KEYS_WINDOW_KEYS, SCREEN_ENGINE_KEYS, SECRET_KEYS, UNCONNECTED_KEYS, FILE_KEYS, LANGUAGE_PICKER_KEYS, PICTURE_KEYS, RENDERER_KEYS, PAGE_KEYS, STEP_KEYS, TAGS_KEYS, TREE_KEYS, THEME_PICKER_KEYS, has_letters};

/// What making texts keys gives: the modules as rewritten, and each
/// language's values by key, the default's first.
pub struct Keyed {
    /// Each module's path with its text, keys in place of texts.
    pub modules: Vec<(String, String)>,
    /// Each language's value of every key, in the order the keys were made.
    pub languages: BTreeMap<String, BTreeMap<String, String>>,
    /// The keys in the order they were made, for writing the files.
    pub order: Vec<String>,
    /// The comment lines each old file started with, kept on top.
    pub comments: BTreeMap<String, String>,
}

impl Keyed {
    /// The language file of `language`, every key in the order made.
    pub fn language_file(&self, language: &str) -> String {
        let values = &self.languages[language];
        let mut text = self.comments.get(language).cloned().unwrap_or_default();
        text.push_str(&format!("translation {language}\n"));
        for key in &self.order {
            text.push_str(&format!("    {key} {}\n", quote(values.get(key).map_or("", String::as_str))));
        }
        text
    }
}

/// Makes the texts of `modules` keys, in a project whose default language
/// is `default`, taking the translations of the old `translations` files.
pub fn make_keys(modules: &[(&str, &str)], translations: &[(&str, &str)], default: &str) -> Keyed {
    make_keys_beside(modules, translations, default, &BTreeMap::new())
}

/// Like `make_keys`, beside the keys the project has already with their
/// default values: a new text that says the same as one of them takes its
/// key, another gets a key of its own. Only the new keys are in the result.
pub fn make_keys_beside(modules: &[(&str, &str)], translations: &[(&str, &str)], default: &str, existing: &BTreeMap<String, String>) -> Keyed {
    let mut maker = Maker::default();
    for (key, value) in existing {
        maker.keys.insert(key.clone(), Made { text: value.clone(), was: value.clone(), slots: Vec::new() });
    }
    let mut rewritten = Vec::new();
    for (path, text) in modules {
        rewritten.push((path.to_string(), maker.module(text)));
    }
    // Clarity's own words; an image part bound to a picture has its own, and a wizard.
    let pictures = modules.iter().any(|(_, text)| text.lines().any(|l| l.trim_start().starts_with("image ") && l.contains(" bind ")));
    let wizards = modules.iter().any(|(_, text)| text.lines().any(|l| l.trim_start().starts_with("steps ")));
    let files = modules.iter().any(|(_, text)| text.lines().any(|l| l.trim_start().starts_with("file ") && l.contains(" bind ")));
    let paged = modules.iter().any(|(_, text)| text.lines().any(|l| l.trim_start().starts_with("table ") && l.split_whitespace().any(|w| w == "page")));
    let trees = modules.iter().any(|(_, text)| text.lines().any(|l| l.trim_start().starts_with("tree ") || l.trim_start().starts_with("treetable ")));
    let tags = modules.iter().any(|(_, text)| text.lines().any(|l| l.trim_start().starts_with("tags ") && l.contains(" bind ")));
    let commands = modules.iter().any(|(_, text)| text.lines().any(|l| l.starts_with("command ")));
    // Only the project file says which database connects with a secret: a
    // secret setting or an `on unconnected` may mean it does.
    let passwords = modules.iter().any(|(_, text)| text.lines().any(|l| l.trim_start().starts_with("password ")));
    let unconnected = modules.iter().any(|(_, text)| text.lines().any(|l| l.trim() == "on unconnected" || l.trim_start().starts_with("on unconnected:") || (l.starts_with("    ") && l.contains(": secret"))));
    // The pickers of the click targets `language` and `theme` have a heading.
    // The settings' own `language` and `theme` are labeled with the same words.
    let clicks_to = |target: &str| {
        modules.iter().any(|(_, text)| {
            text.lines().any(|l| l.split_whitespace().collect::<Vec<_>>().windows(2).any(|w| w == ["->", target]) || (l.starts_with(' ') && l.trim() == target))
        })
    };
    let own = RENDERER_KEYS
        .iter()
        .chain(if pictures { PICTURE_KEYS.as_slice() } else { &[] })
        .chain(if wizards { STEP_KEYS.as_slice() } else { &[] })
        .chain(if files { FILE_KEYS.as_slice() } else { &[] })
        .chain(if tags { TAGS_KEYS.as_slice() } else { &[] })
        .chain(if paged { PAGE_KEYS.as_slice() } else { &[] })
        .chain(if trees { TREE_KEYS.as_slice() } else { &[] })
        .chain(if clicks_to("language") { LANGUAGE_PICKER_KEYS.as_slice() } else { &[] })
        .chain(if clicks_to("theme") { THEME_PICKER_KEYS.as_slice() } else { &[] })
        .chain(if commands { CLI_KEYS.as_slice() } else { &[] })
        .chain(if unconnected { UNCONNECTED_KEYS.as_slice() } else { &[] })
        .chain(if passwords { SECRET_KEYS.as_slice() } else { &[] })
        .chain(if clicks_to("keys") { KEYS_WINDOW_KEYS.as_slice() } else { &[] })
        .chain(if modules.iter().any(|(_, text)| text.lines().any(|l| l.trim() == "beside the screen")) { SCREEN_ENGINE_KEYS.as_slice() } else { &[] });
    for (key, value) in own {
        maker.add_exact(key, value);
    }
    // The keys the project had stay out of the result.
    for key in existing.keys() {
        maker.keys.remove(key);
    }
    let mut languages = BTreeMap::new();
    let mut comments = BTreeMap::new();
    for (path, text) in translations {
        let language = std::path::Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let head: String = text.lines().take_while(|l| l.trim_start().starts_with("//")).map(|l| format!("{l}\n")).collect();
        if !head.is_empty() {
            comments.insert(language, head);
        }
    }
    let defaults: BTreeMap<String, String> = maker.keys.iter().map(|(k, v)| (k.clone(), v.text.clone())).collect();
    languages.insert(default.to_string(), defaults);
    for (path, text) in translations {
        let language = std::path::Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if language == default {
            continue;
        }
        let old = old_translations(text);
        let values = maker
            .keys
            .iter()
            .map(|(key, made)| {
                let translated = old.get(&made.was).map(|t| rename_slots(t, &made.slots)).unwrap_or_default();
                (key.clone(), translated)
            })
            .collect();
        languages.insert(language, values);
    }
    Keyed { modules: rewritten, languages, order: maker.order, comments }
}

/// A key made, with the text it was and how its slots were renamed.
struct Made {
    /// The default language's value.
    text: String,
    /// The text as written before, which old translations name.
    was: String,
    /// Each slot's path as written before, with its name now.
    slots: Vec<(String, String)>,
}

#[derive(Default)]
struct Maker {
    keys: BTreeMap<String, Made>,
    order: Vec<String>,
}

/// Where a line is: the block it is in, and its name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    None,
    Entity,
    Enum,
    Window,
    Theme,
    Engine,
    Code,
}

impl Maker {
    /// The key for `text` named `base`: `base` itself, or with a number when
    /// another text has it. The same text named alike gets the same key.
    fn add(&mut self, base: &str, text: &str, was: &str, slots: Vec<(String, String)>) -> String {
        let base = if base.is_empty() { "text".to_string() } else { base.to_string() };
        let mut key = base.clone();
        let mut n = 2;
        while let Some(made) = self.keys.get(&key) {
            if made.text == text {
                return key;
            }
            key = format!("{base}{n}");
            n += 1;
        }
        self.keys.insert(key.clone(), Made { text: text.to_string(), was: was.to_string(), slots });
        self.order.push(key.clone());
        key
    }

    /// One of Clarity's own words, by its fixed key.
    fn add_exact(&mut self, key: &str, text: &str) {
        if !self.keys.contains_key(key) {
            self.keys.insert(key.to_string(), Made { text: text.to_string(), was: text.to_string(), slots: Vec::new() });
            self.order.push(key.to_string());
        }
    }

    /// The module with keys in place of its texts.
    fn module(&mut self, source: &str) -> String {
        let (lines, _) = lex(source);
        let mut edits: Vec<(usize, usize, String)> = Vec::new();
        let (mut block, mut owner) = (Block::None, String::new());
        for line in &lines {
            let tokens = &line.tokens;
            let Some(first) = tokens.first() else { continue };
            if line.indent == 0 {
                (block, owner) = top(tokens);
                // A theme's or engine's own line holds no text.
                continue;
            }
            match block {
                Block::Entity => self.field(line, &owner, &mut edits),
                Block::Enum => self.variant(line, &owner, &mut edits),
                Block::Theme if first.text == "title" => self.at(tokens.get(1), &format!("{}Theme", lower(&owner)), &mut edits),
                Block::Engine if first.text == "says" => {
                    for token in tokens[1..].iter().filter(|t| t.kind == TokenKind::Str) {
                        let base = words_key(&token.text, &[]);
                        self.at(Some(token), &base, &mut edits);
                    }
                }
                Block::Window => self.window_line(line, &owner, &mut edits),
                Block::Code => self.code_line(line, &mut edits),
                _ => {}
            }
        }
        let mut text = source.to_string();
        edits.sort_by(|a, b| b.0.cmp(&a.0));
        for (start, end, with) in edits {
            text.replace_range(start..end, &with);
        }
        text
    }

    /// The text token `token` as key `base`, when it has letters.
    fn at(&mut self, token: Option<&Token>, base: &str, edits: &mut Vec<(usize, usize, String)>) {
        let Some(token) = token.filter(|t| t.kind == TokenKind::Str && has_letters(&t.text)) else { return };
        let key = self.add(base, &token.text, &token.text, Vec::new());
        edits.push((token.span.start as usize, token.span.end as usize, format!("@{key}")));
    }

    /// `name: text(80), label "Full name", help "..."`: its label and help, a
    /// label made from the name when it has none.
    fn field(&mut self, line: &Line, entity: &str, edits: &mut Vec<(usize, usize, String)>) {
        let tokens = &line.tokens;
        if tokens.len() < 2 || tokens[0].kind != TokenKind::Ident || tokens[1].kind != TokenKind::Colon {
            return;
        }
        let field = &tokens[0].text;
        let base = crate::names::camel_case(&format!("{entity} {field}"));
        let label = following(tokens, "label");
        match label {
            Some(token) if token.kind == TokenKind::Str => self.at(Some(token), &format!("{base}Label"), edits),
            Some(_) => {}
            None => {
                let key = self.add(&format!("{base}Label"), &crate::model::default_label(field), &crate::model::default_label(field), Vec::new());
                edits.push((line.span.end as usize, line.span.end as usize, format!(", label @{key}")));
            }
        }
        self.at(following(tokens, "help"), &format!("{base}Help"), edits);
    }

    /// `Private label "Private person"`: its label, made from its name when it has none.
    fn variant(&mut self, line: &Line, decl: &str, edits: &mut Vec<(usize, usize, String)>) {
        let tokens = &line.tokens;
        let Some(name) = tokens.first().filter(|t| t.kind == TokenKind::Ident) else { return };
        let base = crate::names::camel_case(&format!("{decl} {}", name.text));
        match following(tokens, "label") {
            Some(token) if token.kind == TokenKind::Str => self.at(Some(token), &base, edits),
            Some(_) => {}
            None => {
                let label = crate::model::default_label(&name.text);
                let key = self.add(&base, &label, &label, Vec::new());
                edits.push((line.span.end as usize, line.span.end as usize, format!(" label @{key}")));
            }
        }
    }

    /// A line of a window or the app: its title, data, parts and frame.
    fn window_line(&mut self, line: &Line, owner: &str, edits: &mut Vec<(usize, usize, String)>) {
        let tokens = &line.tokens;
        let first = &tokens[0].text;
        let owner = lower(owner);
        let name = following(tokens, "name").filter(|t| t.kind == TokenKind::Ident).map(|t| t.text.clone());
        match first.as_str() {
            "title" => self.at(tokens.get(1), &format!("{owner}Title"), edits),
            "status" => self.at(tokens.get(1), &format!("{owner}Status"), edits),
            "data" => {
                // A text value that starts with a text people read.
                if let Some(data) = tokens.get(1) {
                    self.at(following(tokens, "default"), &crate::names::camel_case(&format!("{owner} {}", data.text)), edits);
                }
            }
            // A ribbon's tab or group is named; a tab of a part is named by its title.
            "tab" | "group" => {
                let base = name.clone().unwrap_or_else(|| format!("{owner}{}Tab", upper(&words_key(tokens.get(1).map_or("", |t| t.text.as_str()), &[]))));
                self.at(tokens.get(1), &base, edits);
            }
            // A step of a wizard is named by its title.
            "step" => {
                let base = format!("{owner}{}Step", upper(&words_key(tokens.get(1).map_or("", |t| t.text.as_str()), &[])));
                self.at(tokens.get(1), &base, edits);
            }
            // A section of an accordion is named by its title.
            "section" => {
                let base = format!("{owner}{}Section", upper(&words_key(tokens.get(1).map_or("", |t| t.text.as_str()), &[])));
                self.at(tokens.get(1), &base, edits);
            }
            "item" => {
                let base = name.clone().unwrap_or_else(|| format!("{}Item", words_key(tokens.get(1).map_or("", |t| t.text.as_str()), &[])));
                self.at(tokens.get(1), &base, edits);
            }
            // An icon part's first word is the icon's name, no text people read.
            "icon" => {
                let part = name.clone().unwrap_or_else(|| "icon".to_string());
                self.at(following(tokens, "tooltip"), &format!("{part}Tooltip"), edits);
            }
            // A custom part (DESIGN 5.19): each text property's text is named after it.
            _ if first.starts_with(|c: char| c.is_ascii_uppercase()) => {
                let part = name.clone().unwrap_or_else(|| crate::names::camel_case(first));
                for pair in tokens.windows(2) {
                    if pair[1].kind == TokenKind::Str && pair[0].kind == TokenKind::Ident {
                        self.at(Some(&pair[1]), &format!("{part}{}", upper(&pair[0].text)), edits);
                    }
                }
            }
            _ => {
                let Some(kind) = crate::widgets::by_keyword(first).map(|_| first.clone()).or_else(|| (first == "button").then(|| first.clone())) else {
                    return;
                };
                let part = name.clone().unwrap_or(kind);
                // A label or help text for an input is named after the input.
                let base = match (first.as_str(), following(tokens, "for").map(|t| t.text.clone())) {
                    ("label", Some(input)) => format!("{input}Label"),
                    ("help", Some(input)) => format!("{input}Help"),
                    _ => format!("{part}Caption"),
                };
                self.at(tokens.get(1), &base, edits);
                self.at(following(tokens, "placeholder"), &format!("{part}Placeholder"), edits);
                self.at(following(tokens, "tooltip"), &format!("{part}Tooltip"), edits);
            }
        }
    }

    /// A line of code: a message, a question and its buttons, a refusal, a
    /// caption or an error, a text put in a value, and the text a function
    /// returns.
    fn code_line(&mut self, line: &Line, edits: &mut Vec<(usize, usize, String)>) {
        let tokens = &line.tokens;
        // `ask "Delete it?" yes "Delete" no "Keep" then ...`: the question and
        // its buttons; a default after `for` is a value, not a text people read.
        // In a command `ask` gives its answer, so it also follows `let n =` or `if`.
        if let Some(at) = tokens.iter().position(|t| t.kind == TokenKind::Ident && t.text == "ask") {
            let asked: Vec<&Token> = tokens
                .iter()
                .skip(at + 1)
                .take_while(|t| !(t.kind == TokenKind::Ident && matches!(t.text.as_str(), "for" | "then")))
                .filter(|t| t.kind == TokenKind::Str)
                .collect();
            for token in asked {
                self.code_text(token, edits);
            }
            return;
        }
        let text = match tokens.first().map(|t| t.text.as_str()) {
            // A local's text is code's own.
            Some("let") => None,
            Some("message") => tokens.iter().skip(1).find(|t| t.kind == TokenKind::Str),
            Some("refuse") | Some("return") => tokens.get(1),
            // A command's `print`, and the `help` of a command, an option or the cli block.
            Some("print" | "help") => tokens.get(1),
            Some("option") => following(tokens, "help"),
            // `part.caption = "..."`, `part.error = "..."`, `value = "..."`; a
            // record's field holds data, not a text people read.
            _ => match tokens.iter().position(|t| t.kind == TokenKind::Assign) {
                Some(1) if tokens.len() == 3 => tokens.get(2),
                Some(3) if tokens.len() == 5 && tokens[1].kind == TokenKind::Dot && matches!(tokens[2].text.as_str(), "caption" | "error") => tokens.get(4),
                _ => None,
            },
        };
        if let Some(token) = text {
            self.code_text(token, edits);
        }
    }

    /// A text of code people read, made a key with its slots filled by name.
    fn code_text(&mut self, token: &Token, edits: &mut Vec<(usize, usize, String)>) {
        if token.kind != TokenKind::Str || !has_letters(&token.text) {
            return;
        }
        let slots = slot_names(&token.text);
        let value = rename_slots(&token.text, &slots);
        let names: Vec<String> = slots.iter().map(|(_, n)| n.clone()).collect();
        let key = self.add(&words_key(&value, &names), &value, &token.text, slots.clone());
        let args = if slots.is_empty() {
            String::new()
        } else {
            format!("({})", slots.iter().map(|(path, name)| format!("{name} = {path}")).collect::<Vec<_>>().join(", "))
        };
        edits.push((token.span.start as usize, token.span.end as usize, format!("@{key}{args}")));
    }
}

/// The block a top-level line starts, and its name.
fn top(tokens: &[Token]) -> (Block, String) {
    let name = match tokens[0].text.as_str() {
        "settings" => crate::parser::SETTINGS_ENTITY.to_string(),
        _ => tokens.get(1).map(|t| t.text.clone()).unwrap_or_default(),
    };
    let block = match tokens[0].text.as_str() {
        "entity" => Block::Entity,
        // The settings' fields are keyed as an entity's: `settingsPageSizeLabel`.
        "settings" => Block::Entity,
        "enum" => Block::Enum,
        "page" | "app" => Block::Window,
        "theme" => Block::Theme,
        "engine" => Block::Engine,
        "code" | "function" | "action" | "command" | "cli" => Block::Code,
        _ => Block::None,
    };
    (block, name)
}

/// The token after the word `word` on the line.
fn following<'a>(tokens: &'a [Token], word: &str) -> Option<&'a Token> {
    tokens.windows(2).find(|w| w[0].kind == TokenKind::Ident && w[0].text == word).map(|w| &w[1])
}

/// A key made of a text's first words and its slots: `Saved {name}` is `savedName`.
fn words_key(text: &str, _slots: &[String]) -> String {
    let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).take(5).collect();
    crate::names::camel_case(&words.join(" "))
}

/// Each slot's path with the name it gets: its last word, or the whole path
/// when another slot ends in the same word.
fn slot_names(text: &str) -> Vec<(String, String)> {
    let paths = crate::translations::slots(text);
    let last = |p: &str| p.rsplit('.').next().unwrap_or(p).to_string();
    let mut named: Vec<(String, String)> = Vec::new();
    for path in &paths {
        if named.iter().any(|(p, _)| p == path) {
            continue;
        }
        let shared = paths.iter().filter(|other| last(other) == last(path)).count() > 1;
        let name = if shared { crate::names::camel_case(&path.replace('.', " ")) } else { crate::names::camel_case(&last(path)) };
        named.push((path.clone(), name));
    }
    named
}

/// `text` with each slot's path renamed.
fn rename_slots(text: &str, slots: &[(String, String)]) -> String {
    let mut out = text.to_string();
    for (path, name) in slots {
        out = out.replace(&format!("{{{path}}}"), &format!("{{{name}}}"));
    }
    out
}

/// An old translation file: each text with its translation.
fn old_translations(text: &str) -> BTreeMap<String, String> {
    let (lines, _) = lex(text);
    lines
        .iter()
        .filter_map(|l| match l.tokens.as_slice() {
            [from, to] if from.kind == TokenKind::Str && to.kind == TokenKind::Str => Some((from.text.clone(), to.text.clone())),
            _ => None,
        })
        .collect()
}

fn lower(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_lowercase().chain(chars).collect()).unwrap_or_default()
}

fn upper(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// Adds `keys` (each with its default value) to the language files in the
/// module folder `dir`: with its value in the default language, empty in
/// the others, a key a file has already left as it is. Without a file for
/// the default language, it is made. Gives each file written, with what it
/// held before.
pub fn add_keys(dir: &std::path::Path, default: &str, keys: &[(String, String)]) -> std::io::Result<Vec<(std::path::PathBuf, Option<String>, String)>> {
    let planned = plan_keys(dir, default, keys)?;
    for (path, _, after) in &planned {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(path, after)?;
    }
    Ok(planned)
}

/// What `add_keys` would write: each language file that changes, with what
/// it holds now (none for one not there yet) and after.
pub fn plan_keys(dir: &std::path::Path, default: &str, keys: &[(String, String)]) -> std::io::Result<Vec<(std::path::PathBuf, Option<String>, String)>> {
    let folder = dir.join(crate::translations::TRANSLATIONS_DIR);
    let mut languages: Vec<std::path::PathBuf> = match std::fs::read_dir(&folder) {
        Ok(entries) => entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "clarity")).collect(),
        Err(_) => Vec::new(),
    };
    let default_file = folder.join(format!("{default}.clarity"));
    if !languages.contains(&default_file) {
        languages.push(default_file);
    }
    languages.sort();
    let mut written = Vec::new();
    for path in languages {
        let language = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let before = std::fs::read_to_string(&path).ok();
        let (parsed, _) = crate::translations::parse(before.as_deref().unwrap_or(""));
        let mut after = before.clone().unwrap_or_else(|| format!("translation {language}\n"));
        for (key, value) in keys {
            if parsed.entries.iter().any(|e| &e.key == key) {
                continue;
            }
            let value = if language == default { value.as_str() } else { "" };
            if !after.ends_with('\n') {
                after.push('\n');
            }
            after.push_str(&format!("    {key} {}\n", quote(value)));
        }
        if before.as_deref() != Some(after.as_str()) {
            written.push((path, before, after));
        }
    }
    Ok(written)
}

/// Loads `sources` as a project whose texts are keys: their texts made
/// keys as `make_keys` does, the language files made from them and from
/// `translations` (old ones, texts translated by what they said), checked
/// together. For tests and tools that start from texts as written.
pub fn load_keyed<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>, translations: &[(&str, &str)], default: &str) -> crate::Project {
    let sources: Vec<(&str, &str)> = sources.into_iter().collect();
    let made = make_keys(&sources, translations, default);
    let mut project = crate::load_sources(made.modules.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    project.locale.language = Some(default.to_string());
    let files: Vec<(String, String)> = made.languages.keys().map(|l| (format!("translations/{l}.clarity"), made.language_file(l))).collect();
    crate::translations::attach(&mut project, files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    project
}

/// One module's text with every text people read made a key, beside the
/// keys the project has (`existing`, by their default values): what an edit
/// or a template wrote as text comes back as keys, with the new keys and
/// their values. A text that says the same as an existing key takes it.
pub fn key_module(path: &str, text: &str, default: &str, existing: &BTreeMap<String, String>) -> (String, Vec<(String, String)>) {
    let made = make_keys_beside(&[(path, text)], &[], default, existing);
    let keys = made.order.iter().map(|k| (k.clone(), made.languages[default][k].clone())).collect();
    (made.modules.into_iter().next().map(|(_, t)| t).unwrap_or_default(), keys)
}

/// A module's text with key `from` named `to` wherever design and code name
/// it; a string that says the name stays as it is.
pub fn rename_key_in_module(text: &str, from: &str, to: &str) -> String {
    let (lines, _) = lex(text);
    let mut out = text.to_string();
    let mut spots: Vec<(usize, usize)> = lines
        .iter()
        .flat_map(|l| &l.tokens)
        .filter(|t| t.kind == TokenKind::Key && t.text == from)
        .map(|t| (t.span.start as usize, t.span.end as usize))
        .collect();
    spots.sort_by(|a, b| b.0.cmp(&a.0));
    for (start, end) in spots {
        out.replace_range(start..end, &format!("@{to}"));
    }
    out
}

/// A language file's text with key `from` named `to`, its value kept.
pub fn rename_key_in_language(text: &str, from: &str, to: &str) -> String {
    let (lines, _) = lex(text);
    let mut out = text.to_string();
    let mut spots: Vec<(usize, usize)> = lines
        .iter()
        .filter_map(|l| l.tokens.first().filter(|t| t.kind == TokenKind::Ident && t.text == from && l.tokens.len() == 2))
        .map(|t| (t.span.start as usize, t.span.end as usize))
        .collect();
    spots.sort_by(|a, b| b.0.cmp(&a.0));
    for (start, end) in spots {
        out.replace_range(start..end, to);
    }
    out
}
