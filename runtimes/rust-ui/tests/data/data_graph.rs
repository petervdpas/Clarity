//! A graph (docs/GRAPH-PLAN.md): the records of its list as nodes, the
//! records of its `edges` as edges between them, drawn by kind; a click
//! picks a node (`select` once), a double click opens it; dragging a node
//! moves it; pointing at a node or an edge tells its own fields; the zoom
//! buttons zoom and show it whole again.

use super::data_rows_host;

use clarity_ui::element::{find_type, Element};
use clarity_ui::parts::data::graph::GraphPart;
use clarity_ui::screen::Screen;
use clarity_ui::testing::near;
use data_rows_host::{click_at, laid, shot_path, DataHost};
use serde_json::json;

fn made(extra: &str) -> (Screen, std::rc::Rc<DataHost>) {
    let host = DataHost::new();
    host.set(
        "things",
        json!([
            {"id": 1, "name": "Ada", "kind": "person", "owner": "Bob", "notes": "writes code"},
            {"id": 2, "name": "Shop", "kind": "place", "owner": "Eve", "notes": ""},
            {"id": 3, "name": "Order 7", "kind": "record", "owner": "Ada", "notes": "paid"},
            {"id": 4, "name": "Bob", "kind": "person", "owner": "", "notes": ""}
        ]),
    );
    host.set(
        "links",
        json!([
            {"id": 10, "source": 1, "target": 2, "field": "works at", "since": 2024},
            {"id": 11, "source": 3, "target": 1, "field": "ordered by", "since": 2025},
            {"id": 12, "source": 4, "target": 2, "field": "owns", "since": 2020},
            {"id": 13, "source": 4, "target": 99, "field": "lost"}
        ]),
    );
    let markup = format!(
        r#"<Graph at="1,1" span="12" rows="10" name="map" bind="things" edges="links" nodeLabel="name" nodeKind="kind" edgeLabel="field" onselect="picked" onopen="opened" nodeLooks="{{&quot;record&quot;:{{&quot;shape&quot;:&quot;box&quot;}}}}" {extra} />"#
    );
    let mut screen = window_of_graph(&markup, host.clone());
    let mut shot = laid(&mut screen, 900, 600);
    if let Some(path) = shot_path(&format!("data-graph{}.png", extra.split('"').nth(1).map(|l| format!("-{l}")).unwrap_or_default())) {
        shot.save(path).ok();
    }
    (screen, host)
}

fn window_of_graph(markup: &str, host: std::rc::Rc<DataHost>) -> Screen {
    clarity_ui::testing::window_of(markup, host)
}

fn graph(screen: &Screen) -> &GraphPart {
    find_type::<GraphPart>(screen.root(), |_| true).expect("a graph")
}

#[test]
fn records_become_nodes_and_edges_whose_ends_are_there() {
    let (screen, _) = made("");
    let (shown, _) = graph(&screen).laid();
    assert_eq!(shown.nodes.iter().map(|n| n.label.as_str()).collect::<Vec<_>>(), ["Ada", "Shop", "Order 7", "Bob"]);
    assert_eq!(shown.edges.len(), 3, "the edge to a node that is not there is left out");
    assert!(clarity_ui::parts::Part::grows(graph(&screen)));
}

#[test]
fn a_click_picks_a_node_once_and_a_double_click_opens_it() {
    let (mut screen, host) = made("");
    let (x, y) = graph(&screen).node_point(2).expect("Order 7 shows");
    click_at(&mut screen, x, y, 1);
    click_at(&mut screen, x, y, 1);
    assert_eq!(graph(&screen).chosen_row(), Some(2));
    assert_eq!(host.events(), ["select"]);
    let (x, y) = graph(&screen).node_point(0).unwrap();
    click_at(&mut screen, x, y, 2);
    assert_eq!(host.events(), ["select", "select", "open"]);
    assert_eq!(graph(&screen).chosen_row(), Some(0));
}

