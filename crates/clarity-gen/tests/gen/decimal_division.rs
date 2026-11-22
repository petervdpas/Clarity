//! `a / b` of decimals goes through each runtime's division, which divides as
//! every stack does and makes dividing by zero a value error
//! (conformance/decimal-division.json).

const SHOP: &str = "module shop

entity Item
    price: decimal(12, 2), label @itemPrice
    parts: integer, label @itemParts

function each(item: Item) -> decimal
    return item.price / item.parts

page Items: window
    title @itemsTitle
    data current: Item
";

#[test]
fn every_stack_divides_decimals_through_its_runtime() {
    let project = clarity_spec::load_converted([("shop/shop.clarity", SHOP)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    for (stack, divides) in [
        ("rust desktop", "clarity_runtime::decimal_div("),
        ("go desktop", "clarity.DecimalDiv("),
        ("csharp desktop", "Clarity.Runtime.Values.Divide("),
        ("c desktop", "clarity_decimal_div("),
        ("cpp desktop", " / "),
    ] {
        let settings = clarity_project::parse(&format!("project Shop\n    stack {stack}\n    datasource main sqlite \"data/shop.db\"\n")).unwrap();
        let files: Vec<clarity_gen::OutFile> = clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{stack}: {e:#?}")).into_iter().flat_map(|p| p.files).collect();
        let all: String = files.iter().map(|f| f.text.clone()).collect::<Vec<_>>().join("\n");
        assert!(all.contains(divides), "{stack}: the division does not go through {divides}");
    }
}
