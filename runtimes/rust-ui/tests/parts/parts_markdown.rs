//! The markdown part, as the C# MarkdownPartTests check it: Markdown read
//! into blocks the same way on every stack (conformance/markdown.json),
//! shown as headings, paragraphs, list items and code, wrapped to its width
//! and scrolling when long; its text a resource file by name, the text a key
//! gives, or the text it binds; a link opening through the host.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::input::{Modifiers, PointerButton};
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::containers::markdown::blocks::{self, Block, BlockKind, Span};
use clarity_ui::parts::containers::markdown::{MarkdownPart, MarkdownRun, BAR_ROOM};
use clarity_ui::parts::markdown_memo::{MarkdownMemoPart, SWITCH_HEIGHT};
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
        BlockKind::Quote => json!({ "kind": "quote", "depth": block.depth, "spans": spans }),
        BlockKind::Rule => json!({ "kind": "rule" }),
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

/// A book's text reads like a page: room around it, headings by their
/// level with space around them, quotes set in with a bar, a scene break
/// centred, line breaks kept, and lines no longer than a reading width.
#[test]
fn it_reads_like_a_page() {
    let text = "# One\n\n## Two\n\n### Three\n\nA line,  \nand the next.\n\n> Quoted words.\n\n***\n\nAfter.";
    let (mut screen, _) = markdown(r#"bind="notes""#, |h| h.inner.set("notes", json!(text)), 800, 600);
    let runs = part(&screen).runs();
    let find = |start: &str| -> MarkdownRun { runs.iter().find(|r| r.text.starts_with(start)).cloned().unwrap_or_else(|| panic!("no run {start}")) };
    let (one, two, three, plain) = (find("One"), find("Two"), find("Three"), find("A "));
    assert!(one.size > two.size && two.size > three.size && three.size > plain.size, "{} {} {} {}", one.size, two.size, three.size, plain.size);
    assert_eq!("heading", one.font, "headings take the theme's heading font");
    assert!(plain.place.x >= 8.0 && one.place.y >= 8.0, "room around the text: {:?}", plain.place);
    assert!(two.place.y - one.place.bottom() > 6.0, "space between headings");
    assert!(find("and").place.y > plain.place.y, "a hard line break starts a new line");
    let quoted = find("Quoted");
    assert!(quoted.place.x > plain.place.x, "a quote is set in");
    assert_eq!(1, part(&screen).quote_bars().len());
    let rules = part(&screen).rules();
    assert_eq!(1, rules.len());
    let page = part(&screen).node().bounds.width;
    let centre = rules[0].x + rules[0].width / 2.0;
    assert!(rules[0].width < page / 2.0 && (centre - plain.place.x - (part(&screen).column_width()) / 2.0).abs() < 2.0, "a scene break is centred: {:?}", rules[0]);
    let mut shot = Shot::of(&mut screen, 800, 600, 1.0);
    keep(&mut shot, "markdown-page.png");
    // Wide, the lines keep a reading width, from the left.
    let words = vec!["squares"; 300].join(" ");
    let (screen, _) = markdown(r#"bind="notes""#, |h| h.inner.set("notes", json!(words)), 2400, 600);
    let runs = part(&screen).runs();
    let left = runs.iter().map(|r| r.place.x).fold(f64::MAX, f64::min);
    let right = runs.iter().map(|r| r.place.right()).fold(0.0, f64::max);
    let width = part(&screen).node().bounds.width;
    assert!(right - left < width * 0.7, "a reading width: {left}..{right} of {width}");
    assert!(left < 20.0, "from the left: {left}..{right} of {width}");
    let _ = BAR_ROOM;
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

// -- A memo of Markdown ------------------------------------------------------------

/// `memo ... markdown`: Write and Preview on its top; what is written shows
/// in the preview as a page, and Write brings the editor back.
#[test]
fn a_memo_of_markdown_writes_and_previews() {
    let host = Rc::new(Host::default());
    host.inner.set("notes", json!("# Start"));
    let mut screen = window_of(r#"<Memo name="manuscript" at="1,1" span="6" rows="6" bind="notes" markdown="true" />"#, host.clone());
    Shot::of(&mut screen, 800, 400, 1.0);
    let id = part_named(&screen, "manuscript");
    let get = |screen: &Screen| screen.element(id).and_then(|e| e.as_any().downcast_ref::<MarkdownMemoPart>()).map(|m| (m.previewing(), m.editor().editor().text(), m.editor().editor().markdown, m.page().runs().iter().map(|r| r.text.trim().to_string()).collect::<Vec<_>>())).expect("a markdown memo");
    let (previewing, text, marks, _) = get(&screen);
    assert!(!previewing && marks);
    assert_eq!("# Start", text);
    // Typed at its end, then Preview: the page shows what was typed.
    let b = screen.element(id).unwrap().node().bounds;
    screen.pointer_pressed(b.x + 200.0, b.y + SWITCH_HEIGHT + 10.0, PointerButton::Left, 1);
    screen.pointer_released(b.x + 200.0, b.y + SWITCH_HEIGHT + 10.0, PointerButton::Left);
    screen.key_pressed(clarity_ui::KeyInput::plain(clarity_ui::Key::End));
    screen.text_typed(" here");
    let (preview_x, write_x) = (b.x + 90.0, b.x + 20.0);
    screen.pointer_pressed(preview_x, b.y + SWITCH_HEIGHT / 2.0, PointerButton::Left, 1);
    screen.pointer_released(preview_x, b.y + SWITCH_HEIGHT / 2.0, PointerButton::Left);
    Shot::of(&mut screen, 800, 400, 1.0);
    let (previewing, _, _, runs) = get(&screen);
    assert!(previewing, "Preview shows the page");
    assert_eq!(vec!["Start".to_string(), "here".to_string()], runs);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "markdown-memo-preview.png");
    screen.pointer_pressed(write_x, b.y + SWITCH_HEIGHT / 2.0, PointerButton::Left, 1);
    screen.pointer_released(write_x, b.y + SWITCH_HEIGHT / 2.0, PointerButton::Left);
    assert!(!get(&screen).0, "Write brings the editor back");
}

/// A link into the application (`clarity:fragment/412`) opens no browser: its
/// address goes in the part's values as `link`, and `on link` runs.
#[test]
fn a_link_into_the_application_runs_on_link() {
    let (mut screen, host) = markdown(r#"bind="notes" onlink="linkFollowed""#, |h| h.inner.set("notes", json!("As [the stable said](clarity:fragment/412), it rained.")), 800, 400);
    let runs = part(&screen).runs();
    let run = runs.iter().find(|r| r.link.as_deref() == Some("clarity:fragment/412")).cloned().expect("the link shows as a link");
    let page = part(&screen).page();
    let (x, y) = (page.x + run.place.x + 2.0, page.y + run.place.y + run.place.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    assert!(host.inner.opened.borrow().is_empty(), "no browser opens");
    let link = host.inner.state(Some("rules")).and_then(|s| s.values).and_then(|v| v.get("link").cloned());
    assert_eq!(Some(json!("clarity:fragment/412")), link);
    assert!(host.inner.logged().iter().any(|l| l.contains("link linkFollowed")), "{:?}", host.inner.logged());
}
