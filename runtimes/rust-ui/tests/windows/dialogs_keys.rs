//! Clarity's keys window (DESIGN 5.28), as the C# KeysWindowTests have it:
//! every command per place with its keys; Change, then press the new key; a
//! key another command has in the same place offers a swap; Reset and Reset
//! all; refused keys say why; a modifier on its own is no key. As the root
//! of a window of its own, and as a modal overlay over the window the person
//! works in.

use std::cell::RefCell;
use std::rc::Rc;

use clarity_runtime::keys::PersonKeys;
use clarity_ui::dialogs::keys::{written, KeepKeys, KeyCommandInfo, KeyLayerInfo, KeyPlace, Push, Role};
use clarity_ui::dialogs::KeysWindow;
use clarity_ui::element::GridPanel;
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::shortcuts;
use clarity_ui::{Element, Overlay, Screen, ThemeLook};

const SHOTS: &str = "/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots";

fn command(id: &str, caption: &str, keys: &[&str]) -> KeyCommandInfo {
    KeyCommandInfo { id: id.into(), caption: caption.into(), keys: keys.iter().map(|k| k.to_string()).collect(), place: None }
}

fn catalog() -> Vec<KeyLayerInfo> {
    vec![
        KeyLayerInfo { id: String::new(), title: None, commands: vec![command("keysItem", "Keys", &[]), command("quitItem", "Quit", &["Ctrl+Q"])] },
        KeyLayerInfo { id: "Card".into(), title: Some("Card".into()), commands: vec![command("Card.saveButton", "Save", &["Ctrl+S"])] },
    ]
}

fn many() -> Vec<KeyLayerInfo> {
    vec![KeyLayerInfo { id: String::new(), title: None, commands: (0..40).map(|i| command(&format!("c{i}"), &format!("Command {i}"), &[])).collect() }]
}

#[allow(non_snake_case)]
fn ctrl_k() -> KeyInput {
    KeyInput::new(Key::K, Modifiers::CONTROL)
}

/// What the window kept last, as the settings would keep it.
type Kept = Rc<RefCell<Option<PersonKeys>>>;

fn window_of(layers: Vec<KeyLayerInfo>, person: PersonKeys, root: bool) -> (KeysWindow, Kept) {
    let kept: Kept = Rc::new(RefCell::new(None));
    let keeper = kept.clone();
    let keep: KeepKeys = Rc::new(move |keys: &PersonKeys| *keeper.borrow_mut() = Some(keys.clone()));
    (KeysWindow::new(layers, person, keep, root), kept)
}

/// The window as the root of a screen of its own, which hands it its keys first.
fn shown_with(layers: Vec<KeyLayerInfo>, height: f64) -> (Screen, Kept) {
    let (window, kept) = window_of(layers, PersonKeys::default(), true);
    let mut screen = Screen::new(Box::new(window), ThemeLook::clarity(false));
    screen.resize(500.0, height, 1.0);
    (screen, kept)
}

fn shown() -> (Screen, Kept) {
    shown_with(catalog(), 640.0)
}

fn window(screen: &Screen) -> &KeysWindow {
    screen.find::<KeysWindow>().expect("the keys window shows")
}

fn button<'a>(screen: &'a Screen, role: Role, command: Option<&str>) -> &'a Push {
    window(screen).button(role, command)
}

fn press(screen: &mut Screen, role: Role, command: Option<&str>) {
    let b = button(screen, role, command).node().bounds;
    screen.click(b.x + b.width / 2.0, b.y + b.height / 2.0);
}

fn choices(screen: &Screen) -> clarity_runtime::keys::Choices {
    window(screen).person().choices
}

fn says(screen: &Screen) -> Vec<String> {
    window(screen).says()
}

#[test]
fn lists_every_command_by_place_with_its_keys() {
    let (screen, _) = shown();
    let said = says(&screen);
    for expected in ["Keys", "Everywhere", "Quit", "Ctrl+Q", "Card", "Save", "Ctrl+S", "None"] {
        assert!(said.iter().any(|s| s == expected), "{expected} in {said:?}");
    }
    // Everywhere first, then the window's own.
    let at = |text: &str| said.iter().position(|s| s == text).unwrap();
    assert!(at("Everywhere") < at("Card"));
    assert_eq!("Keys", clarity_ui::dialogs::keys::title());
}

