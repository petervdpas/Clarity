//! Command-line programs (DESIGN 5.20): reading a command line against the
//! program's commands, writing its help, printing a table and asking on a
//! terminal, as conformance/command-line.json says every runtime does.
//! A generated program describes itself as a `Program` and runs the
//! command `parse` finds with the values it read.

use std::collections::BTreeMap;
use std::io::{BufRead, IsTerminal, Write};
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;

/// Clarity's own words of the command line, in English: what a program
/// shows for a key its translations leave out.
pub const WORDS: [(&str, &str); 20] = [
    ("cliUsage", "Usage"),
    ("cliCommands", "Commands"),
    ("cliOptions", "Options"),
    ("cliCommandWord", "command"),
    ("cliOptionsWord", "options"),
    ("cliHelpOption", "Shows this help"),
    ("cliDefault", "default {value}"),
    ("cliRequiredMark", "required"),
    ("cliUnknownCommand", "There is no command {command}"),
    ("cliUnknownOption", "There is no option {option}"),
    ("cliMissingArgument", "{argument} is missing"),
    ("cliTooManyArguments", "There are more values than arguments"),
    ("cliNeedsValue", "{option} needs a value"),
    ("cliWrongValue", "{name} cannot be {value}"),
    ("cliTooSmall", "{name} is at least {min}"),
    ("cliTooLarge", "{name} is at most {max}"),
    ("cliRequired", "{option} is required"),
    ("cliWrongAnswer", "{value} does not fit; try again"),
    ("cliNoTerminal", "There is nobody to answer: {question}"),
    ("cliChooseNumber", "Type a number from 1 to {count}"),
];

/// What an argument or option holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Text,
    Integer,
    Decimal,
    Money,
    Date,
    Datetime,
    Duration,
    Enum,
    Bool,
}

/// An argument or an option of a command; names as typed (`call-after`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Param {
    pub name: String,
    pub kind: Kind,
    /// The values of an enum, by name.
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub min: Option<String>,
    #[serde(default)]
    pub max: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub help: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Command {
    /// The words typed: `["customer", "add"]`.
    pub line: Vec<String>,
    #[serde(default)]
    pub help: Option<String>,
    #[serde(default)]
    pub arguments: Vec<Param>,
    #[serde(default)]
    pub options: Vec<Param>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Program {
    pub name: String,
    #[serde(default)]
    pub help: Option<String>,
    /// In the order they are declared.
    pub commands: Vec<Command>,
}

/// Clarity's words in the language shown: the translation of a key, or
/// its English.
pub struct Words<'a> {
    translated: &'a dyn Fn(&str) -> Option<String>,
}

impl<'a> Words<'a> {
    pub fn new(translated: &'a dyn Fn(&str) -> Option<String>) -> Self {
        Words { translated }
    }

