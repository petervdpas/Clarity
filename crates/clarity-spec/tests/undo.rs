//! Undo and redo (DESIGN 5.29): built-in targets, as `quit` is, for a menu
//! item, a toolbar button or a button in a window.

use clarity_spec::{Severity, load_sources};

const APP: &str = "module m

page Main: window
    menu name mainMenu
        item \"Edit\" name editItem
            item \"Undo\" name undoItem key \"Ctrl+Z\" on click -> undo
            item \"Redo\" name redoItem key \"Ctrl+Y\" on click -> redo
    toolbar name mainToolbar
        button \"Undo\" name undoButton on click -> undo
    button \"Redo\" name redoButton at 1,1 span 2 on click -> redo
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn undo_and_redo_are_built_in_targets() {
    assert!(errors(APP).is_empty(), "{:#?}", errors(APP));
    assert!(clarity_spec::frame::BUILT_IN_COMMANDS.contains(&"undo"));
    assert!(clarity_spec::frame::BUTTON_COMMANDS.contains(&"redo"));
}

/// Why an undo was refused is a Clarity word, which every project with windows translates.
#[test]
fn why_an_undo_was_refused_is_a_clarity_word() {
    assert_eq!(clarity_spec::translations::UNDO_KEYS, [("undoChangedSince", "This was changed since, so it cannot be taken back")]);
    let project = load_sources([("m.clarity", APP)]);
    let keys: Vec<String> = clarity_spec::translations::keys(&project).into_iter().map(|k| k.key).collect();
    assert!(keys.contains(&"undoChangedSince".to_string()), "{keys:?}");
}

const OPTIONS: &str = "module m

entity Shape
    name: text, label \"Name\"

page Main: window
    data shapes: list<Shape>
    data pin: text
    canvas name drawing at 1,1 span 6 rows 4 no undo
    table name shapeGrid at 1,5 span 6 rows 3 bind shapes columns name edit name no undo
    textbox name pinBox at 1,8 span 2 bind pin no undo
    textbox name noteBox at 3,8 span 2
";

/// Every part the person edits has undo, unless its line says `no undo`.
#[test]
fn no_undo_is_a_part_option_of_the_parts_the_person_edits() {
    assert!(errors(OPTIONS).is_empty(), "{:#?}", errors(OPTIONS));
    let project = load_sources([("m.clarity", OPTIONS)]);
    let widgets = &project.files[0].pages[0].widgets;
    let no_undo: Vec<(&str, bool)> = widgets.iter().map(|w| (w.props.name.as_deref().unwrap(), w.props.no_undo)).collect();
    assert_eq!(no_undo, [("drawing", true), ("shapeGrid", true), ("pinBox", true), ("noteBox", false)]);
    // The designer writes it back as it is written.
    let line = clarity_spec::widgets::widget_line(&widgets[2].props);
    assert!(line.ends_with("no undo"), "{line}");
    // A button edits nothing, so it has no undo to switch off.
    let button = errors(&OPTIONS.replace("    textbox name noteBox at 3,8 span 2\n", "    button \"Go\" name goButton at 3,8 span 2 no undo\n"));
    assert!(button.iter().any(|e| e.contains("undo")), "{button:#?}");
}

/// A part's edits are undone in its form or panel, or on their own; one without undo has none.
#[test]
fn a_parts_undo_scope_is_its_form_or_itself() {
    let source = "module m

entity Customer
    name: text, label \"Name\"

page Main: window
    data current: Customer
    form name customerForm at 1,1 span 6 rows 2 bind current
        textbox name nameBox at 1,1 span 4 bind current.name
    canvas name drawing at 1,3 span 6 rows 4
    textbox name pinBox at 1,8 span 2 no undo
    button \"Go\" name goButton at 3,8 span 2
";
    let project = load_sources([("m.clarity", source)]);
    assert!(errors(source).is_empty(), "{:#?}", errors(source));
    let widgets = &project.files[0].pages[0].widgets;
    let scope = |name: &str| clarity_spec::widgets::undo_scope(widgets, name);
    assert_eq!(scope("nameBox").as_deref(), Some("customerForm"));
    assert_eq!(scope("drawing").as_deref(), Some("drawing"));
    assert_eq!(scope("pinBox"), None);
    assert_eq!(scope("goButton"), None);
}

/// A picture loaded into an image part is an edit too; each part carries its undo scope, worked out once for every screen.
#[test]
fn an_image_part_has_undo_and_each_part_carries_its_scope() {
    let source = "module m

entity Album
    title: text, label \"Title\"
    cover: image, label \"Cover\"

page Main: window
    data album: Album
    form name albumForm at 1,1 span 6 rows 4 bind album
        image name cover at 1,1 span 3 rows 3 bind album.cover
    image name photo at 7,1 span 3 rows 3 bind album.cover no undo
";
    assert!(errors(source).is_empty(), "{:#?}", errors(source));
    let project = load_sources([("m.clarity", source)]);
    let widgets = &project.files[0].pages[0].widgets;
    assert_eq!(widgets[0].children[0].props.undo_scope.as_deref(), Some("albumForm"));
    assert_eq!(widgets[1].props.undo_scope, None);
    // Worked out, never written back.
    assert!(!clarity_spec::widgets::widget_line(&widgets[0].children[0].props).contains("undo"));
}

/// A part in a sidebar or the footer has its scope too.
#[test]
fn a_part_in_a_sidebar_has_its_undo_scope() {
    let source = "module m

page Main: window
    data note: text
    sidebar left name side
        textbox name noteBox at 1,1 span 2 bind note
    text name mainText at 1,1 span 6
";
    assert!(errors(source).is_empty(), "{:#?}", errors(source));
    let project = load_sources([("m.clarity", source)]);
    let side = &project.files[0].pages[0].frame.sidebars[0].widgets[0];
    assert_eq!(side.props.undo_scope.as_deref(), Some("noteBox"));
}
