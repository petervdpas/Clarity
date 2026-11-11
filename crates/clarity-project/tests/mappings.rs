//! The mapping page's backend (docs/FORMATS-PLAN.md, step 4): a mapping as a
//! form, and a sample file laid over it, read through Clarity.Formats.

use std::path::Path;

use clarity_project::mappings::{mapping, preview_mapping};

const ORDERS: &str = "module orders

entity Order
    number: text(20)
    customer: text(100)
    lines: list<Line>

entity Line
    order: ref Order
    product: text(80)
    quantity: integer

mapping OrderFeed of Order from xml
    at \"shop/orders/order\"
    number at \"@number\"
    customer at \"customer/name\"
    lines of Line at \"lines/line\"
        order is \"..\"
        product at \"@product\"
";

const SAMPLE: &str = r#"<shop><orders>
  <order number="A-1"><customer><name>Ann</name></customer><lines><line product="Coffee" quantity="2"/><line product="Tea" quantity="1"/></lines></order>
  <order number="A-2"><customer><name>Bob</name></customer></order>
</orders></shop>"#;

fn project(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("clarity-mappings-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(root.join("shop.clarity"), "project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n").unwrap();
    std::fs::write(dir.join("orders.clarity"), ORDERS).unwrap();
    std::fs::write(dir.join("sample.xml"), SAMPLE).unwrap();
    (root, dir)
}

/// The records and the records inside them, each as its entity's fields hold
/// them, the first rows only, with how many there are.
#[test]
fn a_sample_is_laid_over_the_mapping() {
    let (root, dir) = project("preview");
    let entry = mapping(&dir, "OrderFeed").unwrap();
    assert_eq!(entry.item as usize, ORDERS.find("mapping OrderFeed").unwrap());
    assert_eq!(entry.form.fields[2].inside.as_ref().unwrap().parent, "order");
    let preview = preview_mapping(&dir, "OrderFeed", Path::new("sample.xml"), 1).unwrap();
    assert_eq!(preview.problem, None);
    let shown: Vec<(&str, &str, String, usize)> = preview.tables.iter().map(|t| (t.name.as_str(), t.entity.as_str(), t.columns.join(","), t.count)).collect();
    assert_eq!(shown, [("", "Order", "number,customer".to_string(), 2), ("lines", "Line", "order,product,quantity".to_string(), 2)]);
    assert_eq!(preview.tables[0].rows, [["A-1", "Ann"]]);
    assert_eq!(preview.tables[1].rows, [["1", "Coffee", "2"]]);
    std::fs::remove_dir_all(&root).unwrap();
}

/// A file that does not fit gives the read's message in place of the tables.
#[test]
fn a_file_that_does_not_fit_says_why() {
    let (root, dir) = project("problem");
    std::fs::write(dir.join("bad.xml"), SAMPLE.replace("quantity=\"2\"", "quantity=\"two\"")).unwrap();
    let preview = preview_mapping(&dir, "OrderFeed", Path::new("bad.xml"), 10).unwrap();
    assert!(preview.tables.is_empty());
    let problem = preview.problem.unwrap();
    assert!(problem.contains("quantity"), "{problem}");
    assert_eq!(preview_mapping(&dir, "Nothing", Path::new("bad.xml"), 10).unwrap_err(), "the project has no mapping Nothing");
    std::fs::remove_dir_all(&root).unwrap();
}

/// The preview shows values as the fields hold them: a yes or no as true or
/// false, a decimal in its shortest form, a date as a date.
#[test]
fn values_are_shown_as_their_fields_hold_them() {
    let (root, dir) = project("kinds");
    std::fs::write(dir.join("prices.clarity"), "module prices\n\nentity Price\n    name: text(40)\n    price: decimal(10, 2)\n    active: bool\n    since: date\n\nmapping PriceList of Price from csv\n    name at \"Name\"\n").unwrap();
    std::fs::write(dir.join("prices.csv"), "Name,price,active,since\nTea,2.50,true,2026-11-01\nCoffee,3,false,\n").unwrap();
    let preview = preview_mapping(&dir, "PriceList", Path::new("prices.csv"), 10).unwrap();
    assert_eq!(preview.problem, None);
    assert_eq!(preview.tables[0].rows, [["Tea", "2.5", "true", "2026-11-01"], ["Coffee", "3", "false", ""]]);
    std::fs::remove_dir_all(&root).unwrap();
}

/// Where a mapping is used: a data source laid over it, and code reading or
/// writing through it, by file and line.
#[test]
fn the_uses_of_a_mapping_are_named() {
    let (root, dir) = project("uses");
    std::fs::write(root.join("shop.clarity"), "project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource feed file \"sample.xml\" external using OrderFeed\n").unwrap();
    std::fs::write(dir.join("orders.clarity"), format!("{ORDERS}\nfunction ordersIn(text: text) -> list<Order>\n    return read list<Order> from text using OrderFeed\n")).unwrap();
    let uses = clarity_project::mappings::mapping_uses(&dir, "OrderFeed").unwrap();
    let line = ORDERS.lines().count() + 3;
    assert_eq!(uses, ["data source feed in shop.clarity".to_string(), format!("code in orders.clarity line {line}")]);
    assert!(clarity_project::mappings::mapping_uses(&dir, "Other").unwrap().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}