    /// The word of `key` with its slots filled.
    pub fn say(&self, key: &str, slots: &[(&str, &str)]) -> String {
        let english = || WORDS.iter().find(|(k, _)| *k == key).map_or(key, |(_, v)| *v).to_string();
        let mut text = (self.translated)(key).filter(|v| !v.is_empty()).unwrap_or_else(english);
        for (name, value) in slots {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }
}

/// What a command line becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    /// Run `command` (its index in the program) with every argument's
    /// value and then every option's, in their order: text as read, none
    /// for the empty value.
    Run { command: usize, values: Vec<Option<String>> },
    /// Help to write to standard output; the program ends with 0.
    Help(String),
    /// A wrong command line: the key and slots of its message, and what is
    /// written to standard error; the program ends with 2.
    Wrong { key: &'static str, slots: Vec<(String, String)>, said: String },
}

/// Reads `args`, the words after the program's name.
pub fn parse(program: &Program, args: &[String], words: &Words) -> Parsed {
    let end = args.iter().position(|a| a == "--").unwrap_or(args.len());
    let mut asked_help = args[..end].iter().any(|a| a == "--help");
    let mut rest: Vec<&str> = args.iter().enumerate().filter(|(i, a)| *i >= end || *a != "--help").map(|(_, a)| a.as_str()).collect();
    if rest.first() == Some(&"help") {
        asked_help = true;
        rest.remove(0);
    }
    // The command, or the group read so far.
    let mut path: Vec<&str> = Vec::new();
    let mut at = 0;
    let command = loop {
        let Some(word) = rest.get(at).copied().filter(|w| !w.starts_with("--")) else {
            if rest.get(at).is_some() && !asked_help && path.is_empty() {
                let option = rest[at].split('=').next().unwrap_or_default().to_string();
                return wrong(program, &[], None, words, "cliUnknownOption", &[("option", &option)]);
            }
            return Parsed::Help(group_help(program, &path, words));
        };
        let mut line = path.clone();
        line.push(word);
        if let Some(i) = program.commands.iter().position(|c| c.line.iter().map(String::as_str).eq(line.iter().copied())) {
            at += 1;
            break i;
        }
        if program.commands.iter().any(|c| c.line.len() > line.len() && c.line.iter().map(String::as_str).take(line.len()).eq(line.iter().copied())) {
            path = line;
            at += 1;
            continue;
        }
        return wrong(program, &path, None, words, "cliUnknownCommand", &[("command", &line.join(" "))]);
    };
    let c = &program.commands[command];
    if asked_help {
        return Parsed::Help(command_help(program, c, words));
    }
    let say_wrong = |key: &'static str, slots: &[(&str, &str)]| wrong(program, &[], Some(c), words, key, slots);

    // The words after the command: options by name, and the arguments.
    let words_after = &rest[at..];
    let ends_at = words_after.iter().position(|w| *w == "--");
    let option_of = |word: &str| -> Option<(usize, Option<String>)> {
        let (name, value) = match word.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (word, None),
        };
        let name = name.strip_prefix("--")?;
        c.options.iter().position(|o| o.name == name).map(|i| (i, value))
    };
    for (i, word) in words_after.iter().enumerate() {
        if ends_at.is_some_and(|e| i >= e) {
            break;
        }
        if word.starts_with("--") && option_of(word).is_none() {
            let option = word.split('=').next().unwrap_or_default();
            return say_wrong("cliUnknownOption", &[("option", option)]);
        }
    }
    let mut given: Vec<(usize, String)> = Vec::new();
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < words_after.len() {
        let word = words_after[i];
        if ends_at == Some(i) {
            positional.extend(words_after[i + 1..].iter().map(|w| w.to_string()));
            break;
        }
        if word.starts_with("--") {
            let (index, value) = option_of(word).expect("checked above");
            let option = &c.options[index];
            let value = match (option.kind, value) {
                (Kind::Bool, None) => "true".to_string(),
                (_, Some(value)) => value,
                (_, None) => match words_after.get(i + 1) {
                    Some(next) => {
                        i += 1;
                        next.to_string()
                    }
                    None => return say_wrong("cliNeedsValue", &[("option", &format!("--{}", option.name))]),
                },
            };
            given.retain(|(g, _)| *g != index);
            given.push((index, value));
        } else {
            positional.push(word.to_string());
        }
        i += 1;
    }

    // Each value in the order it was typed: of its kind, then within its range.
    let mut read: Vec<(String, &Param, String)> = Vec::new();
    for (argument, value) in c.arguments.iter().zip(&positional) {
        read.push((argument.name.clone(), argument, value.clone()));
    }
    for (index, value) in &given {
        let option = &c.options[*index];
        read.push((format!("--{}", option.name), option, value.clone()));
    }
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    for (name, param, value) in &read {
        match read_value(param, value) {
            Some(v) => {
                values.insert(name.clone(), v);
            }
            None => return say_wrong("cliWrongValue", &[("name", name), ("value", value)]),
        }
    }
    for (name, param, _) in &read {
        let value = &values[name];
        if let Some(min) = &param.min
            && below(value, min)
        {
            return say_wrong("cliTooSmall", &[("name", name), ("min", min)]);
        }
        if let Some(max) = &param.max
            && below(max, value)
        {
            return say_wrong("cliTooLarge", &[("name", name), ("max", max)]);
        }
    }
    if let Some(missing) = c.arguments.get(positional.len()) {
        return say_wrong("cliMissingArgument", &[("argument", &missing.name)]);
    }
    if positional.len() > c.arguments.len() {
        return say_wrong("cliTooManyArguments", &[]);
    }
    if let Some(required) = c.options.iter().enumerate().find(|(i, o)| o.required && !given.iter().any(|(g, _)| g == i)) {
        return say_wrong("cliRequired", &[("option", &format!("--{}", required.1.name))]);
    }
    let mut out: Vec<Option<String>> = c.arguments.iter().map(|a| values.get(&a.name).cloned()).collect();
    for option in &c.options {
        let typed = values.get(&format!("--{}", option.name)).cloned();
        out.push(typed.or_else(|| option.default.clone()).or_else(|| (option.kind == Kind::Bool).then(|| "false".to_string())));
    }
    Parsed::Run { command, values: out }
}

