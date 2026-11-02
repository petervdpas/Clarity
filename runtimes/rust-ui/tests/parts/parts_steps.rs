//! A wizard against the cases every renderer shares (conformance/steps.json):
//! its buttons pressed where the person sees them, the handler of `on next`
//! refusing or choosing the step as a window's handler does, through a host
//! that says whether it refused. Also its words, its own step without a name,
//! and how it looks.

use std::cell::RefCell;
use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::containers::steps::StepsPart;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Element, Screen};
use serde_json::Value;

/// A window whose wizard's handlers behave as a case says, and say what ran on which step.
struct WizardHost {
    inner: Rc<TestHost>,
    handler: Value,
    ran: RefCell<Vec<String>>,
}

impl PartHost for WizardHost {
    fn read(&self, path: &str) -> Option<Value> {
        self.inner.read(path)
    }

    fn write(&self, part: &str, path: &str, value: Value) {
        self.inner.write(part, path, value)
    }

    fn edited(&self, part: &str) {
        self.inner.edited(part)
    }

    fn state(&self, name: Option<&str>) -> Option<PartState> {
        self.inner.state(name)
    }

    fn set_state(&self, name: &str, state: PartState) {
        clarity_ui::PartHost::set_state(&*self.inner, name, state)
    }

    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.inner.run(part, call);
        let step = self.inner.state(part).and_then(|s| s.step).unwrap_or(1);
        self.ran.borrow_mut().push(format!("{} {}", call.event, step));
        if call.event != "next" {
            return true;
        }
        if self.handler["refuses"].as_array().is_some_and(|r| r.iter().any(|s| s.as_i64() == Some(step))) {
            return false;
        }
        if self.handler["jumps"]["from"].as_i64() == Some(step) {
            let to = self.handler["jumps"]["to"].as_i64();
            self.inner.set_state(part.unwrap(), |s| s.step = to);
        }
        true
    }

    fn text(&self, written: Option<&str>) -> String {
        self.inner.text(written)
    }

    fn changes(&self) -> u64 {
        self.inner.changes()
    }
}

fn steps_file() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/steps.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/steps.json")).expect("it is JSON")
}

