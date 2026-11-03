//! A whole window: its parts in the window's grid inside the content
//! padding, scrolling with the wheel when the window is smaller than its
//! rows; the frame before the grid in Tab order; a disabled or hidden
//! button's shortcut presses nothing; a sidebar that fills the window and
//! gives it back takes the same room; markup that cannot be read says why
//! in the window.

use std::rc::Rc;

use clarity_ui::geometry::Rect;
use clarity_ui::grid::{GAP, ROW_HEIGHT};
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::markup;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, Shot, TestHost};
use clarity_ui::window::view::CONTENT_PADDING;
use clarity_ui::window::WindowView;
use clarity_ui::{Element, Screen, ThemeLook};

fn shown(markup_text: &str, width: f64, height: f64) -> (Screen, Rc<TestHost>) {
    let host = TestHost::new();
    let spec = markup::read(markup_text).expect("the window's markup reads");
    let as_host: Rc<dyn PartHost> = host.clone();
    let mut screen = Screen::new(Box::new(WindowView::new(&spec, as_host)), ThemeLook::clarity(false));
    screen.resize(width, height, 1.0);
    Shot::of(&mut screen, width as i32, height as i32, 1.0);
    (screen, host)
}

fn view(screen: &Screen) -> &WindowView {
    screen.root().as_any().downcast_ref::<WindowView>().unwrap()
}

fn view_mut(screen: &mut Screen) -> &mut WindowView {
    screen.root_mut().as_any_mut().downcast_mut::<WindowView>().unwrap()
}

#[test]
fn a_menu_item_s_key_runs_its_handler_while_the_menu_is_closed() {
    let (mut screen, host) = shown(
        r#"<Window name="Keys"><Button name="more" at="1,1" span="3" caption="More" split="true"><Menu><Item caption="Print" key="Ctrl+P" onclick="print" /></Menu></Button></Window>"#,
        600.0,
        300.0,
    );
    press(&mut screen, Key::P, Modifiers::CONTROL);
    assert_eq!(vec!["run more click print"], host.logged());
}