/// `value` as text of `param`'s kind, or none when it is not one.
pub fn read_value(param: &Param, value: &str) -> Option<String> {
    match param.kind {
        Kind::Text => Some(value.to_string()),
        Kind::Bool => matches!(value, "true" | "false").then(|| value.to_string()),
        Kind::Integer => {
            let digits = value.strip_prefix('-').unwrap_or(value);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            value.parse::<i64>().ok().map(|n| n.to_string())
        }
        Kind::Decimal | Kind::Money => {
            let digits = value.strip_prefix('-').unwrap_or(value);
            let (whole, part) = digits.split_once('.').unwrap_or((digits, "0"));
            let plain = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
            if !plain(whole) || !plain(part) {
                return None;
            }
            Decimal::from_str(value).ok().map(|d| d.normalize().to_string())
        }
        Kind::Date => {
            if value.len() != 10 {
                return None;
            }
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok().map(|d| d.format("%Y-%m-%d").to_string())
        }
        Kind::Datetime => {
            let moment = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M"))
                .ok()?;
            (value.len() == 16 || value.len() == 19).then(|| moment.format("%Y-%m-%dT%H:%M:%S").to_string())
        }
        Kind::Duration => duration(value).map(|s| s.to_string()),
        Kind::Enum => param.values.iter().find(|v| v.eq_ignore_ascii_case(value)).cloned(),
    }
}

/// Whole seconds from `45`, `3:45` or `1:02:03`, as formats.json reads them.
fn duration(text: &str) -> Option<i64> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    let number = |s: &str| (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse::<i64>().ok()).flatten();
    let two = |s: &str| (s.len() == 2).then(|| number(s)).flatten().filter(|n| *n < 60);
    match parts.as_slice() {
        [s] => number(s),
        [m, s] => Some(number(m)? * 60 + two(s)?),
        [h, m, s] => Some(number(h)? * 3600 + two(m)? * 60 + two(s)?),
        _ => None,
    }
}

/// Whether the number `a` is less than `b`, exactly.
fn below(a: &str, b: &str) -> bool {
    match (Decimal::from_str(a), Decimal::from_str(b)) {
        (Ok(a), Ok(b)) => a < b,
        _ => false,
    }
}

fn wrong(program: &Program, path: &[&str], command: Option<&Command>, words: &Words, key: &'static str, slots: &[(&str, &str)]) -> Parsed {
    let message = words.say(key, slots);
    let usage = match command {
        Some(c) => command_usage(program, c, words),
        None => group_usage(program, path, words),
    };
    Parsed::Wrong { key, slots: slots.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), said: format!("{message}\n{usage}\n") }
}

fn group_usage(program: &Program, path: &[&str], words: &Words) -> String {
    let mut line = vec![program.name.as_str()];
    line.extend(path);
    format!("{}: {} <{}> [{}]", words.say("cliUsage", &[]), line.join(" "), words.say("cliCommandWord", &[]), words.say("cliOptionsWord", &[]))
}

fn command_usage(program: &Program, c: &Command, words: &Words) -> String {
    let mut line = vec![program.name.clone()];
    line.extend(c.line.iter().cloned());
    line.extend(c.arguments.iter().map(|a| format!("<{}>", a.name)));
    format!("{}: {} [{}]", words.say("cliUsage", &[]), line.join(" "), words.say("cliOptionsWord", &[]))
}

/// A heading and its lines, the left column padded to the longest.
fn section(heading: &str, rows: &[(String, String)]) -> Vec<String> {
    let width = rows.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(0);
    let mut out = vec![format!("{heading}:")];
    for (left, right) in rows {
        let pad = " ".repeat(width - left.chars().count() + 2);
        let line = if right.is_empty() { format!("  {left}") } else { format!("  {left}{pad}{right}") };
        out.push(line.trim_end().to_string());
    }
    out
}

