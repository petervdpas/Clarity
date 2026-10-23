//! Trees of U1c: the web stacks carry `tree` and `treetable` in their model
//! for the renderer, and code reads their selected row as a table's;
//! Avalonia writes Clarity's table in tree mode.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, with a window of folders as a tree and a tree table.
const MUSIC: &str = include_str!("fixtures/asks.clarity");

fn generated(backend: Backend, ui: Ui) -> String {
    let project = load_keyed([("music.clarity", MUSIC)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Music", "music");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "missing {expected:?}");
}

#[test]
fn avalonia_writes_trees_as_clarity_s_table_in_tree_mode() {
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, "<c:ClarityTable x:Name=\"folderTree\"");
    has(&csharp, "TreeParent=\"Parent\" ExpandText=\"{x:Static local:Texts.ExpandRow}\" CollapseText=\"{x:Static local:Texts.CollapseRow}\" HeadersVisible=\"False\"");
    has(&csharp, "<c:ClarityTable x:Name=\"folderTable\"");
    has(&csharp, "folderTree.Selected += (_, _) =>");
    has(&csharp, "folderTable.Opened += (_, _) =>");
}

#[test]
fn the_web_stacks_carry_the_trees_in_their_model() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "\"kind\":\"tree\"");
    has(&rust, "\"children\":\"children\"");
    has(&rust, "\"kind\":\"treeTable\"");
}
