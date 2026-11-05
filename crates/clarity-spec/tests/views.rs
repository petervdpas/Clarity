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
    // The view is the window's now, its page and its code: no program has it on its own.
    assert_eq!(file.pages.len(), 1);
    assert!(!file.code.iter().any(|c| c.name.name == "ClassEditor"));
}

const SECTIONS: &str = "module model

page Workspace: window
    data picked: text
    button \"Edit\" name editButton at 1,1 span 2 on click: editClicked
    accordion name parts at 1,2 span 12 rows 8 horizontal
        section \"Class\" view ClassEditor
        section \"Notes\"
            label \"Notes\" name notesLabel at 1,1 span 4
    tabs name pages at 1,10 span 12 rows 6
        tab \"Diagram\" view Diagram

page ClassEditor: view
    data className: text
    textbox name nameBox at 1,1 span 4 bind className
    button \"Save\" name saveButton at 5,1 span 2 on click: saveClicked

page Diagram: view
    data zoom: integer
    label \"Zoom\" name zoomLabel at 1,1 span 2

code Workspace
    handler editClicked
        className = picked
        zoom = 2
        nameBox.enabled = true

code ClassEditor
    handler saveClicked
        className = \"\"
";

#[test]
fn a_section_tab_or_step_shows_a_view_as_a_slideout_does() {
    assert!(errors(SECTIONS).is_empty(), "{:#?}", errors(SECTIONS));
    let file = &project(SECTIONS).files[0];
    let accordion = &file.pages[0].widgets[1];
    assert_eq!(accordion.props.orientation, Some(clarity_spec::model::Orientation::Horizontal));
    assert_eq!(accordion.pages[0].view.as_deref(), Some("ClassEditor"));
    assert_eq!(accordion.pages[1].view, None);
    let steps = SECTIONS.replace("tabs name pages", "steps name pages").replace("        tab \"Diagram\" view Diagram\n", "        step \"Diagram\" view Diagram\n        step \"Done\"\n");
    assert!(errors(&steps).is_empty(), "{:#?}", errors(&steps));
}

#[test]
fn the_rules_of_views_hold_in_sections_tabs_and_steps() {
    has(&SECTIONS.replace("view ClassEditor", "view Nothing"), "there is no view 'Nothing'");
    has(&SECTIONS.replace("page Diagram: view", "page Diagram: window"), "Diagram is a window; a tab shows a view");
    has(&SECTIONS.replace("section \"Class\" view ClassEditor\n", "section \"Class\" view ClassEditor\n            label \"x\" name own at 1,1 span 2\n"), "a section that shows a view holds no parts of its own");
    has(&SECTIONS.replace("tab \"Diagram\" view Diagram", "tab \"Diagram\" view ClassEditor"), "ClassEditor shows once in a window");
    has(&SECTIONS.replace("name notesLabel", "name nameBox"), "ClassEditor has 'nameBox', as Workspace has");
    has(&SECTIONS.replace("    label \"Zoom\" name zoomLabel at 1,1 span 2\n", "    label \"Zoom\" name zoomLabel at 1,1 span 2\n    slideout \"More\" name morePanel span 4 rows 4\n"), "Diagram has a slide-out of its own");
    // A view's sections show no views of their own.
    has(&SECTIONS.replace("    label \"Zoom\" name zoomLabel at 1,1 span 2\n", "    label \"Zoom\" name zoomLabel at 1,1 span 2\n    tabs name inner at 1,2 span 4 rows 4\n        tab \"In\" view ClassEditor\n"), "a view shows in a window, not in another view");
}

#[test]
fn placed_a_sections_view_is_its_parts_and_the_windows_data_and_code() {
    let placed = clarity_spec::views::placed(&project(SECTIONS));
    let file = &placed.files[0];
    let window = &file.pages[0];
    let names = |w: &clarity_spec::model::Widget, i: usize| w.pages[i].children.iter().filter_map(|c| c.props.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&window.widgets[1], 0), ["nameBox", "saveButton"]);
    assert_eq!(names(&window.widgets[1], 1), ["notesLabel"]);
    assert_eq!(names(&window.widgets[2], 0), ["zoomLabel"]);
    assert!(window.data.iter().any(|d| d.name.name == "className") && window.data.iter().any(|d| d.name.name == "zoom"));
    let code = file.code.iter().find(|c| c.name.name == "Workspace").unwrap();
    assert!(code.handlers.iter().any(|h| h.name.name == "saveClicked"));
}

