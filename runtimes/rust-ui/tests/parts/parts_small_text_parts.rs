//! The small parts that show text, as the C# SmallTextPartsTests check
//! them: a heading, a text (a bound value on one line, or wrapped over
//! several rows and scrolling), a separator and a badge.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::containers::separator::SeparatorPart;
use clarity_ui::parts::containers::small::BadgePart;
use clarity_ui::parts::containers::text::TextPart;
use clarity_ui::parts::heading::HeadingPart;
use clarity_ui::parts::{Choice, FieldFacts, Formats, PartHost};
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Element, Modifiers, Rect, Screen};
use serde_json::{json, Value};
use skia_safe::Color;

/// A test host that also says what the values' types and choices are, as a generated window does, in English formats.
#[derive(Default)]
struct Host {
    inner: TestHost,
    types: RefCell<BTreeMap<String, String>>,
    choices: RefCell<BTreeMap<String, Vec<Choice>>>,
}

impl PartHost for Host {
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
        clarity_ui::PartHost::set_state(&self.inner, name, state)
    }
    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.inner.run(part, call)
    }
    fn text(&self, written: Option<&str>) -> String {
        self.inner.text(written)
    }
    fn changes(&self) -> u64 {
        self.inner.changes()
    }
    fn field(&self, path: &str) -> Option<FieldFacts> {
        self.inner.field(path)
    }
    fn type_of(&self, path: &str) -> Option<String> {
        self.types.borrow().get(path).cloned()
    }
    fn choices(&self, path: &str) -> Vec<Choice> {
        self.choices.borrow().get(path).cloned().unwrap_or_default()
    }
    fn formats(&self) -> Formats {
        Formats { language: Some("en-US".into()), currency: None }
    }
}

fn host() -> Rc<Host> {
    Rc::new(Host::default())
}

