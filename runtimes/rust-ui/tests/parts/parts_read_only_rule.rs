//! One read-only rule for every part, as the C# ReadOnlyRuleTests check it:
//! readonly="true" in the markup, or its state set read-only by code. A named
//! part has a state, and that state does not undo what the markup says.

use clarity_ui::parts::text_box::TextBoxPart;
use clarity_ui::parts::{MissingPart, Part};
use clarity_ui::testing::{part_named, window_of, TestHost};
use clarity_ui::Screen;

fn part(screen: &Screen) -> &dyn Part {
    screen.element(part_named(screen, "p")).and_then(|e| e.as_part()).expect("the part is on the screen")
}

#[test]
fn a_named_part_marked_read_only_in_its_markup_is_read_only() {
    for kind in ["Textbox", "Number", "Checkbox", "Slider", "Folder", "Date"] {
        let screen = window_of(&format!(r#"<{kind} at="1,1" span="4" name="p" readonly="true" />"#), TestHost::new());
        assert!(part(&screen).as_any().downcast_ref::<MissingPart>().is_none(), "{kind} is drawn");
        assert!(part(&screen).base().read_only(), "{kind} is read-only");
    }
}

#[test]
fn code_makes_a_part_read_only_by_its_state() {
    let host = TestHost::new();
    let screen = window_of(r#"<Textbox at="1,1" span="4" name="p" />"#, host.clone());
    assert!(!part(&screen).base().read_only());
    host.set_state("p", |s| s.readonly = Some(true));
    assert!(part(&screen).base().read_only());
}

#[test]
fn a_named_read_only_input_takes_no_typing() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Textbox at="1,1" span="4" name="p" readonly="true" />"#, host);
    screen.resize(800.0, 400.0, 1.0);
    let id = part_named(&screen, "p");
    screen.focus(Some(id));
    screen.text_typed("x");
    let text = screen.element(id).unwrap().as_any().downcast_ref::<TextBoxPart>().unwrap().editor().text();
    assert_eq!("", text);
}
