//! What the screen does for every window, each shown as the person meets it:
//! the frame comes before the window's grid in Tab order wherever it is
//! docked; a part that fails to take its place or to draw says why in its
//! place while its neighbours go on; changing an accordion or choosing in a
//! list that only selects makes no edit; a scroll box keeps room for its
//! bar; the long call bar sits at the bottom right; a context menu greys its
//! disabled entries; a designer's tooltip shows over the inner pieces of a
//! part; and a key Clarity.UI has no Key for is matched by its own name.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use clarity_ui::dialogs::long_call::LongCallBar;
use clarity_ui::element::{self, Cx, Element, GridPanel, Node};
use clarity_ui::element_node;
use clarity_ui::frame::buttons::FrameButton;
use clarity_ui::frame::FrameMenuList;
use clarity_ui::geometry::Rect;
use clarity_ui::grid::GridItem;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::markup;
use clarity_ui::painter::Painter;
use clarity_ui::parts::containers::accordion::AccordionPart;
use clarity_ui::parts::containers::scroll::ScrollPart;
use clarity_ui::parts::containers::scroll_view::BAR_THICKNESS;
use clarity_ui::parts::custom::{self, CustomCx, CustomPart};
use clarity_ui::parts::data::list::ListPart;
use clarity_ui::parts::inputs::number::NumberPart;
use clarity_ui::parts::PartHost;
use clarity_ui::screen::TIP_PAUSE;
use clarity_ui::shortcuts;
use clarity_ui::testing::{part_name, part_named, window_of, Shot, TestHost};
use clarity_ui::window::WindowView;
use clarity_ui::{Screen, ThemeLook};
use serde_json::{json, Value};

fn click(screen: &mut Screen, at: Rect, button: PointerButton) {
    let (x, y) = (at.x + at.width / 2.0, at.y + at.height / 2.0);
    screen.pointer_pressed(x, y, button, 1);
    screen.pointer_released(x, y, button);
}

fn windowed(markup: &str, host: Rc<TestHost>, width: f64, height: f64) -> Screen {
    let spec = markup::read(&format!("<Window name=\"Check\">{markup}</Window>")).expect("the markup reads");
    let as_host: Rc<dyn PartHost> = host;
    let mut screen = Screen::new(Box::new(WindowView::new(&spec, as_host)), ThemeLook::clarity(false));
    screen.resize(width, height, 1.0);
    screen
}

