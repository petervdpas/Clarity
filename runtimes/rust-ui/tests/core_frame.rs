//! The frame around a window: its regions read from the window's markup and
//! its app's, each of the window's own replacing the app's, placed around
//! the window's parts; the menu bar with its menus and sub menus, the
//! `language` and `theme` targets picking in place; the toolbar; the frame's
//! keys (designed or the person's) after the window's own buttons'; and a
//! dialog showing no app frame.

#[path = "core_frame_shared.rs"]
mod shared;

use std::cell::RefCell;
use std::rc::Rc;

use clarity_ui::frame::{self, rules, FramePicker, FrameSpec, PickerChoice, ShownFrame};
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, Modifiers, PointerButton};
use clarity_ui::window::view::CONTENT_PADDING;
use clarity_ui::window::WindowView;
use clarity_ui::xml;
use clarity_ui::AppFiles;
use clarity_ui::Element;
use shared::*;

const APP: &str = r#"
    <Menu name="appMenu"><Item caption="App" onclick="fromApp" /></Menu>
    <Toolbar name="appTools"><Item caption="Print" onclick="print" /></Toolbar>
    <Footer status="Ready" columns="12" />
"#;

const MENU: &str = r#"
    <Menu name="mainMenu">
      <Item caption="File">
        <Item caption="Open" key="Ctrl+O" onclick="open" />
        <Item kind="separator" />
        <Item caption="Recent">
          <Item caption="Shop.db" onclick="openRecent" />
        </Item>
        <Item name="wrap" caption="Wrap lines" onclick="toggleWrap" />
        <Item name="customers" caption="Customers" opens="Customers" />
      </Item>
      <Item caption="View">
        <Item caption="Language" opens="language" />
        <Item caption="Theme" opens="theme" />
      </Item>
      <Item caption="Help" onclick="help" />
    </Menu>
"#;

const TOOLBAR: &str = r#"
    <Toolbar name="tools">
      <Item name="save" caption="Save" icon="save" key="Ctrl+S" onclick="save" />
      <Item kind="separator" />
      <Item name="users" caption="Users" opens="Users" />
      <Item caption="Language" opens="language" />
    </Toolbar>
"#;

fn regions(markup: &str) -> Vec<xml::XmlElement> {
    xml::parse(markup).expect("the markup reads").children
}

fn top(screen: &clarity_ui::Screen, caption: &str) -> Rect {
    view(screen).menu_bar().expect("a menu bar").place_of(caption).expect("a top item")
}

fn open(screen: &mut clarity_ui::Screen, caption: &str) {
    draw(screen);
    let at = top(screen, caption);
    click(screen, at);
}

fn picker(chosen: Rc<RefCell<Vec<String>>>, offered: &[(&str, &str, bool)]) -> FramePicker {
    let offered: Vec<PickerChoice> = offered.iter().map(|(v, l, s)| PickerChoice { value: v.to_string(), label: l.to_string(), shown: *s }).collect();
    FramePicker::new(move || offered.clone(), move |v| chosen.borrow_mut().push(v.to_string()))
}

// -- Reading and placing -------------------------------------------------------

