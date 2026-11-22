//! Renaming a window or a view across the project: its `.cxaml` file and
//! class, its code block, and every place that names it: a click that opens
//! it, a content's view line, the app's main window and splash, `open` and
//! `ask` in code. Asked for by name, never inferred.

use clarity_spec::rename::{RenameKind, Renamed, rename};
use clarity_spec::{Severity, converted, load_sources};

const SHOP: &str = "module m

app Shop
    title \"Shop\"
    main Main
    splash Welcome
    menu name mainMenu
        item \"Card\" name cardItem on click -> Card

page Main: window
    title \"Main\"
    button \"Card\" name cardButton at 1,1 span 2 on click -> Card
    button \"Go\" name goButton at 3,1 span 2 on click: go
    content name mainContent at 1,2 span 4 rows 4
        view CardView

page Card: window
    title \"Card\"
    button \"Shown\" name shownButton at 1,1 span 2 on click: shown

page CardView: view
    label \"View\" name viewLabel at 1,1 span 2

page Welcome: splash
    label \"Welcome\" name welcomeLabel at 1,1 span 4

code Main
    handler go
        open Card

code Card
    handler shown
        let a = 1
";

fn sources() -> Vec<(String, String)> {
    converted([("m/m.clarity", SHOP)])
}

fn errors(sources: &[(String, String)]) -> Vec<String> {
    load_sources(sources.iter().map(|(p, t)| (p.as_str(), t.as_str()))).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn renamed(old: &str, new: &str) -> Result<Vec<Renamed>, String> {
    let all = sources();
    let project = load_sources(all.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let source = |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone());
    rename(&project, source, RenameKind::Page, old, new)
}

/// The sources with the renamed files in place, a moved file under its new path only.
fn after(changed: &[Renamed]) -> Vec<(String, String)> {
    let mut all = sources();
    for r in changed {
        all.retain(|(p, _)| Some(p) != r.moved_from.as_ref() && *p != r.path);
        all.push((r.path.clone(), r.text.clone()));
    }
    all
}

fn text<'a>(changed: &'a [Renamed], path: &str) -> &'a str {
    &changed.iter().find(|r| r.path == path).unwrap_or_else(|| panic!("{path} not changed: {changed:#?}")).text
}

#[test]
fn renames_a_window_its_file_and_every_reference() {
    assert!(errors(&sources()).is_empty(), "{:#?}", errors(&sources()));
    let changed = renamed("Card", "Customer").unwrap();
    let own = changed.iter().find(|r| r.path == "m/design/Customer.cxaml").expect("the window's file, moved");
    assert_eq!(own.moved_from.as_deref(), Some("m/design/Card.cxaml"));
    assert!(own.text.contains("class=\"m.Customer\""), "{}", own.text);
    assert!(text(&changed, "m/design/Main.cxaml").contains("opens=\"Customer\""), "{changed:#?}");
    assert!(text(&changed, "m/design/Shop.cxaml").contains("opens=\"Customer\""), "{changed:#?}");
    let code = text(&changed, "m/m.clarity");
    assert!(code.contains("code Customer\n") && code.contains("        open Customer\n") && !code.contains("Card"), "{code}");
    let found = errors(&after(&changed));
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn renames_a_view_the_main_window_and_the_splash() {
    let changed = renamed("CardView", "Details").unwrap();
    assert!(text(&changed, "m/design/Main.cxaml").contains("<View name=\"Details\" />"), "{changed:#?}");
    assert!(errors(&after(&changed)).is_empty(), "{:#?}", errors(&after(&changed)));

    let changed = renamed("Main", "Home").unwrap();
    assert!(text(&changed, "m/design/Shop.cxaml").contains("main=\"Home\""), "{changed:#?}");
    assert!(text(&changed, "m/m.clarity").contains("code Home\n"), "{changed:#?}");
    assert!(errors(&after(&changed)).is_empty(), "{:#?}", errors(&after(&changed)));

    let changed = renamed("Welcome", "Hello").unwrap();
    assert!(text(&changed, "m/design/Shop.cxaml").contains("splash=\"Hello\""), "{changed:#?}");
    assert!(errors(&after(&changed)).is_empty(), "{:#?}", errors(&after(&changed)));
}

#[test]
fn refuses_a_name_that_is_wrong_or_taken() {
    assert!(renamed("Card", "two words").unwrap_err().contains("not a valid"));
    assert!(renamed("Card", "customer").unwrap_err().contains("PascalCase"));
    assert!(renamed("Card", "Main").unwrap_err().contains("already"));
    assert!(renamed("Nope", "Other").unwrap_err().contains("no window or view 'Nope'"));
    assert!(renamed("Card", "Card").unwrap().is_empty());
}