fn wizard_markup(steps: usize, handler: bool) -> String {
    let on_next = if handler { r#" onnext="checkStep""# } else { "" };
    let pages: String = (1..=steps).map(|i| format!(r#"<Step title="Step {i}"><Label at="1,1" span="3" caption="On step {i}" /></Step>"#)).collect();
    format!(r#"<Steps name="wizard" at="1,1" span="12" rows="8"{on_next} onfinish="done">{pages}</Steps>"#)
}

fn window(markup: &str, host: Rc<dyn PartHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(808.0, 400.0, 1.0);
    screen
}

fn wizard<'a>(screen: &'a Screen, name: &str) -> &'a StepsPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<StepsPart>()).expect("a steps part")
}

fn the_wizard(screen: &Screen) -> &StepsPart {
    clarity_ui::element::find_type::<StepsPart>(screen.root(), |_| true).expect("a steps part")
}

fn shown_buttons(part: &StepsPart) -> Vec<&'static str> {
    [("back", part.back()), ("next", part.next()), ("finish", part.finish())].into_iter().filter(|(_, b)| b.is_shown()).map(|(n, _)| n).collect()
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn every_walk_through_a_wizard_is_as_on_every_renderer() {
    let file = steps_file();
    let cases = file["walk"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let count = case["steps"].as_u64().unwrap() as usize;
        let inner = TestHost::new();
        if let Some(start) = case["start"].as_i64() {
            inner.set_state("wizard", |s| s.step = Some(start));
        }
        let host = Rc::new(WizardHost { inner: inner.clone(), handler: case["handler"].clone(), ran: RefCell::new(Vec::new()) });
        let mut screen = window(&wizard_markup(count, case["handler"].is_object()), host.clone());
        for press in case["do"].as_array().unwrap() {
            let press = press.as_str().unwrap();
            screen.resize(808.0, 400.0, 1.0);
            let part = wizard(&screen, "wizard");
            let button = match press {
                "next" => part.next(),
                "back" => part.back(),
                "finish" => part.finish(),
                other => panic!("{name}: unknown button {other}"),
            };
            assert!(button.is_shown(), "{name}: {press} is not shown");
            let at = button.node().bounds;
            screen.click(at.x + at.width / 2.0, at.y + at.height / 2.0);
        }
        screen.resize(808.0, 400.0, 1.0);
        let part = wizard(&screen, "wizard");
        let expect = &case["expect"];
        assert_eq!(expect["shown"].as_i64().unwrap(), part.shown(), "{name}: shown");
        let wanted: Vec<&str> = expect["buttons"].as_array().unwrap().iter().map(|b| b.as_str().unwrap()).collect();
        assert_eq!(wanted, shown_buttons(part), "{name}: buttons");
        let ran: Vec<String> = expect["ran"].as_array().unwrap().iter().map(|r| r.as_str().unwrap().to_string()).collect();
        assert_eq!(ran, host.ran.borrow().clone(), "{name}: ran");
        // Only the shown step's page shows.
        for (i, page) in part.pages().iter().enumerate() {
            assert_eq!(i as i64 + 1 == part.shown(), page.shown, "{name}: page {}", i + 1);
        }
    }
}

#[test]
fn a_wizards_step_is_its_part_state_from_one() {
    let file = steps_file();
    for case in file["step"].as_array().unwrap() {
        let host = TestHost::new();
        if let Some(step) = case["part"]["step"].as_i64() {
            host.set_state("wizard", |s| s.step = Some(step));
        }
        let screen = window(&wizard_markup(3, false), host.clone());
        assert_eq!(case["step"].as_i64().unwrap(), the_wizard(&screen).step());
    }
    for case in file["set"].as_array().unwrap() {
        let host = TestHost::new();
        let mut screen = window(&wizard_markup(3, false), host.clone());
        let id = part_named(&screen, "wizard");
        let step = case["step"].as_i64().unwrap();
        screen.act(id, |e, _| e.as_any().downcast_ref::<StepsPart>().unwrap().set_step(step));
        assert_eq!(case["part"]["step"].as_i64(), host.state(Some("wizard")).unwrap().step);
    }
}

#[test]
fn its_buttons_are_in_claritys_own_words() {
    let screen = window(&wizard_markup(2, false), TestHost::new());
    let part = the_wizard(&screen);
    assert_eq!(("Back".to_string(), "Next".to_string(), "Finish".to_string()), (part.back().text(), part.next().text(), part.finish().text()));
}

#[test]
fn a_plain_host_runs_the_handler_and_the_wizard_moves_on() {
    let host = TestHost::new();
    let mut screen = window(&wizard_markup(3, true), host.clone());
    let at = the_wizard(&screen).next().node().bounds;
    screen.click(at.x + 4.0, at.y + 4.0);
    assert_eq!(Some(2), host.state(Some("wizard")).unwrap().step);
    assert_eq!(vec!["run wizard next checkStep"], host.logged());
}

#[test]
fn a_wizard_without_a_name_keeps_its_own_step() {
    let mut screen = window(&wizard_markup(2, false).replace(r#"name="wizard" "#, ""), TestHost::new());
    let at = the_wizard(&screen).next().node().bounds;
    screen.click(at.x + 4.0, at.y + 4.0);
    assert_eq!(2, the_wizard(&screen).step());
    screen.resize(808.0, 400.0, 1.0);
    let at = the_wizard(&screen).back().node().bounds;
    screen.click(at.x + 4.0, at.y + 4.0);
    assert_eq!(1, the_wizard(&screen).step());
}

#[test]
fn a_disabled_wizard_does_not_move() {
    let host = TestHost::new();
    host.set_state("wizard", |s| s.enabled = Some(false));
    let mut screen = window(&wizard_markup(3, true), host.clone());
    let at = the_wizard(&screen).next().node().bounds;
    screen.click(at.x + 4.0, at.y + 4.0);
    assert_eq!(1, the_wizard(&screen).step());
    assert!(host.logged().is_empty());
}

#[test]
fn enter_on_the_focused_next_moves_on() {
    let host = TestHost::new();
    let mut screen = window(&wizard_markup(3, false), host.clone());
    let next = the_wizard(&screen).next().node().id();
    screen.focus(Some(next));
    screen.key_pressed(clarity_ui::input::KeyInput::plain(clarity_ui::input::Key::Enter));
    assert_eq!(Some(2), host.state(Some("wizard")).unwrap().step);
}

#[test]
fn a_wizard_draws_its_titles_its_step_and_its_buttons() {
    let host = TestHost::new();
    host.set_state("wizard", |s| s.step = Some(2));
    let mut screen = window(&wizard_markup(3, true), host);
    let accent = screen.look().color("accent");
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    keep(&mut shot, "steps.png");
    let part = the_wizard(&screen);
    // Next is the accent button, at the right of the bottom line.
    let next = part.next().node().bounds;
    assert!(near(accent, shot.pixel((next.x + 4.0) as i32, (next.y + next.height / 2.0) as i32), 10));
    assert!(next.right() <= part.node().bounds.right() + 0.5);
    // The page of step 2 lies under the titles.
    let page = &part.pages()[1];
    assert!(page.node().bounds.y > part.node().bounds.y + StepsPart::TITLES_HEIGHT);
}
