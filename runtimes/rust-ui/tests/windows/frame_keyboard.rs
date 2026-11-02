//! A window's frame by keyboard and pointer, as Avalonia had it: Tab reaches
//! the ribbon's tab strip (Left and Right open its tabs) and on into the
//! buttons of the open tab; toolbar and ribbon buttons take the focus, show
//! the focus ring and answer Enter and Space; the sidebar's fold and fill
//! buttons too, and they say what they do. Folding a sidebar leaves whether
//! it fills the window as it was; a double click anywhere on a ribbon but a
//! button folds it; the sidebar's edge shows the resize pointer. F10 or Alt
//! alone gives the menu bar the focus: Left and Right walk its menus, Down
//! opens one, Escape leaves it.

use super::core_frame_shared as shared;

use clarity_ui::frame::buttons::FrameButton;
use clarity_ui::frame::ribbon::{self, RibbonTabStrip};
use clarity_ui::frame::rules;
use clarity_ui::frame::sidebar::SidebarButton;
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Cursor, Key, KeyInput, Modifiers};
use clarity_ui::testing::near;
use clarity_ui::window::WindowView;
use clarity_ui::{Element, ElementId, Screen};
use shared::*;

const RIBBON: &str = r#"
    <Ribbon name="mainRibbon" collapsible="true">
      <Tab title="Home" name="home">
        <Group title="Clipboard" name="clip">
          <Item name="paste" caption="Paste" large="true" onclick="paste" />
          <Item name="cut" caption="Cut" onclick="cut" />
        </Group>
      </Tab>
      <Tab title="Insert" name="insert">
        <Group title="Pictures"><Item caption="Picture" onclick="picture" /></Group>
      </Tab>
      <Tab title="View" name="view">
        <Group title="Zoom"><Item caption="Zoom" onclick="zoom" /></Group>
      </Tab>
    </Ribbon>
"#;

const TOOLBAR: &str = r#"
    <Toolbar name="tools">
      <Item name="save" caption="Save" onclick="save" />
      <Item kind="separator" />
      <Item name="print" caption="Print" onclick="print" />
      <Item name="users" caption="Users" onclick="users" />
      <Item name="mail" caption="Mail" onclick="mail" />
    </Toolbar>
"#;

const NAV: &str = r#"
    <Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" maximizable="true" columns="4">
      <Button name="find" at="1,1" span="4" caption="Find" onclick="find" />
    </Sidebar>
"#;

const MENU: &str = r#"
    <Menu name="mainMenu">
      <Item caption="File"><Item caption="Open" onclick="open" /><Item caption="Recent"><Item caption="Shop.db" onclick="recent" /></Item></Item>
      <Item name="edit" caption="Edit"><Item caption="Undo" onclick="undo" /></Item>
      <Item caption="View"><Item caption="Zoom" onclick="zoom" /></Item>
      <Item caption="Help" onclick="help" />
    </Menu>
"#;

/// What an element of the frame is called in these tests: a button by its caption, the strip of tabs `tabs`, a sidebar's buttons `fill` and `fold`, a part by its name.
fn caption(screen: &Screen, id: Option<ElementId>) -> String {
    let Some(e) = id.and_then(|i| screen.element(i)) else { return "none".into() };
    let any = e.as_any();
    if let Some(button) = any.downcast_ref::<FrameButton>() {
        return button.item().text("caption").unwrap_or("").to_string();
    }
    if any.is::<RibbonTabStrip>() {
        return "tabs".into();
    }
    if let Some(button) = any.downcast_ref::<SidebarButton>() {
        return if button.fills() { "fill" } else { "fold" }.into();
    }
    match e.as_part() {
        Some(part) => part.base().spec.name().unwrap_or("").to_string(),
        None => "other".into(),
    }
}

fn focused(screen: &Screen) -> String {
    caption(screen, screen.focused())
}

fn tab_order(screen: &Screen) -> Vec<String> {
    screen.tab_order().into_iter().map(|id| caption(screen, Some(id))).collect()
}

fn key(screen: &mut Screen, key: Key) {
    press(screen, key, Modifiers::NONE);
}

fn shown_tab(screen: &Screen) -> Option<usize> {
    view(screen).ribbon().unwrap().shown_tab()
}

