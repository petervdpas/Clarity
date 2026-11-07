//! A splitter, as the C# SplitterPartTests check it: panes side by side or
//! one above the other, `pane span n` giving the first of two n of the
//! part's columns, a divider the person drags to share the room anew (the
//! pointer a resize arrow over it), and a pane that scrolls when its parts
//! go further down than its room. Also the scroll box, whose grid scrolls in
//! its own place.

use clarity_ui::markup::PageSpec;
use clarity_ui::parts::containers::container::is_inside;
use clarity_ui::input::Cursor;
use clarity_ui::parts::containers::container;
use clarity_ui::parts::containers::scroll::ScrollPart;
use clarity_ui::parts::containers::splitter::SplitterPart;
use clarity_ui::testing::{near, window_of, Shot, TestHost};
use clarity_ui::{Element, Modifiers, Rect, Screen};

const SPLITTER: &str = r#"
    <Splitter name="split" at="1,1" span="12" rows="6">
      <Pane span="4"><Label at="1,1" span="4" caption="Left" /></Pane>
      <Pane><Label at="1,1" span="8" caption="Right" /></Pane>
    </Splitter>"#;

fn window(markup: &str) -> Screen {
    let mut screen = window_of(markup, TestHost::new());
    screen.resize(808.0, 400.0, 1.0);
    screen
}

fn splitter(screen: &Screen) -> &SplitterPart {
    clarity_ui::element::find_type::<SplitterPart>(screen.root(), |_| true).expect("a splitter")
}