/// The help of the program (`path` empty) or of a group.
pub fn group_help(program: &Program, path: &[&str], words: &Words) -> String {
    let mut lines = Vec::new();
    if path.is_empty()
        && let Some(help) = &program.help
    {
        lines.push(help.clone());
        lines.push(String::new());
    }
    lines.push(group_usage(program, path, words));
    lines.push(String::new());
    let commands: Vec<(String, String)> = program
        .commands
        .iter()
        .filter(|c| c.line.iter().map(String::as_str).take(path.len()).eq(path.iter().copied()) && c.line.len() > path.len())
        .map(|c| (c.line.join(" "), c.help.clone().unwrap_or_default()))
        .collect();
    lines.extend(section(&words.say("cliCommands", &[]), &commands));
    lines.push(String::new());
    lines.extend(section(&words.say("cliOptions", &[]), &[("--help".to_string(), words.say("cliHelpOption", &[]))]));
    lines.join("\n") + "\n"
}

/// The help of one command: what it does, its usage and its options.
pub fn command_help(program: &Program, c: &Command, words: &Words) -> String {
    let mut lines = Vec::new();
    if let Some(help) = &c.help {
        lines.push(help.clone());
        lines.push(String::new());
    }
    lines.push(command_usage(program, c, words));
    lines.push(String::new());
    let mut rows: Vec<(String, String)> = c
        .options
        .iter()
        .map(|o| {
            let left = if o.kind == Kind::Bool { format!("--{}", o.name) } else { format!("--{} <{}>", o.name, o.name) };
            let mut marks = Vec::new();
            if !o.values.is_empty() {
                marks.push(o.values.join(", "));
            }
            if let Some(default) = o.default.as_deref().filter(|_| o.kind != Kind::Bool) {
                marks.push(words.say("cliDefault", &[("value", default)]));
            }
            if o.required {
                marks.push(words.say("cliRequiredMark", &[]));
            }
            let mut right = o.help.clone().unwrap_or_default();
            if !marks.is_empty() {
                if !right.is_empty() {
                    right.push(' ');
                }
                right.push_str(&format!("({})", marks.join("; ")));
            }
            (left, right)
        })
        .collect();
    rows.push(("--help".to_string(), words.say("cliHelpOption", &[])));
    lines.extend(section(&words.say("cliOptions", &[]), &rows));
    lines.join("\n") + "\n"
}

/// `print list columns a, b`: a line of headings, then a line per row,
/// the columns two spaces apart, a column of numbers aligned right.
pub fn table(headings: &[String], right: &[bool], rows: &[Vec<String>]) -> String {
    let heading_row = headings.to_vec();
    let lines: Vec<&Vec<String>> = std::iter::once(&heading_row).chain(rows.iter()).collect();
    let widths: Vec<usize> = (0..headings.len()).map(|i| lines.iter().map(|r| r.get(i).map_or(0, |c| c.chars().count())).max().unwrap_or(0)).collect();
    let mut out = String::new();
    for row in lines {
        let cells: Vec<String> = widths
            .iter()
            .enumerate()
            .map(|(i, width)| {
                let cell = row.get(i).map_or("", String::as_str);
                let pad = " ".repeat(width - cell.chars().count());
                if right.get(i).copied().unwrap_or(false) { format!("{pad}{cell}") } else { format!("{cell}{pad}") }
            })
            .collect();
        out.push_str(cells.join("  ").trim_end());
        out.push('\n');
    }
    out
}

/// What a command asks.
#[derive(Debug, Clone)]
pub enum Asked {
    /// Yes or no, by the words of its two buttons.
    Confirm { yes: String, no: String },
    /// A value of a kind, with its default.
    Value { param: Param },
    /// One of these, by number from 1.
    Choice { choices: Vec<String> },
}

/// Where a command asks: a terminal, or a pipe or script with nobody at it.
pub trait Terminal {
    fn is_terminal(&self) -> bool;
    /// The next line typed, without its end; none at the end of input.
    fn read_line(&mut self) -> Option<String>;
    /// Writes to standard error, where questions go.
    fn write(&mut self, text: &str);
}

/// The terminal the program runs in.
pub struct Console;

impl Terminal for Console {
    /// A terminal, or Clarity's Output panel, which says so with `CLARITY_TERMINAL=1`.
    fn is_terminal(&self) -> bool {
        std::io::stdin().is_terminal() || std::env::var("CLARITY_TERMINAL").is_ok_and(|v| v == "1")
    }

