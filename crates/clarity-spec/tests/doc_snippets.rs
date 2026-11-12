//! The code blocks of a Markdown document, and a block as module source:
//! shared by the tests of docs/LANGUAGE.md and of the manual's pages.

use clarity_spec::widgets;

/// The fenced code blocks of `doc`, in order.
pub fn blocks(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            match current.take() {
                Some(block) => out.push(block),
                None => current = Some(String::new()),
            }
        } else if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    out
}

/// A snippet as a file the parser reads: declarations get a module line,
/// parts (a first line with `at`, as every part line has) get a window
/// around them. Other blocks (the project file and its `signin` block, the
/// folder tree, the grammar) are not module source.
pub fn as_module(block: &str) -> Option<String> {
    let first = block.split_whitespace().next()?;
    match first {
        "module" => Some(block.to_string()),
        "enum" | "entity" | "page" | "app" | "theme" | "code" | "function" | "action" | "engine" | "settings" | "cli" | "command" | "mapping" => {
            Some(format!("module doc\n{block}"))
        }
        word if widgets::by_keyword(word).is_some() && block.lines().next().is_some_and(|l| l.split_whitespace().any(|w| w == "at")) => {
            let indented: String = block.lines().map(|l| format!("    {l}\n")).collect();
            Some(format!("module doc\npage Doc: window\n{indented}"))
        }
        _ => None,
    }
}
