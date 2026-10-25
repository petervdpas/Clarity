//! Secret settings (DESIGN 5.27): a setting of type `secret`, such as a
//! web API's token, kept per person and per application in the system's
//! protected store (Windows' Credential Manager, macOS's Keychain, the
//! Secret Service elsewhere), never in settings.json. Only a password input
//! shows one, and a web API signs its calls with it: `auth bearer
//! "setting:shopToken"`.

use clarity_spec::model::FieldType;
use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module shop

settings
    language
    shopToken: secret, label \"Token of the shop\"

page Preferences: dialog
    password name tokenBox at 1,1 span 6 bind settings.shopToken

page Main: window
    button \"Forget\" name forget at 1,1 span 2 on click: forgetClicked

code Main
    handler forgetClicked
        settings.shopToken = \"\"
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("shop.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(SHOP.contains(old), "{old:?}");
    SHOP.replacen(old, new, 1)
}

#[test]
fn a_secret_is_a_setting_a_password_input_shows() {
    assert_eq!(errors(SHOP), Vec::<String>::new());
    let project = load_sources([("shop.clarity", SHOP)]);
    let settings = project.files[0].entities.iter().find(|e| e.settings).unwrap();
    assert_eq!(settings.fields[1].ty, FieldType::Secret);
    assert_eq!(project.secret_settings(), ["shopToken"]);
}

#[test]
fn only_the_settings_keep_a_secret() {
    let entity = format!("{SHOP}\nentity Account\n    token: secret\n");
    assert_eq!(errors(&entity), ["a secret is a setting of the person, kept in the system's protected store; entity Account cannot keep 'token' as one"]);
}

#[test]
fn no_part_shows_a_secret_but_a_password_input() {
    let shown = with("password name tokenBox at 1,1 span 6", "textbox name tokenBox at 1,1 span 6");
    assert!(errors(&shown).iter().any(|e| e.contains("'shopToken' is secret; a textbox shows")), "{:?}", errors(&shown));
    let label = with("password name tokenBox at 1,1 span 6 bind settings.shopToken", "text name tokenBox at 1,1 span 6 bind settings.shopToken");
    assert!(!errors(&label).is_empty());
}

#[test]
fn a_secret_has_no_default() {
    let given = with("shopToken: secret,", "shopToken: secret, default \"abc\",");
    assert_eq!(errors(&given), ["a secret has no default: it is the person's own ('shopToken')"]);
}


#[test]
fn a_web_api_signs_with_a_secret_setting() {
    let sources = |secret: &str| vec![clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef::web("shop").at("https://shop.example.com", Some(("bearer", secret)))];
    let errors_with = |secret: &str| -> Vec<String> {
        clarity_spec::load_in([("shop.clarity", SHOP)], [], &sources(secret)).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
    };
    assert_eq!(errors_with("setting:shopToken"), Vec::<String>::new());
    assert_eq!(errors_with("setting:apiKey"), ["web API shop signs with setting 'apiKey', which is no secret setting; add it to settings as apiKey: secret"]);
}

#[test]
fn a_database_connects_with_a_secret_setting() {
    let errors_with = |connection: &str| -> Vec<String> {
        let sources = [clarity_spec::SourceRef::sql("main").at(connection, None)];
        clarity_spec::load_in([("shop.clarity", SHOP)], [], &sources).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
    };
    assert_eq!(errors_with("setting:shopToken"), Vec::<String>::new());
    assert_eq!(errors_with("env:SHOP_DB"), Vec::<String>::new());
    assert_eq!(errors_with("setting:shopDb"), ["database main connects with setting 'shopDb', which is no secret setting; add it to settings as shopDb: secret"]);
}