fn shown(markup: &str, host: Rc<TestHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

/// What Tab order calls an element: a frame button's item name, a part's name.
fn name_of(e: &dyn Element) -> String {
    if let Some(button) = e.as_any().downcast_ref::<FrameButton>() {
        return button.item().text("name").unwrap_or("").to_string();
    }
    part_name(e).map(str::to_string).unwrap_or_else(|| e.type_name().to_string())
}

#[test]
fn the_frame_comes_before_the_windows_grid_in_tab_order_wherever_it_is_docked() {
    // The sidebar is on the right, the grid's button at the top left: the frame still comes first.
    let mut screen = windowed(
        r#"<Toolbar name="tools"><Item name="save" caption="Save" onclick="save" /></Toolbar>
            <Sidebar side="right" name="side" width="220" columns="4">
              <Button name="find" at="1,1" span="4" caption="Find" onclick="find" />
            </Sidebar>
            <Button name="inGrid" at="1,1" span="3" caption="In the grid" onclick="here" />"#,
        TestHost::new(),
        1000.0,
        600.0,
    );
    Shot::of(&mut screen, 1000, 600, 1.0);
    let order: Vec<String> = screen.tab_order().into_iter().filter_map(|id| screen.element(id)).map(name_of).collect();
    assert_eq!(order, vec!["save", "find", "inGrid"]);
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(screen.focused().and_then(|f| screen.element(f)).map(name_of).as_deref(), Some("save"));
}

/// An element that cannot take its place.
struct Unplaceable {
    node: Node,
}

impl Element for Unplaceable {
    element_node!();

    fn arrange(&mut self, _area: Rect, _cx: &mut Cx) {
        panic!("no room today");
    }
}

/// A library's part whose drawing fails.
struct UndrawableGauge;

impl CustomPart for UndrawableGauge {
    fn set(&mut self, _name: &str, _value: &Value, _cx: &mut CustomCx) {}

    fn hear(&mut self, _hear: &str, _values: &[Value], _cx: &mut CustomCx) {}

    fn draw(&self, _painter: &Painter, _area: Rect) {
        panic!("no paint today");
    }
}

fn has_error(shot: &mut Shot, place: Rect, error: skia_safe::Color) -> bool {
    let y = (place.y + place.height / 2.0) as i32;
    // The error frame's left edge, from a pixel before the place where there is room for one.
    let from = (place.x as i32 - 1).max(0);
    (from..from + 4).any(|x| shot.pixel(x, y) == error)
}

#[test]
fn a_part_that_fails_to_take_its_place_or_to_draw_says_why_and_the_rest_of_the_window_goes_on() {
    custom::register("UndrawableGauge", || Box::new(UndrawableGauge));
    let host = TestHost::new();
    let spec = markup::read(
        r#"<Window name="Check">
            <Label name="beside" at="7,1" span="6" caption="Beside" />
            <Custom name="dial" at="1,3" span="6" rows="2" part="UndrawableGauge" />
            <Button name="go" at="7,3" span="3" caption="Go" onclick="go" />
        </Window>"#,
    )
    .expect("the markup reads");
    let mut grid = GridPanel::new(12);
    let gauge = Unplaceable { node: Node::new() };
    let gauge_id = gauge.node.id();
    grid.add(Box::new(gauge), GridItem::new(1, 1, 6, 2, false));
    let as_host: Rc<dyn PartHost> = host.clone();
    for part in spec.parts {
        let made = clarity_ui::parts::make(part, as_host.clone());
        let place = clarity_ui::parts::place_of(made.as_ref());
        grid.add(made, place);
    }
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let error = screen.look().color("error");
    let gauge = screen.element(gauge_id).unwrap();
    assert_eq!(gauge.node().failed().as_deref(), Some("no room today"));
    let gauge_place = gauge.node().bounds;
    assert!(gauge_place.width > 0.0);
    assert!(has_error(&mut shot, gauge_place, error), "the part that could not take its place says so in it");
    let dial = part_named(&screen, "dial");
    let dial_place = screen.element(dial).unwrap().node().bounds;
    let failed = element::in_order(screen.element(dial).unwrap()).into_iter().find_map(|e| e.node().failed());
    assert_eq!(failed.as_deref(), Some("no paint today"));
    assert!(has_error(&mut shot, dial_place, error), "the part whose drawing failed says so in it");
    // The parts beside them took their places and work.
    let beside = screen.element(part_named(&screen, "beside")).unwrap().node().bounds;
    assert!(beside.x >= gauge_place.right() && beside.width > 0.0);
    let go = screen.element(part_named(&screen, "go")).unwrap().node().bounds;
    click(&mut screen, go, PointerButton::Left);
    assert_eq!(host.logged(), vec!["run go click go"]);
}

#[test]
fn changing_an_accordion_or_choosing_in_a_list_that_only_selects_is_no_edit() {
    let host = TestHost::new();
    host.set("people", json!([{ "name": "Ann" }, { "name": "Bob" }]));
    host.binds.borrow_mut().insert("peopleList".into(), "people".into());
    let mut screen = shown(
        r#"<Accordion name="fold" at="1,1" span="6" rows="4" onchange="folded">
                <Section title="One" /><Section title="Two" />
              </Accordion>
              <List name="peopleList" at="7,1" span="6" rows="4" bind="people" show="name" onselect="picked" />"#,
        host.clone(),
    );
    Shot::of(&mut screen, 800, 500, 1.0);
    let fold = part_named(&screen, "fold");
    let header = screen.element(fold).unwrap().as_any().downcast_ref::<AccordionPart>().unwrap().headers()[1].node().bounds;
    click(&mut screen, header, PointerButton::Left);
    assert_eq!(host.logged(), vec!["run fold change folded"]);
    let list = part_named(&screen, "peopleList");
    let row = screen.element(list).unwrap().as_any().downcast_ref::<ListPart>().unwrap().row_box(1).expect("a second row");
    click(&mut screen, row, PointerButton::Left);
    assert!(host.logged().iter().any(|l| l.contains("picked")), "{:?}", host.logged());
    // Neither wrote a value nor made an edit of the part's own state.
    assert!(!host.logged().iter().any(|l| l.starts_with("write")), "{:?}", host.logged());
    assert!(host.edits.borrow().is_empty(), "{:?}", host.edits.borrow());
}

#[test]
fn a_scroll_box_keeps_room_for_its_bar_beside_its_parts() {
    let many: String = (1..=10).map(|i| format!(r#"<Button at="1,{i}" span="6" caption="Row {i}" onclick="row" />"#)).collect();
    let mut screen = shown(
        &format!(r#"<Scroll name="full" at="1,1" span="6" rows="3">{many}</Scroll><Scroll name="fits" at="7,1" span="6" rows="3"><Button at="1,1" span="6" caption="One" onclick="one" /></Scroll>"#),
        TestHost::new(),
    );
    Shot::of(&mut screen, 800, 400, 1.0);
    let scroll = |name: &str| screen.element(part_named(&screen, name)).unwrap().as_any().downcast_ref::<ScrollPart>().unwrap();
    let (full, fits) = (scroll("full"), scroll("fits"));
    let full_button = full.inner().node().children()[0].node().bounds;
    let fits_button = fits.inner().node().children()[0].node().bounds;
    let view = full.view().node().bounds;
    assert!(full_button.right() <= view.right() - BAR_THICKNESS, "{} leaves the bar its room in {}", full_button.right(), view.right());
    // Where nothing scrolls the parts have the whole width.
    assert!(fits_button.width - full_button.width >= BAR_THICKNESS);
}

#[test]
fn the_long_call_bar_shows_at_the_bottom_right_of_the_window() {
    let said = Arc::new(Mutex::new(Some((30, 120))));
    let bar = LongCallBar::new(said, Arc::new(AtomicBool::new(false)));
    let mut screen = Screen::new(Box::new(bar), ThemeLook::clarity(false));
    Shot::of(&mut screen, 1000, 700, 1.0);
    let bar = screen.root().as_any().downcast_ref::<LongCallBar>().unwrap();
    let note = bar.note();
    let (right, below) = (1000.0 - note.right(), 700.0 - note.bottom());
    assert!((0.0..=40.0).contains(&right), "right {right}");
    assert!((0.0..=40.0).contains(&below), "below {below}");
    assert_eq!(right, below);
    assert!(note.x > 500.0 && note.y > 500.0);
    // Stop sits inside it, on its right.
    let stop = bar.stop_button().node().bounds;
    assert!(stop.right() <= note.right() && stop.x > note.x + note.width / 2.0);
}

#[test]
fn a_context_menu_greys_its_disabled_entries() {
    let host = TestHost::new();
    host.set("note", json!("Hello"));
    let mut screen = shown(r#"<Textbox name="note" at="1,1" span="6" bind="note" />"#, host);
    Shot::of(&mut screen, 800, 400, 1.0);
    // Nothing is selected: Cut and Copy are disabled, Paste and Select all are not.
    let place = screen.element(part_named(&screen, "note")).unwrap().node().bounds;
    click(&mut screen, place, PointerButton::Right);
    let lists = screen.context_menus().lists();
    assert_eq!(lists.len(), 1);
    let list_id = lists[0];
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let list = screen.element(list_id).unwrap().as_any().downcast_ref::<FrameMenuList>().unwrap();
    let sum = |c: skia_safe::Color| c.r() as i32 + c.g() as i32 + c.b() as i32;
    let mut darkest = |at: usize| {
        let row = list.row_of(at);
        let mut darkest = i32::MAX;
        for x in row.x as i32 + 26..row.x as i32 + 90 {
            for y in row.y as i32 + 2..row.bottom() as i32 - 2 {
                darkest = darkest.min(sum(shot.pixel(x, y)));
            }
        }
        darkest
    };
    let muted = sum(screen.look().color("muted"));
    let (cut, copy, paste) = (darkest(0), darkest(1), darkest(2));
    // An enabled entry's caption is in the text color; a disabled one is dimmed, lighter than muted text.
    assert!(paste < muted, "Paste {paste} is darker than muted {muted}");
    assert!(cut > muted + 60, "Cut {cut} is greyed beyond muted {muted}");
    assert!(copy > muted + 60, "Copy {copy} is greyed beyond muted {muted}");
}

#[test]
fn a_designers_tooltip_shows_over_the_inner_pieces_of_a_part() {
    let host = TestHost::new();
    host.set("post.title", json!("Hi"));
    host.set("post.count", json!(3));
    host.set("post.color", json!("#336699"));
    let mut screen = shown(
        r#"<Textbox name="title" at="1,1" span="6" bind="post.title" tooltip="Title tip" />
            <Spinbox name="count" at="1,2" span="6" bind="post.count" min="0" max="9" tooltip="Count tip" />
            <Color name="color" at="1,4" span="6" bind="post.color" tooltip="Color tip" />"#,
        host,
    );
    Shot::of(&mut screen, 800, 400, 1.0);
    let now = Rc::new(RefCell::new(Instant::now()));
    let clock = now.clone();
    screen.now = Box::new(move || *clock.borrow());
    let tip_at = |screen: &mut Screen, place: Rect| {
        screen.pointer_moved(place.x + place.width / 2.0, place.y + place.height / 2.0);
        *now.borrow_mut() += TIP_PAUSE;
        let at = *now.borrow();
        screen.wake_due(at);
        screen.tip().map(|t| t.text.clone())
    };
    // Where a part's typing is: its editor's place, at the left of the part (beside any arrows).
    let editor_in = |screen: &Screen, name: &str| {
        let place = screen.element(part_named(screen, name)).unwrap().node().bounds;
        Rect::new(place.x, place.y, place.width / 2.0, place.height)
    };
    let title = editor_in(&screen, "title");
    assert_eq!(tip_at(&mut screen, title).as_deref(), Some("Title tip"));
    let up = screen.element(part_named(&screen, "count")).unwrap().as_any().downcast_ref::<NumberPart>().unwrap().up().expect("an up arrow").bounds;
    assert_eq!(tip_at(&mut screen, up).as_deref(), Some("Count tip"));
    let count = editor_in(&screen, "count");
    assert_eq!(tip_at(&mut screen, count).as_deref(), Some("Count tip"));
    let swatch = screen.element(part_named(&screen, "color")).unwrap().as_any().downcast_ref::<clarity_ui::parts::inputs::color::ColorPart>().unwrap().swatch();
    assert_eq!(tip_at(&mut screen, swatch).as_deref(), Some("Color tip"));
}

#[test]
fn a_key_clarity_ui_has_no_key_for_is_matched_by_its_own_name() {
    let f13 = KeyInput::named("F13", Modifiers::CONTROL);
    assert!(shortcuts::matches("Ctrl+F13", &f13, false));
    assert!(!shortcuts::matches("Ctrl+F14", &f13, false));
    assert!(!shortcuts::matches("F13", &f13, false));
    // A button with such a key is pressed by it.
    let host = TestHost::new();
    let mut screen = windowed(r#"<Button name="mark" at="1,1" span="3" caption="Mark" key="Ctrl+F13" onclick="mark" />"#, host.clone(), 800.0, 400.0);
    assert!(screen.key_pressed(f13));
    assert_eq!(host.logged(), vec!["run mark click mark"]);
}

#[test]
fn a_digit_key_is_pressed_once_and_its_character_does_not_press_it_again() {
    let host = TestHost::new();
    let mut screen = windowed(r#"<Button name="seven" at="1,1" span="3" caption="7" key="7" onclick="seven" />"#, host.clone(), 800.0, 400.0);
    for _ in 0..2 {
        screen.key_pressed(KeyInput::plain(Key::D7));
        // The character the key typed does not press it again.
        screen.text_typed("7");
    }
    assert_eq!(host.logged(), vec!["run seven click seven", "run seven click seven"]);
}
