//! The button kinds of U1b: a toggle (a button that stays pressed, bound to
//! a bool or holding its own, as a checkbox), a button group (`buttons`,
//! its buttons and toggles joined in one row) and a split button (`split`
//! on a button, its `menu` block the dropdown behind the arrow).

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, ContainerKind, Event, WidgetKind};
use clarity_spec::{Project, Severity};

const NOTES: &str = "module notes

entity Note
    text: text
    bold: bool

page NoteCard: window
    data note: Note
    toggle \"Bold\" name boldToggle icon \"bold\" at 1,1 span 2 bind note.bold on change: restyle
    buttons name alignButtons at 3,1 span 3
        toggle name leftToggle icon \"text-align-start\" at 1,1 span 1 on change: alignLeft
        toggle name centerToggle icon \"text-align-center\" at 2,1 span 1 on change: alignCenter
        toggle name rightToggle icon \"text-align-end\" at 3,1 span 1 on change: alignRight
    button \"Save\" name saveButton split at 7,1 span 2 on click: save
        menu
            item \"Save as...\" name saveAsItem on click: saveAs
";

const CODE: &str = "
code NoteCard
    handler restyle
        save note

    handler alignLeft
        centerToggle.checked = false
        rightToggle.checked = false

    handler alignCenter
        leftToggle.checked = false

    handler alignRight
        leftToggle.checked = not rightToggle.checked

    handler save
        save note

    handler saveAs
        saveAsItem.enabled = false
";

fn project(page: &str) -> Project {
    let text = format!("{page}{CODE}");
    load_keyed([("notes.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str) -> Vec<String> {
    project(page).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn the_button_kinds_read_and_write_back_as_written() {
    let (file, diagnostics) = parse(NOTES, "notes.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    assert_eq!(parts[0].props.kind, WidgetKind::Toggle);
    assert!(widgets::has_event(WidgetKind::Toggle, Event::Change));
    assert_eq!(parts[1].props.kind, WidgetKind::Buttons);
    assert_eq!(widgets::container(WidgetKind::Buttons), Some(ContainerKind::Plain));
    assert_eq!(parts[1].children.len(), 3);
    assert!(parts[2].props.split);
    assert!(parts[2].menu.is_some());
    let page = &NOTES[NOTES.find("page NoteCard").unwrap()..];
    for line in page.lines().filter(|l| l.trim_start().starts_with(['t', 'b'])) {
        let (file, _) = parse(&NOTES.replace(page, &format!("page NoteCard: window\n    data note: Note\n    {}\n", line.trim())), "notes.clarity");
        if let Some(w) = file.pages.first().and_then(|p| p.widgets.first()) {
            assert_eq!(widgets::widget_line(&w.props), line.trim());
        }
    }
}

#[test]
fn a_window_with_them_checks_clean_and_a_toggle_is_read_and_set_as_checked() {
    let p = project(NOTES);
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let en = &p.translations["en"];
    assert!(en.values().any(|v| v == "Bold") && en.values().any(|v| v == "Save as..."), "{en:?}");
}

#[test]
fn the_rules_of_the_button_kinds() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    // A toggle binds a bool.
    says(NOTES.replace("bind note.bold", "bind note.text"), "a toggle shows bool");
    // A button group holds buttons and toggles, in one row.
    says(
        NOTES.replace("        toggle name rightToggle", "        textbox name noteBox at 4,1 span 1 bind note.text\n        toggle name rightToggle").replace("alignButtons at 3,1 span 3", "alignButtons at 3,1 span 4"),
        "a button group holds buttons and toggles",
    );
    says(NOTES.replace("rightToggle icon \"text-align-end\" at 3,1", "rightToggle icon \"text-align-end\" at 3,2").replace("alignButtons at 3,1 span 3", "alignButtons at 3,1 span 3 rows 2"), "the buttons of a group stand in one row");
    // A split button opens its menu, and runs its click.
    says(NOTES.replace("        menu\n            item \"Save as...\" name saveAsItem on click: saveAs\n", ""), "a split button opens its menu: write a 'menu' block under it");
    says(NOTES.replace("split at 7,1 span 2 on click: save", "split at 7,1 span 2"), "a split button runs its click too: wire 'on click'");
}

#[test]
fn the_designer_gives_a_part_a_menu_with_one_item() {
    use clarity_spec::edit::{Edit, apply};
    let plain = NOTES.replace("        menu\n            item \"Save as...\" name saveAsItem on click: saveAs\n", "");
    let at = plain.find("button \"Save\"").unwrap() as u32;
    let outcome = apply(&plain, &Edit::AddMenu { item: at }).unwrap();
    assert!(outcome.text.contains("split at 7,1 span 2 on click: save\n        menu\n            item \"First item\" name firstItem\n"), "{}", outcome.text);
    let at = outcome.text.find("button \"Save\"").unwrap() as u32;
    assert!(apply(&outcome.text, &Edit::AddMenu { item: at }).is_err(), "one menu per part");
}