#[test]
fn the_ribbons_tab_strip_takes_the_focus_and_the_arrows_open_its_tabs() {
    let (mut screen, host) = window(RIBBON, None);
    draw(&mut screen);
    key(&mut screen, Key::Tab);
    assert_eq!(focused(&screen), "tabs");
    key(&mut screen, Key::Right);
    assert_eq!(host.state_of("mainRibbon").tab, Some(2));
    // A hidden tab is passed over.
    host.set_state("insert", |s| s.visible = Some(false));
    draw(&mut screen);
    key(&mut screen, Key::Left);
    assert_eq!(shown_tab(&screen), Some(1));
    key(&mut screen, Key::Right);
    assert_eq!(shown_tab(&screen), Some(3));
    // The last tab stays the last.
    key(&mut screen, Key::Right);
    assert_eq!(shown_tab(&screen), Some(3));
    key(&mut screen, Key::Left);
    draw(&mut screen);
    // Tab goes on into the open tab's buttons.
    key(&mut screen, Key::Tab);
    assert_eq!(focused(&screen), "Paste");
    key(&mut screen, Key::Tab);
    assert_eq!(focused(&screen), "Cut");
    key(&mut screen, Key::Enter);
    assert_eq!(host.log(), vec!["run cut click cut"]);
}

#[test]
fn toolbar_buttons_are_reached_with_tab_and_pressed_with_enter_and_space() {
    let (mut screen, host) = window(TOOLBAR, None);
    host.set_state("users", |s| s.enabled = Some(false));
    host.set_state("print", |s| s.visible = Some(false));
    draw(&mut screen);
    assert_eq!(tab_order(&screen), vec!["Save", "Mail"]);
    key(&mut screen, Key::Tab);
    assert_eq!(focused(&screen), "Save");
    key(&mut screen, Key::Enter);
    key(&mut screen, Key::Tab);
    key(&mut screen, Key::Space);
    assert_eq!(host.log(), vec!["run save click save", "run mail click mail"]);
}

#[test]
fn a_focused_frame_button_shows_the_focus_ring_in_the_themes_color() {
    let (mut screen, _) = window(TOOLBAR, None);
    draw(&mut screen);
    let place = view(&screen).toolbar().unwrap().placed()[0].1;
    let (x, y) = (place.x as i32, (place.y + place.height / 2.0) as i32);
    let focus = screen.look().color("focus");
    let before = draw(&mut screen).pixel(x, y);
    assert!(!near(focus, before, 20), "{before:?} is near {focus:?} before the focus");
    key(&mut screen, Key::Tab);
    let after = draw(&mut screen).pixel(x, y);
    assert!(near(focus, after, 20), "{after:?} is not near {focus:?}");
}

#[test]
fn the_sidebars_buttons_are_reached_by_keyboard_and_say_what_they_do() {
    let (mut screen, host) = window(NAV, None);
    draw(&mut screen);
    // As Avalonia's header: the fill button, then the fold button, then the sidebar's parts.
    assert_eq!(tab_order(&screen), vec!["fill", "fold", "find"]);
    let tip = |screen: &Screen, fills: bool| {
        let nav = view(screen).sidebar(true).unwrap();
        let place = if fills { nav.fill_button() } else { nav.fold_button() }.unwrap();
        nav.buttons().into_iter().find(|b| b.fills() == fills).unwrap().tooltip_at(place.x + 2.0, place.y + 2.0).unwrap()
    };
    assert_eq!(tip(&screen, true), "Fill the window");
    assert_eq!(tip(&screen, false), "Fold the sidebar away");
    key(&mut screen, Key::Tab);
    key(&mut screen, Key::Enter);
    assert_eq!(host.state_of("nav").maximized, Some(true));
    draw(&mut screen);
    assert_eq!(tip(&screen, true), "Give the window back");
    key(&mut screen, Key::Space);
    assert_eq!(host.state_of("nav").maximized, Some(false));
    key(&mut screen, Key::Tab);
    key(&mut screen, Key::Enter);
    assert_eq!(host.state_of("nav").collapsed, Some(true));
    draw(&mut screen);
    assert_eq!(tip(&screen, false), "Show the sidebar");
    // The screen's tooltip finds it too.
    let fold = view(&screen).sidebar(true).unwrap().fold_button().unwrap();
    let at = screen.at(fold.x + 2.0, fold.y + 2.0).unwrap();
    assert_eq!(screen.element(at).unwrap().tooltip_at(fold.x + 2.0, fold.y + 2.0).as_deref(), Some("Show the sidebar"));
}

