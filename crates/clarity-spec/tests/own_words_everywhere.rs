//! Clarity's own words have one English, in translations.rs. Every runtime
//! carries that English beside the key as its fallback (a program without
//! translation files still says something), so the copies must say exactly
//! what translations.rs says: this sweeps the sources of every runtime and
//! of Clarity.UI for a key of Clarity's own beside a text, in each stack's
//! idiom (`"key", "English"`, and C#'s `new("English") { Key = "key" }`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The folders of runtime source, and the endings of their source files.
const SOURCES: &[(&str, &[&str])] = &[
    ("runtimes/rust/src", &["rs"]),
    ("runtimes/rust-ui/src", &["rs"]),
    ("runtimes/go/clarity", &["go"]),
    ("runtimes/c/clarity", &["c", "h"]),
    ("runtimes/cpp/clarity", &["cpp", "hpp", "h"]),
    ("runtimes/csharp", &["cs"]),
];

fn files(dir: &Path, endings: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            // Build output and tests are no runtime source.
            if !matches!(name.as_str(), "bin" | "obj" | "tests" | "Tests") && !name.ends_with(".Tests") {
                files(&path, endings, out);
            }
        } else if endings.iter().any(|e| name.ends_with(&format!(".{e}"))) && !name.ends_with("_test.go") {
            out.push(path);
        }
    }
}

/// The text of a string literal starting at `from` (after its opening quote), until the closing quote.
fn literal(text: &str, from: usize) -> Option<String> {
    let mut out = String::new();
    let mut chars = text[from..].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            '"' => return Some(out),
            c => out.push(c),
        }
    }
    None
}

/// The calls that give a key with its English: Rust's and C++'s
/// `Text::keyed`, C's `keyed`, Go's `KeyedText`, C#'s `Own`.
const CALLS: &[&str] = &["keyed(\"", "KeyedText(\"", "Own(\""];

/// Every key with its English in `text`: in a call of `CALLS`, in C#'s
/// `new("English") { Key = "key" }`, and in a row (`{"key", "English"},`
/// or `("key", "English"),`) of a table whose declaration says it holds
/// words or English (`WORDS`, `CliWords`, `words_in_english`, `English`).
fn fallbacks(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for call in CALLS {
        for (at, _) in text.match_indices(call) {
            let key_at = at + call.len();
            let Some(key) = literal(text, key_at) else { continue };
            let rest = &text[key_at + key.len() + 1..];
            if let Some(after) = rest.strip_prefix(", \"") {
                if let Some(said) = literal(after, 0) {
                    out.push((key, said));
                }
            }
        }
    }
    for (at, _) in text.match_indices("{ Key = \"") {
        let Some(key) = literal(text, at + "{ Key = \"".len()) else { continue };
        let Some(open) = text[..at].rfind("new(\"") else { continue };
        if let Some(said) = literal(text, open + "new(\"".len()) {
            out.push((key, said));
        }
    }
    let mut table = false;
    let mut before = "";
    for line in text.lines() {
        let row = line.trim();
        if row.ends_with('[') || row.ends_with('{') {
            let declaration = if row == "[" || row == "{" { before } else { row }.to_lowercase();
            table = declaration.contains("words") || declaration.contains("english");
        } else if row.starts_with(']') || row.starts_with('}') {
            table = false;
        } else if table && (row.starts_with("{\"") || row.starts_with("(\"")) {
            if let Some(key) = literal(row, 2) {
                if let Some(said) = row[2 + key.len() + 1..].strip_prefix(", \"").and_then(|r| literal(r, 0)) {
                    out.push((key, said));
                }
            }
        }
        before = row;
    }
    out
}

#[test]
fn every_runtime_falls_back_to_the_english_of_translations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let english: BTreeMap<&str, &str> = clarity_spec::translations::own_words().map(|(k, v)| (*k, *v)).collect();
    let mut wrong = Vec::new();
    let mut found = 0;
    for (dir, endings) in SOURCES {
        let mut paths = Vec::new();
        files(&root.join(dir), endings, &mut paths);
        assert!(!paths.is_empty(), "no source in {dir}");
        for path in paths {
            // Comments may show a call shortened; only code is a fallback.
            let text: String = std::fs::read_to_string(&path)
                .unwrap()
                .lines()
                .filter(|l| !matches!(l.trim_start().get(..2), Some("//" | "/*")) && !l.trim_start().starts_with('*'))
                .map(|l| format!("{l}\n"))
                .collect();
            let shown = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
            for (key, said) in fallbacks(&text) {
                let Some(words) = english.get(key.as_str()) else { continue };
                found += 1;
                if said != *words {
                    wrong.push(format!("{shown}: {key} says {said:?}, translations.rs {words:?}"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(found >= 40, "only {found} fallbacks were found");
}