#[test]
fn a_toolbar_or_ribbon_command_says_where_it_lives() {
    let layers = vec![KeyLayerInfo {
        id: String::new(),
        title: None,
        commands: vec![
            KeyCommandInfo { place: Some(KeyPlace::Toolbar), ..command("printButton", "Print", &[]) },
            KeyCommandInfo { place: Some(KeyPlace::Ribbon { tab: "Home".into(), group: "Edit".into() }), ..command("copyButton", "Copy", &[]) },
            command("quitItem", "Quit", &[]),
        ],
    }];
    let (screen, _) = shown_with(layers, 640.0);
    let said = says(&screen);
    assert!(said.contains(&"Print Toolbar".to_string()), "{said:?}");
    assert!(said.contains(&"Copy Home, Edit".to_string()), "{said:?}");
    assert!(said.contains(&"Quit".to_string()), "{said:?}");
}

#[test]
fn the_catalog_reads_where_a_command_lives_from_the_model() {
    let model = r#"{"keys": [{"id": "", "title": null, "commands": [
        {"id": "printButton", "name": "printButton", "caption": "Print", "keys": [], "place": "toolbar"},
        {"id": "copyButton", "name": "copyButton", "caption": "Copy", "keys": [], "place": {"tab": "Home", "group": "Edit"}},
        {"id": "quitItem", "name": "quitItem", "caption": "Quit", "keys": ["Ctrl+Q"]}
    ]}]}"#;
    let layers = clarity_ui::dialogs::keys::catalog(model);
    let places: Vec<Option<KeyPlace>> = layers[0].commands.iter().map(|c| c.place.clone()).collect();
    assert_eq!(vec![Some(KeyPlace::Toolbar), Some(KeyPlace::Ribbon { tab: "Home".into(), group: "Edit".into() }), None], places);
}

#[test]
fn no_empty_band_sits_under_the_heading() {
    let (mut screen, _) = shown();
    assert_eq!("Change gives a command a new key; Reset gives it its own key back", says(&screen)[1]);
    press(&mut screen, Role::Change, Some("keysItem"));
    assert_eq!("Press the new key; Escape stops", says(&screen)[1]);
}

#[test]
fn a_new_key_is_taken_and_kept_and_reset_puts_it_back() {
    let (mut screen, kept) = shown();
    press(&mut screen, Role::Change, Some("Card.saveButton"));
    assert!(says(&screen).contains(&"Press the new key; Escape stops".to_string()));
    screen.key_pressed(ctrl_k());
    assert_eq!(Some(&vec!["Ctrl+K".to_string()]), choices(&screen).get("Card.saveButton"));
    assert_eq!(Some(vec!["Ctrl+K".to_string()]), kept.borrow().as_ref().and_then(|k| k.choices.get("Card.saveButton").cloned()));
    assert!(says(&screen).contains(&"Ctrl+K".to_string()));
    assert!(!window(&screen).capturing());
    press(&mut screen, Role::Reset, Some("Card.saveButton"));
    assert!(choices(&screen).is_empty());
    assert!(says(&screen).contains(&"Ctrl+S".to_string()));
}

#[test]
fn escape_stops_waiting_for_a_key_and_then_closes() {
    let (mut screen, _) = shown();
    press(&mut screen, Role::Change, Some("Card.saveButton"));
    assert!(window(&screen).capturing());
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(!window(&screen).capturing());
    assert!(!window(&screen).done());
    assert!(choices(&screen).is_empty());
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(window(&screen).done());
    // Close closes it too.
    let (mut again, _) = shown();
    press(&mut again, Role::Close, None);
    assert!(window(&again).done());
}