    fn read_line(&mut self) -> Option<String> {
        let mut line = String::new();
        match std::io::stdin().lock().read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim_end_matches(['\r', '\n']).to_string()),
        }
    }

    fn write(&mut self, text: &str) {
        let mut err = std::io::stderr();
        let _ = err.write_all(text.as_bytes());
        let _ = err.flush();
    }
}

/// Asks `question` and gives the answer as text (`true` or `false` for yes
/// or no, the number of a choice), or the exit code the program ends with.
pub fn ask(question: &str, asked: &Asked, terminal: &mut dyn Terminal, words: &Words) -> Result<String, i32> {
    if !terminal.is_terminal() {
        if let Asked::Value { param } = asked
            && let Some(default) = &param.default
        {
            return Ok(default.clone());
        }
        terminal.write(&format!("{}\n", words.say("cliNoTerminal", &[("question", question)])));
        return Err(2);
    }
    match asked {
        Asked::Confirm { yes, no } => loop {
            terminal.write(&format!("{question} [{yes}/{no}] "));
            match terminal.read_line() {
                None => return Ok("false".to_string()),
                Some(line) if line.trim().is_empty() => return Ok("false".to_string()),
                Some(line) if line.trim().to_lowercase() == yes.to_lowercase() => return Ok("true".to_string()),
                Some(line) if line.trim().to_lowercase() == no.to_lowercase() => return Ok("false".to_string()),
                Some(_) => {}
            }
        },
        Asked::Value { param } => {
            let shown = if param.kind == Kind::Enum { Some(param.values.join("/")) } else { param.default.clone() };
            loop {
                match &shown {
                    Some(shown) => terminal.write(&format!("{question} [{shown}] ")),
                    None => terminal.write(&format!("{question} ")),
                }
                let Some(line) = terminal.read_line() else {
                    terminal.write("\n");
                    return Err(1);
                };
                let line = line.trim();
                if line.is_empty()
                    && let Some(default) = &param.default
                {
                    return Ok(default.clone());
                }
                match read_value(param, line) {
                    Some(value) => return Ok(value),
                    None => terminal.write(&format!("{}\n", words.say("cliWrongAnswer", &[("value", line)]))),
                }
            }
        }
        Asked::Choice { choices } => {
            let listed: String = choices.iter().enumerate().map(|(i, c)| format!("{}. {c}\n", i + 1)).collect();
            terminal.write(&listed);
            let count = choices.len().to_string();
            loop {
                terminal.write(&format!("{question} "));
                let Some(line) = terminal.read_line() else {
                    terminal.write("\n");
                    return Err(1);
                };
                match line.trim().parse::<usize>() {
                    Ok(n) if (1..=choices.len()).contains(&n) => return Ok(n.to_string()),
                    _ => terminal.write(&format!("{}\n", words.say("cliChooseNumber", &[("count", &count)]))),
                }
            }
        }
    }
}

/// A value read from the command line, or answered, as the type code holds
/// it: none is the empty value of that type, as is a text that does not fit.
pub fn typed<T: serde::de::DeserializeOwned + Default>(kind: Kind, text: Option<&str>) -> T {
    let Some(text) = text else { return T::default() };
    let value = match kind {
        Kind::Integer | Kind::Duration => text.parse::<i64>().map_or(serde_json::Value::Null, serde_json::Value::from),
        Kind::Bool => serde_json::Value::Bool(text == "true"),
        _ => serde_json::Value::String(text.to_string()),
    };
    serde_json::from_value(value).unwrap_or_default()
}

/// A value of code as the command line writes it: `42`, `12.5`,
/// `2026-10-23`, an enum value by its name; "" for none.
pub fn text_of<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        Ok(serde_json::Value::Null) | Err(_) => String::new(),
        Ok(other) => other.to_string(),
    }
}

/// The language to show for the system's `locale` (`nl_NL.UTF-8`), among
/// the project's translations; none for the language the project is
/// written in (`base`), or one it has no translation for.
pub fn pick_language(locale: &str, available: &[String], base: &str) -> Option<String> {
    let tag = locale.split(['.', '@']).next().unwrap_or_default().replace('_', "-");
    let primary = |t: &str| t.split('-').next().unwrap_or_default().to_lowercase();
    if tag.is_empty() || tag == "C" || tag == "POSIX" || primary(&tag) == primary(base) {
        return None;
    }
    available.iter().find(|a| **a == tag).or_else(|| available.iter().find(|a| primary(a) == primary(&tag))).cloned()
}

