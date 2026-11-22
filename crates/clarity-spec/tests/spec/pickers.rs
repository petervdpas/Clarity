//! The built-in click targets `language` and `theme` open a picker whose
//! heading is one of Clarity's own words, a key like any text people read:
//! `languagePicker` and `themePicker`, there only when the project uses that
//! click target.

use clarity_spec::Severity;
use clarity_spec::keyed::load_keyed;

const SHOP: &str = "module shop

app Shop
    main Card
    menu name mainMenu
        item \"Language\" name languageItem on click -> language
        item \"Theme\" name themeItem on click -> theme

page Card: window
    button \"Save\" name saveButton at 1,1 span 2

theme Shop
    color accent \"#1f6f5c\"
";

#[test]
fn a_picker_heading_is_a_key_when_its_click_target_is_used() {
    let p = load_keyed([("shop.clarity", SHOP)], &[], "en");
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let en = &p.translations["en"];
    assert_eq!(en.get("languagePicker").map(String::as_str), Some("Language"));
    assert_eq!(en.get("themePicker").map(String::as_str), Some("Theme"));
    let used: Vec<String> = clarity_spec::translations::keys(&p).into_iter().map(|k| k.key).collect();
    assert!(used.contains(&"languagePicker".to_string()) && used.contains(&"themePicker".to_string()), "{used:?}");
}

#[test]
fn without_the_click_target_there_is_no_heading_key() {
    let plain = SHOP.replace("        item \"Language\" name languageItem on click -> language\n", "").replace("        item \"Theme\" name themeItem on click -> theme\n", "        item \"Save\" name saveItem on click -> close\n");
    let p = load_keyed([("shop.clarity", plain.as_str())], &[], "en");
    let en = &p.translations["en"];
    assert!(!en.contains_key("languagePicker") && !en.contains_key("themePicker"), "{en:?}");
    let only_theme = SHOP.replace("        item \"Language\" name languageItem on click -> language\n", "");
    let en = load_keyed([("shop.clarity", only_theme.as_str())], &[], "en").translations["en"].clone();
    assert!(!en.contains_key("languagePicker") && en.contains_key("themePicker"), "{en:?}");
}
