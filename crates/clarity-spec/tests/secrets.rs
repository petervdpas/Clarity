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

/// A database that connects with a secret setting opens only once the person
/// has kept it: the application shows the window where it is entered first.
#[test]
fn the_window_that_shows_a_secret_connection_is_found() {
    let shop = SHOP.replace("shopToken", "shopDb");
    let project = |source: &str| clarity_spec::load_in([("shop.clarity", source)], [], &[clarity_spec::SourceRef::sql("main").at("setting:shopDb", None)]);
    let found = project(&shop);
    assert_eq!(found.secret_connections(), ["shopDb"]);
    assert_eq!(found.connection_window().map(|p| p.name.name.as_str()), Some("Preferences"));
    // Without a window to enter it in, the database could never open.
    let hidden = shop.replace("    password name tokenBox at 1,1 span 6 bind settings.shopDb\n", "");
    let errors: Vec<String> = project(&hidden).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect();
    assert_eq!(errors, ["database main connects with setting 'shopDb', which no window shows and no 'on unconnected' handles; add a password input bound to settings.shopDb, such as in a settings window, or an app handler on unconnected"]);
}

/// An app may take over (DESIGN 5.27): `on unconnected` runs instead of the
/// window that shows the secret, with why the databases did not open.
#[test]
fn an_app_handles_its_databases_not_opening() {
    let shop = SHOP.replace("shopToken", "shopDb").replace("    password name tokenBox at 1,1 span 6 bind settings.shopDb\n", "");
    let app = |handler: &str| {
        format!("{shop}\napp Shop\n    main Main\n    on unconnected: notConnected\n\ncode Shop\n    handler {handler}\n        message reason\n")
    };
    let errors_of = |source: &str| -> Vec<String> {
        clarity_spec::load_in([("shop.clarity", source)], [], &[clarity_spec::SourceRef::sql("main").at("setting:shopDb", None)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
    };
    assert_eq!(errors_of(&app("notConnected(reason: text)")), Vec::<String>::new());
    let none = app("notConnected").replace("        message reason\n", "        message \"no database\"\n");
    assert_eq!(errors_of(&none), Vec::<String>::new());
    assert_eq!(
        errors_of(&app("notConnected(reason: integer)")).first().map(String::as_str),
        Some("handler notConnected takes (reason: integer); 'on unconnected' gives one text, why the databases did not open")
    );
}
