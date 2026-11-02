//! The markdown part: Markdown read into blocks the same way on every stack
//! (conformance/markdown.json), shown as headings, paragraphs, list items
//! and code, wrapped to its width and scrolling when long; its text a
//! resource file by name, the text a key gives, or the text it binds; a link
//! opening through the host.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::input::{Modifiers, PointerButton};
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::containers::markdown::blocks::{self, Block, BlockKind, Span};
use clarity_ui::parts::containers::markdown::{MarkdownPart, MarkdownRun};
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::{Element, Screen};
use serde_json::{json, Map, Value};

/// A test host that also carries resource files, as a generated application does.
#[derive(Default)]
struct Host {
    inner: TestHost,
    resources: RefCell<BTreeMap<String, Vec<u8>>>,
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
    fn change_state(&self, name: &str, change: &mut dyn FnMut(&mut PartState)) {
        self.inner.change_state(name, change)
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
    fn open_address(&self, address: &str) {
        self.inner.open_address(address)
    }
    fn resource(&self, name: &str) -> Option<Vec<u8>> {
        self.resources.borrow().get(name).cloned()
    }
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

/// A markdown part named `rules` six columns wide and four rows high, drawn once.
fn markdown(attributes: &str, given: impl FnOnce(&Host), width: i32, height: i32) -> (Screen, Rc<Host>) {
    let host = Rc::new(Host::default());
    given(&host);
    let mut screen = window_of(&format!(r#"<Markdown name="rules" at="1,1" span="6" rows="4" {attributes} />"#), host.clone());
    Shot::of(&mut screen, width, height, 1.0);
    (screen, host)
}

fn part<'a>(screen: &'a Screen) -> &'a MarkdownPart {
    screen.element(part_named(screen, "rules")).and_then(|e| e.as_any().downcast_ref::<MarkdownPart>()).expect("a markdown part")
}

// -- Reading ---------------------------------------------------------------------

/// A block as the conformance file writes it: only what holds is listed.
fn written(block: &Block) -> Value {
    let spans: Vec<Value> = block.spans.iter().map(written_span).collect();
    match block.kind {
        BlockKind::Heading => json!({ "kind": "heading", "level": block.level, "spans": spans }),
        BlockKind::Paragraph => json!({ "kind": "paragraph", "spans": spans }),
        BlockKind::Item => match block.number {
            Some(number) => json!({ "kind": "item", "depth": block.depth, "ordered": true, "number": number, "spans": spans }),
            None => json!({ "kind": "item", "depth": block.depth, "ordered": false, "spans": spans }),
        },
        BlockKind::Code => json!({ "kind": "code", "text": block.text }),
    }
}

fn written_span(span: &Span) -> Value {
    let mut out = Map::new();
    out.insert("text".into(), json!(span.text));
    if span.bold {
        out.insert("bold".into(), json!(true));
    }
    if span.italic {
        out.insert("italic".into(), json!(true));
    }
    if span.code {
        out.insert("code".into(), json!(true));
    }
    if let Some(link) = &span.link {
        out.insert("link".into(), json!(link));
    }
    Value::Object(out)
}

#[test]
fn markdown_reads_as_every_renderer_reads_it() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/markdown.json");
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for case in file["cases"].as_array().unwrap() {
        let read: Vec<Value> = blocks::read(case["markdown"].as_str().unwrap()).iter().map(written).collect();
        assert_eq!(case["blocks"], Value::Array(read.clone()), "{}: {}", case["name"], Value::Array(read));
    }
}

// -- The part --------------------------------------------------------------------

#[test]
fn its_text_is_a_resource_file_a_key_s_text_or_what_it_binds() {
    let (screen, _) = markdown(r#"source="rules.md""#, |h| drop(h.resources.borrow_mut().insert("rules.md".into(), "\u{feff}# From the file".into())), 800, 400);
    assert_eq!("# From the file", part(&screen).markdown());
    let (screen, _) = markdown(r#"source="gone.md""#, |_| {}, 800, 400);
    assert_eq!("", part(&screen).markdown());
    let (screen, _) = markdown(r#"caption="@rulesText""#, |h| drop(h.inner.texts.borrow_mut().insert("rulesText".into(), "Play *fair*.".into())), 800, 400);
    assert_eq!("Play *fair*.", part(&screen).markdown());
    let (mut screen, host) = markdown(r#"bind="game.notes""#, |h| h.inner.set("game.notes", json!("One")), 800, 400);
    assert_eq!("One", part(&screen).markdown());
    host.inner.set("game.notes", json!("Two"));
    assert_eq!("Two", part(&screen).markdown());
    Shot::of(&mut screen, 800, 400, 1.0);
    let runs = part(&screen).runs();
    assert_eq!(1, runs.len());
    assert_eq!("Two", runs[0].text.trim());
}

#[test]
fn it_shows_headings_paragraphs_items_and_code() {
    let text = "# Rules\n\nSee [the site](https://example.com) **now** and *then*.\n\n- one\n1. two\n\n```\nlet x = 1\n```";
    let (mut screen, _) = markdown(r#"bind="notes""#, |h| h.inner.set("notes", json!(text)), 800, 400);
    let runs = part(&screen).runs();
    let find = |start: &str| -> MarkdownRun { runs.iter().find(|r| r.text.starts_with(start)).cloned().unwrap_or_else(|| panic!("no run {start}")) };
    let heading = find("Rules");
    let plain = find("See");
    assert!(heading.bold);
    assert!(heading.size > plain.size);
    let line: String = runs.iter().filter(|r| r.place.y == plain.place.y).map(|r| r.text.as_str()).collect();
    assert_eq!("See the site now and then.", line.trim());
    assert_eq!(Some("https://example.com"), runs.iter().find(|r| r.text == "the ").and_then(|r| r.link.as_deref()));
    assert_eq!(Some("https://example.com"), find("site").link.as_deref());
    assert!(find("now").bold);
    assert!(find("then").italic);
    assert!(runs.iter().any(|r| r.text == "\u{2022}"));
    assert!(runs.iter().any(|r| r.text == "1."));
    let code = find("let");
    assert!(code.code);
    assert!(part(&screen).code_boxes().iter().any(|b| b.contains(code.place.x + 1.0, code.place.y + 1.0)));
    assert!(find("one").place.x > plain.place.x);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "markdown.png");
}

#[test]
fn a_long_paragraph_wraps_to_the_width_and_a_long_text_scrolls() {
    let words = vec!["squares"; 200].join(" ");
    let (mut screen, _) = markdown(r#"bind="notes""#, |h| h.inner.set("notes", json!(words)), 400, 300);
    let runs = part(&screen).runs();
    let mut lines: Vec<i64> = runs.iter().map(|r| r.place.y as i64).collect();
    lines.dedup();
    assert!(lines.len() > 3);
    let width = part(&screen).node().bounds.width;
    for run in &runs {
        assert!(run.place.right() <= width + 0.5, "{} beyond {width}", run.place);
    }
    assert!(part(&screen).max_scroll_y() > 0.0);
    let b = part(&screen).node().bounds;
    screen.wheel(b.x + 10.0, b.y + 10.0, 0.0, 40.0, Modifiers::default());
    assert_eq!(40.0, part(&screen).scroll_y());
    // It scrolls no further than its end.
    screen.wheel(b.x + 10.0, b.y + 10.0, 0.0, 1.0e6, Modifiers::default());
    assert_eq!(part(&screen).max_scroll_y(), part(&screen).scroll_y());
    let mut shot = Shot::of(&mut screen, 400, 300, 1.0);
    keep(&mut shot, "markdown-scrolled.png");
}

#[test]
fn a_link_opens_through_the_host_and_plain_text_does_not() {
    let (mut screen, host) = markdown(r#"bind="notes""#, |h| h.inner.set("notes", json!("See [the site](https://example.com) now.")), 800, 400);
    let click = |screen: &mut Screen, run: &MarkdownRun| {
        let page = part(screen).page();
        let (x, y) = (page.x + run.place.x + 2.0, page.y + run.place.y + run.place.height / 2.0);
        screen.pointer_pressed(x, y, PointerButton::Left, 1);
        screen.pointer_released(x, y, PointerButton::Left);
    };
    let runs = part(&screen).runs();
    click(&mut screen, runs.iter().find(|r| r.text.starts_with("See")).unwrap());
    assert!(host.inner.opened.borrow().is_empty());
    click(&mut screen, runs.iter().find(|r| r.link.is_some()).unwrap());
    assert_eq!(vec!["https://example.com".to_string()], host.inner.opened.borrow().clone());
}

#[test]
fn a_link_is_drawn_in_the_accent() {
    let (mut screen, _) = markdown(r#"bind="notes""#, |h| h.inner.set("notes", json!("[Rules](https://example.com)")), 800, 400);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let runs = part(&screen).runs();
    assert_eq!(1, runs.len());
    let run = &runs[0];
    let page = part(&screen).page();
    let accent = screen.look().color("accent");
    let mut found = false;
    for x in (page.x + run.place.x) as i32..(page.x + run.place.right()) as i32 {
        for y in (page.y + run.place.y) as i32..(page.y + run.place.bottom()) as i32 {
            found |= shot.pixel(x, y) == accent;
        }
    }
    assert!(found);
    keep(&mut shot, "markdown-link.png");
}
