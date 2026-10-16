use clarity_spec::code::edit::{add_handler, code_text, set_code};
use clarity_spec::edit::{Edit, apply};
use clarity_spec::model::Handler;
use clarity_spec::widgets::Event;
use clarity_spec::{Severity, load_sources};

const FILE: &str = "module m

page Card: window
    title \"Card\"
    button \"Save\" name save at 1,1 span 2 on click: save_clicked

page Other: window
    title \"Other\"
";

const WITH_CODE: &str = "module m

page Card: window
    title \"Card\"
    button \"Save\" name save at 1,1 span 2 on click: save_clicked

page Other: window
    title \"Other\"

code Card
    handler save_clicked
        // Store and leave.
        close

    handler unused
        return
";

#[test]
fn reads_the_code_of_a_window_without_its_indentation() {
    assert_eq!(
        code_text(WITH_CODE, "Card"),
        "handler save_clicked\n    // Store and leave.\n    close\n\nhandler unused\n    return\n"
    );
    assert_eq!(code_text(WITH_CODE, "Other"), "");
}

#[test]
fn writes_a_new_code_block_at_the_end() {
    let text = set_code(FILE, "Card", "handler save_clicked\n    close\n").unwrap();
    assert_eq!(text, format!("{FILE}\ncode Card\n    handler save_clicked\n        close\n"));
    let project = load_sources([("m.clarity", text.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
}

#[test]
fn replaces_an_existing_block_and_keeps_the_rest() {
    let text = set_code(WITH_CODE, "Card", "handler save_clicked\n    return\n").unwrap();
    assert_eq!(
        text,
        WITH_CODE.replace(
            "    handler save_clicked\n        // Store and leave.\n        close\n\n    handler unused\n        return\n",
            "    handler save_clicked\n        return\n"
        )
    );
}

#[test]
fn a_block_in_the_middle_is_replaced_in_place() {
    let source = WITH_CODE.replace("\npage Other: window\n    title \"Other\"\n", "") + "\npage Other: window\n";
    let text = set_code(&source, "Card", "handler save_clicked\n    close").unwrap();
    assert!(text.ends_with("code Card\n    handler save_clicked\n        close\n\npage Other: window\n"), "{text}");
}

#[test]
fn empty_code_removes_the_block() {
    let text = set_code(WITH_CODE, "Card", "  \n").unwrap();
    assert_eq!(text, FILE);
}

#[test]
fn refuses_code_for_an_unknown_window_and_tabs() {
    assert!(set_code(FILE, "Nope", "handler x\n").unwrap_err().to_string().contains("no window or app 'Nope'"));
    assert!(set_code(FILE, "Card", "handler x\n\tclose\n").unwrap_err().to_string().contains("tab"));
}

#[test]
fn adds_an_empty_handler_once() {
    let text = add_handler(FILE, "Card", "save_clicked").unwrap();
    assert!(text.ends_with("code Card\n    handler save_clicked\n"), "{text}");
    assert_eq!(add_handler(&text, "Card", "save_clicked").unwrap(), text);
    let more = add_handler(WITH_CODE, "Card", "other_clicked").unwrap();
    assert!(more.ends_with("    handler unused\n        return\n\n    handler other_clicked\n"), "{more}");
}

#[test]
fn wiring_an_event_writes_its_handler_in_the_same_edit() {
    let item = FILE.find("button").unwrap() as u32;
    let mut props = clarity_spec::parser::parse(FILE, "m.clarity").0.pages[0].widgets[0].props.clone();
    props.handlers = vec![Handler {
        event: Event::Click,
        handler: "store_clicked".into(),
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
                name: "store_clicked".into(),
            },
        ],
    };
    let text = apply(FILE, &edit).unwrap().text;
    assert!(text.contains("on click: store_clicked"), "{text}");
    assert!(text.ends_with("code Card\n    handler store_clicked\n"), "{text}");
    let project = load_sources([("m.clarity", text.as_str())]);
    let warnings: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Warning).collect();
    assert!(warnings.is_empty(), "{warnings:#?}");
}

#[test]
fn set_code_is_an_edit() {
    let text = apply(
        FILE,
        &Edit::SetCode {
            owner: "Card".into(),
            text: "handler save_clicked\n    close\n".into(),
        },
    )
    .unwrap()
    .text;
    assert!(text.ends_with("code Card\n    handler save_clicked\n        close\n"));
}

#[test]
fn maps_problems_onto_the_code_panel_lines() {
    let source = set_code(FILE, "Card", "handler save_clicked\n    let x = nothing\n").unwrap();
    let project = load_sources([("m.clarity", source.as_str())]);
    let problems = clarity_spec::code::edit::problems_in_code(&source, "Card", "m.clarity", &project.diagnostics);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert_eq!(problems[0].message, "unknown name 'nothing'");
    assert_eq!((problems[0].span.line, problems[0].span.col), (2, 13));
}
