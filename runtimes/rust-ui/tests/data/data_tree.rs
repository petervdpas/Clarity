//! A tree and a tree table, a table in tree mode (C# TreePartTests), against
//! the cases every stack shares (conformance/trees.json): rows under their
//! parents, indented, the open ones showing theirs; a toggle opens and
//! closes a row, and a row chosen by code opens the rows above it. And
//! moving rows by hand (C# ReorderPartTests, conformance/reorder.json): a
//! row dragged onto another, the chosen row moved with Alt and an arrow key.

use super::data_rows_host;

use std::rc::Rc;

use clarity_ui::element::find_type;
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::data::list::ListPart;
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::screen::Screen;
use clarity_ui::testing::{part_named, window_of};
use data_rows_host::{case_file, click, laid, shot_path, DataHost};
use serde_json::{json, Value};

fn tree(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a tree")
}

fn act(screen: &mut Screen, act: impl FnOnce(&mut TablePart)) {
    let id = part_named(screen, "folders");
    screen.act(id, |e, _| act(e.as_any_mut().downcast_mut::<TablePart>().unwrap()));
}

fn folders(rows: &Value) -> Value {
    Value::Array(
        rows.as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, r)| json!({ "id": r["id"], "name": format!("Folder {}", r["id"]), "parent": r.get("parent").cloned().unwrap_or(Value::Null), "position": i + 1 }))
            .collect(),
    )
}

fn made(folders: Value, kind: &str, fields: &str) -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("folders", folders);
    let markup = format!(r#"<{kind} at="1,1" span="12" rows="8" name="folders" bind="folders" {fields} onselect="picked" />"#);
    let mut screen = window_of(&markup, host.clone());
    laid(&mut screen, 800, 400);
    (screen, host)
}