#[test]
fn dragging_a_node_moves_it_and_dragging_the_background_pans() {
    let (mut screen, _) = made("");
    let (x, y) = graph(&screen).node_point(3).unwrap();
    let other = graph(&screen).node_point(0).unwrap();
    screen.pointer_pressed(x, y, clarity_ui::input::PointerButton::Left, 1);
    screen.pointer_moved(x + 60.0, y + 30.0);
    screen.pointer_released(x + 60.0, y + 30.0, clarity_ui::input::PointerButton::Left);
    let (nx, ny) = graph(&screen).node_point(3).unwrap();
    assert!((nx - x - 60.0).abs() < 1.0 && (ny - y - 30.0).abs() < 1.0, "moved to ({nx}, {ny}) from ({x}, {y})");
    assert_eq!(graph(&screen).node_point(0).unwrap(), other, "the others stay");
    // The background drags the whole graph along.
    screen.pointer_pressed(20.0, 580.0, clarity_ui::input::PointerButton::Left, 1);
    screen.pointer_moved(60.0, 560.0);
    screen.pointer_released(60.0, 560.0, clarity_ui::input::PointerButton::Left);
    let (ox, oy) = graph(&screen).node_point(0).unwrap();
    assert!((ox - other.0 - 40.0).abs() < 1.0 && (oy - other.1 + 20.0).abs() < 1.0);
}