#[test]
fn a_frame_is_read_from_its_regions() {
    let frame = FrameSpec::read(&regions(
        r#"<App name="Shop">
          <Menu name="mainMenu"><Item caption="File"><Item caption="Open" key="Ctrl+O" onclick="open" /><Item kind="separator" /></Item></Menu>
          <Toolbar name="tools"><Item caption="Save" icon="save" onclick="save" /></Toolbar>
          <Ribbon name="mainRibbon" collapsible="true" collapsed="true">
            <Tab title="Home" name="home"><Group title="Clipboard" name="clip"><Item caption="Paste" large="true" onclick="paste" /></Group></Tab>
          </Ribbon>
          <Sidebar side="right" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" maximizable="true" columns="4">
            <Label at="1,1" span="4" caption="Filters" />
          </Sidebar>
          <Footer status="@ready" columns="6" />
        </App>"#,
    ));
    let menu = frame.menu.as_ref().unwrap();
    assert_eq!(menu.name.as_deref(), Some("mainMenu"));
    assert_eq!(menu.items[0].items[0].text("shortcut"), Some("Ctrl+O"));
    assert!(rules::is_separator(&menu.items[0].items[1]));
    assert_eq!(frame.toolbar.as_ref().unwrap().items[0].text("icon"), Some("save"));
    let ribbon = frame.ribbon.as_ref().unwrap();
    assert!(ribbon.collapsible && ribbon.collapsed);
    assert_eq!((ribbon.tabs[0].title.as_str(), ribbon.tabs[0].groups[0].title.as_str()), ("Home", "Clipboard"));
    assert!(ribbon.tabs[0].groups[0].items[0].flag("large"));
    assert!(frame.left.is_none());
    let nav = frame.right.as_ref().unwrap();
    assert_eq!((nav.name.as_deref(), nav.width, nav.min, nav.max, nav.columns), (Some("nav"), 280.0, Some(200.0), Some(480.0), 4));
    assert!(nav.resizable && nav.collapsible && nav.maximizable && !nav.collapsed);
    assert_eq!(nav.parts[0].kind, "label");
    let footer = frame.footer.as_ref().unwrap();
    assert_eq!((footer.status.as_deref(), footer.columns), (Some("@ready"), 6));
}

