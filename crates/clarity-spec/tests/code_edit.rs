use clarity_spec::code::edit::{add_handler, code_text, set_code};
use clarity_spec::edit::{Edit, apply};
use clarity_spec::model::Handler;
use clarity_spec::widgets::Event;
use clarity_spec::load_converted;

const FILE: &str = "module m

page Card: window
    title \"Card\"
    button \"Save\" name save at 1,1 span 2 on click: saveClicked

page Other: window
    title \"Other\"
";

const WITH_CODE: &str = "module m

page Card: window
    title \"Card\"
    button \"Save\" name save at 1,1 span 2 on click: saveClicked

page Other: window
    title \"Other\"

code Card
    handler saveClicked
        // Store and leave.
        close

    handler unused
        return
";

#[test]
fn reads_the_code_of_a_window_without_its_indentation() {
    assert_eq!(
        code_text(WITH_CODE, "Card"),
        "handler saveClicked\n    // Store and leave.\n    close\n\nhandler unused\n    return\n"
    );
    assert_eq!(code_text(WITH_CODE, "Other"), "");
}

#[test]
fn writes_a_new_code_block_at_the_end() {
    let text = set_code(FILE, "Card", "handler saveClicked\n    close\n").unwrap();
    assert_eq!(text, format!("{FILE}\ncode Card\n    handler saveClicked\n        close\n"));
    let project = load_converted([("m.clarity", text.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
}

#[test]
fn replaces_an_existing_block_and_keeps_the_rest() {
    let text = set_code(WITH_CODE, "Card", "handler saveClicked\n    return\n").unwrap();
    assert_eq!(
        text,
        WITH_CODE.replace(
            "    handler saveClicked\n        // Store and leave.\n        close\n\n    handler unused\n        return\n",
            "    handler saveClicked\n        return\n"
        )
    );
}

#[test]
fn a_block_in_the_middle_is_replaced_in_place() {
    let source = WITH_CODE.replace("\npage Other: window\n    title \"Other\"\n", "") + "\npage Other: window\n";
    let text = set_code(&source, "Card", "handler saveClicked\n    close").unwrap();
    assert!(text.ends_with("code Card\n    handler saveClicked\n        close\n\npage Other: window\n"), "{text}");
}

#[test]
fn empty_code_removes_the_block() {
    let text = set_code(WITH_CODE, "Card", "  \n").unwrap();
    assert_eq!(text, FILE);
}

#[test]
fn refuses_code_for_an_unknown_window_and_tabs() {
    // Code for a window or app that does not exist: the project's check says so (the window has a file of its own).
    let nope = set_code(FILE, "Nope", "handler x\n").unwrap();
    assert!(load_converted([("m.clarity", nope.as_str())]).diagnostics.iter().any(|d| d.message.contains("no window, app or entity 'Nope'")));
    assert!(set_code(FILE, "Card", "handler x\n\tclose\n").unwrap_err().to_string().contains("tab"));
}

#[test]
fn adds_an_empty_handler_once() {
    let text = add_handler(FILE, "Card", "saveClicked").unwrap();
    assert!(text.ends_with("code Card\n    handler saveClicked\n"), "{text}");
    assert_eq!(add_handler(&text, "Card", "saveClicked").unwrap(), text);
    let more = add_handler(WITH_CODE, "Card", "otherClicked").unwrap();
    assert!(more.ends_with("    handler unused\n        return\n\n    handler otherClicked\n"), "{more}");
}

#[test]
fn wiring_an_event_writes_its_handler_in_the_same_edit() {
    // The window in its own file, the code in the module's, as a project holds them.
    let files = clarity_spec::converted([("m.clarity", FILE)]);
    let module = files.iter().find(|(p, _)| p == "m.clarity").map(|(_, t)| t.clone()).expect("the module");
    let (_, markup) = files.iter().find(|(p, _)| p.ends_with("design/Card.cxaml")).expect("the window's file");
    let button = clarity_spec::cxaml::read_page(markup, "").0.expect("the window").widgets[0].clone();
    let item = button.span.start;
    let mut props = button.props.clone();
    props.handlers = vec![Handler {
        event: Event::Click,
        handler: "storeClicked".into(),
        args: Vec::new(),
    }];
    let edit = Edit::Batch {
        edits: vec![
            Edit::UpdateWidget {
                item,
                props,
                container: None,
            },
            Edit::AddHandler {
                owner: "Card".into(),
                name: "storeClicked".into(),
            },
        ],
    };
    // Split as the designer splits it: the wiring to the window's file, the handler to the module.
    let (design, code) = clarity_spec::design_edit::split(&edit);
    let (design, code) = (design.expect("the wiring is the window's"), code.expect("the handler is the module's"));
    let card = clarity_spec::cxaml::apply(None, markup, &design).unwrap_or_else(|e| panic!("{e}")).text;
    assert!(card.contains("<Button name=\"save\" at=\"1,1\" span=\"2\" caption=\"Save\" onclick=\"storeClicked\" />"), "{card}");
    let text = apply(&module, &code).unwrap().text;
    assert!(text.ends_with("code Card\n    handler storeClicked\n"), "{text}");
    let mut after: Vec<(String, String)> = files.clone();
    for (path, written) in &mut after {
        if path == "m.clarity" {
            *written = text.clone();
        } else if path.ends_with("design/Card.cxaml") {
            *written = card.clone();
        }
    }
    let project = clarity_spec::load_sources(after.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
}

#[test]
fn set_code_is_an_edit() {
    let text = apply(
        FILE,
        &Edit::SetCode {
            owner: "Card".into(),
            text: "handler saveClicked\n    close\n".into(),
        },
    )
    .unwrap()
    .text;
    assert!(text.ends_with("code Card\n    handler saveClicked\n        close\n"));
}

#[test]
fn maps_problems_onto_the_code_panel_lines() {
    let written = set_code(FILE, "Card", "handler saveClicked\n    let x = nothing\n").unwrap();
    // The module as a project holds it: its window in a file of its own.
    let files = clarity_spec::converted([("m.clarity", written.as_str())]);
    let source = files[0].1.clone();
    let project = clarity_spec::load_sources(files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let problems = clarity_spec::code::edit::problems_in_code(&source, "Card", "m.clarity", &project.diagnostics);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert_eq!(problems[0].message, "unknown name 'nothing'");
    assert_eq!((problems[0].span.line, problems[0].span.col), (2, 13));
}
