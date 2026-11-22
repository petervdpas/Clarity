//! `a + b`, `a - b`, `a * b` and `-a` of numbers go through each runtime's
//! checked arithmetic, which makes a whole number or decimal that does not
//! fit a value error (conformance/overflow.json); C++ decimals check in their
//! own operators.

const SHOP: &str = "module shop

entity Item
    price: decimal(12, 2), label @itemPrice
    parts: integer, label @itemParts

function total(item: Item) -> decimal
    return item.price * item.parts + item.price - 1

function spare(item: Item) -> integer
    return item.parts * 2 + item.parts - 1

function owed(item: Item) -> integer
    return -item.parts

page Items: window
    title @itemsTitle
    data current: Item
";

#[test]
fn every_stack_works_numbers_out_through_its_runtime() {
    let project = clarity_spec::load_converted([("shop/shop.clarity", SHOP)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    for (stack, checks) in [
        ("rust desktop", ["clarity_runtime::whole_mul(", "clarity_runtime::whole_add(", "clarity_runtime::whole_sub(", "clarity_runtime::whole_neg(", "clarity_runtime::decimal_mul(", "clarity_runtime::decimal_add(", "clarity_runtime::decimal_sub("]),
        ("go desktop", ["clarity.WholeMul(", "clarity.WholeAdd(", "clarity.WholeSub(", "clarity.WholeNeg(", "clarity.DecimalMul(", "clarity.DecimalAdd(", "clarity.DecimalSub("]),
        ("csharp desktop", ["Clarity.Runtime.Values.Multiply(", "Clarity.Runtime.Values.Add(", "Clarity.Runtime.Values.Subtract(", "Clarity.Runtime.Values.Negate(", "Clarity.Runtime.Values.Multiply(", "Clarity.Runtime.Values.Add(", "Clarity.Runtime.Values.Subtract("]),
        ("c desktop", ["clarity_whole_mul(", "clarity_whole_add(", "clarity_whole_sub(", "clarity_whole_neg(", "clarity_decimal_mul(", "clarity_decimal_add(", "clarity_decimal_sub("]),
        ("cpp desktop", ["clarity::whole_mul(", "clarity::whole_add(", "clarity::whole_sub(", "clarity::whole_neg(", " * ", " + ", " - "]),
    ] {
        let settings = clarity_project::parse(&format!("project Shop\n    stack {stack}\n    datasource main sqlite \"data/shop.db\"\n")).unwrap();
        let files: Vec<clarity_gen::OutFile> = clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{stack}: {e:#?}")).into_iter().flat_map(|p| p.files).collect();
        let all: String = files.iter().map(|f| f.text.clone()).collect::<Vec<_>>().join("\n");
        for check in checks {
            assert!(all.contains(check), "{stack}: the numbers do not go through {check}");
        }
    }
}
