//! Views in slide-outs (DESIGN 5.33): `slideout ... view ClassEditor` shows
//! the view there; its parts and data join the window's, so the window's
//! code sets what it shows, and the generators get the view placed.

use clarity_spec::keyed::load_keyed;
use clarity_spec::{Project, Severity};

const MODEL: &str = "module model

page Workspace: window
    data picked: text
    button \"Edit\" name editButton at 1,1 span 2 on click: editClicked
    slideout \"Class\" name classPanel view ClassEditor pinnable span 6 rows 6

page ClassEditor: view
    data className: text
    label \"Name\" name nameLabel at 1,1 span 2 for nameBox
    textbox name nameBox at 3,1 span 4 bind className
    button \"Save\" name saveButton at 5,2 span 2 on click: saveClicked

code Workspace
    handler editClicked
        className = picked
        nameBox.enabled = true
        classPanel.visible = true

code ClassEditor
    handler saveClicked
        className = \"\"
";

fn project(text: &str) -> Project {
    load_keyed([("model.clarity", text)], &[], "en")
}

fn errors(text: &str) -> Vec<String> {
    project(text).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn has(text: &str, words: &str) {
    let found = errors(text);
    assert!(found.iter().any(|m| m.contains(words)), "{words}: {found:#?}");
}

#[test]
fn a_window_shows_a_view_in_a_slideout_and_its_code_reaches_the_views_parts_and_data() {
    assert!(errors(MODEL).is_empty(), "{:#?}", errors(MODEL));
    let file = &project(MODEL).files[0];
    let panel = &file.pages[0].widgets[1];
    assert_eq!(panel.props.view.as_deref(), Some("ClassEditor"));
    assert!(clarity_spec::widgets::widget_line(&panel.props).contains(" view ClassEditor"));
}

#[test]
fn the_rules_of_views() {
    has(&MODEL.replace("view ClassEditor", "view Nothing"), "there is no view 'Nothing'");
    has(&MODEL.replace("page ClassEditor: view", "page ClassEditor: window"), "ClassEditor is a window; a slide-out shows a view");
    has(&MODEL.replace("pinnable span 6 rows 6\n", "pinnable span 6 rows 6\n        label \"x\" name own at 1,1 span 2\n"), "holds no parts of its own");
    has(&MODEL.replace("data picked: text", "data className: text"), "ClassEditor has 'className', as Workspace has");
    has(&MODEL.replace("name editButton", "name nameBox"), "ClassEditor has 'nameBox', as Workspace has");
    has(
        &MODEL.replace("    slideout \"Class\" name classPanel view ClassEditor pinnable span 6 rows 6\n", "    slideout \"Class\" name classPanel view ClassEditor pinnable span 6 rows 6\n    slideout \"Again\" name againPanel view ClassEditor span 6 rows 6\n"),
        "ClassEditor shows once in a window",
    );
    // The view's own code reaches only its own parts.
    has(&MODEL.replace("        className = \"\"\n", "        classPanel.visible = false\n"), "classPanel");
}

#[test]
fn placed_the_views_parts_data_and_handlers_are_the_windows() {
    let placed = clarity_spec::views::placed(&project(MODEL));
    let file = &placed.files[0];
    let window = &file.pages[0];
    let panel = &window.widgets[1];
    let names: Vec<&str> = panel.children.iter().filter_map(|w| w.props.name.as_deref()).collect();
    assert_eq!(names, ["nameLabel", "nameBox", "saveButton"]);
    assert!(window.data.iter().any(|d| d.name.name == "className"));
    let code = file.code.iter().find(|c| c.name.name == "Workspace").unwrap();
    assert!(code.handlers.iter().any(|h| h.name.name == "saveClicked"));
    // The view stays as it is, so the designer still shows it on its own.
    assert_eq!(file.pages[1].widgets.len(), 3);
}