#[test]
fn a_key_another_command_has_in_the_same_place_offers_a_swap() {
    let (mut screen, _) = shown();
    press(&mut screen, Role::Change, Some("keysItem"));
    screen.key_pressed(KeyInput::new(Key::Q, Modifiers::CONTROL));
    assert!(choices(&screen).is_empty());
    let offer = "Ctrl+Q is the key of Quit. Swap them?".to_string();
    assert!(says(&screen).contains(&offer));
    press(&mut screen, Role::Swap, None);
    assert_eq!(Some(&vec!["Ctrl+Q".to_string()]), choices(&screen).get("keysItem"));
    assert_eq!(Some(&Vec::<String>::new()), choices(&screen).get("quitItem"));
    assert!(!says(&screen).contains(&offer));
    press(&mut screen, Role::ResetAll, None);
    assert!(choices(&screen).is_empty());
}

#[test]
fn a_swap_can_be_cancelled() {
    let (mut screen, _) = shown();
    press(&mut screen, Role::Change, Some("keysItem"));
    screen.key_pressed(KeyInput::new(Key::Q, Modifiers::CONTROL));
    press(&mut screen, Role::Cancel, None);
    assert!(choices(&screen).is_empty());
    assert!(!says(&screen).contains(&"Ctrl+Q is the key of Quit. Swap them?".to_string()));
}

#[test]
fn a_key_of_another_place_is_allowed_and_said() {
    let (mut screen, _) = shown();
    press(&mut screen, Role::Change, Some("Card.saveButton"));
    screen.key_pressed(KeyInput::new(Key::Q, Modifiers::CONTROL));
    assert_eq!(Some(&vec!["Ctrl+Q".to_string()]), choices(&screen).get("Card.saveButton"));
    assert!(says(&screen).contains(&"Ctrl+Q is also the key of Quit, elsewhere; here Save goes first".to_string()));
}

#[test]
fn a_key_the_inputs_need_is_refused_saying_why() {
    let (mut screen, _) = shown();
    press(&mut screen, Role::Change, Some("Card.saveButton"));
    screen.key_pressed(KeyInput::plain(Key::A));
    assert!(choices(&screen).is_empty());
    assert!(says(&screen).contains(&"A cannot be used: a key without Ctrl or Alt is typing".to_string()));
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert!(says(&screen).contains(&"Tab cannot be used: Tab moves between fields".to_string()));
    screen.key_pressed(KeyInput::new(Key::Enter, Modifiers::CONTROL));
    assert!(says(&screen).contains(&"Ctrl+Enter cannot be used: Enter confirms".to_string()));
    // It still waits for a key it can take.
    screen.key_pressed(ctrl_k());
    assert_eq!(Some(&vec!["Ctrl+K".to_string()]), choices(&screen).get("Card.saveButton"));
}

#[test]
fn a_bare_modifier_is_no_key_and_the_wait_goes_on() {
    let (mut screen, _) = shown();
    press(&mut screen, Role::Change, Some("Card.saveButton"));
    // Ctrl going down before the K of Ctrl+K.
    screen.key_pressed(KeyInput::new(Key::Control, Modifiers::CONTROL));
    assert!(window(&screen).capturing());
    assert!(choices(&screen).is_empty());
    screen.key_pressed(ctrl_k());
    assert_eq!(Some(&vec!["Ctrl+K".to_string()]), choices(&screen).get("Card.saveButton"));
}

#[test]
fn reset_is_only_offered_for_a_key_the_person_changed() {
    let (mut screen, _) = shown();
    assert!(!button(&screen, Role::Reset, Some("Card.saveButton")).enabled());
    assert!(!button(&screen, Role::ResetAll, None).enabled());
    let person = PersonKeys { choices: [("Card.saveButton".to_string(), vec!["Ctrl+K".to_string()])].into_iter().collect() };
    screen.act_root::<KeysWindow, _>(|w, cx| w.set_person(person, cx));
    assert!(button(&screen, Role::Reset, Some("Card.saveButton")).enabled());
    assert!(button(&screen, Role::ResetAll, None).enabled());
    // It shows what the person's keys are now, set from anywhere.
    assert!(says(&screen).contains(&"Ctrl+K".to_string()));
}

