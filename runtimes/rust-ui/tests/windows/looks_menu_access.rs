//! The menu bar's access keys, as desktop programs have them: each top item
//! has a letter of its caption (its first letter, else the next one no item
//! before it took), underlined while Alt is held; Alt with that letter opens
//! its menu. Alt pressed and let go alone keeps the letters shown, and then
//! the letter alone opens the menu; any other key, or Escape, ends that.

use super::core_frame_shared as shared;

use clarity_ui::frame::rules;
use clarity_ui::input::{Key, KeyInput, Modifiers};
use shared::*;

const MENU: &str = r#"
    <Menu name="mainMenu">
      <Item caption="File"><Item caption="Open" onclick="open" /></Item>
      <Item caption="View"><Item caption="Zoom" onclick="zoom" /></Item>
      <Item caption="Format"><Item caption="Bold" onclick="bold" /></Item>
      <Item caption="Help" onclick="help" />
    </Menu>
"#;

fn open_captions(screen: &clarity_ui::Screen) -> Vec<Vec<String>> {
    lists(screen).iter().map(|l| captions(l)).collect()
}

fn release(screen: &mut clarity_ui::Screen, key: Key, held: Modifiers) {
    screen.key_released(KeyInput::new(key, held));
}

#[test]
fn each_top_item_has_its_first_free_letter() {
    let captions: Vec<String> = ["File", "View", "Format", "Help", "", "Fv"].iter().map(|s| s.to_string()).collect();
    assert_eq!(vec![Some(0), Some(0), Some(1), Some(0), None, None], rules::access_keys(&captions));
}

#[test]
fn alt_with_a_letter_opens_that_menu() {
    let (mut screen, _) = window(MENU, None);
    draw(&mut screen);
    press(&mut screen, Key::Alt, Modifiers::ALT);
    press(&mut screen, Key::V, Modifiers::ALT);
    assert_eq!(vec![vec!["Zoom".to_string()]], open_captions(&screen));
    release(&mut screen, Key::Alt, Modifiers::NONE);
    // Format's letter is its o: File took the F.
    press(&mut screen, Key::Escape, Modifiers::NONE);
    press(&mut screen, Key::O, Modifiers::ALT);
    assert_eq!(vec![vec!["Bold".to_string()]], open_captions(&screen));
}

#[test]
fn a_top_item_without_items_runs_as_a_click() {
    let (mut screen, host) = window(MENU, None);
    draw(&mut screen);
    press(&mut screen, Key::H, Modifiers::ALT);
    assert_eq!(host.log(), vec!["run  click help"]);
}

#[test]
fn alt_held_shows_the_letters_and_alt_alone_keeps_them_for_a_plain_letter() {
    let (mut screen, _) = window(MENU, None);
    draw(&mut screen);
    assert!(!view(&screen).menu_bar().unwrap().access_shown());
    press(&mut screen, Key::Alt, Modifiers::ALT);
    assert!(view(&screen).menu_bar().unwrap().access_shown());
    draw(&mut screen).save(shot_path("looks-menu-access")).unwrap();
    release(&mut screen, Key::Alt, Modifiers::NONE);
    assert!(view(&screen).menu_bar().unwrap().access_shown(), "Alt alone keeps them shown");
    press(&mut screen, Key::F, Modifiers::NONE);
    assert_eq!(vec![vec!["Open".to_string()]], open_captions(&screen));
    assert!(!view(&screen).menu_bar().unwrap().access_shown());
}

#[test]
fn another_key_after_alt_alone_ends_the_letters() {
    let (mut screen, _) = window(MENU, None);
    draw(&mut screen);
    press(&mut screen, Key::Alt, Modifiers::ALT);
    release(&mut screen, Key::Alt, Modifiers::NONE);
    press(&mut screen, Key::Escape, Modifiers::NONE);
    assert!(!view(&screen).menu_bar().unwrap().access_shown());
    press(&mut screen, Key::F, Modifiers::NONE);
    assert!(lists(&screen).is_empty(), "a plain letter is no menu's once the letters went");
}

#[test]
fn alt_used_with_a_letter_shows_no_letters_after() {
    let (mut screen, _) = window(MENU, None);
    draw(&mut screen);
    press(&mut screen, Key::Alt, Modifiers::ALT);
    press(&mut screen, Key::F, Modifiers::ALT);
    release(&mut screen, Key::Alt, Modifiers::NONE);
    assert!(!view(&screen).menu_bar().unwrap().access_shown());
}
