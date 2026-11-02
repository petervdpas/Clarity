//! A panel and a form, as the C# PanelPartTests check them: a frame with its
//! caption in the theme's header, its parts in a grid of its own as wide as
//! it spans; a form without a caption has no frame; a foldable one folds to
//! its header (conformance/folding.json); it grows when a part in it does,
//! and tells a list in it that it is inside.

use clarity_ui::element::find_type;
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::containers::container;
use clarity_ui::parts::containers::panel::{self, PanelPart};
use clarity_ui::parts::{Part, PartHost};
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::{Element, Screen};
use serde_json::Value;
use std::rc::Rc;

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

const PANEL: &str = r#"<Panel name="box" at="7,1" span="6" rows="4" caption="Customer"><Label name="inner" at="4,1" span="3" caption="Inner" /></Panel>"#;

#[test]
fn a_panels_parts_are_in_a_grid_as_wide_as_it_spans_under_its_header() {
    let screen = window(PANEL, TestHost::new(), 808.0);
    let panel = first::<PanelPart>(&screen);
    // Twelve columns of 60 with 8 between: the panel is the right half.
    assert_eq!(Rect::new(408.0, 0.0, 400.0, 168.0), panel.node().bounds);
    let inner = screen.element(part_named(&screen, "inner")).unwrap().node().bounds;
    // Its own six columns are the window's: the part at its fourth starts at the window's tenth.
    assert_eq!(612.0, inner.x);
    assert_eq!(196.0, inner.width);
    assert!(inner.y > 20.0);
}

#[test]
fn a_panel_shows_its_caption_in_the_themes_header_and_its_frame() {
    let mut screen = window(PANEL, TestHost::new(), 808.0);
    let look = screen.look().clone();
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    keep(&mut shot, "containers-panel.png");
    assert_eq!(look.part("header", None).background.unwrap(), shot.pixel(800, 6));
    assert_eq!(look.color("strong_border"), shot.pixel(405, 100));
}

