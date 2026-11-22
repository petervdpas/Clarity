//! Every tool Clarity offers agents is called by a test (docs/ledger/modularity.md,
//! refactor phase 0): a tool added without one fails here.

use std::path::{Path, PathBuf};

use clarity_agent::Tools;
use clarity_tools::ProjectTools;

/// The test code of the repository's crates: `tests/` files, and what follows
/// `#[cfg(test)]` in a source file.
fn test_code() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut code = String::new();
    let mut folders: Vec<PathBuf> = vec![root.join("crates"), root.join("app/src-tauri/src")];
    while let Some(dir) = folders.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "target") {
                    folders.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != "every_tool.rs") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                if path.components().any(|c| c.as_os_str() == "tests") {
                    code.push_str(&text);
                } else if let Some(at) = text.find("#[cfg(test)]") {
                    code.push_str(&text[at..]);
                }
            }
        }
    }
    code
}

#[test]
fn every_tool_is_called_by_a_test() {
    let tools = ProjectTools::new(None, std::env::temp_dir());
    let code = test_code();
    let untested: Vec<String> = tools
        .definitions()
        .into_iter()
        .map(|t| t.name)
        .filter(|name| {
            // Called: the name as an argument (`call("name"`, `ok(&mut tools, "name"`), not its definition (`tool("name"`) or dispatch (`"name" =>`).
            let quoted = format!("\"{name}\"");
            !code.match_indices(&quoted).any(|(at, _)| {
                let before = code[..at].trim_end();
                let after = code[at + quoted.len()..].trim_start();
                !before.ends_with("tool(") && !after.starts_with("=>") && (before.ends_with('(') || before.ends_with(','))
            })
        })
        .collect();
    assert!(untested.is_empty(), "tools no test calls: {untested:?}");
}
