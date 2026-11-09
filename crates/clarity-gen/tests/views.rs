//! Views in a window's content on every stack (DESIGN 5.33): the view's
//! parts are written in the content of the window's markup, its handlers in
//! the window's code, so every stack shows them as the window's own;
//! `main.swap(X)` opens the page of X.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const MODEL: &str = "module model

page Workspace: window
    data picked: text
    button \"Edit\" name editButton at 1,1 span 2 on click: editClicked
    slideout \"Class\" name classPanel span 6 rows 6
        content name classContent at 1,1 span 6 rows 6
            view ClassEditor

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

const SWAPPED: &str = "module model

page Workspace: window
    button \"Other\" name otherButton at 1,1 span 2 on click: otherClicked
    content name main at 1,2 span 9 rows 6
        view ClassEditor
        view OtherEditor

page ClassEditor: view
    button \"Save\" name saveButton at 11,1 span 2

page OtherEditor: view
    label \"Other\" name otherLabel at 1,1 span 2

code Workspace
    handler otherClicked
        main.swap(OtherEditor)
";

/// Content on every stack: its views as pages of `<Content>`, each keeping
/// the view's columns, and `main.swap(OtherEditor)` opening the second.
#[test]
fn content_shows_its_views_and_swaps_them_on_every_stack() {
    let project = clarity_spec::keyed::load_keyed([("model.clarity", SWAPPED)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let opened = [(Backend::Rust, "ui.set_tab("), (Backend::Go, "SetTab("), (Backend::C, "clarity_ui_set_tab("), (Backend::Cpp, "set_tab("), (Backend::Csharp, "Main.Tab = ")];
    for (backend, swap) in opened {
        let mut settings = starter("Model", "model");
        settings.stack.backend = backend;
        settings.stack.ui = Ui::Clarity;
        let code: String = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}")).iter().map(|f| format!("{}\n", f.text)).collect();
        let at = code.find("<Content name=\"main\"").unwrap_or_else(|| panic!("{backend:?} has no content"));
        let inside = &code[at..at + code[at..].find("</Content>").unwrap_or_else(|| panic!("{backend:?}: the content holds nothing"))];
        assert!(inside.contains("name=\"saveButton\"") && inside.contains("name=\"otherLabel\""), "{backend:?}: {inside}");
        assert!(inside.contains("<View name=\"ClassEditor\" columns=\"12\""), "{backend:?}: {inside}");
        assert!(code.lines().any(|l| l.contains(swap) && l.to_lowercase().contains("main") && l.contains('2')), "{backend:?} opens no second page with {swap}");
    }
}


/// The designer draws a window as it looks: each view in its content, its
/// parts designed in place (the view stands in the window's file, so an
/// edit of a part changes the view), and the content can be picked too.
#[test]
fn the_designer_shows_the_views_in_a_windows_content() {
    use clarity_gen::preview::{files_json, Shown};
    let project = clarity_spec::keyed::load_keyed([("model.clarity", SWAPPED)], &[], "en");
    let designed: serde_json::Value = serde_json::from_str(&files_json(&project, "Model", &Shown::Window("Workspace".into()), true, &Default::default())).unwrap();
    let markup = designed["windows"]["Workspace"].as_str().expect("the window's markup");
    let content = markup.lines().find(|l| l.contains("<Content name=\"main\"")).expect("the content");
    assert!(content.contains("designItem="), "{content}");
    let save = markup.lines().find(|l| l.contains("name=\"saveButton\"")).expect("the view's part shows in the content");
    assert!(save.contains("designItem="), "{save}");
}

/// A tree of levels has no list of its own until the program loads it, so
/// the designer and the preview show it with sample rows of its levels.
#[test]
fn the_designer_and_preview_show_a_tree_of_levels_with_sample_rows() {
    use clarity_gen::preview::{files_json, PreviewProgram, Shown};
    let source = "module m\n\nentity Project\n    name: text\n\nentity Domain\n    project: ref Project\n    name: text\n\npage Main: window\n    tree name modelTree at 1,1 span 4 rows 8\n        level Project show name\n            level Domain under project show name\n";
    let project = clarity_spec::keyed::load_keyed([("m.clarity", source)], &[], "en");
    let designed: serde_json::Value = serde_json::from_str(&files_json(&project, "M", &Shown::Window("Main".into()), true, &Default::default())).unwrap();
    let tree = designed["windows"]["Main"].as_str().unwrap().lines().find(|l| l.contains("<Tree name=\"modelTree\"")).unwrap().to_string();
    assert!(tree.contains("bind=\"modelTreeRows\"") && tree.contains("designItem="), "{tree}");
    let data = PreviewProgram::new(project, "Main").window_data("Main");
    let labels: Vec<&str> = data["modelTreeRows"].as_array().unwrap().iter().map(|r| r["label"].as_str().unwrap()).collect();
    assert_eq!(labels, ["Project 1", "Domain 1", "Domain 2", "Project 2"]);
}
