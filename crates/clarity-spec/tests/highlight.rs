//! Clarity text in colors: each piece of a line with what it is, as the
//! lexer reads it, so the editor colors what the language sees.

use clarity_spec::highlight::{Piece, PieceKind, highlight};

/// The pieces of `source` as `(text, kind)`, for reading in a test.
fn pieces(source: &str) -> Vec<(String, PieceKind)> {
    let lines: Vec<Vec<u16>> = source.split('\n').map(|l| l.encode_utf16().collect()).collect();
    highlight(source)
        .into_iter()
        .map(|Piece { line, from, to, kind }| (String::from_utf16_lossy(&lines[line as usize - 1][from as usize..to as usize]), kind))
        .collect()
}

fn has(source: &str, text: &str, kind: PieceKind) {
    let found = pieces(source);
    assert!(found.iter().any(|(t, k)| t == text && *k == kind), "no {text:?} as {kind:?} in {found:#?}");
}

#[test]
fn declarations_and_their_names() {
    let source = "module game\n\nengine Board\n    keep squares: list<integer> = [0, 0]\n";
    has(source, "module", PieceKind::Keyword);
    has(source, "engine", PieceKind::Keyword);
    has(source, "Board", PieceKind::Type);
    has(source, "keep", PieceKind::Keyword);
    has(source, "squares", PieceKind::Name);
    has(source, "list", PieceKind::Type);
    has(source, "integer", PieceKind::Type);
    has(source, "0", PieceKind::Number);
}

#[test]
fn parts_their_properties_keys_and_texts() {
    let source = "    button @newGameCaption name newGame at 1,10 span 12 key \"Ctrl+N\" on click: restartClicked\n";
    has(source, "button", PieceKind::Keyword);
    has(source, "@newGameCaption", PieceKind::Key);
    has(source, "name", PieceKind::Keyword);
    has(source, "newGame", PieceKind::Name);
    has(source, "span", PieceKind::Keyword);
    has(source, "\"Ctrl+N\"", PieceKind::Text);
    has(source, "on", PieceKind::Keyword);
    has(source, "click", PieceKind::Keyword);
    has(source, "restartClicked", PieceKind::Name);
    // A part's data is a name, whatever word it is.
    has("    text name statusText at 1,9 bind status\n", "status", PieceKind::Name);
}

#[test]
fn a_word_set_or_called_is_a_name_whatever_it_is_elsewhere() {
    let source = "        title = Board.play(square)\n        size(3)\n";
    has(source, "title", PieceKind::Name);
    has(source, "size", PieceKind::Name);
    has(source, "play", PieceKind::Name);
    has(source, "Board", PieceKind::Type);
}

#[test]
fn code_statements_literals_and_comments() {
    let source = "        // The answer.\n        if moves == 9 and not done\n            return none\n        let ok = true\n";
    has(source, "// The answer.", PieceKind::Comment);
    has(source, "if", PieceKind::Keyword);
    has(source, "and", PieceKind::Keyword);
    has(source, "not", PieceKind::Keyword);
    has(source, "return", PieceKind::Keyword);
    has(source, "none", PieceKind::Literal);
    has(source, "true", PieceKind::Literal);
    has(source, "ok", PieceKind::Name);
}

#[test]
fn the_slots_of_a_text_stand_out() {
    let source = "        message \"Saved {customer.name}, {{not a slot}}\"\n";
    has(source, "\"Saved ", PieceKind::Text);
    has(source, "{customer.name}", PieceKind::Slot);
    has(source, ", {{not a slot}}\"", PieceKind::Text);
}

#[test]
fn a_language_file_is_keys_and_texts() {
    let source = "// Dutch.\ntranslation nl\n    saveCaption \"Opslaan\"\n";
    has(source, "translation", PieceKind::Keyword);
    has(source, "saveCaption", PieceKind::Key);
    has(source, "\"Opslaan\"", PieceKind::Text);
}

