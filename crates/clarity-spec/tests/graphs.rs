//! The graph part (docs/GRAPH-PLAN.md): nodes and edges, two lists of
//! records of any kind the app declares; the part names which fields are
//! a node's key, label and kind and an edge's ends, label and kind, and
//! how each kind looks. Every other field is the node's or edge's own.

use clarity_spec::model::{EdgeLine, GraphLayout, NodeShape};
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Severity, load_sources};

const MODEL: &str = "module ideas

entity Thing
    name: text
    kind: text
    owner: text
    weight: integer

entity Link
    source: integer
    target: integer
    field: text
    strength: integer
";

const MAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Window class="ideas.Map">
  <Data name="things" type="list&lt;Thing&gt;" />
  <Data name="links" type="list&lt;Link&gt;" />
  <Graph name="map" at="1,1" span="12" rows="10" bind="things" edges="links" nodeLabel="name" nodeKind="kind" edgeLabel="field" layout="layered" show="owner, weight" nodeLooks="{&quot;person&quot;:{&quot;shape&quot;:&quot;box&quot;,&quot;color&quot;:&quot;accent&quot;}}" edgeLooks="{&quot;&quot;:{&quot;line&quot;:&quot;dashed&quot;}}" onselect="picked" onopen="opened" />
</Window>
"#;

const CODE: &str = "
code Map
    handler picked
        map.selected.weight = 1
    handler opened
        map.selected.weight = 2
";

fn errors(map: &str) -> Vec<String> {
    let module = format!("{MODEL}{CODE}");
    load_sources([("ideas.clarity", module.as_str()), ("ideas/design/Map.cxaml", map)])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(MAP.contains(old), "{old:?}");
    MAP.replacen(old, new, 1)
}

#[test]
fn a_graph_of_records_checks_and_reads_its_fields_and_looks() {
    assert_eq!(errors(MAP), Vec::<String>::new());
    let module = format!("{MODEL}{CODE}");
    let project = load_sources([("ideas.clarity", module.as_str()), ("ideas/design/Map.cxaml", MAP)]);
    let map = &project.pages().next().unwrap().widgets[0].props;
    assert_eq!(map.kind, WidgetKind::Graph);
    assert_eq!((map.edges.as_deref(), map.node_label.as_deref(), map.node_kind.as_deref()), (Some("links"), Some("name"), Some("kind")));
    assert_eq!(map.layout, Some(GraphLayout::Layered));
    assert_eq!(map.node_looks["person"].shape, Some(NodeShape::Box));
    assert_eq!(map.edge_looks[""].line, Some(EdgeLine::Dashed));
    assert_eq!(map.show, ["owner", "weight"]);
}

#[test]
fn a_graph_needs_no_edges_and_no_fields_named() {
    let bare = r#"<Graph name="map" at="1,1" span="12" rows="10" bind="things" />"#;
    let start = MAP.find("<Graph").unwrap();
    let end = MAP[start..].find("/>").unwrap() + start + 2;
    let map = format!("{}{bare}{}", &MAP[..start], &MAP[end..]).replace("onselect", "");
    let code_free = map.replace(CODE, "");
    let e: Vec<String> = errors(&code_free).into_iter().filter(|m| !m.contains("handler")).collect();
    assert_eq!(e, Vec::<String>::new());
}

