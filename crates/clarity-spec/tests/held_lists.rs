//! A stored record's list of records (`order.lines`) loads with the record by
//! the one `ref Order` of Line, on every stack; code reads it, and saves each
//! listed record by itself.

const SHOP: &str = "module shop

entity Order
    number: text, label @orderNumber
    lines: list<Line>, owned, label @orderLines

entity Line
    order: ref Order, label @lineOrder
    product: text, label @lineProduct

action linesOf(o: Order) -> list<Line>
    return o.lines

action fill(o: Order, given: list<Line>)
    effects db.write
    o.lines = given
";

fn errors(text: &str) -> Vec<String> {
    let project = clarity_spec::load_in([("shop.clarity", text)], Vec::<(&str, &str)>::new(), &[]);
    project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn a_stored_record_s_list_is_read_by_its_one_ref_back_and_never_set() {
    let said = errors(SHOP);
    assert_eq!(said, vec!["'o.lines' comes from the database with its record: save each record of it by itself".to_string()], "only the setting is refused");
    let project = clarity_spec::load_in([("shop.clarity", SHOP)], Vec::<(&str, &str)>::new(), &[]);
    let (listed, key) = project.held_list_key("Order", "lines").expect("Line's one ref Order");
    assert_eq!((listed.name.name.as_str(), key.name.name.as_str()), ("Line", "order"));
}

#[test]
fn a_list_without_exactly_one_ref_back_cannot_be_read() {
    let none = SHOP.replace("    order: ref Order, label @lineOrder\n", "");
    assert!(errors(&none).iter().any(|m| m == "'o.lines' loads by the one field 'ref Order' of Line, which it needs exactly once"), "{:#?}", errors(&none));
    let two = SHOP.replace("    order: ref Order, label @lineOrder\n", "    order: ref Order, label @lineOrder\n    origin: ref Order, label @lineOrder\n");
    assert!(errors(&two).iter().any(|m| m.contains("which it needs exactly once")), "{:#?}", errors(&two));
}