#[test]
fn a_pressed_key_reads_as_clarity_writes_keys() {
    let w = |key: Key, held: Modifiers| written(KeyInput::new(key, held));
    assert_eq!(Some("Ctrl+Shift+N".to_string()), w(Key::N, Modifiers::CONTROL | Modifiers::SHIFT));
    assert_eq!(Some("Ctrl+1".to_string()), w(Key::D1, Modifiers::CONTROL));
    assert_eq!(Some("Alt+Left".to_string()), w(Key::Left, Modifiers::ALT));
    assert_eq!(Some("Backspace".to_string()), w(Key::Backspace, Modifiers::NONE));
    assert_eq!(Some("Ctrl++".to_string()), w(Key::Plus, Modifiers::CONTROL));
    assert_eq!(Some("Shift+F5".to_string()), w(Key::F5, Modifiers::SHIFT));
    assert_eq!(Some("Ctrl+Alt+Shift+Meta+X".to_string()), w(Key::X, Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::META));
    assert_eq!(None, w(Key::Other, Modifiers::CONTROL));
    // A modifier on its own is no key: Ctrl going down before K waits for the K.
    assert_eq!(None, w(Key::Control, Modifiers::CONTROL));
    assert_eq!(None, w(Key::Shift, Modifiers::SHIFT));
    assert_eq!(None, w(Key::Alt, Modifiers::ALT));
    // A key Clarity.UI has no Key for is written by the name its host gives it; one without a word for a name is none.
    assert_eq!(Some("Ctrl+F13".to_string()), written(KeyInput::named("F13", Modifiers::CONTROL)));
    assert_eq!(None, written(KeyInput::named("Tab", Modifiers::CONTROL)));
    assert_eq!(None, written(KeyInput::named("Page Up", Modifiers::CONTROL)));
    assert_eq!(None, written(KeyInput::named("x", Modifiers::CONTROL)));
    // A character typed is no key either.
    assert_eq!(None, written(KeyInput::typed('*')));
    // Each key written so reads back as the same key.
    let named = [
        Key::Tab, Key::Enter, Key::Escape, Key::Space, Key::Backspace, Key::Delete, Key::Insert,
        Key::Left, Key::Right, Key::Up, Key::Down, Key::Home, Key::End, Key::PageUp, Key::PageDown,
        Key::Plus, Key::Minus, Key::Multiply, Key::Slash, Key::Period, Key::Comma, Key::Equal, Key::Semicolon,
        Key::Quote, Key::Backslash, Key::BracketLeft, Key::BracketRight, Key::Backquote,
    ];
    let every = named.into_iter().chain(Key::LETTERS).chain(Key::DIGITS).chain(Key::FUNCTIONS);
    for key in every {
        let text = w(key, Modifiers::CONTROL).unwrap_or_else(|| panic!("{key:?} is written"));
        assert_eq!(key, shortcuts::key_of(shortcuts::parts(&text).1), "{key:?} as {text}");
    }
}

#[test]
fn its_buttons_stay_put_when_it_says_something() {
    let (mut screen, _) = shown();
    let change = button(&screen, Role::Change, Some("Card.saveButton")).node().bounds;
    let close = button(&screen, Role::Close, None).node().bounds;
    press(&mut screen, Role::Change, Some("Card.saveButton"));
    screen.key_pressed(KeyInput::plain(Key::A));
    assert_eq!(change, button(&screen, Role::Change, Some("Card.saveButton")).node().bounds);
    assert_eq!(close, button(&screen, Role::Close, None).node().bounds);
    press(&mut screen, Role::Change, Some("keysItem"));
    screen.key_pressed(KeyInput::new(Key::Q, Modifiers::CONTROL));
    assert!(says(&screen).contains(&"Ctrl+Q is the key of Quit. Swap them?".to_string()));
    assert_eq!(change, button(&screen, Role::Change, Some("Card.saveButton")).node().bounds);
    assert_eq!(close, button(&screen, Role::Close, None).node().bounds);
}