fn page(columns: Option<i64>) -> PageSpec {
    PageSpec { kind: "pane".to_string(), title: None, columns, grid: None, children: Vec::new(), item: None }
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn a_pane_span_gives_the_first_pane_its_columns_and_the_second_the_rest() {
    let screen = window(SPLITTER);
    let part = splitter(&screen);
    assert_eq!(vec![4, 8], part.panes().iter().map(|p| p.columns).collect::<Vec<_>>());
    assert!((part.shares()[0] - 1.0 / 3.0).abs() < 1e-6);
    // The first pane is a third of the room beside the divider.
    let divider = part.dividers()[0].node().bounds;
    assert!((divider.x - (808.0 - SplitterPart::DIVIDER_THICKNESS) / 3.0).abs() < 1e-3, "divider at {}", divider.x);
}

#[test]
fn without_a_span_or_one_above_the_other_each_pane_has_all_the_columns() {
    assert_eq!(vec![12, 12], SplitterPart::pane_columns(&[page(None), page(None)], 12, false));
    assert_eq!(vec![12, 12], SplitterPart::pane_columns(&[page(Some(4)), page(None)], 12, true));
    assert_eq!(vec![12, 12, 12], SplitterPart::pane_columns(&[page(Some(4)), page(None), page(None)], 12, false));
    let screen = window(&SPLITTER.replace(r#"rows="6""#, r#"rows="6" orientation="vertical""#));
    let part = splitter(&screen);
    assert!(part.vertical());
    assert!(part.panes()[1].node().bounds.y > part.panes()[0].node().bounds.bottom());
}

#[test]
fn dragging_the_divider_shares_the_room_anew() {
    let mut screen = window(SPLITTER);
    let divider = splitter(&screen).dividers()[0].node().bounds;
    assert_eq!(Some(splitter(&screen).dividers()[0].node().id()), screen.at(divider.x + 2.0, 50.0));
    screen.pointer_pressed(divider.x + 2.0, 50.0, clarity_ui::PointerButton::Left, 1);
    screen.pointer_moved(404.0 + 2.0, 50.0);
    screen.pointer_released(404.0 + 2.0, 50.0, clarity_ui::PointerButton::Left);
    screen.resize(808.0, 400.0, 1.0);
    let part = splitter(&screen);
    assert!((part.dividers()[0].node().bounds.x - 404.0).abs() < 1e-3);
    assert!((part.shares()[0] - 0.5).abs() < 0.01);
    // No pane gets less than a tenth.
    screen.pointer_pressed(406.0, 50.0, clarity_ui::PointerButton::Left, 1);
    screen.pointer_moved(0.0, 50.0);
    screen.pointer_released(0.0, 50.0, clarity_ui::PointerButton::Left);
    screen.resize(808.0, 400.0, 1.0);
    let part = splitter(&screen);
    assert!((part.shares()[0] - 0.1).abs() < 1e-6);
    assert!(part.panes()[0].node().bounds.x < part.dividers()[0].node().bounds.x);
}

#[test]
fn a_disabled_splitter_keeps_its_shares() {
    let host = TestHost::new();
    host.set_state("split", |s| s.enabled = Some(false));
    let mut screen = window_of(SPLITTER, host);
    screen.resize(808.0, 400.0, 1.0);
    let divider = splitter(&screen).dividers()[0].node().bounds;
    screen.pointer_pressed(divider.x + 2.0, 50.0, clarity_ui::PointerButton::Left, 1);
    screen.pointer_moved(600.0, 50.0);
    screen.pointer_released(600.0, 50.0, clarity_ui::PointerButton::Left);
    assert!((splitter(&screen).shares()[0] - 1.0 / 3.0).abs() < 1e-6);
}

#[test]
fn a_pane_whose_parts_go_further_than_its_room_scrolls_so_none_is_out_of_reach() {
    let rows: String = (1..=8).map(|r| format!(r#"<Label at="1,{r}" span="8" caption="Row {r}" />"#)).collect();
    let markup = format!(r#"<Splitter name="split" at="1,1" span="12" rows="3"><Pane span="4"><Label at="1,1" span="4" /></Pane><Pane>{rows}</Pane></Splitter>"#);
    let mut screen = window(&markup);
    // A place lower than the splitter asks for, as in a window too low for it.
    let id = splitter(&screen).node().id();
    let low = Rect::new(0.0, 0.0, 808.0, 124.0);
    screen.act(id, |e, cx| e.arrange(low, cx));
    let part = splitter(&screen);
    let view = &part.views()[1];
    assert!(view.max_scroll_y() > 0.0);
    assert_eq!(0.0, part.views()[0].max_scroll_y());
    let vb = view.node().bounds;
    let last = part.panes()[1].node().children().last().unwrap().node().bounds;
    assert!(last.bottom() > vb.bottom());
    screen.wheel(vb.x + 20.0, vb.y + 20.0, 0.0, 1000.0, Modifiers::NONE);
    let part = splitter(&screen);
    let view = &part.views()[1];
    assert_eq!(view.max_scroll_y(), view.scroll_y());
    let last = part.panes()[1].node().children().last().unwrap().node().bounds;
    assert!(last.bottom() <= view.node().bounds.bottom() + 0.5);
}

#[test]
fn a_splitter_asks_room_for_its_padding_so_a_pane_of_its_own_rows_needs_no_scrolling() {
    let rows: String = (1..=3).map(|r| format!(r#"<Label at="1,{r}" span="8" />"#)).collect();
    let mut screen = window(&format!(r#"<Splitter at="1,1" span="12" rows="3"><Pane span="4"><Label at="1,1" span="4" /></Pane><Pane>{rows}</Pane></Splitter>"#));
    screen.resize(808.0, 600.0, 1.0);
    let part = splitter(&screen);
    assert!(part.around() > 0.0);
    assert_eq!(0.0, part.views()[1].max_scroll_y());
    let grid = screen.root().as_any().downcast_ref::<clarity_ui::element::GridPanel>().unwrap();
    assert!(grid.content >= 3.0 * 36.0 + 2.0 * 8.0 + part.around());
}

#[test]
fn it_draws_its_border_and_divider() {
    let mut screen = window(SPLITTER);
    let strong = screen.look().color("strong_border");
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    keep(&mut shot, "splitter.png");
    // Once it is drawn, nothing frames what is drawn next.
    assert!(!is_inside());
    let part = splitter(&screen);
    let d = part.dividers()[0].node().bounds;
    assert!(near(strong, shot.pixel((d.x + d.width / 2.0) as i32, (d.y + d.height / 2.0) as i32), 10));
    let b = part.node().bounds;
    assert!(near(strong, shot.pixel((b.x + b.width / 2.0) as i32, b.y as i32), 40));
}

#[test]
fn a_scroll_boxs_grid_scrolls_when_its_parts_go_further_than_its_rows() {
    let rows: String = (1..=10).map(|r| format!(r#"<Label at="1,{r}" span="3" />"#)).collect();
    let mut screen = window(&format!(r#"<Scroll at="1,1" span="6" rows="3">{rows}</Scroll>"#));
    let scroll = clarity_ui::element::find_type::<ScrollPart>(screen.root(), |_| true).expect("a scroll box");
    assert_eq!(10.0 * 36.0 + 9.0 * 8.0 - 124.0, scroll.view().max_scroll_y());
    assert_eq!(6, scroll.inner().columns);
    screen.wheel(20.0, 20.0, 0.0, 50.0, Modifiers::NONE);
    let scroll = clarity_ui::element::find_type::<ScrollPart>(screen.root(), |_| true).unwrap();
    let first = &scroll.inner().node().children()[0];
    assert_eq!(-50.0, first.node().bounds.y);
    assert!(container::is_inside_part(first.as_part().unwrap()));
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    keep(&mut shot, "containers-scroll.png");
}

#[test]
fn the_pointer_over_a_divider_is_the_resize_arrow_of_its_way() {
    let mut screen = window(SPLITTER);
    let d = splitter(&screen).dividers()[0].node().bounds;
    screen.pointer_moved(d.x + 2.0, 50.0);
    assert_eq!(Cursor::ResizeLeftRight, screen.cursor());
    screen.pointer_moved(d.x + 100.0, 50.0);
    assert_eq!(Cursor::Default, screen.cursor());
    let mut screen = window(&SPLITTER.replace(r#"rows="6""#, r#"rows="6" orientation="vertical""#));
    let d = splitter(&screen).dividers()[0].node().bounds;
    screen.pointer_moved(d.x + 50.0, d.y + 2.0);
    assert_eq!(Cursor::ResizeUpDown, screen.cursor());
}
