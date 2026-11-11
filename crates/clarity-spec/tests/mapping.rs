//! Mappings (docs/FORMATS-PLAN.md, step 4): `mapping OrderFeed of Order from
//! xml` says, for this case only, where a file holds the records and each
//! field, for every format alike; code reads and writes `using` it.

use clarity_spec::code::{ExprKind, MappingUse, MappingValue, StmtKind};
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_converted};

const ORDERS: &str = "module orders

entity Order
    number: text(20)
    customer: text(100)
    lines: list<Line>

entity Line
    order: ref Order
    product: text(80)
    quantity: integer

entity Contact
    name: text(80)
    email: text(120)

mapping OrderFeed of Order from xml
    at \"shop/orders/order\"
    number at \"@number\"
    customer at \"customer/name\"
    lines of Line at \"lines/line\"
        order is \"..\"
        product at \"@product\"
    check \"orders.xsd\"
    indent true

mapping Contacts of Contact from csv
    name at \"Full name\"
    separator \";\"

function ordersIn(text: text) -> list<Order>
    return read list<Order> from text using OrderFeed

function orderIn(text: text) -> Order
    return read Order from text using OrderFeed

function written(orders: list<Order>) -> text
    return text of orders using OrderFeed

function contactsIn(text: text) -> list<Contact>
    return read list<Contact> from text using Contacts

function inline(text: text) -> list<Order>
    return read list<Order> from xml text at \"shop/orders/order\"
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("orders.clarity", source)]).diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(ORDERS.contains(old), "{old:?}");
    ORDERS.replacen(old, new, 1)
}

/// The one error `source` has, and the line it is on (from 1).
fn one_error(source: &str) -> (String, usize) {
    let loaded = load_converted([("orders.clarity", source)]);
    let found: Vec<_> = loaded.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert_eq!(found.len(), 1, "{found:#?}");
    (found[0].message.clone(), found[0].span.line as usize)
}

fn line_of(source: &str, text: &str) -> usize {
    source.lines().position(|l| l.contains(text)).unwrap_or_else(|| panic!("{text:?}")) + 1
}

