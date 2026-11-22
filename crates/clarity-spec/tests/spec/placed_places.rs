//! Placed for the generators, a view's code joins each window that shows it
//! and a tree's code is written for its window. The checker keys the types
//! of expressions by their place in a file, so no two handlers of a placed
//! file may hold an expression at the same place: else one's types are the
//! other's, and the generators write a record where a reference is.

use clarity_spec::Project;
use std::collections::HashMap;

/// The first place two handlers of one placed file share, if any.
fn shared_place(project: &Project) -> Option<String> {
    let placed = clarity_spec::views::placed(project);
    for file in &placed.files {
        let mut seen: HashMap<(u32, u32), String> = HashMap::new();
        for code in &file.code {
            let handlers = code.handlers.iter().map(|h| (h.name.name.clone(), &h.body));
            let events = code.events.iter().enumerate().map(|(i, e)| (format!("event {i}"), &e.body));
            for (name, body) in handlers.chain(events) {
                let owner = format!("{}.{name}", code.name.name);
                let mut clash = None;
                clarity_spec::code::each_expr_in(body, &mut |e| {
                    let at = (e.span.start, e.span.end);
                    match seen.get(&at) {
                        Some(other) if *other != owner => clash = clash.take().or(Some(format!("{}: {other} and {owner} at {at:?}", file.path))),
                        _ => {
                            seen.insert(at, owner.clone());
                        }
                    }
                });
                if clash.is_some() {
                    return clash;
                }
            }
        }
    }
    None
}

const TWO_WINDOWS: &str = "module shop

page First: window
    data picked: text
    content name main at 1,1 span 12 rows 4
        view Shared

page Second: window
    data picked: text
    content name main at 1,1 span 12 rows 4
        view Shared

page Shared: view
    button \"Go\" name goButton at 1,1 span 2 on click: goClicked

code Shared
    handler goClicked
        picked = \"x\"
";

#[test]
fn a_view_shown_by_two_windows_has_places_of_its_own_in_each() {
    let project = clarity_spec::load_converted([("shop.clarity", TWO_WINDOWS)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(shared_place(&project), None);
}

#[test]
fn no_example_shares_a_place_between_handlers() {
    let examples = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let mut checked = 0;
    for entry in std::fs::read_dir(examples).unwrap().flatten() {
        let dir = entry.path();
        if !dir.is_dir() || !dir.with_extension("clarity").exists() {
            continue;
        }
        let project = clarity_spec::load_dir(&dir).unwrap();
        assert_eq!(shared_place(&project), None, "{}", dir.display());
        checked += 1;
    }
    assert!(checked > 10, "the examples: {checked}");
}

const WINDOW: &str = "module shop
use parts

page Main: window
    data picked: text
    content name main at 1,1 span 12 rows 4
        view Shared

code Main
    handler mainOpened
        picked = \"a\"
";

const VIEW: &str = "module parts

page Shared: view
    data note: text
    button \"Go\" name goButton at 1,1 span 2 on click: goClicked

code Shared
    handler goClicked
        note = \"x\"
";

#[test]
fn a_view_from_another_file_has_places_of_its_own_in_the_window_s() {
    let project = clarity_spec::load_converted([("shop.clarity", WINDOW), ("parts.clarity", VIEW)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| &d.message).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(shared_place(&project), None);
}