#[test]
fn pointing_tells_the_fields_that_play_no_role() {
    let (screen, _) = made("");
    let (x, y) = graph(&screen).node_point(0).unwrap();
    let told = screen_tooltip(&screen, x, y);
    assert!(told.starts_with("Ada\n"), "{told}");
    assert!(told.contains("owner: Bob") && told.contains("notes: writes code"), "{told}");
    assert!(!told.contains("kind:") && !told.contains("id:"), "roles are not told again: {told}");
    let (screen, _) = made(r#"show="owner""#);
    let (x, y) = graph(&screen).node_point(0).unwrap();
    assert_eq!(screen_tooltip(&screen, x, y), "Ada\nowner: Bob");
}

#[test]
fn pointing_at_an_edge_tells_its_label_ends_and_fields() {
    let (screen, _) = made(r#"layout="circle""#);
    let a = graph(&screen).node_point(0).unwrap();
    let b = graph(&screen).node_point(1).unwrap();
    let middle = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let told = screen_tooltip(&screen, middle.0, middle.1 + 0.0);
    assert!(told.starts_with("works at (Ada - Shop)"), "{told}");
    assert!(told.contains("since: 2024"), "{told}");
}

#[test]
fn the_zoom_buttons_zoom_and_show_it_whole() {
    let (mut screen, _) = made("");
    let (zoom_out, whole, zoom_in) = graph(&screen).zoom_buttons();
    click_at(&mut screen, zoom_in.x + 5.0, zoom_in.y + 5.0, 1);
    assert!((graph(&screen).zoom() - 1.2).abs() < 1e-9);
    click_at(&mut screen, zoom_out.x + 5.0, zoom_out.y + 5.0, 1);
    click_at(&mut screen, zoom_out.x + 5.0, zoom_out.y + 5.0, 1);
    assert!(graph(&screen).zoom() < 1.0);
    click_at(&mut screen, whole.x + 5.0, whole.y + 5.0, 1);
    assert_eq!(graph(&screen).zoom(), 1.0);
}

#[test]
fn every_layout_draws_every_node() {
    for layout in ["force", "layered", "radial", "circle"] {
        let (mut screen, _) = made(&format!(r#"layout="{layout}""#));
        let bounds = graph(&screen).node().bounds;
        let points: Vec<(f64, f64)> = (0..4).map(|r| graph(&screen).node_point(r).unwrap()).collect();
        assert!(points.iter().all(|(x, y)| bounds.contains(*x, *y)), "{layout}: {points:?} inside {bounds:?}");
        let mut shot = laid(&mut screen, 900, 600);
        let (x, y) = points[0];
        let background = shot.pixel(5, 590);
        assert!(!near(shot.pixel(x as i32, y as i32), background, 10), "{layout}: a node is drawn at its point");
    }
}

fn screen_tooltip(screen: &Screen, x: f64, y: f64) -> String {
    graph(screen).tooltip_at(x, y).unwrap_or_default()
}

#[test]
fn a_larger_graph_stays_inside_its_place_with_its_nodes_apart() {
    let host = DataHost::new();
    let kinds = ["domain", "object", "table", "view"];
    host.set("things", serde_json::Value::Array((0..40).map(|i| json!({"id": i, "name": format!("Item {i}"), "kind": kinds[i % 4]})).collect()));
    // Each node to the one before it in its group of eight, and every eighth to the first.
    let links: Vec<serde_json::Value> = (1..40).map(|i| if i % 8 == 0 { json!({"source": i, "target": 0}) } else { json!({"source": i, "target": i - 1}) }).collect();
    host.set("links", serde_json::Value::Array(links));
    let mut screen = clarity_ui::testing::window_of(r#"<Graph at="1,1" span="12" rows="10" name="map" bind="things" edges="links" nodeLabel="name" nodeKind="kind" />"#, host);
    let mut shot = laid(&mut screen, 900, 600);
    if let Some(path) = shot_path("data-graph-larger.png") {
        shot.save(path).ok();
    }
    let bounds = graph(&screen).node().bounds;
    let points: Vec<(f64, f64)> = (0..40).map(|r| graph(&screen).node_point(r).unwrap()).collect();
    assert!(points.iter().all(|(x, y)| bounds.contains(*x, *y)));
    for a in 0..40 {
        for b in a + 1..40 {
            let d = ((points[a].0 - points[b].0).powi(2) + (points[a].1 - points[b].1).powi(2)).sqrt();
            assert!(d > 8.0, "{a} and {b} stand on each other");
        }
    }
}

// A graph of the data model: the records of several lists, each its entity's
// kind, joined by their references (the model the project works out, written
// into the markup as `graphModel`).

const MODEL: &str = "{&quot;lists&quot;:[{&quot;list&quot;:&quot;customers&quot;,&quot;entity&quot;:&quot;Customer&quot;,&quot;label&quot;:&quot;name&quot;,&quot;refs&quot;:[]},{&quot;list&quot;:&quot;orders&quot;,&quot;entity&quot;:&quot;Order&quot;,&quot;label&quot;:&quot;number&quot;,&quot;refs&quot;:[{&quot;field&quot;:&quot;customer&quot;,&quot;to&quot;:&quot;Customer&quot;}]}]}";

fn shop(extra: &str) -> (Screen, std::rc::Rc<DataHost>) {
    let host = DataHost::new();
    *host.window.borrow_mut() = Some("Map".into());
    host.set("customers", json!([{"id": 1, "name": "Ada", "city": "London"}, {"id": 2, "name": "Bob", "city": "Leeds"}]));
    host.set(
        "orders",
        json!([
            {"id": 1, "number": "O-1", "customer": 1, "total": 10},
            {"id": 2, "number": "O-2", "customer": 1, "total": 20},
            {"id": 3, "number": "O-3", "customer": 2, "total": 30}
        ]),
    );
    host.set("kindPicked", json!(""));
    host.set("idPicked", json!(0));
    let markup = format!(r#"<Graph at="1,1" span="12" rows="10" name="map" lists="customers, orders" graphModel="{MODEL}" pickedKind="kindPicked" pickedId="idPicked" onselect="picked" onopen="opened" {extra} />"#);
    let mut screen = clarity_ui::testing::window_of(&markup, host.clone());
    let mut shot = laid(&mut screen, 900, 600);
    if let Some(path) = shot_path(&format!("data-graph-model{}.png", if extra.is_empty() { "" } else { "-controls" })) {
        shot.save(path).ok();
    }
    (screen, host)
}

#[test]
fn records_of_several_lists_are_nodes_of_their_entity_joined_by_references() {
    let (screen, _) = shop("");
    let (shown, _) = graph(&screen).laid();
    let nodes: Vec<(&str, &str)> = shown.nodes.iter().map(|n| (n.kind.as_str(), n.label.as_str())).collect();
    assert_eq!(nodes, [("Customer", "Ada"), ("Customer", "Bob"), ("Order", "O-1"), ("Order", "O-2"), ("Order", "O-3")]);
    let edges: Vec<(&str, &str, &str)> = shown.edges.iter().map(|e| (shown.nodes[e.from].label.as_str(), shown.nodes[e.to].label.as_str(), e.label.as_str())).collect();
    assert_eq!(edges, [("O-1", "Ada", "customer"), ("O-2", "Ada", "customer"), ("O-3", "Bob", "customer")]);
}

#[test]
fn picking_a_node_writes_its_entity_and_id_and_code_s_change_is_the_pick() {
    let (mut screen, host) = shop("");
    let (x, y) = graph(&screen).node_point_at((1, 2)).expect("O-3 shows");
    click_at(&mut screen, x, y, 1);
    assert_eq!((host.get("kindPicked"), host.get("idPicked")), (json!("Order"), json!(3)));
    assert_eq!(host.events(), ["select"]);
    assert_eq!(graph(&screen).picked(), Some((1, 2)));
    // Code picks Bob.
    host.set("kindPicked", json!("Customer"));
    host.set("idPicked", json!(2));
    assert_eq!(graph(&screen).picked(), Some((0, 1)));
    let told = graph(&screen).tooltip_at(x, y).unwrap();
    assert_eq!(told, "O-3\ncustomer: 2\ntotal: 30", "its own fields, not its id or label");
}

#[test]
fn hops_show_what_lies_near_the_picked_node_and_the_control_changes_them() {
    let (mut screen, host) = shop(r#"hops="1""#);
    assert_eq!(graph(&screen).laid().0.nodes.len(), 5, "nothing picked: all of it");
    host.set("kindPicked", json!("Customer"));
    host.set("idPicked", json!(1));
    laid(&mut screen, 900, 600);
    let labels: Vec<String> = graph(&screen).laid().0.nodes.iter().map(|n| n.label.clone()).collect();
    assert_eq!(labels, ["Ada", "O-1", "O-2"]);
    let (_, _, more) = graph(&screen).hops_control().expect("a hops control");
    click_at(&mut screen, more.x + 5.0, more.y + 5.0, 1);
    assert_eq!(graph(&screen).hops(), Some(2));
    assert_eq!(graph(&screen).laid().0.nodes.len(), 3, "two hops from Ada still reaches only her orders");
    let (fewer, _, _) = graph(&screen).hops_control().unwrap();
    for _ in 0..3 {
        click_at(&mut screen, fewer.x + 5.0, fewer.y + 5.0, 1);
    }
    assert_eq!(graph(&screen).hops(), Some(1), "never fewer than one");
}

#[test]
fn the_legend_hides_and_shows_a_kind() {
    let (mut screen, _) = shop(r#"legend="true""#);
    let chip = graph(&screen).chip_of("Order").expect("a chip for orders");
    click_at(&mut screen, chip.x + 5.0, chip.y + 5.0, 1);
    let kinds: Vec<String> = graph(&screen).laid().0.nodes.iter().map(|n| n.kind.clone()).collect();
    assert_eq!(kinds, ["Customer", "Customer"]);
    let mut shot = laid(&mut screen, 900, 600);
    if let Some(path) = shot_path("data-graph-model-legend.png") {
        shot.save(path).ok();
    }
    let chip = graph(&screen).chip_of("Order").expect("its chip stays to show it again");
    click_at(&mut screen, chip.x + 5.0, chip.y + 5.0, 1);
    assert_eq!(graph(&screen).laid().0.nodes.len(), 5);
}

/// Where a node stands in the graph's own units, by its list and row.
fn place_of(screen: &Screen, at: (usize, usize)) -> (f64, f64) {
    let (shown, places) = graph(screen).laid();
    places[shown.node_at(at.0, at.1).expect("it shows")]
}

#[test]
fn picking_hops_and_the_legend_move_no_node() {
    let (mut screen, host) = shop(r#"hops="1" legend="true""#);
    let ada = place_of(&screen, (0, 0));
    host.set("kindPicked", json!("Customer"));
    host.set("idPicked", json!(1));
    laid(&mut screen, 900, 600);
    assert_eq!(graph(&screen).laid().0.nodes.len(), 3, "only Ada's part shows");
    assert_eq!(place_of(&screen, (0, 0)), ada, "she stays where she stood");
    let chip = graph(&screen).chip_of("Order").unwrap();
    click_at(&mut screen, chip.x + 5.0, chip.y + 5.0, 1);
    assert_eq!(place_of(&screen, (0, 0)), ada);
}

#[test]
fn a_moved_node_stays_where_it_was_left_as_the_graph_changes_and_reopens() {
    let (mut screen, host) = shop(r#"hops="2""#);
    let (x, y) = graph(&screen).node_point_at((0, 1)).unwrap();
    screen.pointer_pressed(x, y, clarity_ui::input::PointerButton::Left, 1);
    screen.pointer_moved(x + 50.0, y + 40.0);
    screen.pointer_released(x + 50.0, y + 40.0, clarity_ui::input::PointerButton::Left);
    let bob = place_of(&screen, (0, 1));
    // Another record, and a pick: the graph changes, Bob stays.
    host.set("orders", json!([{"id": 1, "number": "O-1", "customer": 1}, {"id": 3, "number": "O-3", "customer": 2}, {"id": 4, "number": "O-4", "customer": 2}]));
    host.set("kindPicked", json!("Customer"));
    host.set("idPicked", json!(2));
    laid(&mut screen, 900, 600);
    assert_eq!(place_of(&screen, (0, 1)), bob);
    // The window opened again keeps it for the person.
    let (screen, _) = shop(r#"hops="2""#);
    let again = place_of(&screen, (0, 1));
    assert!((again.0 - bob.0).abs() <= 0.5 && (again.1 - bob.1).abs() <= 0.5, "{again:?} and {bob:?}");
}