#[test]
fn a_graph_names_only_fields_its_records_have() {
    let e = errors(&with(r#"nodeLabel="name""#, r#"nodeLabel="title""#));
    assert!(e.iter().any(|m| m.contains("Thing has no field 'title'")), "{e:?}");
    let e = errors(&with(r#"edgeLabel="field""#, r#"edgeLabel="field" edgeFrom="from""#));
    assert!(e.iter().any(|m| m.contains("Link has no field 'from'")), "{e:?}");
    let e = errors(&with(r#"show="owner, weight""#, r#"show="owner, colour""#));
    assert!(e.iter().any(|m| m.contains("Thing has no field 'colour'")), "{e:?}");
}

#[test]
fn a_graph_s_edges_are_a_list_of_records() {
    let e = errors(&with(r#"edges="links""#, r#"edges="lines""#));
    assert!(e.iter().any(|m| m.contains("unknown data 'lines'")), "{e:?}");
    let e = errors(&with(r#"edges="links" "#, ""));
    assert!(e.iter().any(|m| m.contains("name the list of its edges first")), "{e:?}");
}

#[test]
fn a_graph_s_limit_and_looks_are_checked() {
    let e = errors(&with(r#"layout="layered""#, r#"layout="layered" limit="0""#));
    assert!(e.iter().any(|m| m.contains("a graph shows 1 to 5000 nodes, not 0")), "{e:?}");
    let e = errors(&with("&quot;box&quot;", "&quot;star&quot;"));
    assert!(!e.is_empty(), "an unknown shape is refused");
    let e = errors(&with(r#"layout="layered""#, r#"layout="spiral""#));
    assert!(!e.is_empty(), "an unknown layout is refused");
}

#[test]
fn only_a_graph_takes_the_graph_s_properties() {
    let mut table = widgets::new_props(WidgetKind::Table, 1, 1);
    table.edges = Some("links".into());
    assert!(widgets::validate_props(&table).is_err_and(|e| e.contains("'edges' does not apply")));
    let mut graph = widgets::new_props(WidgetKind::Graph, 1, 1);
    graph.edges = Some("links".into());
    graph.layout = Some(GraphLayout::Radial);
    assert!(widgets::validate_props(&graph).is_ok());
}

const SHOP: &str = "module shop

entity Customer
    name: text
    city: text

entity Supplier
    name: text

entity Order
    number: text
    customer: ref Customer
    supplier: ref Supplier
    total: decimal(10, 2)
";

const PEOPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Window class="shop.People">
  <Data name="customers" type="list&lt;Customer&gt;" />
  <Data name="orders" type="list&lt;Order&gt;" />
  <Data name="kindPicked" type="text" />
  <Data name="idPicked" type="integer" />
  <Graph name="map" at="1,1" span="12" rows="10" lists="customers, orders" hops="2" legend="true" pickedKind="kindPicked" pickedId="idPicked" show="city, total" />
</Window>
"#;

fn shop_errors(window: &str) -> Vec<String> {
    load_sources([("shop.clarity", SHOP), ("shop/design/People.cxaml", window)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_graph_of_lists_knows_their_entities_labels_and_references_to_each_other() {
    assert_eq!(shop_errors(PEOPLE), Vec::<String>::new());
    let project = load_sources([("shop.clarity", SHOP), ("shop/design/People.cxaml", PEOPLE)]);
    let map = &project.pages().next().unwrap().widgets[0].props;
    let model = map.graph_model.as_ref().expect("worked out");
    let lists: Vec<(&str, &str, Option<&str>)> = model.lists.iter().map(|l| (l.list.as_str(), l.entity.as_str(), l.label.as_deref())).collect();
    assert_eq!(lists, [("customers", "Customer", Some("name")), ("orders", "Order", Some("number"))]);
    let refs: Vec<(&str, &str)> = model.lists[1].refs.iter().map(|r| (r.field.as_str(), r.to.as_str())).collect();
    assert_eq!(refs, [("customer", "Customer")], "the supplier is not shown, so its reference is no edge");
    assert!(model.lists[0].refs.is_empty());
    assert_eq!((map.hops, map.legend), (Some(2), true));
}

#[test]
fn a_graph_of_lists_binds_nothing_and_its_picked_data_fits() {
    let e = shop_errors(&PEOPLE.replace(r#"lists="customers, orders""#, r#"bind="customers" lists="customers, orders""#));
    assert!(e.iter().any(|m| m.contains("a graph of lists takes its nodes")), "{e:?}");
    let e = shop_errors(&PEOPLE.replace(r#"lists="customers, orders""#, r#"lists="customers, invoices""#));
    assert!(e.iter().any(|m| m.contains("unknown data 'invoices'")), "{e:?}");
    let e = shop_errors(&PEOPLE.replace(r#"pickedId="idPicked""#, r#"pickedId="kindPicked""#));
    assert!(e.iter().any(|m| m.contains("'kindPicked' is to hold the picked node's id; declare it as integer")), "{e:?}");
    let e = shop_errors(&PEOPLE.replace(r#"show="city, total""#, r#"show="city, colour""#));
    assert!(e.iter().any(|m| m.contains("none of Customer, Order has a field 'colour'")), "{e:?}");
    let e = shop_errors(&PEOPLE.replace(r#"hops="2""#, r#"hops="11""#));
    assert!(e.iter().any(|m| m.contains("a graph shows 1 to 10 hops")), "{e:?}");
    let e = shop_errors(&PEOPLE.replace(r#"lists="customers, orders" "#, ""));
    assert!(e.iter().any(|m| m.contains("a graph shows the records of the list it binds")), "{e:?}");
}