/// A key that points at a resource: `resource` is a word of the language.
#[test]
fn a_resource_value_in_a_language_file() {
    let source = "translation nl\n    gameRules resource \"rules-nl.md\"\n";
    has(source, "gameRules", PieceKind::Key);
    has(source, "resource", PieceKind::Keyword);
    has(source, "\"rules-nl.md\"", PieceKind::Text);
}

#[test]
fn columns_count_as_the_editor_counts_them() {
    // UTF-16, as JavaScript counts: the é before the key is one unit.
    let source = "    label \"é\" @k\n";
    has(source, "@k", PieceKind::Key);
    let key = highlight(source).into_iter().find(|p| p.kind == PieceKind::Key).unwrap();
    assert_eq!((key.from, key.to), (14, 16));
}

#[test]
fn every_line_of_every_example_starts_with_a_word_the_highlighter_knows() {
    // A line that starts with a word is a declaration, a setting, a part or a
    // statement, unless the word is set or called: the highlighter colors it
    // as a keyword, so a new word of the language cannot go uncolored.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut files = Vec::new();
    // The examples, and Clarity's own theme, which the designer shows too.
    let mut folders = vec![root, std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("themes")];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() && !path.ends_with("out") && !path.ends_with("data") {
                folders.push(path);
            } else if path.extension().is_some_and(|e| e == "clarity") {
                files.push(path);
            }
        }
    }
    assert!(files.len() > 5);
    let mut unknown = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file).unwrap();
        // Translation files are keys and texts.
        if source.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with("//")).is_some_and(|l| l.starts_with("translation ")) {
            continue;
        }
        for (i, line) in source.lines().enumerate() {
            let first = highlight(line).into_iter().next();
            let Some(first) = first else { continue };
            if matches!(first.kind, PieceKind::Name) {
                let word: String = line.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                let rest = line.trim_start()[word.len()..].trim_start();
                // Set, called, a field or a parameter: names by right.
                if rest.starts_with('=') || rest.starts_with('(') || rest.starts_with('.') || rest.starts_with('[') || rest.starts_with(':') {
                    continue;
                }
                unknown.push(format!("{}:{}: {line}", file.display(), i + 1));
            }
        }
    }
    assert!(unknown.is_empty(), "lines starting with a word the highlighter does not know:\n{}", unknown.join("\n"));
}

/// The grammar other editors color `.clarity` files with is written from
/// the highlighter's own words, so it cannot fall behind the language.
/// `CLARITY_WRITE_GRAMMAR=1 cargo test -p clarity-spec --test highlight` writes it again.
#[test]
fn the_editor_grammar_is_the_highlighters() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../editors/vscode/syntaxes/clarity.tmLanguage.json");
    let grammar = serde_json::to_string_pretty(&clarity_spec::highlight::textmate_grammar()).unwrap() + "\n";
    if std::env::var("CLARITY_WRITE_GRAMMAR").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &grammar).unwrap();
    }
    let written = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(written == grammar, "{} is not the highlighter's grammar; write it with CLARITY_WRITE_GRAMMAR=1", path.display());
    // Every keyword of the language is in it.
    for word in clarity_spec::highlight::keywords() {
        assert!(grammar.contains(&format!("|{word}|")) || grammar.contains(&format!("({word}|")) || grammar.contains(&format!("|{word})")), "{word}");
    }
}

/// Web APIs (DESIGN 5.23): the calls, every page, and the lines under a
/// web API data source.
#[test]
fn web_api_calls_and_their_paging() {
    let call = "    let reply = http patch \"/orders/1\" from shop with body\n    let all = http get \"/orders\" from shop every page\n";
    for word in ["http", "patch", "from", "with", "get", "every", "page"] {
        has(call, word, PieceKind::Keyword);
    }
    let lines = "    datasource forms rest \"https://forms.example.com/api\"\n        paging cursor \"after\" next \"meta.next\" items \"data\"\n        openapi \"forms.yaml\"\n        pick schema \"Entry\"\n";
    for word in ["paging", "cursor", "next", "items", "openapi", "pick", "schema"] {
        has(lines, word, PieceKind::Keyword);
    }
    for word in ["page", "offset", "link"] {
        has(&format!("        paging {word}\n"), word, PieceKind::Keyword);
    }
}
