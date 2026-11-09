//! Views (DESIGN 5.33): a view shows only in a window's content (`content`
//! with a `view X` line); a slide-out, tab, step or section that shows one
//! holds a content. The view's parts and data join the window's, so the
//! window's code sets what it shows, and the generators get the view placed.

use clarity_spec::keyed::load_keyed;
use clarity_spec::{Project, Severity};

const MODEL: &str = "module model

page Workspace: window
    data picked: text
    button \"Edit\" name editButton at 1,1 span 2 on click: editClicked
    slideout \"Class\" name classPanel pinnable span 6 rows 6
        content name classContent at 1,1 span 6 rows 6
            view ClassEditor

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
fn a_window_shows_a_view_in_content_and_its_code_reaches_the_views_parts_and_data() {
    assert!(errors(MODEL).is_empty(), "{:#?}", errors(MODEL));
    let file = &project(MODEL).files[0];
    let content = &file.pages[0].widgets[1].children[0];
    assert_eq!(content.pages[0].view.as_deref(), Some("ClassEditor"));
}

/// Without content nothing shows a view: a slide-out, tab, step or section that names one is refused.
#[test]
fn a_view_shows_only_in_content() {
    let slideout = MODEL.replace("    slideout \"Class\" name classPanel pinnable span 6 rows 6\n        content name classContent at 1,1 span 6 rows 6\n            view ClassEditor\n", "    slideout \"Class\" name classPanel view ClassEditor pinnable span 6 rows 6\n");
    has(&slideout, "a slide-out does not show a view: a view shows in content; put a content in the slide-out with view ClassEditor");
    let tab = MODEL.replace("    slideout \"Class\" name classPanel pinnable span 6 rows 6\n        content name classContent at 1,1 span 6 rows 6\n            view ClassEditor\n", "    tabs name pages at 1,2 span 12 rows 6\n        tab \"Class\" view ClassEditor\n").replace("        classPanel.visible = true\n", "");
    has(&tab, "a tab does not show a view: a view shows in content; put a content in the tab with view ClassEditor");
    let section = tab.replace("tabs name pages", "accordion name pages").replace("tab \"Class\" view ClassEditor", "section \"Class\" view ClassEditor");
    has(&section, "a section does not show a view");
}

#[test]
fn the_rules_of_views() {
    has(&MODEL.replace("view ClassEditor", "view Nothing"), "there is no view 'Nothing'");
    has(&MODEL.replace("page ClassEditor: view", "page ClassEditor: window"), "ClassEditor is a window; content shows a view");
    has(&MODEL.replace("            view ClassEditor\n", "            view ClassEditor\n                label \"x\" name own at 1,1 span 2\n"), "holds no parts of its own");
    has(&MODEL.replace("data picked: text", "data className: text"), "ClassEditor has 'className', as Workspace has");
    has(&MODEL.replace("name editButton", "name nameBox"), "ClassEditor has 'nameBox', as Workspace has");
    has(
        &MODEL.replace("            view ClassEditor\n", "            view ClassEditor\n    slideout \"Again\" name againPanel span 6 rows 6\n        content name againContent at 1,1 span 6 rows 6\n            view ClassEditor\n"),
        "ClassEditor shows once in a window",
    );
    // Its handlers join the window's code: a name the window's code or another view placed there has is a clash.
    has(&MODEL.replace("    handler saveClicked\n        className = \"\"\n", "    handler editClicked\n        className = \"\"\n").replace("on click: saveClicked", "on click: editClicked"), "ClassEditor has handler 'editClicked', as Workspace has");
    let two = MODEL.replace("            view ClassEditor\n", "            view ClassEditor\n            view OtherEditor\n")
        + "\npage OtherEditor: view\n    button \"Go\" name goButton at 1,1 span 2 on click: saveClicked\n\ncode OtherEditor\n    handler saveClicked\n        goButton.enabled = false\n";
    has(&two, "OtherEditor has handler 'saveClicked', as ClassEditor has");
    // The view's code reaches what shares its window: the window's slide-out is fine.
    assert!(errors(&MODEL.replace("        className = \"\"\n", "        classPanel.visible = false\n")).is_empty());
    // A view may hold content of its own, but never shows itself.
    has(&MODEL.replace("    button \"Save\" name saveButton at 5,2 span 2 on click: saveClicked\n", "    button \"Save\" name saveButton at 5,2 span 2 on click: saveClicked\n    content name inner at 1,3 span 4 rows 2\n        view ClassEditor\n"), "ClassEditor shows itself");
    has(&MODEL.replace("    label \"Name\" name nameLabel at 1,1 span 2 for nameBox\n", "    label \"Name\" name nameLabel at 1,1 span 2 for nameBox\n    slideout \"More\" name morePanel span 4 rows 4\n"), "ClassEditor has a slide-out of its own");
}

#[test]
fn placed_the_views_parts_data_and_handlers_are_the_windows() {
    let placed = clarity_spec::views::placed(&project(MODEL));
    let file = &placed.files[0];
    let window = &file.pages[0];
    let content = &window.widgets[1].children[0];
    let names: Vec<&str> = content.pages[0].children.iter().filter_map(|w| w.props.name.as_deref()).collect();
    assert_eq!(names, ["nameLabel", "nameBox", "saveButton"]);
    assert!(window.data.iter().any(|d| d.name.name == "className"));
    let code = file.code.iter().find(|c| c.name.name == "Workspace").unwrap();
    assert!(code.handlers.iter().any(|h| h.name.name == "saveClicked"));
    // The view is the window's now, its page and its code: no program has it on its own.
    assert_eq!(file.pages.len(), 1);
    assert!(!file.code.iter().any(|c| c.name.name == "ClassEditor"));
}

const TELLING: &str = "module model

page Workspace: window
    data count: integer
    slideout \"Class\" name classPanel span 6 rows 6
        content name classContent at 1,1 span 6 rows 6
            view ClassEditor on saved: classSaved
    accordion name parts at 1,1 span 12 rows 8
        section \"Notes\"
            content name notesContent at 1,1 span 12 rows 6
                view Notes on noted: notesNoted
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
fn a_view_tells_its_window_which_hears_it_on_its_line_of_content() {
    assert!(errors(TELLING).is_empty(), "{:#?}", errors(TELLING));
    let file = &project(TELLING).files[0];
    assert_eq!(file.pages[1].tells[0].name.name, "saved");
    assert_eq!(file.pages[0].widgets[0].children[0].pages[0].tells[0].handler, "classSaved");
    assert_eq!(file.pages[0].widgets[1].pages[0].children[0].pages[0].tells[0].handler, "notesNoted");
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

/// A view keeps its own grid where it shows: content lays its parts out on
/// the view's columns, not its own span, so a part in the view's last
/// columns stays inside (a gap Writer met: a view on 12 columns in tabs of
/// span 9 lost its columns 10 to 12).
#[test]
fn a_view_shown_in_content_keeps_its_own_columns() {
    let text = "module model

page Desk: window
    content name main at 1,1 span 9 rows 8
        view Editor

page Editor: view
    button \"Save\" name saveButton at 11,1 span 2
";
    assert!(errors(text).is_empty(), "{:#?}", errors(text));
    let placed = clarity_spec::views::placed(&project(text));
    let content = &placed.files[0].pages[0].widgets[0];
    assert_eq!(Some(12), content.pages[0].columns, "the view's line");
}
