//! Every window and view of every example opens as the IDE's preview opens it
//! (docs/ledger/modularity.md, refactor phase 0): the preview's files and its
//! program (clarity_gen::preview), drawn by Clarity.UI headless, as the
//! browser page does but in this process, with no server and no browser.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use clarity_gen::preview::{self, PreviewProgram, Shown};
use clarity_runtime::program::Program;
use clarity_ui::headless::Driven;
use clarity_ui::program_wire::{Asks, WireProgram};
use serde_json::{Map, Value, json};

/// The preview's program, asked as the preview server answers `invoke`.
struct Preview(PreviewProgram);

impl Asks for Preview {
    fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        Some(self.0.answer(command, &Value::Object(args)))
    }
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What the preview shows of the project in `dir`: each design file's name and
/// root element (Window, View, Dialog, App), but toolboxes, which show only in
/// the window that holds them.
fn pages_of(dir: &Path) -> Vec<(String, String)> {
    let mut pages: Vec<(String, String)> = std::fs::read_dir(dir.join("design"))
        .map(|d| {
            d.flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().strip_suffix(".cxaml")?.to_string();
                    let text = std::fs::read_to_string(e.path()).ok()?;
                    let root = text.lines().map(str::trim).find(|l| l.starts_with('<') && !l.starts_with("<?") && !l.starts_with("<!--"))?;
                    let kind: String = root[1..].chars().take_while(|c| c.is_ascii_alphabetic()).collect();
                    (kind != "Toolbox").then_some((name, kind))
                })
                .collect()
        })
        .unwrap_or_default();
    pages.sort();
    pages
}

#[test]
fn every_window_of_every_example_opens_in_the_preview() {
    let _turn = crate::choices_turn();
    let examples = repo().join("examples");
    let mut names: Vec<String> = std::fs::read_dir(&examples).unwrap().flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".clarity").map(str::to_string)).collect();
    names.sort();
    assert!(names.len() > 10, "the examples: {names:?}");
    let mut failed = Vec::new();
    let mut opened = 0;
    for name in &names {
        let file = examples.join(format!("{name}.clarity"));
        let dir = examples.join(name);
        let loaded = preview::load(&file, &dir).unwrap_or_else(|e| panic!("{name}: {e}"));
        // An app is shown as an empty window of its name in its frame; the rest each as a window of its own.
        for (page, kind) in pages_of(&dir) {
            let app = kind == "App";
            let shown = if app { Shown::App(page.clone()) } else { Shown::Window(page.clone()) };
            let text = preview::with_desktop(&preview::files_json(&loaded.project, &loaded.title, &shown, false, &Default::default()), loaded.desktop);
            let files: Value = serde_json::from_str(&text).unwrap();
            let resources = loaded.dir.join("resources");
            let app_files = clarity_ui::browser::files_from_json(&files, &|n: &str| std::fs::read(resources.join(n)).ok());
            let program: Arc<dyn Program> = Arc::new(WireProgram::new(Preview(PreviewProgram::new(loaded.project.clone(), shown.main()))));
            let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut driven = Driven::start(program, app_files);
                driven.step(&json!({ "do": "settle", "ms": 50 }));
                driven.step(&json!({ "do": "read" }))
            }));
            let read = match run {
                Ok(read) => read,
                Err(_) => {
                    failed.push(format!("{name} {page}: Clarity.UI stopped"));
                    continue;
                }
            };
            let said: Vec<String> = read["said"].as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect()).unwrap_or_default();
            let shown_parts = read["shown"].as_object().map_or(0, |s| s.len());
            // An app's empty window shows no parts of its own; every other page shows its parts.
            if read["ok"] != json!(true) || !said.is_empty() || (shown_parts == 0 && !app) {
                failed.push(format!("{name} {page}: ok {}, {shown_parts} parts shown, said {said:?}", read["ok"]));
            } else {
                opened += 1;
            }
        }
    }
    assert!(failed.is_empty(), "windows that do not open in the preview ({opened} did): {failed:#?}");
}