fn window(markup: &str, host: Rc<Host>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn bounds(screen: &Screen, name: &str) -> Rect {
    screen.element(part_named(screen, name)).unwrap().node().bounds
}

fn part<'a, T: 'static>(screen: &'a Screen, name: &str) -> &'a T {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<T>()).expect("a part of that kind")
}

/// Whether any pixel of a place is near `color`.
fn inked(shot: &mut Shot, place: Rect, color: Color) -> bool {
    (place.y as i32..place.bottom() as i32).any(|y| (place.x as i32..place.right() as i32).any(|x| near(shot.pixel(x, y), color, 30)))
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn a_heading_draws_its_caption_in_the_heading_size() {
    let host = host();
    host.inner.texts.borrow_mut().insert("ordersTitle".into(), "Orders".into());
    let mut screen = window(r#"<Heading at="1,1" span="6" name="title" caption="@ordersTitle" />"#, host);
    assert_eq!("Orders", part::<HeadingPart>(&screen, "title").text());
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let text = screen.look().color("text");
    assert!(inked(&mut shot, bounds(&screen, "title"), text));
    keep(&mut shot, "heading.png");
}

#[test]
fn a_text_shows_its_value_as_the_person_reads_it() {
    let host = host();
    host.inner.set("total", json!(1250.5));
    host.inner.set("paid", json!(true));
    host.inner.set("level", json!("High"));
    host.inner.set("name", json!("Ada"));
    host.inner.fields.borrow_mut().insert("total".into(), FieldFacts { format: Some("decimal 2".into()), ..FieldFacts::default() });
    host.choices.borrow_mut().insert(
        "level".into(),
        vec![Choice { value: json!("Low"), label: "Low".into(), cells: None }, Choice { value: json!("High"), label: "Very high".into(), cells: None }],
    );
    let screen = window(
        r#"<Text at="1,1" span="3" name="a" bind="total" />
           <Text at="4,1" span="3" name="b" bind="paid" />
           <Text at="7,1" span="3" name="c" bind="level" />
           <Text at="10,1" span="3" name="d" bind="name" />
           <Text at="1,2" span="3" name="e" />"#,
        host,
    );
    let shown: Vec<String> = ["a", "b", "c", "d", "e"].iter().map(|n| part::<TextPart>(&screen, n).text()).collect();
    assert_eq!(vec!["1,250.50", "Yes", "Very high", "Ada", ""], shown);
}

#[test]
fn a_text_over_several_rows_wraps_and_scrolls_with_the_wheel() {
    let host = host();
    let story = format!("{}\nThe end.", vec!["Once upon a time"; 30].join(" "));
    host.inner.set("story", json!(story));
    let mut screen = window(r#"<Text at="1,1" span="2" rows="2" name="story" bind="story" />"#, host.clone());
    Shot::of(&mut screen, 800, 400, 1.0);
    {
        let lines = part::<TextPart>(&screen, "story").lines().expect("over several rows it has its lines");
        assert!(!lines.focusable());
        assert_eq!(story, lines.text());
    }
    let b = bounds(&screen, "story");
    screen.wheel(b.x + 5.0, b.y + 5.0, 0.0, 40.0, Modifiers::NONE);
    assert_eq!(40.0, part::<TextPart>(&screen, "story").lines().unwrap().scroll_y());
    // A value changed by code shows.
    host.inner.set("story", json!("Short."));
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!("Short.", part::<TextPart>(&screen, "story").lines().unwrap().text());
}

#[test]
fn a_long_text_on_one_line_is_cut_off_with_an_ellipsis() {
    let host = host();
    let long = "m".repeat(200);
    host.inner.set("note", json!(long));
    let mut screen = window(r#"<Text at="1,1" span="1" name="note" bind="note" />"#, host);
    let width = bounds(&screen, "note").width;
    let mut surface = skia_safe::surfaces::raster_n32_premul((10, 10)).unwrap();
    screen.draw(surface.canvas());
    let painter = clarity_ui::Painter::new(surface.canvas(), screen.look(), screen.fonts());
    let fitted = painter.fit(&long, width, 14.0, "text", false);
    assert!(fitted.ends_with('\u{2026}'));
    assert!(screen.fonts().measure(&fitted, "text", 14.0, false) <= width);
    assert_eq!("short", painter.fit("short", width, 14.0, "text", false));
}

#[test]
fn a_separator_is_a_line_across_or_down_in_the_strong_border_color() {
    let mut screen = window(r#"<Separator name="a" at="1,1" span="12" /><Separator name="d" at="1,2" rows="3" orientation="vertical" />"#, host());
    assert!(!part::<SeparatorPart>(&screen, "a").vertical());
    assert!(part::<SeparatorPart>(&screen, "d").vertical());
    let (a, d) = (bounds(&screen, "a"), bounds(&screen, "d"));
    let (line, background) = (screen.look().color("strong_border"), screen.look().color("background"));
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "separator.png");
    assert_eq!(line, shot.pixel((a.x + a.width / 2.0) as i32, (a.y + a.height / 2.0) as i32));
    assert_eq!(line, shot.pixel((d.x + d.width / 2.0) as i32, (d.y + 10.0) as i32));
    assert_eq!(background, shot.pixel(d.x as i32 + 1, (d.y + 10.0) as i32));
}

#[test]
fn a_badge_shows_its_caption_or_its_value_in_the_accent_pill() {
    let host = host();
    host.inner.set("unread", json!(1200));
    host.types.borrow_mut().insert("unread".into(), "integer".into());
    host.inner.set("paid", json!(true));
    host.inner.set("level", json!("high"));
    host.choices.borrow_mut().insert("level".into(), vec![Choice { value: json!("high"), label: "Very high".into(), cells: None }]);
    let mut screen = window(
        r#"<Badge at="1,1" span="2" name="plain" />
           <Badge at="3,1" span="2" name="count" bind="unread" />
           <Badge at="5,1" span="2" name="paid" bind="paid" />
           <Badge at="7,1" span="2" name="level" bind="level" />"#,
        host,
    );
    assert_eq!("New", part::<BadgePart>(&screen, "plain").text());
    assert_eq!("1,200", part::<BadgePart>(&screen, "count").text());
    assert_eq!("Yes", part::<BadgePart>(&screen, "paid").text());
    assert_eq!("Very high", part::<BadgePart>(&screen, "level").text());
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let b = bounds(&screen, "plain");
    let look = screen.look().clone();
    assert_eq!(look.color("accent"), shot.pixel(b.x as i32 + 4, (b.y + b.height / 2.0) as i32));
    assert!(inked(&mut shot, b, look.color("on_accent")));
    assert_eq!(look.color("background"), shot.pixel(b.right() as i32 - 2, (b.y + b.height / 2.0) as i32));
    keep(&mut shot, "badge.png");
}

/// A heading that binds a field shows its value in the heading size, as a text part does in the text size.
#[test]
fn a_heading_that_binds_a_field_shows_its_value() {
    let h = host();
    h.inner.set("chapter.title", serde_json::json!("Arrival"));
    let screen = window(r#"<Heading name="title" at="1,1" span="6" caption="Chapter" bind="chapter.title" /><Heading name="plain" at="1,2" span="6" caption="Chapters" />"#, h);
    assert_eq!("Arrival", part::<HeadingPart>(&screen, "title").text());
    assert_eq!("Chapters", part::<HeadingPart>(&screen, "plain").text());
}