#[test]
fn it_wants_a_size_of_its_own_and_at_most_640_high() {
    let (screen, _) = shown();
    let (width, height) = window(&screen).wanted();
    assert!((400.0..=640.0).contains(&width), "{width}");
    assert!((200.0..=640.0).contains(&height), "{height}");
    let (tall, _) = window_of(many(), PersonKeys::default(), true);
    assert_eq!(640.0, tall.wanted().1);
}

#[test]
fn it_scrolls_with_the_wheel_when_the_commands_are_taller() {
    let (mut screen, _) = shown_with(many(), 400.0);
    assert!(button(&screen, Role::Close, None).node().bounds.y > 400.0);
    screen.wheel(250.0, 200.0, 0.0, 10000.0, Modifiers::NONE);
    let close = button(&screen, Role::Close, None);
    assert!(close.node().bounds.bottom() <= 400.0);
    assert!(close.is_shown());
    // A button scrolled away is out of reach.
    assert!(!button(&screen, Role::Change, Some("c0")).is_shown());
    press(&mut screen, Role::Close, None);
    assert!(window(&screen).done());
    screen.wheel(250.0, 200.0, 0.0, -10000.0, Modifiers::NONE);
    assert!(button(&screen, Role::Change, Some("c0")).is_shown());
}

#[test]
fn as_a_modal_overlay_it_takes_keys_through_the_focus_and_closes() {
    let mut screen = Screen::new(Box::new(GridPanel::new(12)), ThemeLook::clarity(false));
    screen.resize(900.0, 700.0, 1.0);
    let (keys, _) = window_of(catalog(), PersonKeys::default(), false);
    let closed = Rc::new(RefCell::new(false));
    let told = closed.clone();
    let id = screen.open(Overlay { closed: Some(Box::new(move || *told.borrow_mut() = true)), ..Overlay::dialog(Box::new(keys)) });
    assert_eq!(Some(id), screen.focused());
    // A centered panel over the dimmed window.
    let panel = window(&screen).panel();
    assert!(panel.width < 900.0 && panel.x > 0.0);
    assert!((panel.x + panel.width / 2.0 - 450.0).abs() <= 1.0);
    press(&mut screen, Role::Change, Some("Card.saveButton"));
    screen.key_pressed(ctrl_k());
    assert_eq!(Some(&vec!["Ctrl+K".to_string()]), choices(&screen).get("Card.saveButton"));
    // Tab while waiting is a key it refuses, not a move of the focus.
    press(&mut screen, Role::Change, Some("quitItem"));
    let focused = screen.focused();
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(focused, screen.focused());
    assert!(says(&screen).contains(&"Tab cannot be used: Tab moves between fields".to_string()));
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(!*closed.borrow());
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(*closed.borrow());
    assert!(screen.overlays().is_empty());
}

#[test]
fn enter_on_a_focused_button_presses_it() {
    let (mut screen, _) = shown();
    let change = button(&screen, Role::Change, Some("Card.saveButton")).node().id();
    screen.focus(Some(change));
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(window(&screen).capturing());
}

#[test]
fn it_draws_in_the_theme_as_a_root_and_as_an_overlay() {
    let look = ThemeLook::clarity(false);
    let (mut screen, _) = shown();
    let mut root = clarity_ui::testing::Shot::of(&mut screen, 500, 640, 1.0);
    root.save(format!("{SHOTS}/r1-dialogs-keys-root.png")).unwrap();
    // The heading is written at the top left.
    assert!(root.any_not(16, 16, 104, 28, look.color("background")));
    let mut under = Screen::new(Box::new(GridPanel::new(12)), look.clone());
    under.resize(900.0, 700.0, 1.0);
    let (over, _) = window_of(catalog(), PersonKeys::default(), false);
    under.open(Overlay::dialog(Box::new(over)));
    let panel = window(&under).panel();
    let mut shot = clarity_ui::testing::Shot::of(&mut under, 900, 700, 1.0);
    shot.save(format!("{SHOTS}/r1-dialogs-keys-over.png")).unwrap();
    // The window under it is dimmed; the panel is in the panel color.
    assert_ne!(look.color("background"), shot.pixel(5, 5));
    assert_eq!(look.color("panel"), shot.pixel(panel.x as i32 + 6, panel.bottom() as i32 - 6));
}
