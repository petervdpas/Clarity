//! Command-line programs against Clarity's shared cases
//! (conformance/command-line.json): reading a command line, the help, a
//! printed table and asking on a terminal.

use clarity_runtime::cli::{self, Asked, Kind, Param, Parsed, Program, Terminal, Words};
use serde_json::Value;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../conformance/command-line.json")).unwrap()
}

fn english(_: &str) -> Option<String> {
    None
}

#[test]
fn the_words_are_the_shared_ones() {
    let cases = cases();
    let words = cases["words"].as_object().unwrap();
    assert_eq!(words.len(), cli::WORDS.len());
    for (key, value) in cli::WORDS {
        assert_eq!(words[key].as_str(), Some(value), "{key}");
    }
}

#[test]
fn reads_command_lines() {
    let cases = cases();
    let program: Program = serde_json::from_value(cases["program"].clone()).unwrap();
    let words = Words::new(&english);
    for case in cases["parse"].as_array().unwrap() {
        let what = case["what"].as_str().unwrap();
        let args: Vec<String> = case["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().to_string()).collect();
        let parsed = cli::parse(&program, &args, &words);
        if let Some(run) = case.get("run") {
            let Parsed::Run { command, values } = parsed else { panic!("{what}: {parsed:?}") };
            let c = &program.commands[command];
            assert_eq!(c.line.join(" "), run["command"].as_str().unwrap(), "{what}");
            let names = c.arguments.iter().chain(&c.options).map(|p| p.name.as_str());
            let got: serde_json::Map<String, Value> = names.zip(values).map(|(n, v)| (n.to_string(), v.map_or(Value::Null, Value::String))).collect();
            assert_eq!(Value::Object(got), run["values"], "{what}");
        } else if let Some(help) = case.get("help") {
            assert_eq!(parsed, Parsed::Help(help.as_str().unwrap().to_string()), "{what}");
        } else {
            let wrong = &case["wrong"];
            let Parsed::Wrong { key, slots, said } = parsed else { panic!("{what}: {parsed:?}") };
            assert_eq!(key, wrong["key"].as_str().unwrap(), "{what}");
            let slots: serde_json::Map<String, Value> = slots.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
            assert_eq!(Value::Object(slots), wrong["slots"], "{what}");
            assert_eq!(said, wrong["said"].as_str().unwrap(), "{what}");
        }
    }
}

#[test]
fn prints_tables() {
    for case in cases()["table"].as_array().unwrap() {
        let texts = |v: &Value| v.as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect::<Vec<_>>();
        let headings = texts(&case["headings"]);
        let right: Vec<bool> = case["right"].as_array().unwrap().iter().map(|b| b.as_bool().unwrap()).collect();
        let rows: Vec<Vec<String>> = case["rows"].as_array().unwrap().iter().map(texts).collect();
        assert_eq!(cli::table(&headings, &right, &rows), case["said"].as_str().unwrap(), "{}", case["what"]);
    }
}

/// A terminal that types the case's lines and keeps what is written.
struct Typed {
    terminal: bool,
    lines: Vec<Option<String>>,
    said: String,
}

impl Terminal for Typed {
    fn is_terminal(&self) -> bool {
        self.terminal
    }

    fn read_line(&mut self) -> Option<String> {
        if self.lines.is_empty() { None } else { self.lines.remove(0) }
    }

    fn write(&mut self, text: &str) {
        self.said.push_str(text);
    }
}

#[test]
fn asks_on_a_terminal() {
    let words = Words::new(&english);
    for case in cases()["ask"].as_array().unwrap() {
        let what = case["what"].as_str().unwrap();
        let form = &case["form"];
        let asked = match form["kind"].as_str().unwrap() {
            "confirm" => Asked::Confirm { yes: form["yes"].as_str().unwrap().into(), no: form["no"].as_str().unwrap().into() },
            "choice" => Asked::Choice { choices: form["choices"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().into()).collect() },
            _ => Asked::Value {
                param: Param {
                    name: String::new(),
                    kind: serde_json::from_value::<Kind>(form["type"].clone()).unwrap(),
                    values: form.get("values").map(|v| serde_json::from_value(v.clone()).unwrap()).unwrap_or_default(),
                    default: form.get("default").and_then(|d| d.as_str()).map(str::to_string),
                    ..Param::default()
                },
            },
        };
        let mut terminal = Typed {
            terminal: case["terminal"].as_bool().unwrap(),
            lines: case["typed"].as_array().unwrap().iter().map(|l| l.as_str().map(str::to_string)).collect(),
            said: String::new(),
        };
        let answer = cli::ask(case["question"].as_str().unwrap(), &asked, &mut terminal, &words);
        match case.get("exit") {
            Some(code) => assert_eq!(answer, Err(code.as_i64().unwrap() as i32), "{what}"),
            None => assert_eq!(answer, Ok(case["answer"].as_str().unwrap().to_string()), "{what}"),
        }
        assert_eq!(terminal.said, case["said"].as_str().unwrap(), "{what}");
    }
}

#[test]
fn values_read_from_the_command_line_become_typed() {
    assert_eq!(cli::typed::<i64>(Kind::Integer, Some("42")), 42);
    assert_eq!(cli::typed::<i64>(Kind::Integer, None), 0);
    assert_eq!(cli::typed::<String>(Kind::Text, Some("Ada")), "Ada");
    assert_eq!(cli::typed::<bool>(Kind::Bool, Some("true")), true);
    assert_eq!(cli::typed::<rust_decimal::Decimal>(Kind::Money, Some("12.5")).to_string(), "12.5");
    assert_eq!(cli::typed::<Option<chrono::NaiveDate>>(Kind::Date, Some("2026-10-23")).map(|d| d.to_string()), Some("2026-10-23".into()));
    assert_eq!(cli::typed::<Option<chrono::NaiveDate>>(Kind::Date, None), None);
    // A duration is whole seconds.
    assert_eq!(cli::typed::<i64>(Kind::Duration, Some("90")), 90);
    // And back: a value as the command line writes it.
    assert_eq!(cli::text_of(&42i64), "42");
    assert_eq!(cli::text_of(&"Ada".to_string()), "Ada");
    assert_eq!(cli::text_of(&None::<chrono::NaiveDate>), "");
}

#[test]
fn the_language_is_the_systems_when_the_project_has_it() {
    let available = ["en".to_string(), "nl".to_string()];
    assert_eq!(cli::pick_language("nl_NL.UTF-8", &available, "en"), Some("nl".to_string()));
    assert_eq!(cli::pick_language("en_US.UTF-8", &available, "en"), None);
    assert_eq!(cli::pick_language("de_DE.UTF-8", &available, "en"), None);
    assert_eq!(cli::pick_language("C", &available, "en"), None);
}
