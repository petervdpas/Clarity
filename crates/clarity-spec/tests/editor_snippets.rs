//! The snippets of the editor package (editors/vscode/snippets/clarity.json)
//! are Clarity: each one, with its placeholders filled as offered, reads
//! without an error of the language.

use clarity_spec::{Severity, load_sources};

/// A snippet's body with every placeholder as it is offered: `${1:name}`
/// is `name`, `${2|app,window|}` is `app`, `$1` repeats what 1 is, and a
/// tab stop with nothing offered (`$0`) is `typed`.
fn filled(body: &str, typed: &str) -> String {
    let mut values = std::collections::HashMap::new();
    let mut out = String::new();
    let mut rest = body;
    while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        rest = &rest[at + 1..];
        if let Some(inner) = rest.strip_prefix('{') {
            let end = inner.find('}').expect("a closed placeholder");
            let (number, value) = match inner[..end].split_once(':') {
                Some((n, v)) => (n.to_string(), v.to_string()),
                None => {
                    let (n, choices) = inner[..end].split_once('|').expect("a choice");
                    (n.to_string(), choices.trim_end_matches('|').split(',').next().unwrap_or_default().to_string())
                }
            };
            values.insert(number, value.clone());
            out.push_str(&value);
            rest = &inner[end + 1..];
        } else {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            // A tab stop with nothing offered is what is typed there.
            out.push_str(values.get(&digits).map_or(typed, String::as_str));
            rest = &rest[digits.len()..];
        }
    }
    out.push_str(rest);
    out
}

#[test]
fn every_snippet_is_clarity() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../editors/vscode/snippets/clarity.json");
    let snippets: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let snippets = snippets.as_object().unwrap();
    assert!(snippets.len() >= 8);
    for (name, snippet) in snippets {
        let lines: Vec<String> = snippet["body"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect();
        let prefix = snippet["prefix"].as_str().unwrap();
        // Where the cursor ends (`$0`) is what is typed next: a statement in
        // a block, nothing elsewhere.
        let stands = if matches!(prefix, "code" | "handler" | "if" | "for") { "return" } else { "" };
        let text: String = filled(&lines.join("\n"), stands).lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join("\n");
        // A statement or a handler goes where it is written: in code behind.
        let (path, source) = match prefix {
            "translation" => continue,
            "handler" => ("m.clarity", format!("module m\n\npage CustomerCard: window\n\ncode CustomerCard\n{}\n", indent(&text, 1))),
            "if" | "for" => ("m.clarity", format!("module m\n\npage CustomerCard: window\n    data items: list<Customer>\n\nentity Customer\n    name: text\n\ncode CustomerCard\n    handler saveClicked\n        let condition = true\n{}\n", indent(&text, 2))),
            "code" => ("m.clarity", format!("module m\n\npage CustomerCard: window\n\n{text}\n")),
            _ => ("m.clarity", format!("module m\n\n{text}\n")),
        };
        let errors: Vec<String> = load_sources([(path, source.as_str())])
            .diagnostics
            .into_iter()
            .filter(|d| d.severity == Severity::Error && !d.message.contains("translation") && !d.message.contains("key"))
            .map(|d| d.message)
            .collect();
        assert!(errors.is_empty(), "snippet {name}:\n{source}\n{errors:#?}");
    }
}

fn indent(text: &str, depth: usize) -> String {
    text.lines().map(|l| if l.is_empty() { String::new() } else { format!("{}{l}", "    ".repeat(depth)) }).collect::<Vec<_>>().join("\n")
}