#[test]
fn a_form_without_a_caption_has_no_frame_and_its_grid_lines_up_with_the_windows() {
    let screen = window(r#"<Form at="1,2" span="12" rows="3"><Label name="l" at="1,1" span="2" /></Form>"#, TestHost::new(), 808.0);
    let form = first::<PanelPart>(&screen);
    assert!(form.bare());
    assert_eq!(Rect::new(0.0, 44.0, 128.0, 36.0), screen.element(part_named(&screen, "l")).unwrap().node().bounds);
    assert_eq!(0.0, form.around());
}

#[test]
fn a_container_grows_when_a_part_in_it_does() {
    let screen = window(
        r#"<Panel name="a" at="1,1" span="6" rows="3"><Image at="1,1" span="3" rows="2" /></Panel>
           <Panel name="b" at="7,1" span="6" rows="3"><Label at="1,1" span="3" /></Panel>
           <Tabs name="c" at="1,4" span="6" rows="3"><Tab title="A" /><Tab title="B"><Image at="1,1" /></Tab></Tabs>"#,
        TestHost::new(),
        800.0,
    );
    let grows = |name: &str| screen.element(part_named(&screen, name)).and_then(|e| e.as_part()).unwrap().grows();
    assert!(grows("a"));
    assert!(!grows("b"));
    assert!(grows("c"));
}

#[test]
fn a_part_in_a_container_is_inside_it_and_one_on_the_window_is_not() {
    let screen = window(&format!(r#"{PANEL}<Label name="outer" at="1,6" span="3" />"#), TestHost::new(), 808.0);
    let part = |name: &str| screen.element(part_named(&screen, name)).and_then(|e| e.as_part()).unwrap();
    assert!(container::is_inside_part(part("inner")));
    assert!(!container::is_inside_part(part("outer")));
    assert!(first::<PanelPart>(&screen).frames_parts());
    // As it draws, what the panel holds is drawn inside it.
    assert!(!container::is_inside());
    {
        let _framed = container::framing();
        assert!(container::is_inside());
    }
    assert!(!container::is_inside());
}

#[test]
fn a_foldable_panel_folds_to_its_header_when_the_person_presses_it() {
    let host = TestHost::new();
    let mut screen = window(&PANEL.replace("caption=", r#"foldable="true" caption="#), host.clone(), 808.0);
    let inner_id = part_named(&screen, "inner");
    let inner = screen.element(inner_id).unwrap().node().bounds;
    assert_eq!(Some(inner_id), screen.at(620.0, inner.y + 5.0));
    click(&mut screen, 500.0, 8.0);
    assert_eq!(Some(true), host.state(Some("box")).unwrap().folded);
    assert!(first::<PanelPart>(&screen).is_folded());
    screen.resize(808.0, 400.0, 1.0);
    // Folded, its parts take no pointer and its place below the header is the window's.
    assert_ne!(Some(inner_id), screen.at(620.0, inner.y + 5.0));
    assert_ne!(Some(first::<PanelPart>(&screen).node().id()), screen.at(620.0, 100.0));
    assert!(first::<PanelPart>(&screen).node().bounds.height < 40.0);
    // A handler unfolds it by its state.
    host.set_state("box", |s| s.folded = Some(false));
    screen.resize(808.0, 400.0, 1.0);
    assert_eq!(Some(inner_id), screen.at(620.0, inner.y + 5.0));
    // A press in its body is no press on its header.
    click(&mut screen, 450.0, 150.0);
    assert_eq!(Some(false), host.state(Some("box")).unwrap().folded);
}

#[test]
fn a_foldable_panel_folds_by_keys_too() {
    let host = TestHost::new();
    let mut screen = window(&PANEL.replace("caption=", r#"foldable="true" caption="#), host.clone(), 808.0);
    let id = screen.root().node().children().iter().find(|c| c.as_any().is::<PanelPart>()).unwrap().node().id();
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::plain(Key::Space));
    assert_eq!(Some(true), host.state(Some("box")).unwrap().folded);
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(Some(false), host.state(Some("box")).unwrap().folded);
    // Tab reaches it: one that does not fold takes no focus.
    let plain = window(PANEL, TestHost::new(), 808.0);
    assert!(!first::<PanelPart>(&plain).focusable());
}

#[test]
fn a_foldable_part_is_folded_as_its_state_says_else_as_designed() {
    let cases = conformance("folding.json");
    for case in cases["cases"].as_array().unwrap() {
        let host = TestHost::new();
        let design = if case["design"].as_bool().unwrap() { r#" folded="true""# } else { "" };
        let named = case["part"].get("folded").and_then(Value::as_bool);
        if let Some(folded) = named {
            host.set_state("box", |s| s.folded = Some(folded));
        }
        let name = if named.is_some() { r#" name="box""# } else { "" };
        let screen = window(&format!(r#"<Panel{name} at="1,1" span="6" rows="3" foldable="true"{design} />"#), host, 800.0);
        assert_eq!(case["folded"].as_bool().unwrap(), first::<PanelPart>(&screen).is_folded(), "{case}");
    }
}

#[test]
fn a_panel_code_hides_draws_nothing() {
    let host = TestHost::new();
    let mut screen = window(PANEL, host.clone(), 808.0);
    host.set_state("box", |s| s.visible = Some(false));
    let background = screen.look().color("background");
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    assert_eq!(background, shot.pixel(800, 6));
}

#[test]
fn a_panel_with_a_header_asks_room_for_it_and_a_bare_form_asks_none() {
    let rows: String = (1..=3).map(|r| format!(r#"<Label at="1,{r}" span="4" />"#)).collect();
    let screen = window(&format!(r#"<Panel name="p" at="1,1" span="12" rows="3" caption="Box">{rows}</Panel><Form name="f" at="1,5" span="12" rows="3">{rows}</Form>"#), TestHost::new(), 808.0);
    let panel = find_type::<PanelPart>(screen.root(), |p| p.base().name() == Some("p")).unwrap();
    assert!(panel.around() > 0.0);
    assert!(panel.inner().node().bounds.height >= panel.inner().content - 0.5);
    let form = find_type::<PanelPart>(screen.root(), |p| p.base().name() == Some("f")).unwrap();
    assert_eq!(0.0, form.around());
}

#[test]
fn enter_in_an_input_of_a_form_runs_its_submit() {
    let host = TestHost::new();
    let screen = window(r#"<Form name="f" at="1,1" span="12" rows="2" onsubmit="save"><Textbox name="t" at="1,1" span="4" /></Form><Textbox name="out" at="1,4" span="4" />"#, host.clone(), 800.0);
    assert!(panel::submit(screen.root(), part_named(&screen, "t")));
    assert_eq!(vec!["run f submit save"], host.logged());
    // An input in no form submits nothing.
    assert!(!panel::submit(screen.root(), part_named(&screen, "out")));
}