#[test]
fn folding_a_sidebar_leaves_whether_it_fills_the_window_as_it_was() {
    let (mut screen, host) = window(NAV, None);
    draw(&mut screen);
    screen.act_root::<WindowView, _>(|v, cx| {
        let nav = v.sidebar_mut(true).unwrap();
        nav.fill(true, cx);
        nav.fold(true, cx);
    });
    assert_eq!(host.state_of("nav").maximized, Some(true));
    assert!(!view(&screen).sidebar(true).unwrap().maximized());
    draw(&mut screen);
    assert_eq!(view(&screen).sidebar(true).unwrap().node().bounds.width, rules::STRIP_WIDTH);
    screen.act_root::<WindowView, _>(|v, cx| v.sidebar_mut(true).unwrap().fold(false, cx));
    draw(&mut screen);
    assert!(view(&screen).sidebar(true).unwrap().maximized());
    assert_eq!(view(&screen).sidebar(true).unwrap().node().bounds.width, 1000.0);
}

#[test]
fn an_unnamed_sidebar_folded_and_shown_again_fills_the_window_again() {
    let (mut screen, _) = window(r#"<Sidebar side="right" width="300" collapsible="true" maximizable="true" />"#, None);
    draw(&mut screen);
    screen.act_root::<WindowView, _>(|v, cx| {
        let side = v.sidebar_mut(false).unwrap();
        side.fill(true, cx);
        side.fold(true, cx);
        side.fold(false, cx);
    });
    draw(&mut screen);
    assert_eq!(view(&screen).sidebar(false).unwrap().node().bounds.width, 1000.0);
}

#[test]
fn a_double_click_anywhere_on_the_ribbon_folds_it_but_on_a_button_only_presses_it() {
    let (mut screen, host) = window(RIBBON, None);
    draw(&mut screen);
    // On a button: it runs, the ribbon stays open.
    let cut = view(&screen).ribbon().unwrap().buttons()[1].1;
    click_at(&mut screen, cut, 2);
    assert_eq!(host.log(), vec!["run cut click cut"]);
    assert!(!view(&screen).ribbon().unwrap().collapsed());
    // On the groups, away from the buttons: it folds.
    let b = view(&screen).ribbon().unwrap().node().bounds;
    click_at(&mut screen, Rect::new(b.right() - 40.0, b.y + ribbon::TABS_HEIGHT + 30.0, 10.0, 10.0), 2);
    assert_eq!(host.state_of("mainRibbon").collapsed, Some(true));
    draw(&mut screen);
    // On the strip beside the tabs: it unfolds.
    click_at(&mut screen, Rect::new(b.right() - 40.0, b.y + 5.0, 10.0, 10.0), 2);
    assert_eq!(host.state_of("mainRibbon").collapsed, Some(false));
}

#[test]
fn the_sidebars_edge_shows_the_resize_pointer() {
    let (mut screen, _) = window(NAV, None);
    draw(&mut screen);
    let nav = view(&screen).sidebar(true).unwrap();
    let edge = nav.edge().unwrap();
    let (x, y) = (edge.x + edge.width / 2.0, 300.0);
    assert_eq!(nav.cursor_at(x, y), Some(Cursor::ResizeLeftRight));
    assert_eq!(nav.cursor_at(nav.node().bounds.x + 40.0, y), None);
    screen.pointer_moved(x, y);
    assert_eq!(screen.cursor(), Cursor::ResizeLeftRight);
    screen.act_root::<WindowView, _>(|v, cx| v.sidebar_mut(true).unwrap().fold(true, cx));
    draw(&mut screen);
    let nav = view(&screen).sidebar(true).unwrap();
    assert_eq!(nav.cursor_at(nav.node().bounds.right() - 2.0, y), None);
}

fn bar_id(screen: &Screen) -> ElementId {
    view(screen).menu_bar().unwrap().node().id()
}

fn marked(screen: &Screen) -> Option<String> {
    view(screen).menu_bar().unwrap().marked().and_then(|i| i.text("caption")).map(str::to_string)
}

fn focus_bar(screen: &mut Screen) -> bool {
    screen.act_root::<WindowView, _>(|v, cx| v.focus_menu_bar(cx)).unwrap()
}

#[test]
fn the_menu_bar_takes_the_focus_and_the_arrows_walk_its_menus() {
    let (mut screen, host) = window(&format!(r#"{MENU}<Button name="b" at="1,1" span="3" caption="B" onclick="b" />"#), None);
    draw(&mut screen);
    let button = clarity_ui::testing::part_named(&screen, "b");
    screen.focus(Some(button));
    host.set_state("edit", |s| s.enabled = Some(false));
    assert!(focus_bar(&mut screen));
    assert_eq!(screen.focused(), Some(bar_id(&screen)));
    assert_eq!(marked(&screen).as_deref(), Some("File"));
    // Right and Left move the marked item, past the disabled one, round the ends.
    key(&mut screen, Key::Right);
    assert_eq!(marked(&screen).as_deref(), Some("View"));
    key(&mut screen, Key::Right);
    key(&mut screen, Key::Right);
    assert_eq!(marked(&screen).as_deref(), Some("File"));
    key(&mut screen, Key::Left);
    assert_eq!(marked(&screen).as_deref(), Some("Help"));
    // Escape leaves the bar, the focus back where it was.
    key(&mut screen, Key::Escape);
    assert_eq!(screen.focused(), Some(button));
    assert_eq!(marked(&screen), None);
    // Down opens the marked menu with its first item marked.
    focus_bar(&mut screen);
    key(&mut screen, Key::Down);
    let file = lists(&screen);
    assert_eq!(file.len(), 1);
    assert_eq!((captions(file[0])[0].as_str(), file[0].marked()), ("Open", Some(0)));
    assert_eq!(screen.focused(), Some(file[0].node().id()));
    // In the open menu, Right goes to the next top menu, Left back.
    key(&mut screen, Key::Right);
    let view_menu = lists(&screen);
    assert_eq!(view_menu.len(), 1);
    assert_eq!(captions(view_menu[0]), vec!["Zoom"]);
    assert_eq!(view_menu[0].marked(), Some(0));
    key(&mut screen, Key::Left);
    assert_eq!(captions(lists(&screen)[0])[0], "Open");
    // Right on an item with a sub menu opens that instead.
    key(&mut screen, Key::Down);
    key(&mut screen, Key::Right);
    assert_eq!(lists(&screen).len(), 2);
    key(&mut screen, Key::Left);
    // Left from the first menu goes round to Help, which has no menu: the bar takes the focus with it marked.
    key(&mut screen, Key::Left);
    assert!(lists(&screen).is_empty());
    assert_eq!(screen.focused(), Some(bar_id(&screen)));
    assert_eq!(marked(&screen).as_deref(), Some("Help"));
    // Enter on an item without a menu runs it and leaves the bar.
    key(&mut screen, Key::Enter);
    assert_eq!(host.log(), vec!["run  click help"]);
    assert_eq!(screen.focused(), Some(button));
}

#[test]
fn f10_and_alt_alone_give_the_menu_bar_the_focus() {
    let (mut screen, _) = window(MENU, None);
    draw(&mut screen);
    key(&mut screen, Key::F10);
    assert_eq!(screen.focused(), Some(bar_id(&screen)));
    assert_eq!(marked(&screen).as_deref(), Some("File"));
    key(&mut screen, Key::Escape);
    assert_eq!(screen.focused(), None);
    screen.key_pressed(KeyInput::new(Key::Alt, Modifiers::ALT));
    screen.key_released(KeyInput::new(Key::Alt, Modifiers::NONE));
    assert_eq!(screen.focused(), Some(bar_id(&screen)));
    assert_eq!(marked(&screen).as_deref(), Some("File"));
}

#[test]
fn a_menu_bar_that_shows_nothing_does_not_take_the_focus() {
    let (mut screen, host) = window(MENU, None);
    host.set_state("mainMenu", |s| s.visible = Some(false));
    draw(&mut screen);
    assert!(!focus_bar(&mut screen));
    assert_eq!(screen.focused(), None);
    // Tab does not reach a menu bar that is not in use.
    host.set_state("mainMenu", |s| s.visible = Some(true));
    draw(&mut screen);
    assert!(!screen.tab_order().contains(&bar_id(&screen)));
}