#[test]
fn a_mapping_is_read_with_its_lines() {
    let (file, diagnostics) = parse(ORDERS, "orders.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let feed = &file.mappings[0];
    assert_eq!((feed.name.name.as_str(), feed.entity.name.as_str(), feed.format.name.as_str()), ("OrderFeed", "Order", "xml"));
    assert_eq!(feed.at.as_ref().unwrap().value, "shop/orders/order");
    let fields: Vec<(&str, Option<&str>)> = feed.fields.iter().map(|f| (f.field.name.as_str(), f.at.as_ref().map(|a| a.value.as_str()))).collect();
    assert_eq!(fields, vec![("number", Some("@number")), ("customer", Some("customer/name")), ("lines", Some("lines/line"))]);
    let inside = feed.fields[2].inside.as_ref().unwrap();
    assert_eq!(inside.entity.name, "Line");
    assert_eq!(inside.parent.as_ref().map(|p| (p.field.name.as_str(), p.path.value.as_str())), Some(("order", "..")));
    assert_eq!(inside.fields[0].field.name, "product");
    assert_eq!(feed.check.as_ref().unwrap().value, "orders.xsd");
    assert_eq!(feed.options[0].name.name, "indent");
    assert_eq!(feed.options[0].value, MappingValue::Bool(true));
    assert_eq!(file.mappings[1].options[0].value, MappingValue::Text(";".into()));
}

#[test]
fn code_reads_and_writes_using_a_mapping() {
    let (file, _) = parse(ORDERS, "orders.clarity");
    let StmtKind::Return { value: Some(read) } = &file.functions[0].body[0].kind else { panic!() };
    assert!(matches!(&read.kind, ExprKind::ReadFormat { mapping: MappingUse::Named { name }, .. } if name.name == "OrderFeed"), "{read:#?}");
    let StmtKind::Return { value: Some(write) } = &file.functions[2].body[0].kind else { panic!() };
    assert!(matches!(&write.kind, ExprKind::FormatOf { mapping: MappingUse::Named { name }, .. } if name.name == "OrderFeed"), "{write:#?}");
    let StmtKind::Return { value: Some(inline) } = &file.functions[4].body[0].kind else { panic!() };
    assert!(matches!(&inline.kind, ExprKind::ReadFormat { mapping: MappingUse::Inline { format, .. }, .. } if format.name == "xml"));
    assert_eq!(clarity_spec::code::print::expr(write), "text of orders using OrderFeed");
    assert_eq!(clarity_spec::code::print::expr(read), "read list<Order> from text using OrderFeed");
}

#[test]
fn the_orders_check() {
    assert_eq!(errors(ORDERS), Vec::<String>::new());
}

#[test]
fn a_mapping_names_an_entity_and_its_fields() {
    let source = with("mapping OrderFeed of Order", "mapping OrderFeed of Purchase");
    let errs = errors(&source);
    assert!(errs.contains(&"unknown entity 'Purchase'".to_string()), "{errs:?}");
    let source = with("    customer at \"customer/name\"", "    buyer at \"customer/name\"");
    assert_eq!(one_error(&source), ("Order has no field 'buyer'".to_string(), line_of(&source, "buyer at")));
    let source = with("        product at \"@product\"", "        price at \"@price\"");
    assert_eq!(one_error(&source), ("Line has no field 'price'".to_string(), line_of(&source, "price at")));
}

#[test]
fn a_field_is_mapped_once() {
    let source = with("    customer at \"customer/name\"", "    customer at \"customer/name\"\n    customer at \"buyer\"");
    assert_eq!(one_error(&source), ("field 'customer' is mapped twice".to_string(), line_of(&source, "customer at \"buyer\"")));
    let source = with("        product at \"@product\"", "        order at \"@order\"");
    assert_eq!(one_error(&source).0, "field 'order' is mapped twice");
}

#[test]
fn records_inside_are_a_list_or_record_of_the_entity_named() {
    let source = with("    lines of Line at \"lines/line\"", "    lines of Contact at \"lines/line\"");
    assert_eq!(one_error(&source), ("field 'lines' holds Line records, not records of Contact".to_string(), line_of(&source, "lines of Contact")));
    let source = with("    customer at \"customer/name\"", "    customer of Line at \"customer\"");
    assert_eq!(one_error(&source).0, "field 'customer' is text(100), not records of Line");
    let source = with("    lines of Line at \"lines/line\"\n        order is \"..\"\n        product at \"@product\"", "    lines at \"lines/line\"");
    assert_eq!(one_error(&source).0, "field 'lines' holds records: write lines of Line, with their lines under it");
}

#[test]
fn is_names_a_ref_to_the_record_around() {
    let source = with("        order is \"..\"", "        quantity is \"..\"");
    assert_eq!(one_error(&source), ("'is \"..\"' names the Order a Line sits in: field 'quantity' is integer, not ref Order".to_string(), line_of(&source, "quantity is")));
    let source = with("        order is \"..\"", "        order is \"../..\"");
    assert_eq!(one_error(&source).0, "'is' names the record around, written \"..\", as in order is \"..\"");
}

#[test]
fn a_path_is_one_the_format_can_have() {
    // CSV has columns only: no attributes, no steps, no `at`.
    let source = with("    name at \"Full name\"", "    name at \"@name\"");
    assert_eq!(one_error(&source), ("CSV has columns only: a path in it is a column's name, not \"@name\"".to_string(), line_of(&source, "name at \"@name\"")));
    let source = with("    name at \"Full name\"", "    at \"rows\"\n    name at \"Full name\"");
    assert_eq!(one_error(&source).0, "CSV holds its records in rows: a mapping from csv takes no 'at'");
    // An attribute where the format has none.
    let source = format!("{ORDERS}\nmapping Feed of Contact from json\n    name at \"person/@name\"\n");
    assert_eq!(one_error(&source).0, "JSON has no attributes: '@name' in \"person/@name\" names one");
    // Where it has them, an attribute ends a field's path.
    let source = with("    number at \"@number\"", "    number at \"@number/x\"");
    assert_eq!(one_error(&source).0, "an attribute is the last step of a path: \"@number/x\" goes on after '@number'");
    let source = with("    at \"shop/orders/order\"", "    at \"shop/@orders\"");
    assert_eq!(one_error(&source).0, "records are elements, not an attribute: \"shop/@orders\" ends at '@orders'");
    let source = with("    customer at \"customer/name\"", "    customer at \"customer//name\"");
    assert_eq!(one_error(&source).0, "the path \"customer//name\" has an empty step; write the steps with one slash between");
}

#[test]
fn options_are_the_formats_and_of_their_kind() {
    let source = with("    indent true", "    indent \"yes\"");
    assert_eq!(one_error(&source), ("the option 'indent' of xml is true or false, not text".to_string(), line_of(&source, "indent \"yes\"")));
    let source = with("    separator \";\"", "    indent true");
    assert_eq!(one_error(&source).0, "csv has no option 'indent'; it has the options separator, noHeader, quote, encoding");
    let source = with("    indent true", "    indent true\n    indent false");
    assert_eq!(one_error(&source).0, "the option 'indent' is given twice");
}

#[test]
fn check_takes_a_schema_of_the_format() {
    let source = with("    separator \";\"", "    check \"contacts.xsd\"");
    assert_eq!(one_error(&source).0, "CSV has no schemas to check a file against; the formats with schemas are xml (xsd)");
    let source = with("    check \"orders.xsd\"", "    check \"orders.json\"");
    assert_eq!(one_error(&source).0, "a schema of XML is a xsd file, not \"orders.json\"");
}

#[test]
fn the_schema_is_a_file_of_the_project() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("mapping_schema_project");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("orders.clarity"), ORDERS).unwrap();
    let missing = |dir: &std::path::Path| -> Vec<String> {
        clarity_spec::load_dir(dir).unwrap().diagnostics.iter().filter(|d| d.severity == Severity::Error && d.message.contains("mapping")).map(|d| d.message.clone()).collect()
    };
    assert_eq!(missing(&dir), vec!["mapping OrderFeed checks against \"orders.xsd\", which is not in the project's folder".to_string()]);
    std::fs::write(dir.join("orders.xsd"), "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"/>").unwrap();
    assert_eq!(missing(&dir), Vec::<String>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_format_and_mapping_are_known() {
    let source = with("mapping Contacts of Contact from csv", "mapping Contacts of Contact from toml");
    assert_eq!(one_error(&source).0, "there is no format 'toml'; the formats are json, csv, xml, yaml");
    let source = with("read Order from text using OrderFeed", "read Order from text using OrderFed");
    assert_eq!(one_error(&source), ("there is no mapping 'OrderFed'".to_string(), line_of(&source, "using OrderFed")));
}

#[test]
fn a_mapping_is_used_with_its_entity() {
    let source = with("function contactsIn(text: text) -> list<Contact>\n    return read list<Contact> from text using Contacts", "function contactsIn(text: text) -> list<Contact>\n    return read list<Contact> from text using OrderFeed");
    assert_eq!(one_error(&source), ("mapping OrderFeed lays a file over Order, not Contact".to_string(), line_of(&source, "read list<Contact> from text using OrderFeed")));
    let source = format!("{ORDERS}\nfunction contacts(people: list<Contact>) -> text\n    return text of people using OrderFeed\n");
    assert_eq!(one_error(&source).0, "mapping OrderFeed lays a file over Order, not list<Contact>");
}

#[test]
fn a_mapping_gives_what_writing_needs() {
    // XML's writing needs `at`; the mapping is where it is given.
    let source = with("    at \"shop/orders/order\"\n", "");
    assert_eq!(one_error(&source).0, "'text of ... using OrderFeed' needs 'at': the elements around the records and the record's own, like at \"orders/order\"");
}

/// The text of `name`'s declaration in ORDERS, from its first line to the blank line after it.
fn declared(name: &str) -> String {
    let start = ORDERS.find(&format!("mapping {name} ")).unwrap();
    let end = ORDERS[start..].find("\n\n").map_or(ORDERS.len(), |i| start + i + 1);
    ORDERS[start..end].to_string()
}

/// The IDE's mapping page edits a mapping as a form of plain texts; the
/// printer writes the form back as the language writes it, the same text.
#[test]
fn a_mapping_prints_back_as_it_was_written() {
    use clarity_spec::code::mapping::{form_of, text};
    let (file, _) = parse(ORDERS, "orders.clarity");
    for mapping in &file.mappings {
        let form = form_of(mapping);
        assert_eq!(text(&form), declared(&mapping.name.name));
        assert_eq!(clarity_spec::code::print::mapping(mapping), declared(&mapping.name.name));
    }
    let form = form_of(&file.mappings[0]);
    assert_eq!((form.at.as_str(), form.check.as_str()), ("shop/orders/order", "orders.xsd"));
    let lines = form.fields[2].inside.as_ref().unwrap();
    assert_eq!((lines.entity.as_str(), lines.parent.as_str()), ("Line", "order"));
    assert_eq!(form.options[0].value, "true");
}

/// A form leaves out what is empty: a field without a path is found by its
/// name, an option without a value is the format's default; a bool option
/// is written bare, a text option in quotes.
#[test]
fn a_form_leaves_out_what_is_empty() {
    use clarity_spec::code::mapping::{MappedFieldForm, MappingForm, MappingOptionForm, text};
    let form = MappingForm {
        name: "OrderFeed".into(),
        entity: "Order".into(),
        format: "xml".into(),
        at: String::new(),
        fields: vec![
            MappedFieldForm { field: "number".into(), at: String::new(), inside: None },
            MappedFieldForm { field: "customer".into(), at: "customer/\"name\"".into(), inside: None },
        ],
        check: String::new(),
        options: vec![MappingOptionForm { name: "indent".into(), value: "true".into() }, MappingOptionForm { name: "root".into(), value: String::new() }],
    };
    assert_eq!(text(&form), "mapping OrderFeed of Order from xml\n    customer at \"customer/\\\"name\\\"\"\n    indent true\n");
    let csv = MappingForm { format: "csv".into(), options: vec![MappingOptionForm { name: "separator".into(), value: ";".into() }], fields: Vec::new(), ..form };
    assert_eq!(text(&csv), "mapping OrderFeed of Order from csv\n    separator \";\"\n");
}

/// The options object every stack passes for a mapping, keyed by field names.
#[test]
fn a_mapping_is_one_options_object() {
    let (file, _) = parse(ORDERS, "orders.clarity");
    assert_eq!(
        clarity_spec::code::mapping::options_json(&file.mappings[0]),
        r#"{"at":"shop/orders/order","fields":{"number":"@number","customer":"customer/name"},"nested":{"lines":{"at":"lines/line","fields":{"product":"@product"},"parent":"order"}},"indent":true}"#
    );
}

/// Saving the mapping page replaces the one declaration, every other line of
/// the module as written; a new mapping goes at the end.
#[test]
fn a_saved_mapping_keeps_the_rest_of_the_module() {
    use clarity_spec::code::mapping::{form_of, text};
    use clarity_spec::edit::{Edit, apply};
    let (file, _) = parse(ORDERS, "orders.clarity");
    let mut form = form_of(&file.mappings[1]);
    form.fields[0].at = "Name".into();
    form.options.clear();
    let item = file.mappings[1].span.start;
    let saved = apply(ORDERS, &Edit::ReplaceDecl { item, text: text(&form) }).unwrap().text;
    assert_eq!(saved, ORDERS.replacen("    name at \"Full name\"\n    separator \";\"\n", "    name at \"Name\"\n", 1));
    let added = apply(ORDERS, &Edit::AppendDecl { text: "mapping People of Contact from json\n".into() }).unwrap().text;
    assert!(added.ends_with("\n\nmapping People of Contact from json\n"), "{added}");
    assert_eq!(errors(&added), Vec::<String>::new());
}

/// Deleting a mapping removes its declaration and nothing else.
#[test]
fn a_mapping_is_removed_as_one_declaration() {
    use clarity_spec::edit::{Edit, apply};
    let (file, _) = parse(ORDERS, "orders.clarity");
    let item = file.mappings[1].span.start;
    let removed = apply(ORDERS, &Edit::Remove { item }).unwrap().text;
    let (left, _) = parse(&removed, "orders.clarity");
    assert_eq!(left.mappings.iter().map(|m| m.name.name.as_str()).collect::<Vec<_>>(), ["OrderFeed"]);
    assert!(!removed.contains("Full name") && removed.contains("function contactsIn"), "{removed}");
}
