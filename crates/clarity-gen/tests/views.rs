//! Views in slide-outs on every stack (DESIGN 5.33): the view's parts are
//! written in the slide-out of the window's markup, its handlers in the
//! window's code, so every stack shows them as the window's own.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const MODEL: &str = "module model

page Workspace: window
    data picked: text
    button \"Edit\" name editButton at 1,1 span 2 on click: editClicked
    slideout \"Class\" name classPanel view ClassEditor span 6 rows 6

page ClassEditor: view
    data className: text
    textbox name nameBox at 1,1 span 4 bind className
    button \"Save\" name saveButton at 5,1 span 2 on click: saveClicked

code Workspace
    handler editClicked
        className = picked
        classPanel.visible = true

code ClassEditor
    handler saveClicked
        className = \"\"
";

fn generated(backend: Backend) -> String {
    let project = clarity_spec::keyed::load_keyed([("model.clarity", MODEL)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = starter("Model", "model");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

#[test]
fn the_views_parts_stand_in_the_slideout_on_every_stack() {
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
        let code = generated(backend);
        let at = code.find("<Slideout name=\"classPanel\"").unwrap_or_else(|| panic!("{backend:?} has no slide-out"));
        let inside = &code[at..at + code[at..].find("</Slideout>").unwrap_or_else(|| panic!("{backend:?}: the slide-out holds nothing"))];
        assert!(inside.contains("name=\"nameBox\"") && inside.contains("name=\"saveButton\""), "{backend:?}: {inside}");
        // The view is no window of its own.
        assert!(!code.contains("// ui/windows/ClassEditor.cxaml") && !code.contains("ClassEditor.cxaml\n"), "{backend:?}");
    }
}

#[test]
fn the_designer_draws_a_view_on_its_own_and_the_preview_shows_it_in_the_window() {
    use clarity_gen::preview::{files_json, Shown};
    let project = clarity_spec::keyed::load_keyed([("model.clarity", MODEL)], &[], "en");
    let view: serde_json::Value = serde_json::from_str(&files_json(&project, "Model", &Shown::Window("ClassEditor".into()), true, &Default::default())).unwrap();
    let markup = view["windows"]["ClassEditor"].as_str().expect("the view's markup");
    assert!(markup.contains("name=\"nameBox\"") && markup.contains("designItem="), "{markup}");
    let shown: serde_json::Value = serde_json::from_str(&files_json(&project, "Model", &Shown::Window("Workspace".into()), false, &Default::default())).unwrap();
    assert!(shown["windows"]["Workspace"].as_str().unwrap().contains("name=\"saveButton\""));
}

/// A view shown in a tab keeps its own grid on every stack: the tab's markup
/// carries the view's columns, so its last columns stay inside the tabs.
#[test]
fn a_view_in_a_tab_carries_its_columns_on_every_stack() {
    let text = MODEL.replace("    slideout \"Class\" name classPanel view ClassEditor span 6 rows 6\n", "    slideout \"Class\" name classPanel view ClassEditor span 6 rows 6\n    tabs name sides at 1,2 span 9 rows 6\n        tab \"Class\" view ClassEditor\n");
    let project = clarity_spec::keyed::load_keyed([("model.clarity", text.as_str())], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    if !errors.is_empty() {
        // One view shows in one place: then the slide-out goes.
        let single = MODEL.replace("    slideout \"Class\" name classPanel view ClassEditor span 6 rows 6\n", "    tabs name sides at 1,2 span 9 rows 6\n        tab \"Class\" view ClassEditor\n").replace("        classPanel.visible = true\n", "");
        let project = clarity_spec::keyed::load_keyed([("model.clarity", single.as_str())], &[], "en");
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        check(&project);
    } else {
        check(&project);
    }
    fn check(project: &clarity_spec::Project) {
        for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
            let mut settings = starter("Model", "model");
            settings.stack.backend = backend;
            settings.stack.ui = Ui::Clarity;
            let code: String = generate(project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}")).iter().map(|f| f.text.clone()).collect();
            let tab = code.lines().find(|l| l.trim_start().starts_with("<Tab ")).unwrap_or_default().to_string();
            assert!(tab.contains("columns=\"12\""), "{backend:?}: {tab}");
        }
    }
}