#[test]
fn a_tree_shows_its_rows_as_every_stack_does() {
    let trees = case_file("trees.json");
    for c in trees["cases"].as_array().unwrap() {
        let rows = folders(&c["rows"]);
        let (mut screen, _) = made(rows.clone(), "Tree", r#"show="name""#);
        for id in c["open"].as_array().unwrap() {
            let place = rows.as_array().unwrap().iter().position(|f| f["id"] == *id).unwrap();
            act(&mut screen, |t| t.toggle(place));
        }
        let t = tree(&screen);
        let shown: Vec<String> = t.shown_places().iter().map(|p| t.tree_place(*p).unwrap()).map(|s| format!("{},{},{}", s.0, s.1, s.2)).collect();
        let expect: Vec<String> = c["shows"].as_array().unwrap().iter().map(|s| format!("{},{},{}", s[0], s[1], s[2])).collect();
        assert_eq!(expect, shown, "{}", c["name"]);
    }
    for c in trees["reveal"].as_array().unwrap() {
        let rows = folders(&c["rows"]);
        let (mut screen, host) = made(rows.clone(), "Tree", r#"show="name""#);
        let at = c["at"].as_u64().unwrap() as usize;
        host.choose("folders", Some(at));
        laid(&mut screen, 800, 400);
        let t = tree(&screen);
        let opens: Vec<Value> = rows.as_array().unwrap().iter().enumerate().filter(|(i, _)| t.is_open(*i)).map(|(_, f)| f["id"].clone()).collect();
        assert_eq!(c["opens"].as_array().unwrap(), &opens, "{}", c["name"]);
    }
}

/// A tree by key: rows without ids, named by `key` and arranged under the
/// row whose key their `up` holds (conformance/trees.json `byKey`).
#[test]
fn a_tree_by_key_shows_its_rows_as_every_stack_does() {
    let by_key = &case_file("trees.json")["byKey"];
    let keyed = |rows: &Value| Value::Array(rows.as_array().unwrap().iter().map(|r| json!({ "key": r["key"], "up": r["up"], "name": format!("Node {}", r["key"]) })).collect());
    for c in by_key["cases"].as_array().unwrap() {
        let rows = keyed(&c["rows"]);
        let (mut screen, _) = made(rows.clone(), "Tree", r#"show="name" rowKey="key" parentKey="up""#);
        for key in c["open"].as_array().unwrap() {
            let place = rows.as_array().unwrap().iter().position(|r| r["key"] == *key).unwrap();
            if !tree(&screen).is_open(place) {
                act(&mut screen, |t| t.toggle(place));
            }
        }
        let t = tree(&screen);
        let shown: Vec<String> = t.shown_places().iter().map(|p| t.tree_place(*p).unwrap()).map(|s| format!("{},{},{}", s.0, s.1, s.2)).collect();
        let expect: Vec<String> = c["shows"].as_array().unwrap().iter().map(|s| format!("{},{},{}", s[0], s[1], s[2])).collect();
        assert_eq!(expect, shown, "{}", c["name"]);
    }
    for c in by_key["reveal"].as_array().unwrap() {
        let rows = keyed(&c["rows"]);
        let (mut screen, host) = made(rows.clone(), "TreeTable", r#"columns="name" rowKey="key" parentKey="up""#);
        host.choose("folders", Some(c["at"].as_u64().unwrap() as usize));
        laid(&mut screen, 800, 400);
        let t = tree(&screen);
        let opens: Vec<Value> = rows.as_array().unwrap().iter().enumerate().filter(|(i, _)| t.is_open(*i)).map(|(_, r)| r["key"].clone()).collect();
        assert_eq!(c["opens"].as_array().unwrap(), &opens);
    }
}

fn documents() -> Value {
    json!([
        { "id": 1, "name": "Documents", "parent": null },
        { "id": 2, "name": "Invoices", "parent": 1 },
        { "id": 3, "name": "2026", "parent": 2 },
        { "id": 4, "name": "Music", "parent": null },
    ])
}

fn names(screen: &Screen) -> Vec<String> {
    let t = tree(screen);
    t.shown_places().iter().map(|p| t.records()[*p]["name"].as_str().unwrap().to_string()).collect()
}

#[test]
fn a_toggle_shows_the_rows_under_a_row_and_a_chosen_row_opens_its_parents() {
    let (mut screen, host) = made(documents(), "Tree", r#"show="name""#);
    assert_eq!(vec!["Documents", "Music"], names(&screen));
    assert!(tree(&screen).header_box("name").is_none());
    assert_eq!("Expand", tree(&screen).toggle_word(0));
    // A press on the toggle opens the row and chooses none.
    let b = tree(&screen).toggle_box(0).unwrap();
    click(&mut screen, b);
    assert_eq!(vec!["Documents", "Invoices", "Music"], names(&screen));
    assert_eq!("Collapse", tree(&screen).toggle_word(0));
    assert!(host.events().is_empty());
    // The rows under a row are indented.
    laid(&mut screen, 800, 400);
    assert!(tree(&screen).toggle_box(1).unwrap().x > tree(&screen).toggle_box(0).unwrap().x);
    host.choose("folders", Some(2));
    laid(&mut screen, 800, 400);
    assert_eq!(vec!["Documents", "Invoices", "2026", "Music"], names(&screen));
    assert_eq!(Some(clarity_runtime::trees::TreeShown(2, 2, false)), tree(&screen).tree_place(2));
    let mut shot = laid(&mut screen, 800, 400);
    if let Some(path) = shot_path("data-tree.png") {
        shot.save(path).unwrap();
    }
    let b = tree(&screen).toggle_box(0).unwrap();
    click(&mut screen, b);
    assert_eq!(vec!["Documents", "Music"], names(&screen));
    // A click on a row chooses it.
    laid(&mut screen, 800, 400);
    let b = tree(&screen).cell_box(3, "name").unwrap();
    click(&mut screen, b);
    assert_eq!(vec!["select"], host.events());
}

#[test]
fn right_and_left_open_and_close_the_chosen_row() {
    let (mut screen, host) = made(documents(), "Tree", r#"show="name""#);
    host.choose("folders", Some(0));
    let id = part_named(&screen, "folders");
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::new(Key::Right, Modifiers::NONE));
    assert!(tree(&screen).is_open(0));
    screen.key_pressed(KeyInput::new(Key::Down, Modifiers::NONE));
    assert_eq!(Some(1), tree(&screen).chosen());
    screen.key_pressed(KeyInput::new(Key::Up, Modifiers::NONE));
    screen.key_pressed(KeyInput::new(Key::Left, Modifiers::NONE));
    assert!(!tree(&screen).is_open(0));
}

#[test]
fn a_tree_table_is_a_table_in_tree_mode_with_its_header() {
    let (mut screen, _) = made(documents(), "TreeTable", r#"columns="name, id""#);
    let t = tree(&screen);
    assert!(t.is_tree());
    assert!(clarity_ui::parts::Part::grows(t));
    assert!(t.header_box("name").is_some());
    let fields: Vec<&str> = t.columns().iter().map(|c| c.field.as_str()).collect();
    assert_eq!(vec!["name", "id"], fields);
    assert_eq!(vec![0, 3], t.shown_places());
    assert_eq!("1", t.shown(0, "id"));
    act(&mut screen, |t| t.toggle(0));
    let mut shot = laid(&mut screen, 800, 400);
    if let Some(path) = shot_path("data-tree-table.png") {
        shot.save(path).unwrap();
    }
}

// -- Moving rows by hand ----------------------------------------------------------

fn row_box(screen: &Screen, place: usize) -> Option<Rect> {
    match find_type::<TablePart>(screen.root(), |_| true) {
        Some(t) => t.row_box(place),
        None => find_type::<ListPart>(screen.root(), |_| true).unwrap().row_box(place),
    }
}

fn drop_mark(screen: &Screen) -> bool {
    match find_type::<TablePart>(screen.root(), |_| true) {
        Some(t) => t.drop().is_some(),
        None => find_type::<ListPart>(screen.root(), |_| true).unwrap().drop().is_some(),
    }
}

fn chosen(screen: &Screen) -> Option<usize> {
    match find_type::<TablePart>(screen.root(), |_| true) {
        Some(t) => t.chosen(),
        None => find_type::<ListPart>(screen.root(), |_| true).unwrap().chosen(),
    }
}


fn run(case: &Value, is_tree: bool) {
    let name = case["name"].as_str().unwrap();
    let mv = &case["move"];
    // The pointer drops a row of a list before or after another, never into it.
    if !is_tree && mv["where"] == "into" {
        return;
    }
    let rows = folders(&case["rows"]);
    let host = DataHost::new();
    host.set("folders", rows.clone());
    let markup = if is_tree {
        r#"<TreeTable at="1,1" span="12" rows="10" name="folders" bind="folders" columns="name" reorder="position" onreorder="moved" onselect="picked" />"#
    } else {
        r#"<List at="1,1" span="12" rows="10" name="folders" bind="folders" show="name" reorder="position" onreorder="moved" onselect="picked" />"#
    };
    let mut screen = window_of(markup, host.clone());
    laid(&mut screen, 800, 600);
    if is_tree {
        // Every row open, so every row shows.
        for _ in 0..rows.as_array().unwrap().len() {
            act(&mut screen, |t| {
                for i in 0..t.records().len() {
                    if !t.is_open(i) {
                        t.toggle(i);
                    }
                }
            });
        }
        laid(&mut screen, 800, 600);
    }
    let row = mv["row"].as_u64().unwrap() as usize;
    if let Some(key) = mv.get("key").and_then(Value::as_str) {
        host.choose("folders", Some(row));
        let id = part_named(&screen, "folders");
        screen.focus(Some(id));
        let arrow = match key {
            "up" => Key::Up,
            "down" => Key::Down,
            "left" => Key::Left,
            _ => Key::Right,
        };
        screen.key_pressed(KeyInput::new(arrow, Modifiers::ALT));
    } else {
        let from = row_box(&screen, row).unwrap_or_else(|| panic!("{name}: row {row} does not show"));
        let target = mv["target"].as_u64().unwrap() as usize;
        let onto = row_box(&screen, target).unwrap_or_else(|| panic!("{name}: the target does not show"));
        let y = match mv["where"].as_str().unwrap() {
            "before" => onto.y + 3.0,
            "after" => onto.bottom() - 3.0,
            _ => onto.y + onto.height / 2.0,
        };
        screen.pointer_pressed(from.x + 40.0, from.y + from.height / 2.0, PointerButton::Left, 1);
        screen.pointer_moved(from.x + 40.0, from.y + from.height / 2.0 + 6.0);
        screen.pointer_moved(onto.x + 40.0, y);
        // A mark shows where it lands, unless that is the row itself.
        assert_eq!(target != row, drop_mark(&screen), "{name}: the drop mark");
        laid(&mut screen, 800, 600);
        screen.pointer_released(onto.x + 40.0, y, PointerButton::Left);
        assert!(!drop_mark(&screen), "{name}: the mark stays");
    }
    let log = host.logged();
    if case["refused"].as_bool() == Some(true) {
        assert!(log.iter().all(|l| !l.contains("reorder") && !l.starts_with("write")), "{name}: was not refused {log:?}");
        assert_eq!(rows, host.get("folders"), "{name}: the order changed");
        return;
    }
    let now = host.get("folders");
    let now = now.as_array().unwrap();
    let ids: Vec<Value> = rows.as_array().unwrap().iter().map(|r| r["id"].clone()).collect();
    let order: Vec<usize> = now.iter().map(|r| ids.iter().position(|i| *i == r["id"]).unwrap()).collect();
    let wanted: Vec<usize> = case["order"].as_array().unwrap().iter().map(|o| o.as_u64().unwrap() as usize).collect();
    assert_eq!(wanted, order, "{name}: the order");
    let numbers: Vec<Value> = now.iter().map(|r| r["position"].clone()).collect();
    assert_eq!(case["numbers"].as_array().unwrap(), &numbers, "{name}: the numbers");
    if let Some(parents) = case.get("parents").and_then(Value::as_array) {
        let got: Vec<Value> = now.iter().map(|r| r["parent"].clone()).collect();
        assert_eq!(parents, &got, "{name}: the parents");
    }
    // One edit, the moved row chosen, then reorder.
    assert_eq!(1, log.iter().filter(|l| l.starts_with("write")).count(), "{name}: {log:?}");
    let moved = now.iter().position(|r| r["id"] == ids[row]).unwrap();
    assert_eq!(Some(moved), chosen(&screen), "{name}: the chosen row");
    assert_eq!("run folders reorder moved", log.last().unwrap(), "{name}");
}

#[test]
fn rows_of_a_list_move_as_every_stack_moves_them() {
    for case in case_file("reorder.json")["lists"].as_array().unwrap() {
        run(case, false);
    }
}

#[test]
fn rows_of_a_tree_move_as_every_stack_moves_them() {
    for case in case_file("reorder.json")["trees"].as_array().unwrap() {
        run(case, true);
    }
}

#[test]
fn without_reorder_rows_stay_put() {
    let host = DataHost::new();
    host.set("folders", json!([{ "id": 1 }, { "id": 2 }]));
    let mut screen = window_of(r#"<List at="1,1" span="12" rows="6" name="folders" bind="folders" show="name" />"#, host.clone());
    laid(&mut screen, 800, 400);
    host.choose("folders", Some(0));
    let id = part_named(&screen, "folders");
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::new(Key::Down, Modifiers::ALT));
    let from = row_box(&screen, 0).unwrap();
    let onto = row_box(&screen, 1).unwrap();
    screen.pointer_pressed(from.x + 20.0, from.y + 5.0, PointerButton::Left, 1);
    screen.pointer_moved(onto.x + 20.0, onto.bottom() - 3.0);
    screen.pointer_released(onto.x + 20.0, onto.bottom() - 3.0, PointerButton::Left);
    assert!(host.logged().iter().all(|l| !l.starts_with("write")));
    assert_eq!(json!(1), host.get("folders[0].id"));
}

// -- The pointer taken away (C# CheckFixDataTests) ----------------------------------

#[test]
fn a_row_dragged_when_the_pointer_is_taken_away_stays_where_it_was() {
    let host = DataHost::new();
    host.set("folders", serde_json::Value::Array((1..=3).map(|i| serde_json::json!({ "id": i, "name": format!("Folder {i}"), "position": i })).collect()));
    let mut screen = clarity_ui::testing::window_of(r#"<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name" reorder="position" />"#, host.clone());
    laid(&mut screen, 800, 400);
    let before = host.get("folders");
    let from = tree(&screen).cell_box(0, "name").unwrap();
    let onto = tree(&screen).row_box(2).unwrap();
    screen.pointer_pressed(from.x + 20.0, from.y + from.height / 2.0, clarity_ui::input::PointerButton::Left, 1);
    screen.pointer_moved(from.x + 20.0, from.y + from.height / 2.0 + 10.0);
    screen.pointer_moved(onto.x + 20.0, onto.bottom() - 3.0);
    assert!(tree(&screen).drop().is_some());
    screen.pointer_cancelled();
    assert!(tree(&screen).drop().is_none());
    screen.pointer_released(onto.x + 20.0, onto.bottom() - 3.0, clarity_ui::input::PointerButton::Left);
    assert_eq!(before, host.get("folders"));
    assert!(!host.logged().iter().any(|l| l.starts_with("write")));
}

/// `fold buttons`: Expand all and Collapse all above a tree's rows, in a
/// tree and a tree table alike, which open and close every row; code's
/// `tree.foldButtons` wins over the design, and a tree without them has none.
#[test]
fn fold_buttons_open_and_close_every_row() {
    let rows = folders(&json!([{ "id": 1 }, { "id": 2, "parent": 1 }, { "id": 3, "parent": 2 }, { "id": 4 }, { "id": 5, "parent": 4 }]));
    for (kind, fields) in [("Tree", r#"show="name" foldButtons="true""#), ("TreeTable", r#"columns="name" foldButtons="true""#)] {
        let (mut screen, _) = made(rows.clone(), kind, fields);
        assert_eq!(tree(&screen).shown_places().len(), 2, "{kind}: only the top rows at first");
        let expand = tree(&screen).fold_box(true).expect("an Expand all button");
        let collapse = tree(&screen).fold_box(false).expect("a Collapse all button");
        let first_row = tree(&screen).row_box(0).unwrap();
        assert!(expand.bottom() <= first_row.y + 0.5, "{kind}: the buttons are above the rows: {expand:?} {first_row:?}");
        click(&mut screen, expand);
        assert_eq!(tree(&screen).shown_places().len(), 5, "{kind}: Expand all opens every row");
        click(&mut screen, collapse);
        assert_eq!(tree(&screen).shown_places().len(), 2, "{kind}: Collapse all closes every row");
    }
    let (screen, _) = made(rows.clone(), "Tree", r#"show="name""#);
    assert!(tree(&screen).fold_box(true).is_none(), "no buttons without fold buttons");
    // Code turns them off where the design has them, and on where it has not.
    let (mut screen, host) = made(rows.clone(), "Tree", r#"show="name" foldButtons="true""#);
    clarity_ui::parts::PartHost::set_state(host.as_ref(), "folders", clarity_runtime::ui::PartState { fold_buttons: Some(false), ..Default::default() });
    laid(&mut screen, 800, 400);
    assert!(tree(&screen).fold_box(true).is_none(), "code's false wins over the design");
    let (mut screen, host) = made(rows, "Tree", r#"show="name""#);
    clarity_ui::parts::PartHost::set_state(host.as_ref(), "folders", clarity_runtime::ui::PartState { fold_buttons: Some(true), ..Default::default() });
    laid(&mut screen, 800, 400);
    assert!(tree(&screen).fold_box(true).is_some(), "code's true shows them");
}
