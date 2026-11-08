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

#[test]
fn a_horizontal_accordion_stands_its_headers_side_by_side_with_the_open_section_beside_its_own() {
    let host = TestHost::new();
    host.set_state("faq", |s| s.open = Some(2));
    let inside: String = (1..=3).map(|i| format!(r#"<Section title="Section {i}"><Label at="1,1" span="3" caption="Inside {i}" /></Section>"#)).collect();
    let mut screen = window(&format!(r#"<Accordion name="faq" at="1,1" span="12" rows="8" orientation="horizontal" onchange="opened">{inside}</Accordion>"#), host.clone(), 800.0);
    let part = first::<AccordionPart>(&screen);
    let whole = part.node().bounds;
    let headers: Vec<Rect> = part.headers().iter().map(|h| h.node().bounds).collect();
    let page = part.pages()[1].node().bounds;
    // Each header is a strip as high as the accordion, narrower than it is high.
    for h in &headers {
        assert!((h.y - whole.y).abs() < 0.001 && (h.height - whole.height).abs() < 0.001, "{h:?} in {whole:?}");
        assert!(h.width < h.height);
    }
    assert!(headers[1].x > headers[0].right());
    assert!(page.x >= headers[1].right());
    assert!(headers[2].x >= page.right());
    assert!(headers[2].right() <= whole.right() + 0.001);
    assert!(page.width > whole.width / 2.0, "the open section has most of the room: {page:?}");
    // Its sections have its whole height, so it asks no room beyond its rows.
    assert!((page.height - whole.height).abs() < 0.001);
    assert_eq!(0.0, part.around());
    // A press on a header opens it, as a vertical one does.
    click(&mut screen, headers[2].x + 10.0, headers[2].y + 40.0);
    assert_eq!(Some(3), host.state(Some("faq")).unwrap().open);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "containers-accordion-horizontal.png");
}

/// A section's `color` tints its header, its text black or white as reads;
/// one without keeps the theme's look.
#[test]
fn a_section_header_takes_its_own_color() {
    let host = TestHost::new();
    let markup = r##"<Accordion name="faq" at="1,1" span="6" rows="8"><Section title="Blue" color="#2e86de"><Label at="1,1" span="3" caption="In" /></Section><Section title="Plain"><Label at="1,1" span="3" caption="In" /></Section><Section title="Pale" color="#f6e58d" /></Accordion>"##;
    let mut screen = window(markup, host, 800.0);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let part = first::<AccordionPart>(&screen);
    let headers = part.headers();
    assert_eq!(Some("#2e86de"), headers[0].tint.as_deref());
    assert_eq!(None, headers[1].tint);
    let at = |h: &clarity_ui::parts::containers::container::ContainerButton| clarity_ui::Element::node(h).bounds;
    let (blue, plain) = (at(headers[0]), at(headers[1]));
    let inside = |shot: &mut Shot, b: Rect| shot.pixel((b.right() - 6.0) as i32, (b.y + b.height / 2.0) as i32);
    let c = inside(&mut shot, blue);
    assert_eq!((0x2e, 0x86, 0xde), (c.r(), c.g(), c.b()), "the header is drawn in its color");
    let p = inside(&mut shot, plain);
    assert_ne!((0x2e, 0x86, 0xde), (p.r(), p.g(), p.b()), "a header without keeps the theme's look");
    let white = clarity_ui::parts::containers::container::readable_on(skia_safe::Color::from_rgb(0x1e, 0x3a, 0x8a));
    let dark = clarity_ui::parts::containers::container::readable_on(skia_safe::Color::from_rgb(0xf6, 0xe5, 0x8d));
    assert_eq!(skia_safe::Color::WHITE, white);
    assert_ne!(skia_safe::Color::WHITE, dark, "dark text on a pale header");
    keep(&mut shot, "accordion-colors.png");
}
