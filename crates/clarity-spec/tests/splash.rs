//! An application's splash screen (docs/APP-ICON-SPLASH-PLAN.md, step 3):
//! a page of the kind `splash`, which the app names (`splash Welcome for 2
//! seconds`) and the application shows itself while it starts. It is
//! optional: an app without the line starts as it always did.

use clarity_spec::model::PageKind;
use clarity_spec::{load_converted, Severity};

const APP: &str = "module shop
app Shop
    title \"Shop\"
    main Main
    splash Welcome for 2 seconds
page Main: window
    label \"Hello\" name hello at 1,1 span 2
page Welcome: splash
    size 480 x 280
    heading \"Shop\" name nameHeading at 1,1 span 6
    text name status at 1,3 span 6
";

fn load(text: &str) -> clarity_spec::Project {
    load_converted([("shop.clarity", text)])
}

fn messages(text: &str, severity: Severity) -> Vec<String> {
    load(text).diagnostics.into_iter().filter(|d| d.severity == severity).map(|d| d.message).collect()
}

fn has(text: &str, severity: Severity, words: &str) {
    let found = messages(text, severity);
    assert!(found.iter().any(|m| m.contains(words)), "{words}: {found:#?}");
}

#[test]
fn the_app_names_its_splash_page_and_how_long_it_shows_at_least() {
    assert!(messages(APP, Severity::Error).is_empty(), "{:#?}", messages(APP, Severity::Error));
    assert!(messages(APP, Severity::Warning).is_empty(), "{:#?}", messages(APP, Severity::Warning));
    let project = load(APP);
    let app = &project.apps().nth(0).unwrap();
    assert_eq!(app.splash.as_ref().map(|s| s.name.as_str()), Some("Welcome"));
    assert_eq!(app.splash_for, Some(2000));
    assert_eq!(project.pages().nth(1).unwrap().kind, PageKind::Splash);
    let half = load(&APP.replace("for 2 seconds", "for 1.5 seconds"));
    assert_eq!(half.apps().nth(0).unwrap().splash_for, Some(1500));
    let bare = load(&APP.replace(" for 2 seconds", ""));
    assert_eq!(bare.apps().nth(0).unwrap().splash_for, None, "without a time it closes as soon as the main window shows");
}

#[test]
fn a_splash_is_optional() {
    let without = APP.replace("    splash Welcome for 2 seconds\n", "").replace("page Welcome: splash\n    size 480 x 280\n    heading \"Shop\" name nameHeading at 1,1 span 6\n    text name status at 1,3 span 6\n", "");
    assert!(messages(&without, Severity::Error).is_empty());
    assert!(messages(&without, Severity::Warning).is_empty(), "no splash, nothing said");
    assert_eq!(load(&without).apps().next().unwrap().splash, None);
}

#[test]
fn the_rules_of_a_splash() {
    has(&APP.replace("splash Welcome", "splash Nothing"), Severity::Error, "there is no splash page 'Nothing'");
    has(&APP.replace("splash Welcome", "splash Main"), Severity::Error, "Main is a window; the app's splash shows a splash page");
    has(&APP.replace("for 2 seconds", "for 0 seconds"), Severity::Error, "from 0.1 to 30");
    has(&APP.replace("for 2 seconds", "for 2 minutes"), Severity::Error, "seconds");
    has(&APP.replace("    splash Welcome for 2 seconds\n", "    splash Welcome\n    splash Welcome\n"), Severity::Error, "duplicate 'splash'");
    // Nothing opens it: the application shows and closes it.
    has(&APP.replace("label \"Hello\" name hello at 1,1 span 2", "button \"Again\" name again at 1,1 span 2 on click -> Welcome"), Severity::Error, "Welcome is the splash screen");
    has(&APP.replace("    main Main\n", "    main Welcome\n"), Severity::Error, "must be a page of kind window");
    // A splash page no app shows is said.
    has(&APP.replace("    splash Welcome for 2 seconds\n", ""), Severity::Warning, "Welcome is a splash page no app shows");
}

const CODE: &str = "module shop
app Shop
    main Main
    splash Welcome
    on start: loadAll
page Main: window
    label \"Hello\" name hello at 1,1 span 2
page Welcome: splash
    text name status at 1,1 span 6
code Shop
    handler loadAll
        splash.status = \"Loading the catalogue\"
";

#[test]
fn code_says_what_loads_on_the_splash() {
    assert!(messages(CODE, Severity::Error).is_empty(), "{:#?}", messages(CODE, Severity::Error));
    has(&CODE.replace("splash.status = \"Loading the catalogue\"", "splash.status = 3"), Severity::Error, "cannot set text to");
    has(&CODE.replace("splash.status", "splash.title"), Severity::Error, "the splash has only a status");
    has(&CODE.replace("    splash Welcome\n", ""), Severity::Error, "the app shows no splash");
}

#[test]
fn the_designer_adds_a_splash_page_that_says_what_loads() {
    use clarity_spec::edit::Edit;
    use clarity_spec::templates::PageTemplate;
    let source = APP.replace("    splash Welcome for 2 seconds\n", "").replace("page Welcome: splash\n    size 480 x 280\n    heading \"Shop\" name nameHeading at 1,1 span 6\n    text name status at 1,3 span 6\n", "");
    let project = load(&source);
    let added = clarity_spec::cxaml::added(&project, std::path::Path::new("shop.clarity"), &source, &Edit::AddWindow { name: "Welcome".into(), template: PageTemplate::Splash }).unwrap().unwrap();
    let text = &added.files[0].1;
    assert!(text.contains("<Splash class=\"shop.Welcome\" width=\"480\" height=\"280\">"), "{text}");
    assert!(text.contains("name=\"status\""), "a status part for code to set: {text}");
}

/// A designed page lays its parts on exact square cells: `grid 56 columns
/// rows 10` on a 560 px splash is cells of 10 by 10 pixels, no gaps, rows that keep their height.
#[test]
fn a_page_may_set_its_own_fine_grid() {
    let fine = APP.replace("    size 480 x 280\n", "    size 560 x 280\n    grid 56 columns rows 10\n").replace("at 1,1 span 6", "at 1,1 span 30 rows 4").replace("at 1,3 span 6", "at 1,6 span 30 rows 2");
    assert!(messages(&fine, Severity::Error).is_empty(), "{:#?}", messages(&fine, Severity::Error));
    let loaded = load(&fine);
    let page = loaded.pages().nth(1).unwrap();
    assert_eq!((page.columns, page.cell), (56, Some(10)));
    has(&fine.replace("rows 10", "rows 4"), Severity::Error, "a page's rows are 8 to 64 pixels high");
    has(&fine.replace("grid 56 columns", "grid 97 columns"), Severity::Error, "a page grid has 1 to 96 columns");
    let coarse = load(APP);
    assert_eq!(coarse.pages().nth(1).unwrap().cell, None, "without rows the project's rows, with their gaps");
}
