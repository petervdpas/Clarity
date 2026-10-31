//! The shown rows of tables, lists, trees and charts (DESIGN 1.7) against
//! the cases every runtime shares (conformance/table-order.json,
//! table-pages.json, trees.json, charts.json `groups`), and the views a
//! reply fills and `view_event` changes.

use clarity_runtime::charts::{self, ChartRow};
use clarity_runtime::reorder::Row;
use clarity_runtime::tables::{self, TableColumn, TableOrder};
use clarity_runtime::trees;
use clarity_runtime::ui::{Reply, Ui, WindowState};
use clarity_runtime::views::{self, Sent};
use clarity_runtime::Result;
use serde_json::{Map, Value, json};

fn file(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn a_table_shows_its_rows_in_its_order() {
    let all = file(include_str!("../../../conformance/table-order.json"));
    let columns: Vec<TableColumn> = serde_json::from_value(all["columns"].clone()).unwrap();
    let rows: Vec<Map<String, Value>> = serde_json::from_value(all["rows"].clone()).unwrap();
    for case in all["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mut order: Option<TableOrder> = serde_json::from_value(case["order"].clone()).unwrap();
        let (mut events, mut chosen) = (vec![], None);
        for step in case["steps"].as_array().unwrap() {
            match step[0].as_str().unwrap() {
                "sort" if case["sortable"] == true => order = Some(tables::next_order(order.as_ref(), step[1].as_str().unwrap())),
                "sort" => {}
                _ => {
                    let place = tables::shown_order(&rows, &columns, order.as_ref())[step[1].as_u64().unwrap() as usize];
                    events.push(format!("select {place}"));
                    chosen = Some(place);
                }
            }
        }
        let expect = &case["expect"];
        assert_eq!(json!(tables::shown_order(&rows, &columns, order.as_ref())), expect["shown"], "{name}");
        assert_eq!(json!(events), expect["events"], "{name}: events");
        if let Some(c) = expect.get("chosen") {
            assert_eq!(json!(chosen), *c, "{name}: chosen");
        }
    }
}

#[test]
fn a_table_shows_a_page_at_a_time() {
    let all = file(include_str!("../../../conformance/table-pages.json"));
    for case in all["pages"].as_array().unwrap() {
        let n = |name: &str| case[name].as_i64().unwrap();
        let paged = tables::page_of(n("count") as usize, n("size") as usize, n("page"));
        assert_eq!((paged.page as i64, paged.pages as i64, paged.from as i64, paged.to as i64), (n("shows"), n("pages"), n("from"), n("to")), "{case}");
        assert_eq!(paged.text(), case["text"].as_str().unwrap(), "{case}");
    }
    for case in all["selected"].as_array().unwrap() {
        let page = tables::page_with(case["size"].as_u64().unwrap() as usize, case["at"].as_u64().unwrap() as usize);
        assert_eq!(json!(page), case["page"], "{case}");
    }
}

fn tree_rows(case: &Value) -> Vec<Row> {
    case["rows"].as_array().unwrap().iter().map(|r| Row { id: r["id"].clone(), parent: r["parent"].clone() }).collect()
}

#[test]
fn a_tree_shows_its_open_rows() {
    let all = file(include_str!("../../../conformance/trees.json"));
    for case in all["cases"].as_array().unwrap() {
        let shown = trees::shown_tree(&tree_rows(case), case["open"].as_array().unwrap());
        assert_eq!(serde_json::to_value(&shown).unwrap(), case["shows"], "{}", case["name"]);
    }
    for case in all["reveal"].as_array().unwrap() {
        let opens = trees::above(&tree_rows(case), case["at"].as_u64().unwrap() as usize);
        assert_eq!(json!(opens), case["opens"], "{case}");
    }
}

#[test]
fn a_chart_groups_its_rows() {
    let all = file(include_str!("../../../conformance/charts.json"));
    for case in all["groups"].as_array().unwrap() {
        let rows: Vec<ChartRow> = serde_json::from_value(case["rows"].clone()).unwrap();
        let order = case["order"].as_array().cloned().unwrap_or_default();
        let groups = charts::groups(&rows, case["by"].as_str().unwrap(), case["measure"].as_str().unwrap(), &order);
        let gives = case["gives"].as_array().unwrap();
        assert_eq!(groups.len(), gives.len(), "{}", case["name"]);
        for (g, want) in groups.iter().zip(gives) {
            assert_eq!(g.key, want["key"], "{}", case["name"]);
            assert_eq!(g.value, want["value"].as_f64().unwrap(), "{}", case["name"]);
        }
    }
}

fn drawn(state: WindowState) -> Result<Reply> {
    let data = state.data.clone();
    Ok(Ui::from_state(&state).reply(&state.window, data))
}

fn tasks() -> WindowState {
    let mut data = Map::new();
    data.insert("tasks".into(), json!([{ "id": 1, "title": "Milk", "up": null }, { "id": 2, "title": "bread", "up": 1 }, { "id": 3, "title": "Apples", "up": 2 }]));
    let mut state = WindowState::new("Tasks", data);
    let view = |v: Value| v.as_object().unwrap().clone();
    state.views.insert("grid".into(), view(json!({ "kind": "table", "list": "tasks", "columns": [{ "field": "title", "kind": "text" }], "order": null, "page": 2, "at": 1 })));
    state.views.insert("all".into(), view(json!({ "kind": "table", "list": "tasks", "columns": [], "order": null, "page": null })));
    state.views.insert("tree".into(), view(json!({ "kind": "tree", "list": "tasks", "parent": "up", "key": "id", "open": [] })));
    state.views.insert("chart".into(), view(json!({ "kind": "chart", "list": "tasks", "by": "title", "byKind": "text", "measure": "count", "of": "", "values": [] })));
    state
}

fn filled(state: WindowState) -> Reply {
    let sent = Sent::of(&state);
    let mut reply = drawn(state).unwrap();
    views::replied(&sent, &mut reply).unwrap();
    reply
}

#[test]
fn a_reply_fills_what_each_view_shows() {
    let reply = filled(tasks());
    let v = &reply.state.views;
    assert_eq!(v["grid"]["shown"], json!([0, 1]));
    assert_eq!(v["grid"]["pages"], json!({ "page": 1, "pages": 2, "from": 1, "to": 2, "count": 3 }));
    assert_eq!(v["grid"]["at"], json!(1));
    assert_eq!(v["all"]["shown"], json!([0, 1, 2]), "no page: every row");
    assert!(v["all"].get("pages").is_none());
    assert_eq!(v["tree"]["shown"], json!([[0, 0, true]]));
    assert_eq!(v["chart"]["groups"].as_array().map(Vec::len), Some(3));
    assert!(serde_json::to_value(&reply).unwrap()["state"]["views"].is_object());
    assert!(serde_json::to_value(drawn(WindowState::new("Empty", Map::new())).unwrap()).unwrap()["state"].get("views").is_none(), "no views: none travel");
}

#[test]
fn a_row_chosen_by_the_handler_turns_the_page_and_opens_the_tree() {
    let state = tasks();
    let sent = Sent::of(&state);
    let mut reply = drawn(state).unwrap();
    reply.state.selected.insert("grid".into(), 2);
    reply.state.selected.insert("tree".into(), 2);
    views::replied(&sent, &mut reply).unwrap();
    assert_eq!(reply.state.views["grid"]["at"], json!(2));
    assert_eq!(reply.state.views["grid"]["shown"], json!([2]));
    assert_eq!(reply.state.views["tree"]["open"], json!([1, 2]));
    // Every reply shows the chosen row, as a move can put it under a closed row.
    let mut state = reply.state.clone();
    state.views.get_mut("tree").unwrap().insert("open".into(), json!([]));
    assert_eq!(filled(state.clone()).state.views["tree"]["open"], json!([1, 2]));
    // A view event keeps what the person did: the page turned to, the row closed.
    let sent = Sent::view_event(&state);
    let mut turned = views::event(state.clone(), "grid", &action(json!({ "page": 1 })), drawn).unwrap();
    views::replied(&sent, &mut turned).unwrap();
    assert_eq!(turned.state.views["grid"]["at"], json!(1));
    assert_eq!(turned.state.views["tree"]["open"], json!([]));
}

#[test]
fn a_page_is_kept_within_the_pages() {
    let mut state = tasks();
    state.views.get_mut("grid").unwrap().insert("at".into(), json!(9));
    assert_eq!(filled(state).state.views["grid"]["at"], json!(2));
}

fn action(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn a_view_event_sorts_turns_and_opens() {
    let state = tasks();
    let sorted = views::event(state.clone(), "grid", &action(json!({ "sort": "title" })), drawn).unwrap();
    assert_eq!(sorted.state.views["grid"]["order"], json!({ "field": "title", "desc": false }));
    assert_eq!(sorted.state.views["grid"]["at"], json!(1));
    let back = views::event(sorted.state, "grid", &action(json!({ "sort": "title" })), drawn).unwrap();
    assert_eq!(back.state.views["grid"]["order"], json!({ "field": "title", "desc": true }));
    let turned = views::event(state.clone(), "grid", &action(json!({ "page": 2 })), drawn).unwrap();
    assert_eq!(turned.state.views["grid"]["at"], json!(2));
    let opened = views::event(state.clone(), "tree", &action(json!({ "open": 1 })), drawn).unwrap();
    assert_eq!(opened.state.views["tree"]["open"], json!([1]));
    let closed = views::event(opened.state, "tree", &action(json!({ "close": 1 })), drawn).unwrap();
    assert_eq!(closed.state.views["tree"]["open"], json!([]));
    let nothing = views::event(state.clone(), "grid", &Map::new(), drawn).unwrap();
    assert_eq!(nothing.state.views, state.views, "an empty action changes nothing");
    assert!(views::event(state.clone(), "nowhere", &action(json!({ "page": 1 })), drawn).is_err());
}