#[test]
fn a_sidebar_without_its_settings_is_a_new_sidebars() {
    let frame = FrameSpec::read(&regions(r#"<Window><Sidebar side="left" /></Window>"#));
    let left = frame.left.unwrap();
    assert_eq!((left.width, left.columns, left.min), (240.0, 3, None));
}

#[test]
fn a_windows_own_region_replaces_the_apps() {
    let app = FrameSpec::read(&regions(&format!("<App>{APP}</App>")));
    let own = FrameSpec::read(&regions(r#"<Window><Menu name="mine"><Item caption="Mine" /></Menu></Window>"#));
    let shown = ShownFrame::of(Some(&app), &own);
    let menu = shown.menu.as_ref().unwrap();
    assert_eq!((menu.value.name.as_deref(), menu.inherited), (Some("mine"), false));
    let toolbar = shown.toolbar.as_ref().unwrap();
    assert_eq!((toolbar.value.name.as_deref(), toolbar.inherited), (Some("appTools"), true));
    assert!(shown.footer.as_ref().unwrap().inherited);
    assert!(shown.ribbon.is_none());
    // A dialog shows no app frame.
    let alone = ShownFrame::of(None, &own);
    assert_eq!(alone.menu.unwrap().value.name.as_deref(), Some("mine"));
    assert!(alone.toolbar.is_none());
}

#[test]
fn the_windows_parts_sit_between_the_regions() {
    let (mut screen, _) = window(r#"<Button name="save" at="1,1" span="3" caption="Save" />"#, Some(APP));
    draw(&mut screen);
    let view = view(&screen);
    assert_eq!(view.menu_bar().unwrap().node().bounds, Rect::new(0.0, 0.0, 1000.0, frame::menu_bar::HEIGHT));
    assert_eq!(view.toolbar().unwrap().node().bounds, Rect::new(0.0, frame::menu_bar::HEIGHT, 1000.0, frame::toolbar::HEIGHT));
    assert_eq!(view.footer().unwrap().node().bounds.y, 600.0 - frame::footer::STATUS_HEIGHT);
    let tops = frame::menu_bar::HEIGHT + frame::toolbar::HEIGHT;
    assert_eq!(view.body_place(), Rect::new(0.0, tops, 1000.0, 600.0 - tops - frame::footer::STATUS_HEIGHT));
    // The window's padding is inside the frame.
    assert_eq!(view.parts()[0].node().bounds.y, tops + CONTENT_PADDING);
}

#[test]
fn a_window_without_a_frame_keeps_its_whole_place() {
    let (mut screen, _) = window(r#"<Button name="save" at="1,1" span="3" caption="Save" />"#, None);
    draw(&mut screen);
    let view = view(&screen);
    assert!(view.frame().shown().is_empty());
    assert_eq!(view.body_place(), Rect::new(0.0, 0.0, 1000.0, 600.0));
    assert_eq!(view.parts()[0].node().bounds.y, CONTENT_PADDING);
}

#[test]
fn the_windows_own_button_hears_its_key_before_the_frame() {
    let (mut screen, host) = window(
        r#"<Button name="mine" at="1,1" span="3" caption="Mine" key="Ctrl+N" onclick="windowNew" />"#,
        Some(r#"<Toolbar><Item caption="New" key="Ctrl+N" onclick="frameNew" /></Toolbar>"#),
    );
    press(&mut screen, Key::N, Modifiers::CONTROL);
    assert_eq!(host.log(), vec!["run mine click windowNew"]);
}

#[test]
fn a_frame_draws_in_light_and_dark() {
    let (mut screen, host) = window(
        r#"<Sidebar side="left" name="nav" collapsible="true" maximizable="true"><Label at="1,1" span="3" caption="Nav" /></Sidebar>"#,
        Some(&format!(r#"{APP}<Ribbon name="r"><Tab title="Home"><Group title="Edit"><Item caption="Cut" large="true" icon="scissors" onclick="cut" /></Group></Tab></Ribbon>"#)),
    );
    host.icons.borrow_mut().insert("scissors".into(), "M6 6l12 12M6 18L18 6".into());
    let mut light = draw(&mut screen);
    light.save(shot_path("core-frame-light")).unwrap();
    assert_eq!(light.pixel(2, 2), screen.look().color("frame"));
    screen.show(clarity_ui::ThemeLook::clarity(true));
    let mut dark = draw(&mut screen);
    dark.save(shot_path("core-frame-dark")).unwrap();
    assert_eq!(dark.pixel(2, 2), screen.look().color("frame"));
}

#[test]
fn the_app_frame_is_given_after_the_window_is_made() {
    let (mut screen, host) = window("", None);
    assert!(view(&screen).frame().shown().is_empty());
    let app = regions(&format!("<App name=\"Shop\">{APP}</App>"));
    screen.act_root::<WindowView, _>(|v, cx| v.set_app_frame(Some(&app), cx));
    draw(&mut screen);
    let at = view(&screen).toolbar().unwrap().place_of("Print").unwrap();
    click(&mut screen, at);
    assert_eq!(host.log(), vec!["run  click print"]);
}

#[test]
fn a_dialog_shows_no_app_frame() {
    const MODEL: &str = r#"{"project": {"files": [{"pages": [{"name": {"name": "Main"}, "kind": "window"}, {"name": {"name": "Ask"}, "kind": "dialog"}]}]}}"#;
    const SHOP: &str = r#"<App name="Shop"><Menu><Item caption="File" /></Menu></App>"#;
    let files = AppFiles { apps: &[("Shop", SHOP)], model: MODEL, ..AppFiles::default() };
    assert_eq!(frame::app_frame_of(&files, "Main").map(|r| r.len()), Some(1));
    assert!(frame::app_frame_of(&files, "Ask").is_none());
    assert!(frame::app_frame_of(&AppFiles::default(), "Main").is_none());
}

// -- The menu bar --------------------------------------------------------------

#[test]
fn a_press_opens_a_menu_below_its_caption_and_another_closes_it() {
    let (mut screen, _) = window(MENU, None);
    open(&mut screen, "File");
    assert_eq!(screen.overlays().len(), 1);
    let list = lists(&screen)[0];
    assert_eq!(list.node().bounds.y, top(&screen, "File").bottom() + 1.0);
    assert_eq!(captions(list), vec!["Open", "", "Recent", "Wrap lines", "Customers"]);
    assert!(list.entries()[1].separator);
    assert_eq!(list.entries()[0].key.as_deref(), Some("Ctrl+O"));
    draw(&mut screen).save(shot_path("core-frame-menu")).unwrap();
    let file = top(&screen, "File");
    click(&mut screen, file);
    assert!(screen.overlays().is_empty());
}

#[test]
fn an_item_runs_its_handler_and_the_menu_closes() {
    let (mut screen, host) = window(MENU, None);
    open(&mut screen, "File");
    let at = row(lists(&screen)[0], "Open");
    click(&mut screen, at);
    assert_eq!(host.log(), vec!["run  click open"]);
    assert!(screen.overlays().is_empty());
    assert!(!view(&screen).frame().menus().is_open());
}

#[test]
fn an_item_opens_the_window_it_targets() {
    let (mut screen, host) = window(MENU, None);
    open(&mut screen, "File");
    let at = row(lists(&screen)[0], "Customers");
    click(&mut screen, at);
    assert_eq!(host.log(), vec!["run customers click Customers"]);
}

#[test]
fn a_top_item_without_items_runs_as_a_click() {
    let (mut screen, host) = window(MENU, None);
    open(&mut screen, "Help");
    assert_eq!(host.log(), vec!["run  click help"]);
    assert!(screen.overlays().is_empty());
}

#[test]
fn a_sub_menu_opens_beside_its_item() {
    let (mut screen, host) = window(MENU, None);
    open(&mut screen, "File");
    let (recent, right) = {
        let list = lists(&screen)[0];
        (list.row_of(2), list.node().bounds.right())
    };
    screen.pointer_moved(recent.x + 10.0, recent.y + 5.0);
    draw(&mut screen).save(shot_path("core-frame-submenu")).unwrap();
    assert_eq!(lists(&screen).len(), 2);
    let sub = lists(&screen)[1];
    assert!(sub.node().bounds.x >= right - 4.0);
    let at = row(sub, "Shop.db");
    click(&mut screen, at);
    assert_eq!(host.log(), vec!["run  click openRecent"]);
    assert!(screen.overlays().is_empty());
}

#[test]
fn while_a_menu_is_open_the_pointer_over_another_opens_it() {
    let (mut screen, _) = window(MENU, None);
    open(&mut screen, "File");
    let at = top(&screen, "View");
    screen.pointer_moved(at.x + 5.0, at.y + 5.0);
    assert_eq!(screen.overlays().len(), 1);
    assert_eq!(captions(lists(&screen)[0]), vec!["Language", "Theme"]);
}

#[test]
fn the_arrow_keys_and_enter_pick_an_item_and_escape_closes() {
    let (mut screen, host) = window(MENU, None);
    open(&mut screen, "File");
    press(&mut screen, Key::Down, Modifiers::NONE);
    press(&mut screen, Key::Enter, Modifiers::NONE);
    assert_eq!(host.log(), vec!["run  click open"]);
    open(&mut screen, "File");
    press(&mut screen, Key::Escape, Modifiers::NONE);
    assert!(screen.overlays().is_empty());
}

#[test]
fn the_right_key_opens_a_sub_menu_with_its_first_item_marked_and_left_goes_back() {
    let (mut screen, host) = window(MENU, None);
    open(&mut screen, "File");
    for _ in 0..2 {
        press(&mut screen, Key::Down, Modifiers::NONE);
    }
    press(&mut screen, Key::Right, Modifiers::NONE);
    assert_eq!(lists(&screen).len(), 2);
    assert_eq!(lists(&screen)[1].marked(), Some(0));
    press(&mut screen, Key::Left, Modifiers::NONE);
    assert_eq!(lists(&screen).len(), 1);
    press(&mut screen, Key::Right, Modifiers::NONE);
    press(&mut screen, Key::Enter, Modifiers::NONE);
    assert_eq!(host.log(), vec!["run  click openRecent"]);
}

#[test]
fn a_press_outside_closes_every_menu() {
    let (mut screen, host) = window(MENU, None);
    open(&mut screen, "File");
    let recent = lists(&screen)[0].row_of(2);
    screen.pointer_moved(recent.x + 10.0, recent.y + 5.0);
    assert_eq!(screen.overlays().len(), 2);
    screen.pointer_pressed(900.0, 500.0, PointerButton::Left, 1);
    assert!(screen.overlays().is_empty());
    assert!(host.log().is_empty());
}

#[test]
fn a_named_item_shows_and_takes_clicks_as_its_state_says() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/overlays.json")).unwrap();
    for case in cases["item"].as_array().unwrap() {
        let (mut screen, host) = window(MENU, None);
        let part = &case["part"];
        host.set_state("wrap", |s| {
            s.visible = part["visible"].as_bool();
            s.enabled = part["enabled"].as_bool();
        });
        open(&mut screen, "File");
        let at = lists(&screen)[0].entries().iter().position(|e| e.caption == "Wrap lines");
        assert_eq!(at.is_some(), case["shown"].as_bool().unwrap(), "{case}");
        if let Some(at) = at {
            let place = lists(&screen)[0].row_of(at);
            click(&mut screen, place);
        }
        assert_eq!(host.log().contains(&"run wrap click toggleWrap".to_string()), case["clicks"].as_bool().unwrap(), "{case}");
    }
}

#[test]
fn a_checked_item_shows_its_check_mark() {
    let (mut screen, host) = window(MENU, None);
    host.set_state("wrap", |s| s.checked = Some(true));
    open(&mut screen, "File");
    let list = lists(&screen)[0];
    assert!(list.entries().iter().find(|e| e.caption == "Wrap lines").unwrap().checked);
    assert!(!list.entries()[0].checked);
}

#[test]
fn the_language_item_picks_in_place_the_one_shown_checked() {
    let (mut screen, host) = window(MENU, None);
    let chosen = Rc::new(RefCell::new(Vec::new()));
    view(&screen).frame().set_language(picker(chosen.clone(), &[("en", "English", true), ("nl", "Nederlands", false)]));
    open(&mut screen, "View");
    let language = lists(&screen)[0].row_of(0);
    screen.pointer_moved(language.x + 10.0, language.y + 5.0);
    draw(&mut screen).save(shot_path("core-frame-picker")).unwrap();
    let choices = lists(&screen)[1];
    let shown: Vec<(String, bool)> = choices.entries().iter().map(|e| (e.caption.clone(), e.checked)).collect();
    assert_eq!(shown, vec![("English".to_string(), true), ("Nederlands".to_string(), false)]);
    let at = row(choices, "Nederlands");
    click(&mut screen, at);
    assert_eq!(*chosen.borrow(), vec!["nl"]);
    assert!(screen.overlays().is_empty());
    assert!(host.log().is_empty());
}

#[test]
fn the_theme_item_picks_from_the_themes() {
    let (mut screen, _) = window(MENU, None);
    let chosen = Rc::new(RefCell::new(Vec::new()));
    view(&screen).frame().set_theme(picker(chosen.clone(), &[("clarity", "Clarity", true), ("night", "Night", false)]));
    open(&mut screen, "View");
    let theme = lists(&screen)[0].row_of(1);
    screen.pointer_moved(theme.x + 10.0, theme.y + 5.0);
    let at = row(lists(&screen)[1], "Night");
    click(&mut screen, at);
    assert_eq!(*chosen.borrow(), vec!["night"]);
}

#[test]
fn an_items_key_runs_it_while_its_menu_is_closed() {
    let (mut screen, host) = window(MENU, None);
    press(&mut screen, Key::O, Modifiers::CONTROL);
    assert_eq!(host.log(), vec!["run  click open"]);
}

#[test]
fn what_code_hid_or_disabled_does_not_answer_its_key() {
    let (mut screen, host) = window(
        r#"<Menu name="mainMenu"><Item caption="File">
            <Item name="open" caption="Open" key="Ctrl+O" onclick="open" />
            <Item name="save" caption="Save" key="Ctrl+S" onclick="save" />
        </Item></Menu>"#,
        None,
    );
    host.set_state("open", |s| s.enabled = Some(false));
    host.set_state("save", |s| s.visible = Some(false));
    press(&mut screen, Key::O, Modifiers::CONTROL);
    press(&mut screen, Key::S, Modifiers::CONTROL);
    host.set_state("save", |s| s.visible = Some(true));
    host.set_state("mainMenu", |s| s.visible = Some(false));
    press(&mut screen, Key::S, Modifiers::CONTROL);
    assert!(host.log().is_empty());
}

#[test]
fn a_hidden_menu_bar_takes_no_room_and_a_disabled_one_no_press() {
    let (mut screen, host) = window(MENU, None);
    host.set_state("mainMenu", |s| s.enabled = Some(false));
    open(&mut screen, "Help");
    assert!(host.log().is_empty());
    host.set_state("mainMenu", |s| s.visible = Some(false));
    draw(&mut screen);
    assert_eq!(view(&screen).body_place().y, 0.0);
}

#[test]
fn an_apps_menu_shows_in_a_window_without_its_own_and_its_keys_answer() {
    let (mut screen, host) = window("", Some(MENU));
    draw(&mut screen);
    assert!(view(&screen).menu_bar().is_some());
    assert!(view(&screen).frame().shown().menu.as_ref().unwrap().inherited);
    press(&mut screen, Key::O, Modifiers::CONTROL);
    assert_eq!(host.log(), vec!["run  click open"]);
}

#[test]
fn a_regions_written_empty_or_all_hidden_take_no_room() {
    for (region, item) in [
        (r#"<Menu name="m"><Item caption="File" name="file" /></Menu>"#, "file"),
        (r#"<Toolbar name="t"><Item caption="Print" name="print" onclick="print" /><Item kind="separator" /></Toolbar>"#, "print"),
        (r#"<Ribbon name="r"><Tab title="Home" name="home"><Group title="G"><Item caption="Paste" onclick="paste" /></Group></Tab></Ribbon>"#, "home"),
    ] {
        let (mut screen, host) = window(r#"<Button at="1,1" span="2" caption="Go" />"#, Some(region));
        draw(&mut screen);
        assert!(view(&screen).body_place().y > 0.0, "{region}");
        host.set_state(item, |s| s.visible = Some(false));
        draw(&mut screen);
        assert_eq!(view(&screen).body_place().y, 0.0, "{region}");
    }
    for region in [r#"<Menu name="m" />"#, r#"<Toolbar name="t" />"#, r#"<Ribbon name="r" />"#] {
        let (mut screen, _) = window(r#"<Button at="1,1" span="2" caption="Go" />"#, Some(region));
        draw(&mut screen);
        assert_eq!(view(&screen).body_place().y, 0.0, "{region}");
    }
}

// -- The toolbar ---------------------------------------------------------------

fn button(screen: &clarity_ui::Screen, caption: &str) -> Rect {
    view(screen).toolbar().unwrap().place_of(caption).unwrap()
}

#[test]
fn its_buttons_sit_in_a_row_and_run_when_clicked() {
    let (mut screen, host) = window(TOOLBAR, None);
    host.icons.borrow_mut().insert("save".into(), "M5 5h14v14H5z".into());
    draw(&mut screen).save(shot_path("core-frame-toolbar")).unwrap();
    let placed: Vec<Rect> = view(&screen).toolbar().unwrap().placed().into_iter().map(|(_, r)| r).collect();
    assert_eq!(placed.len(), 4);
    assert!(placed[1].x > placed[0].right() && placed[2].x > placed[1].right());
    let (save, users) = (button(&screen, "Save"), button(&screen, "Users"));
    click(&mut screen, save);
    click(&mut screen, users);
    assert_eq!(host.log(), vec!["run save click save", "run users click Users"]);
}

#[test]
fn the_language_button_opens_its_choices_below_it() {
    let (mut screen, host) = window(TOOLBAR, None);
    let chosen = Rc::new(RefCell::new(Vec::new()));
    view(&screen).frame().set_language(picker(chosen.clone(), &[("en", "English", false), ("de", "Deutsch", true)]));
    draw(&mut screen);
    let at = button(&screen, "Language");
    click(&mut screen, at);
    let list = lists(&screen)[0];
    assert_eq!(list.node().bounds.y, at.bottom() + 1.0);
    let shown: Vec<(String, bool)> = list.entries().iter().map(|e| (e.caption.clone(), e.checked)).collect();
    assert_eq!(shown, vec![("English".to_string(), false), ("Deutsch".to_string(), true)]);
    let first = list.row_of(0);
    click(&mut screen, first);
    assert_eq!(*chosen.borrow(), vec!["en"]);
    assert!(host.log().is_empty());
}

#[test]
fn a_hidden_button_leaves_the_row_and_a_disabled_one_takes_no_click() {
    let (mut screen, host) = window(TOOLBAR, None);
    host.set_state("users", |s| s.visible = Some(false));
    host.set_state("save", |s| s.enabled = Some(false));
    draw(&mut screen);
    assert!(view(&screen).toolbar().unwrap().place_of("Users").is_none());
    let save = button(&screen, "Save");
    click(&mut screen, save);
    press(&mut screen, Key::S, Modifiers::CONTROL);
    assert!(host.log().is_empty());
}

#[test]
fn a_disabled_toolbar_takes_no_click_and_a_hidden_one_no_room() {
    let (mut screen, host) = window(TOOLBAR, None);
    host.set_state("tools", |s| s.enabled = Some(false));
    draw(&mut screen);
    let save = button(&screen, "Save");
    click(&mut screen, save);
    press(&mut screen, Key::S, Modifiers::CONTROL);
    assert!(host.log().is_empty());
    host.set_state("tools", |s| s.visible = Some(false));
    draw(&mut screen);
    assert_eq!(view(&screen).body_place().y, 0.0);
}

#[test]
fn a_buttons_key_runs_it() {
    let (mut screen, host) = window(TOOLBAR, None);
    press(&mut screen, Key::S, Modifiers::CONTROL);
    assert_eq!(host.log(), vec!["run save click save"]);
}

#[test]
fn a_toolbar_and_a_ribbon_button_say_their_caption_and_keys() {
    let host = FrameHost::new();
    host.keys.borrow_mut().insert("Main.find".into(), vec!["F3".into()]);
    let mut screen = window_named(
        r#"<Toolbar name="tools"><Item name="print" caption="Print" key="Ctrl+P" onclick="print" /><Item name="find" caption="Find" key="Ctrl+F" onclick="find" /></Toolbar>
           <Ribbon name="r"><Tab title="Home" name="home"><Group title="G"><Item name="paste" caption="Paste" onclick="paste" /></Group></Tab></Ribbon>"#,
        None,
        host,
        "Main",
    );
    draw(&mut screen);
    let view = view(&screen);
    let tools = view.toolbar().unwrap();
    let print = tools.place_of("Print").unwrap();
    assert_eq!(tools.tooltip_at(print.x + 2.0, print.y + 2.0).as_deref(), Some("Print (Ctrl+P)"));
    let find = tools.place_of("Find").unwrap();
    assert_eq!(tools.tooltip_at(find.x + 2.0, find.y + 2.0).as_deref(), Some("Find (F3)"));
    let ribbon = view.ribbon().unwrap();
    let paste = ribbon.buttons()[0].1;
    assert_eq!(ribbon.tooltip_at(paste.x + 2.0, paste.y + 2.0).as_deref(), Some("Paste"));
    // The screen shows it after the pointer rests.
    let now = std::time::Instant::now();
    screen.now = Box::new(move || now);
    screen.pointer_moved(print.x + 2.0, print.y + 2.0);
    screen.wake_due(now + clarity_ui::screen::TIP_PAUSE);
    assert_eq!(screen.tip().map(|t| t.text.as_str()), Some("Print (Ctrl+P)"));
}

// -- The person's keys ---------------------------------------------------------

#[test]
fn a_buttons_key_set_by_the_person_runs_it_and_its_designed_key_no_more() {
    let host = FrameHost::new();
    host.keys.borrow_mut().insert("Card.saveButton".into(), vec!["Ctrl+K".into()]);
    let mut screen = window_named(r#"<Button at="1,1" span="2" name="saveButton" caption="Save" key="Ctrl+S" onclick="save" />"#, None, host.clone(), "Card");
    press(&mut screen, Key::S, Modifiers::CONTROL);
    assert!(host.log().is_empty());
    press(&mut screen, Key::K, Modifiers::CONTROL);
    assert_eq!(host.log(), vec!["run saveButton click save"]);
}

#[test]
fn a_frame_commands_key_is_the_persons_by_its_place() {
    let host = FrameHost::new();
    host.keys.borrow_mut().insert("print".into(), vec!["Ctrl+J".into()]);
    host.keys.borrow_mut().insert("Card.find".into(), vec!["F3".into()]);
    let mut screen = window_named(
        r#"<Toolbar name="own"><Item name="find" caption="Find" onclick="find" /></Toolbar>"#,
        Some(r#"<Menu name="appMenu"><Item caption="File"><Item name="print" caption="Print" key="Ctrl+P" onclick="print" /></Item></Menu>"#),
        host.clone(),
        "Card",
    );
    press(&mut screen, Key::P, Modifiers::CONTROL);
    assert!(host.log().is_empty());
    press(&mut screen, Key::J, Modifiers::CONTROL);
    press(&mut screen, Key::F3, Modifiers::NONE);
    assert_eq!(host.log(), vec!["run print click print", "run find click find"]);
}

#[test]
fn a_menu_shows_the_key_the_person_set() {
    let host = FrameHost::new();
    host.keys.borrow_mut().insert("print".into(), vec!["Ctrl+J".into()]);
    let screen = window_named(
        "",
        Some(r#"<Menu name="appMenu"><Item caption="File"><Item name="print" caption="Print" key="Ctrl+P" onclick="print" /><Item name="quit" caption="Quit" key="Ctrl+Q" onclick="quit" /></Item></Menu>"#),
        host,
        "Main",
    );
    let frame = view(&screen).frame();
    let entries = frame.entries_of(&frame.shown().menu.as_ref().unwrap().value.items[0].items);
    let keys: Vec<Option<String>> = entries.into_iter().map(|e| e.key).collect();
    assert_eq!(keys, vec![Some("Ctrl+J".to_string()), Some("Ctrl+Q".to_string())]);
}

#[test]
fn a_language_key_opens_its_choices_at_the_top_of_the_windows_parts() {
    let (mut screen, _) = window(r#"<Toolbar><Item caption="Language" key="F7" opens="language" /></Toolbar>"#, None);
    let chosen = Rc::new(RefCell::new(Vec::new()));
    view(&screen).frame().set_language(picker(chosen.clone(), &[("en", "English", true), ("nl", "Nederlands", false)]));
    draw(&mut screen);
    press(&mut screen, Key::F7, Modifiers::NONE);
    assert_eq!(lists(&screen).len(), 1);
    assert_eq!(lists(&screen)[0].node().bounds.y, view(&screen).body_place().y + 1.0);
}

// -- Enter in a form -----------------------------------------------------------

#[test]
fn enter_in_a_forms_input_submits_the_form_and_presses_no_default_button() {
    let (mut screen, host) = window(
        r#"<Form name="f" at="1,1" span="6" rows="3" onsubmit="save"><Textbox name="t" at="1,1" span="6" /></Form>
           <Textbox name="outside" at="1,4" span="6" />
           <Button name="ok" at="1,5" span="2" caption="OK" default="true" onclick="ok" />"#,
        None,
    );
    draw(&mut screen);
    let t = clarity_ui::testing::part_named(&screen, "t");
    screen.focus(Some(t));
    press(&mut screen, Key::Enter, Modifiers::NONE);
    assert_eq!(host.log().last().map(String::as_str), Some("run f submit save"));
    assert!(!host.log().iter().any(|l| l.contains("click ok")));
    let outside = clarity_ui::testing::part_named(&screen, "outside");
    screen.focus(Some(outside));
    press(&mut screen, Key::Enter, Modifiers::NONE);
    assert_eq!(host.log().last().map(String::as_str), Some("run ok click ok"));
}