#[test]
fn a_modal_overlay_over_the_window_keeps_its_keys() {
    let (mut screen, host) = shown(r#"<Window name="Keys"><Button name="ok" at="1,1" span="3" caption="OK" default="true" onclick="ok" /></Window>"#, 600.0, 300.0);
    screen.open(clarity_ui::Overlay { modal: true, ..clarity_ui::Overlay::new(Box::new(clarity_ui::element::Plate::new("panel", None)), Rect::new(0.0, 0.0, 100.0, 100.0)) });
    press(&mut screen, Key::Enter, Modifiers::NONE);
    assert!(host.logged().is_empty(), "{:?}", host.logged());
}

fn press(screen: &mut Screen, key: Key, held: Modifiers) {
    screen.key_pressed(KeyInput::new(key, held));
}

const ORDERS: &str = r#"
    <Window name="Orders">
      <Label name="noteLabel" at="1,1" span="6" caption="Note" />
      <Button name="approveButton" at="7,1" span="6" caption="Approve" onclick="approveClicked" />
      <Textbox name="remark" at="1,2" span="12" />
    </Window>
"#;

#[test]
fn its_parts_are_in_a_grid_of_twelve_inside_the_content_padding() {
    let (screen, _) = shown(ORDERS, 800.0, 400.0);
    let parts = view(&screen).parts();
    assert_eq!(parts.len(), 3);
    let column = (800.0 - 2.0 * CONTENT_PADDING - 11.0 * GAP) / 12.0;
    let first = parts[0].node().bounds;
    assert_eq!((first.x, first.y, first.height), (12.0, 12.0, ROW_HEIGHT));
    assert!((first.width - (6.0 * column + 5.0 * GAP)).abs() < 1e-6, "{first:?}");
    assert!((parts[1].node().bounds.x - (12.0 + 6.0 * (column + GAP))).abs() < 1e-6);
    assert_eq!(parts[2].node().bounds.y, 12.0 + ROW_HEIGHT + GAP);
}

#[test]
fn a_window_that_names_its_columns_has_a_grid_of_them() {
    assert_eq!(markup::read(r#"<Window name="A" />"#).unwrap().columns, 12);
    assert_eq!(markup::read(r#"<Window name="A" span="4" />"#).unwrap().columns, 4);
    let (screen, _) = shown(r#"<Window name="A" span="4"><Button name="b" at="1,1" span="2" /></Window>"#, 424.0, 400.0);
    let width = view(&screen).parts()[0].node().bounds.width;
    assert!((width - ((400.0 - 3.0 * GAP) / 2.0 + GAP)).abs() < 1e-6, "{width}");
}

#[test]
fn a_window_smaller_than_its_rows_scrolls_with_the_wheel() {
    let (mut screen, _) = shown(r#"<Window name="A"><Button name="low" at="1,20" span="2" /></Window>"#, 800.0, 300.0);
    let content = 20.0 * ROW_HEIGHT + 19.0 * GAP;
    assert_eq!(view(&screen).max_scroll_y, content - (300.0 - 2.0 * CONTENT_PADDING));
    let top = view(&screen).parts()[0].node().bounds.y;
    screen.wheel(100.0, 100.0, 0.0, 40.0, Modifiers::NONE);
    Shot::of(&mut screen, 800, 300, 1.0);
    assert_eq!(view(&screen).scroll_y, 40.0);
    assert_eq!(view(&screen).parts()[0].node().bounds.y, top - 40.0);
    screen.wheel(100.0, 100.0, 0.0, -5000.0, Modifiers::NONE);
    assert_eq!(view(&screen).scroll_y, 0.0);
    let mut cx = clarity_ui::Cx::alone();
    assert!(!view_mut(&mut screen).on_wheel(0.0, -40.0, Modifiers::NONE, &mut cx));
}

#[test]
fn a_window_that_fits_does_not_scroll() {
    let (mut screen, _) = shown(ORDERS, 800.0, 400.0);
    assert_eq!(view(&screen).max_scroll_y, 0.0);
    let mut cx = clarity_ui::Cx::alone();
    assert!(!view_mut(&mut screen).on_wheel(0.0, 40.0, Modifiers::NONE, &mut cx));
}

#[test]
fn markup_that_cannot_be_read_says_why_in_the_window() {
    let host: Rc<dyn PartHost> = TestHost::new();
    let view = WindowView::failed("The markup could not be read".into(), host);
    assert!(view.problem.is_some());
    assert!(view.parts().is_empty());
}

#[test]
fn a_disabled_or_hidden_buttons_shortcut_presses_nothing() {
    let (mut screen, host) = shown(
        r#"<Window name="Keys">
            <Textbox name="note" at="1,1" span="6" />
            <Button name="save" at="7,1" span="3" caption="Save" key="Ctrl+S" onclick="save" />
        </Window>"#,
        800.0,
        400.0,
    );
    let note = part_named(&screen, "note");
    let editor = screen.tab_order().into_iter().find(|id| clarity_ui::element::chain(screen.root(), *id).is_some_and(|c| c.contains(&note))).unwrap();
    screen.focus(Some(editor));
    host.set_state("save", |s| s.enabled = Some(false));
    press(&mut screen, Key::S, Modifiers::CONTROL);
    assert!(host.logged().is_empty());
    host.set_state("save", |s| {
        s.enabled = Some(true);
        s.visible = Some(false);
    });
    press(&mut screen, Key::S, Modifiers::CONTROL);
    assert!(host.logged().is_empty());
    host.set_state("save", |s| s.visible = Some(true));
    press(&mut screen, Key::S, Modifiers::CONTROL);
    assert_eq!(host.logged(), vec!["run save click save"]);
}

#[test]
fn a_sidebar_that_fills_the_window_and_gives_it_back_takes_the_same_room() {
    let (mut screen, _) = shown(
        r#"<Window name="Game"><Sidebar side="left" name="help" width="320" collapsible="true" maximizable="true"><Text name="rules" at="1,1" span="12" /></Sidebar><Button name="go" at="1,1" span="12" /></Window>"#,
        697.0,
        510.0,
    );
    assert_eq!(view(&screen).sidebars_width(), 320.0);
    let body = view(&screen).body_place();
    screen.act_root::<WindowView, _>(|v, cx| v.sidebar_mut(true).unwrap().fill(true, cx));
    Shot::of(&mut screen, 697, 510, 1.0);
    assert_eq!(view(&screen).sidebars_width(), 320.0);
    screen.act_root::<WindowView, _>(|v, cx| v.sidebar_mut(true).unwrap().fill(false, cx));
    Shot::of(&mut screen, 697, 510, 1.0);
    assert_eq!(view(&screen).sidebars_width(), 320.0);
    assert_eq!(view(&screen).body_place(), body);
    assert_eq!(body, Rect::new(320.0, 0.0, 377.0, 510.0));
}