fn translated(key: &str) -> Option<String> {
    crate::i18n::value(key)
}

/// What a running command prints, says and asks through: standard output,
/// standard error and the terminal, in the language shown.
pub struct Session {
    terminal: Box<dyn Terminal>,
}

impl Default for Session {
    fn default() -> Self {
        Session { terminal: Box::new(Console) }
    }
}

impl Session {
    fn words() -> Words<'static> {
        Words::new(&translated)
    }

    /// `print value`: one line on standard output.
    pub fn print(&mut self, text: crate::ui::Text) {
        println!("{}", text.show());
    }

    /// `print list columns a, b`: the records as a table on standard output.
    pub fn print_table(&mut self, headings: Vec<crate::ui::Text>, right: Vec<bool>, rows: Vec<Vec<String>>) {
        let headings: Vec<String> = headings.iter().map(|h| h.show()).collect();
        print!("{}", table(&headings, &right, &rows));
    }

    /// `message`: one line on standard error.
    pub fn message(&mut self, text: crate::ui::Text) {
        eprintln!("{}", text.show());
    }

    /// Clarity's word for yes or no, as a table shows a bool.
    pub fn yes_no(value: bool) -> String {
        let (key, english) = if value { ("yes", "Yes") } else { ("no", "No") };
        translated(key).filter(|v| !v.is_empty()).unwrap_or_else(|| english.to_string())
    }

    fn answer(&mut self, question: crate::ui::Text, asked: Asked) -> String {
        match ask(&question.show(), &asked, self.terminal.as_mut(), &Self::words()) {
            Ok(answer) => answer,
            // Nobody to answer, or no answer: the program ends here.
            Err(code) => std::process::exit(code),
        }
    }

    /// `ask ... yes @a no @b`: whether the person said yes. Without the
    /// words the question takes Clarity's OK and Cancel.
    pub fn confirm(&mut self, question: crate::ui::Text, yes: Option<crate::ui::Text>, no: Option<crate::ui::Text>) -> bool {
        let word = |text: Option<crate::ui::Text>, key: &str, english: &str| {
            text.map(|t| t.show()).unwrap_or_else(|| translated(key).filter(|v| !v.is_empty()).unwrap_or_else(|| english.to_string()))
        };
        let asked = Asked::Confirm { yes: word(yes, "ok", "OK"), no: word(no, "cancel", "Cancel") };
        self.answer(question, asked) == "true"
    }

    /// `ask ... for type [default value]`: the value typed.
    pub fn ask<T: serde::de::DeserializeOwned + Default>(&mut self, question: crate::ui::Text, kind: Kind, values: &[&str], default: Option<String>) -> T {
        let param = Param { kind, values: values.iter().map(|v| v.to_string()).collect(), default, ..Param::default() };
        let answer = self.answer(question, Asked::Value { param });
        typed(kind, Some(&answer))
    }

    /// `ask ... for Entity from list`: the position, from 0, of the record chosen.
    pub fn choose(&mut self, question: crate::ui::Text, choices: Vec<String>) -> usize {
        let answer = self.answer(question, Asked::Choice { choices });
        answer.parse::<usize>().map_or(0, |n| n.saturating_sub(1))
    }
}

/// Runs a command-line program: reads the command line, writes help or
/// what is wrong with it, or runs the command it names with its values
/// and gives the exit code its run ends with (DESIGN 5.20).
pub fn run_program(program: &Program, run: impl FnOnce(&mut Session, usize, Vec<Option<String>>) -> crate::Result<i32>) -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match parse(program, &args, &Session::words()) {
        Parsed::Help(help) => {
            print!("{help}");
            0
        }
        Parsed::Wrong { said, .. } => {
            eprint!("{said}");
            2
        }
        Parsed::Run { command, values } => match run(&mut Session::default(), command, values) {
            Ok(code) => code,
            Err(crate::Error::Refused(text)) => {
                eprintln!("{}", text.show());
                1
            }
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
    };
    std::process::ExitCode::from(code.clamp(0, 255) as u8)
}
