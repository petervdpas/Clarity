//! What the every-part project showed live on buttons and toggles: a part
//! with an icon draws it (beside its caption, or alone in the middle), and
//! a split button has an arrow at its right that opens its menu, whose
//! items run their handlers; the rest of it is a plain button.

use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::button::ButtonPart;
use clarity_ui::parts::inputs::choice_list::{ChoiceList, LINE};
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Rect, Screen};

/// A square, as an icon's SVG path on Lucide's 24 by 24 grid.
const SQUARE: &str = "M4 4 L20 4 L20 20 L4 20 Z";

fn host() -> std::rc::Rc<TestHost> {
    let host = TestHost::new();
    host.icons.borrow_mut().insert("star".into(), SQUARE.into());
    host
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

/// The columns of a place that hold a pixel near `color`.
fn inked_columns(shot: &mut Shot, place: Rect, color: skia_safe::Color) -> Vec<i32> {
    (place.x as i32..place.right() as i32).filter(|x| (place.y as i32..place.bottom() as i32).any(|y| near(shot.pixel(*x, y), color, 40))).collect()
}

fn bounds(screen: &Screen, name: &str) -> Rect {
    screen.element(part_named(screen, name)).unwrap().node().bounds
}

#[test]
fn a_toggle_with_only_an_icon_draws_it_in_the_middle() {
    let mut screen = window_of(r#"<Toggle at="1,1" span="2" name="left" icon="star" none="caption" />"#, host());
    let mut shot = Shot::of(&mut screen, 400, 100, 1.0);
    let b = bounds(&screen, "left");
    let inside = Rect::new(b.x + 4.0, b.y + 4.0, b.width - 8.0, b.height - 8.0);
    let inked = inked_columns(&mut shot, inside, screen.look().color("text"));
    assert!(!inked.is_empty(), "the icon is drawn");
    let middle = (inked[0] + inked[inked.len() - 1]) as f64 / 2.0;
    assert!((middle - (b.x + b.width / 2.0)).abs() < 3.0, "in the middle: {middle} of {b:?}");
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/g8c-core-icon-toggle.png").unwrap();
}

#[test]
fn a_toggle_and_a_button_draw_their_icon_before_their_caption() {
    let mut screen = window_of(
        r#"<Toggle at="1,1" span="3" name="fav" caption="Favourite" icon="star" />
           <Button at="1,2" span="3" name="save" caption="Favourite" icon="star" />
           <Button at="4,2" span="3" name="plain" caption="Favourite" />"#,
        host(),
    );
    let mut shot = Shot::of(&mut screen, 600, 120, 1.0);
    let text = screen.look().color("text");
    let first = |shot: &mut Shot, b: Rect| inked_columns(shot, Rect::new(b.x + 3.0, b.y + 3.0, b.width - 6.0, b.height - 6.0), text)[0] as f64 - b.x;
    let plain = first(&mut shot, bounds(&screen, "plain"));
    for name in ["fav", "save"] {
        let with_icon = first(&mut shot, bounds(&screen, name));
        assert!(with_icon < plain - 8.0, "{name}: the icon starts before the caption would alone ({with_icon} vs {plain})");
    }
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/g8c-core-icon-buttons.png").unwrap();
}

const SPLIT: &str = r#"<Button at="1,1" span="3" name="saveButton" caption="Save" split="true" onclick="saved">
    <Menu>
      <Item caption="Save a copy" name="saveCopyItem" onclick="copied" />
      <Item caption="Save all" name="saveAllItem" onclick="allSaved" />
    </Menu>
  </Button>"#;

#[test]
fn a_split_button_draws_an_arrow_and_its_arrow_opens_its_menu() {
    let host = host();
    let mut screen = window_of(SPLIT, host.clone());
    let mut shot = Shot::of(&mut screen, 600, 300, 1.0);
    let b = bounds(&screen, "saveButton");
    let arrow = Rect::new(b.right() - ButtonPart::ARROW_WIDTH, b.y + 4.0, ButtonPart::ARROW_WIDTH - 4.0, b.height - 8.0);
    assert!(!inked_columns(&mut shot, arrow, screen.look().color("text")).is_empty(), "the arrow is drawn");
    click(&mut screen, b.right() - 10.0, b.y + b.height / 2.0);
    assert_eq!(1, screen.overlays().len(), "the menu opened");
    assert!(host.logged().is_empty(), "the arrow is no click");
    let list = screen.overlays()[0].content.as_any().downcast_ref::<ChoiceList>().expect("a list");
    assert_eq!(vec!["Save a copy", "Save all"], list.state.borrow().labels);
    let mut shot = Shot::of(&mut screen, 600, 300, 1.0);
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/g8c-core-split.png").unwrap();
    let place = screen.overlays()[0].place;
    click(&mut screen, place.x + 10.0, place.y + 1.0 + LINE * 0.5);
    assert!(screen.overlays().is_empty());
    assert_eq!(vec!["run saveButton click copied"], host.logged());
}

#[test]
fn a_split_buttons_main_part_clicks_and_down_opens_its_menu() {
    let host = host();
    let mut screen = window_of(SPLIT, host.clone());
    let _ = Shot::of(&mut screen, 600, 300, 1.0);
    let b = bounds(&screen, "saveButton");
    click(&mut screen, b.x + 20.0, b.y + b.height / 2.0);
    assert!(screen.overlays().is_empty());
    assert_eq!(vec!["run saveButton click saved"], host.logged());
    screen.key_pressed(KeyInput::plain(Key::Down));
    assert_eq!(1, screen.overlays().len(), "Down opened the menu");
    // Opened, no item is marked yet: Down marks the first, and again the second.
    screen.key_pressed(KeyInput::plain(Key::Down));
    screen.key_pressed(KeyInput::plain(Key::Down));
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(screen.overlays().is_empty());
    assert_eq!(vec!["run saveButton click saved", "run saveButton click allSaved"], host.logged());
    screen.key_pressed(KeyInput::plain(Key::Down));
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.overlays().is_empty());
    assert_eq!(2, host.logged().len(), "Escape picks nothing");
}
