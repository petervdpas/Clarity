//! A record holds every field of its entity on every stack, so its JSON (for
//! parts, reports and the wire) carries the same names everywhere
//! (docs/ledger/modularity.md, map: Go left out lists that do not load).

const SHOP: &str = "module shop

entity Order
    number: text, label @orderNumber
    lines: list<Line>, owned, label @orderLines
    tags: list<Tag>, label @orderTags

entity Line
    order: ref Order, label @lineOrder
    product: text, label @lineProduct

entity Tag
    name: text, label @tagName

page Orders: window
    title @ordersTitle
    data current: Order
";

#[test]
fn every_stack_s_record_holds_every_field_of_its_entity() {
    let project = clarity_spec::load_converted([("shop/shop.clarity", SHOP)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    // `lines` loads with the record (one ref back); `tags` does not (Tag refers to no Order), yet the record holds both.
    for (stack, fields) in [
        ("rust desktop", ["lines: Vec<Line>", "tags: Vec<Tag>"]),
        ("go desktop", ["Lines []Line `json:\"lines\"`", "Tags []Tag `json:\"tags\"`"]),
        ("csharp desktop", ["List<Line> Lines", "List<Tag> Tags"]),
        ("c desktop", ["LineList lines;", "TagList tags;"]),
        ("cpp desktop", ["std::vector<Line> lines", "std::vector<Tag> tags"]),
    ] {
        let settings = clarity_project::parse(&format!("project Shop\n    stack {stack}\n    datasource main sqlite \"data/shop.db\"\n")).unwrap();
        let files: Vec<clarity_gen::OutFile> = clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{stack}: {e:#?}")).into_iter().flat_map(|p| p.files).collect();
        // Runs of spaces as one, as Go lines up its struct's columns.
        let all: String = files.iter().map(|f| f.text.split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n");
        for field in fields {
            assert!(all.contains(field), "{stack}: the record does not hold {field}");
        }
    }
}