#[test]
fn a_section_line_writes_its_view_back() {
    use clarity_spec::edit::{apply, Edit};
    let text = SECTIONS;
    let file = &clarity_spec::parser::parse(text, "model.clarity").0;
    let notes = file.pages[0].widgets[1].pages[1].span.start;
    let class = file.pages[0].widgets[1].pages[0].span.start;
    let edited = apply(text, &Edit::SetPageView { target: notes, view: Some("Diagram".into()) }).unwrap().text;
    assert!(edited.contains("        section \"Notes\" view Diagram\n"), "{edited}");
    let cleared = apply(text, &Edit::SetPageView { target: class, view: None }).unwrap().text;
    assert!(cleared.contains("        section \"Class\"\n"), "{cleared}");
}

const TELLING: &str = "module model

page Workspace: window
    data count: integer
    slideout \"Class\" name classPanel view ClassEditor span 6 rows 6 on saved: classSaved
    accordion name parts at 1,1 span 12 rows 8
        section \"Notes\" view Notes on noted: notesNoted
        section \"Other\"

page ClassEditor: view
    tells saved(name: text)
    data className: text
    button \"Save\" name saveButton at 1,1 span 2 on click: saveClicked

page Notes: view
    tells noted
    button \"Note\" name noteButton at 1,1 span 2 on click: noteClicked

code Workspace
    handler classSaved(name: text)
        count = count + 1

    handler notesNoted
        count = 0

code ClassEditor
    handler saveClicked
        tell saved(className)

code Notes
    handler noteClicked
        tell noted()
";

#[test]
fn a_view_tells_its_window_which_hears_it_on_the_place_that_shows_it() {
    assert!(errors(TELLING).is_empty(), "{:#?}", errors(TELLING));
    let file = &project(TELLING).files[0];
    assert_eq!(file.pages[1].tells[0].name.name, "saved");
    let panel = &file.pages[0].widgets[0];
    assert!(clarity_spec::widgets::widget_line(&panel.props).contains(" on saved: classSaved"), "{}", clarity_spec::widgets::widget_line(&panel.props));
    assert_eq!(file.pages[0].widgets[1].pages[0].tells[0].handler, "notesNoted");
}

#[test]
fn the_rules_of_telling() {
    has(&TELLING.replace("tell saved(className)", "tell gone(className)"), "view ClassEditor tells no 'gone'");
    has(&TELLING.replace("on saved: classSaved", "on lost: classSaved"), "ClassEditor tells no 'lost'");
    has(&TELLING.replace("on noted: notesNoted", "on lost: notesNoted"), "Notes tells no 'lost'");
    has(&TELLING.replace("handler classSaved(name: text)", "handler classSaved"), "tell saved of view ClassEditor gives (name: text); handler classSaved takes ()");
    has(&TELLING.replace("    data count: integer\n", "    data count: integer\n    tells changed\n"), "a window tells nothing; a view tells the window it shows in");
}

#[test]
fn placed_a_tell_runs_the_windows_handler_and_an_unheard_one_does_nothing() {
    use clarity_spec::code::{ExprKind, StmtKind};
    let calls = |text: &str, handler: &str| -> Vec<String> {
        let placed = clarity_spec::views::placed(&project(text));
        let code = placed.files[0].code.iter().find(|c| c.name.name == "Workspace").unwrap().clone();
        let h = code.handlers.iter().find(|h| h.name.name == handler).unwrap();
        h.body
            .iter()
            .map(|s| match &s.kind {
                StmtKind::Call { call } => match &call.kind {
                    ExprKind::Call { function, .. } => format!("call {}", function.name),
                    _ => "call".into(),
                },
                StmtKind::Tell { tell, .. } => format!("tell {}", tell.name),
                _ => "other".into(),
            })
            .collect()
    };
    assert_eq!(calls(TELLING, "saveClicked"), ["call classSaved"]);
    assert_eq!(calls(TELLING, "noteClicked"), ["call notesNoted"]);
    let unheard = TELLING.replace(" on saved: classSaved", "");
    assert!(errors(&unheard).is_empty(), "{:#?}", errors(&unheard));
    assert!(calls(&unheard, "saveClicked").is_empty());
}
