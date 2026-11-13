//! Toasts: a toast is a definition of a module, with a name, its text (a
//! key, with slots), how long it shows (its lifespan, in milliseconds), its
//! level, an icon and the window's corner it shows in. Code shows one by its
//! name: `toast saved(name = customer.name)`. An app has as many as it wants,
//! each with its own lifespan: `saved` goes after 3 seconds, `imported`
//! after 8.

use clarity_spec::code::{StmtKind, ToastCorner, ToastLevel};
use clarity_spec::parser::parse;
use clarity_spec::model::Locale;
use clarity_spec::{Project, Severity, icons, load_converted, translations};

const SHOP: &str = "module shop

toast saved
    text @savedToast
    lifespan 3000
    level success
    icon \"check\"
    corner bottomRight

toast imported
    text @importedToast
    lifespan 8000

page Card: window
    title @cardTitle
    data name: text
    button @saveCaption name save at 1,1 span 2 on click: saveClick

code Card
    handler saveClick
        toast saved(name = name)
        toast imported(count = 3)
";

const EN: &str = "translation en
    cardTitle \"Customer card\"
    saveCaption \"Save\"
    savedToast \"Saved {name}\"
    importedToast \"Imported {count} records\"
";

fn project(module: &str) -> Project {
    let mut project = load_converted([("shop.clarity", module)]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    translations::attach(&mut project, [("translations/en.clarity", EN)]);
    project
}

/// The errors that are about toasts (the project leaves out Clarity's own words).
fn toast_errors(module: &str) -> Vec<String> {
    project(module).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).filter(|m| m.contains("toast") || m.contains("lifespan") || m.contains("corner") || m.contains("icon") || m.contains("slot")).collect()
}

#[test]
fn a_toast_is_a_definition_with_its_text_lifespan_level_icon_and_corner() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let saved = &file.toasts[0];
    assert_eq!(saved.name.name, "saved");
    assert_eq!(saved.text.as_ref().map(|k| k.name.as_str()), Some("savedToast"));
    assert_eq!(saved.lifespan, Some(3000));
    assert_eq!(saved.level, ToastLevel::Success);
    assert_eq!(saved.icon.as_deref(), Some("check"));
    assert_eq!(saved.corner, ToastCorner::BottomRight);
    // What a toast leaves out takes Clarity's default: info, no icon, the top right, the app's lifespan.
    let imported = &file.toasts[1];
    assert_eq!((imported.lifespan, imported.level, imported.icon.as_deref(), imported.corner), (Some(8000), ToastLevel::Info, None, ToastCorner::TopRight));
    assert_eq!(clarity_spec::code::DEFAULT_TOAST_LIFESPAN, 5000);
}

#[test]
fn code_shows_a_toast_by_its_name_with_the_slots_of_its_text() {
    let p = project(SHOP);
    assert!(toast_errors(SHOP).is_empty(), "{:#?}", toast_errors(SHOP));
    let handler = &p.files[0].code[0].handlers[0];
    let StmtKind::Toast { name, args } = &handler.body[0].kind else { panic!("{:?}", handler.body[0].kind) };
    assert_eq!(name.name, "saved");
    assert_eq!(args[0].name.name, "name");
}

#[test]
fn the_checker_says_what_is_wrong_with_a_toast() {
    let unknown = SHOP.replace("toast imported(count = 3)", "toast importd(count = 3)");
    assert!(toast_errors(&unknown).iter().any(|e| e.contains("there is no toast importd")), "{:#?}", toast_errors(&unknown));
    let long = SHOP.replace("lifespan 8000", "lifespan 900000");
    assert!(toast_errors(&long).iter().any(|e| e.contains("lifespan") && e.contains("500") && e.contains("120000")), "{:#?}", toast_errors(&long));
    let corner = SHOP.replace("corner bottomRight", "corner middle");
    assert!(toast_errors(&corner).iter().any(|e| e.contains("topRight, topLeft, bottomRight or bottomLeft")), "{:#?}", toast_errors(&corner));
    let level = SHOP.replace("level success", "level loud");
    assert!(project(&level).diagnostics.iter().any(|d| d.message.contains("info, success, warning or error")), "{:#?}", project(&level).diagnostics);
    let icon = SHOP.replace("icon \"check\"", "icon \"chek\"");
    assert!(toast_errors(&icon).iter().any(|e| e.contains("chek")), "{:#?}", toast_errors(&icon));
    let no_text = SHOP.replace("    text @importedToast\n", "");
    assert!(toast_errors(&no_text).iter().any(|e| e.contains("toast imported has no text")), "{:#?}", toast_errors(&no_text));
    let twice = SHOP.replace("toast imported\n", "toast saved\n");
    assert!(toast_errors(&twice).iter().any(|e| e.contains("toast saved") && e.contains("twice")), "{:#?}", toast_errors(&twice));
    let slot = SHOP.replace("toast saved(name = name)", "toast saved(nam = name)");
    assert!(!toast_errors(&slot).is_empty(), "a slot its text does not have is an error");
}

#[test]
fn a_toast_s_icon_goes_to_the_renderer() {
    assert!(icons::used(&project(SHOP)).contains_key("check"));
}
