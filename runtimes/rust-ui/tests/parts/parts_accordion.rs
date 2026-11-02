//! An accordion against the cases every renderer shares
//! (conformance/accordions.json), as the C# AccordionPartTests check it: its
//! headers pressed, the open section shown, the part's state from 1 and 0
//! for none.

use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::element::find_type;
use clarity_ui::geometry::Rect;
use clarity_ui::input::PointerButton;
use clarity_ui::parts::containers::accordion::{self, AccordionPart};
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{window_of, Shot, TestHost};
use clarity_ui::{Element, Screen};
use serde_json::Value;

fn conformance(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).expect("the case file reads")).expect("the case file is JSON")
}

fn window(markup: &str, host: Rc<TestHost>, width: f64) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(width, 400.0, 1.0);
    screen
}

fn first<T: Element>(screen: &Screen) -> &T {
    find_type::<T>(screen.root(), |_| true).expect("the part is on the screen")
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

fn accordion_of(sections: usize, open: i64) -> (Screen, Rc<TestHost>) {
    let host = TestHost::new();
    host.set_state("faq", |s| s.open = Some(open));
    let inside: String = (1..=sections).map(|i| format!(r#"<Section title="Section {i}"><Label at="1,1" span="3" caption="Inside {i}" /></Section>"#)).collect();
    let screen = window(&format!(r#"<Accordion name="faq" at="1,1" span="6" rows="8" onchange="opened">{inside}</Accordion>"#), host.clone(), 800.0);
    (screen, host)
}

#[test]
fn the_open_section_shows_as_on_every_renderer() {
    for case in conformance("accordions.json")["shown"].as_array().unwrap() {
        let (screen, _) = accordion_of(case["sections"].as_u64().unwrap() as usize, case["open"].as_i64().unwrap());
        let part = first::<AccordionPart>(&screen);
        let shown = case["shown"].as_i64().unwrap();
        assert_eq!(shown, accordion::shown(part.open(), part.pages().len()), "{case}");
        for (i, page) in part.pages().iter().enumerate() {
            assert_eq!(i as i64 + 1 == shown, page.is_shown());
            assert_eq!(Some(i as i64 + 1 == shown), part.headers()[i].chevron);
        }
    }
}

#[test]
fn a_press_on_a_header_opens_or_folds_as_on_every_renderer() {
    for case in conformance("accordions.json")["press"].as_array().unwrap() {
        let (mut screen, host) = accordion_of(case["sections"].as_u64().unwrap() as usize, case["open"].as_i64().unwrap());
        let header = first::<AccordionPart>(&screen).headers()[case["press"].as_u64().unwrap() as usize - 1].node().bounds;
        click(&mut screen, header.x + 20.0, header.y + 10.0);
        assert_eq!(case["open after"].as_i64(), host.state(Some("faq")).unwrap().open, "{case}");
        assert_eq!(vec!["run faq change opened"], host.logged());
    }
}

#[test]
fn an_accordions_part_state_opens_the_first_section() {
    let file = conformance("accordions.json");
    // The part's state as it comes: none opens the first section, 0 none.
    for case in file["open"].as_array().unwrap() {
        let host = TestHost::new();
        let state = serde_json::from_value::<PartState>(case["part"].clone()).expect("the part's state reads");
        host.set_state("faq", |s| *s = state.clone());
        let markup = r#"<Accordion name="faq" at="1,1" span="6" rows="8"><Section title="A" /><Section title="B" /><Section title="C" /></Accordion>"#;
        let screen = window(markup, host, 800.0);
        assert_eq!(case["open"].as_i64().unwrap(), first::<AccordionPart>(&screen).open(), "{case}");
    }
    // What setting it sends back.
    for case in file["set"].as_array().unwrap() {
        let state = PartState { open: case["open"].as_i64(), ..PartState::default() };
        assert_eq!(case["part"], serde_json::to_value(&state).unwrap(), "{case}");
    }
}

#[test]
fn the_open_sections_parts_fill_the_room_under_its_header_and_the_rest_follow() {
    let (mut screen, _) = accordion_of(3, 2);
    let part = first::<AccordionPart>(&screen);
    let headers: Vec<Rect> = part.headers().iter().map(|h| h.node().bounds).collect();
    let page = part.pages()[1].node().bounds;
    assert!(page.y >= headers[1].bottom());
    assert!(headers[2].y >= page.bottom());
    assert!(headers[2].bottom() <= part.node().bounds.bottom() + 0.001);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "containers-accordion.png");
}
