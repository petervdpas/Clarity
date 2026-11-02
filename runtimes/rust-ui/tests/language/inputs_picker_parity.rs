//! The language picker of the frame, through the real picker and the
//! program's choices rather than a fake: each language by its own name, the
//! one shown checked, and a choice that becomes the language shown, which
//! the choices tell (their `changes` move) so the windows and the settings
//! follow. On its own, since the language chosen is the program's.

use clarity_ui::frame::rules::{FramePicker, PickerChoice};
use clarity_ui::AppFiles;

const MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "translations": {"en": {}, "nl": {}, "de": {}}}}"#;

const FILES: AppFiles = AppFiles { windows: &[], apps: &[], themes: &[], model: MODEL, resources: &[], title: "Languages", width: 800.0, height: 400.0 };

#[test]
fn the_language_picker_offers_each_language_by_its_own_name_and_a_choice_is_told() {
    let _alone = super::one_at_a_time();
    clarity_ui::choices::start(&FILES, None);
    let picker = FramePicker::language();
    let mut choices = (picker.choices)();
    choices.sort_by(|a, b| a.value.cmp(&b.value));
    let name = |tag: &str| clarity_ui::choices::languages().into_iter().find(|o| o.name == tag).map(|o| o.label).unwrap();
    let choice = |value: &str, shown: bool| PickerChoice { value: value.into(), label: name(value), shown };
    assert_eq!(vec![choice("de", false), choice("en", true), choice("nl", false)], choices);
    assert_eq!("nederlands", name("nl").to_lowercase());
    let before = clarity_ui::choices::changes();
    (picker.choose)("nl");
    assert_eq!("nl", clarity_ui::choices::language());
    assert_eq!(before + 1, clarity_ui::choices::changes());
    assert!((picker.choices)().contains(&choice("nl", true)));
    (picker.choose)("en");
}
