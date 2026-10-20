use clarity_spec::code::{FunctionKind, StmtKind};
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_sources};

const SALES: &str = "module sales

enum OrderStatus
    Pending
    Approved

entity Customer
    name: text(80)
    credit_limit: money

entity Line
    amount: money

entity Order
    status: OrderStatus
    customer: ref Customer
    lines: list<Line>

function order_total(order: Order) -> money
    ensure result >= 0
    let total: money = 0
    for line in order.lines
        total = total + line.amount
    return total

action approve(order: Order)
    require order.status == Pending
    require order_total(order) <= order.customer.credit_limit
    effects db.write
    order.status = Approved
    save order

page OrderForm: window
    data order: Order
    button \"Approve\" name approveButton at 1,1 span 2 on click: approveClicked

code OrderForm
    handler approveClicked
        approve(order)
        let total = order_total(order)
        message \"Total {total}\"
";

fn problems_of(sources: &[(&str, &str)]) -> Vec<(Severity, String)> {
    load_sources(sources.iter().copied())
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

fn errors(source: &str) -> Vec<String> {
    problems_of(&[("sales.clarity", source)])
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect()
}

/// SALES with `extra` declarations appended.
fn with(extra: &str) -> String {
    format!("{SALES}\n{extra}")
}

#[test]
fn valid_modules_have_no_problems() {
    assert_eq!(problems_of(&[("sales.clarity", SALES)]), vec![]);
}

#[test]
fn parses_functions_and_actions() {
    let (file, diags) = parse(SALES, "sales.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(file.functions.len(), 2);
    let total = &file.functions[0];
    assert_eq!(total.kind, FunctionKind::Function);
    assert_eq!(total.name.name, "order_total");
    assert_eq!(total.params.len(), 1);
    assert_eq!(total.params[0].name.name, "order");
    assert!(total.returns.is_some());
    assert_eq!(total.ensures.len(), 1);
    assert!(matches!(total.body[1].kind, StmtKind::For { .. }));
    assert!(matches!(total.body[2].kind, StmtKind::Return { value: Some(_) }));

    let approve = &file.functions[1];
    assert_eq!(approve.kind, FunctionKind::Action);
    assert!(approve.returns.is_none());
    assert_eq!(approve.requires.len(), 2);
    assert_eq!(approve.effects.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["db.write"]);
    assert!(matches!(file.code[0].handlers[0].body[0].kind, StmtKind::Call { .. }));
}

#[test]
fn functions_have_no_effects() {
    assert_eq!(
        errors(&with("function store(order: Order)\n    save order\n")),
        ["'save' is not allowed in a function; make it an action"]
    );
    assert_eq!(
        errors(&with("function twice(order: Order)\n    approve(order)\n")),
        ["function 'twice' cannot call action 'approve'"]
    );
    assert_eq!(
        errors(&with("function hello(order: Order)\n    message \"hi\"\n")),
        ["'message' belongs in a handler, not in a function"]
    );
}

#[test]
fn actions_declare_their_effects() {
    assert_eq!(
        errors(&SALES.replace("    effects db.write\n", "")),
        ["action 'approve' saves but does not declare 'effects db.write'"]
    );
    assert_eq!(
        errors(&SALES.replace("effects db.write", "effects db.wrte")),
        [
            "unknown effect 'db.wrte'; the effects are db.read, db.write, files.read and files.write",
            "action 'approve' saves but does not declare 'effects db.write'"
        ]
    );
    assert_eq!(
        errors(&with("action approve_all(order: Order)\n    effects db.read\n    approve(order)\n")),
        ["action 'approve_all' calls 'approve', which needs 'effects db.write'"]
    );
    assert_eq!(
        errors(&with("action shout(order: Order)\n    message \"hi\"\n")),
        ["'message' belongs in a handler, not in an action"]
    );
}

#[test]
fn checks_calls() {
    assert_eq!(
        errors(&SALES.replace("let total = order_total(order)", "let total = order_total(order, order)")),
        ["order_total takes 1 argument, not 2"]
    );
    assert_eq!(
        errors(&SALES.replace("let total = order_total(order)", "let total = order_total(order.status)")),
        ["argument 'order' of order_total is an Order, not OrderStatus"]
    );
    assert_eq!(
        errors(&SALES.replace("approve(order)\n", "nope(order)\n")),
        ["unknown function 'nope'"]
    );
    assert_eq!(
        errors(&SALES.replace("approve(order)\n", "let x = approve(order)\n")),
        ["approve does not return a value"]
    );
}

#[test]
fn checks_returns() {
    assert_eq!(
        errors(&with("function sign(n: integer) -> integer\n    if n < 0\n        return -1\n")),
        ["function 'sign' does not return a value on every path"]
    );
    assert!(errors(&with(
        "function sign(n: integer) -> integer\n    if n < 0\n        return -1\n    else\n        return 1\n"
    ))
    .is_empty());
    assert_eq!(
        errors(&with("action noop(order: Order)\n    return 1\n")),
        ["noop does not return a value; use 'return' alone"]
    );
    assert_eq!(
        errors(&with("function zero() -> integer\n    return\n")),
        ["zero must return integer"]
    );
    assert_eq!(
        errors(&with("function zero() -> integer\n    return \"0\"\n")),
        ["zero must return integer, not text"]
    );
}

#[test]
fn checks_loops_conditions_and_typed_locals() {
    assert_eq!(
        errors(&with("function f(order: Order) -> integer\n    for x in order\n        return 1\n    return 0\n")),
        ["'for' needs a list, not Order"]
    );
    assert_eq!(
        errors(&SALES.replace("require order.status == Pending", "require order.customer.credit_limit")),
        ["a condition must be bool, not money"]
    );
    assert_eq!(
        errors(&with("function f() -> integer\n    let x: integer = \"a\"\n    return x\n")),
        ["cannot set integer to text"]
    );
    assert_eq!(
        errors(&with("function f(order: Order) -> integer\n    order\n    return 0\n")),
        ["expected a statement, like 'x = value'"]
    );
    assert_eq!(
        errors(&with("function f(order: Order) -> integer\n    ensure result > order\n    return 0\n")),
        ["cannot compare integer with Order"]
    );
}

#[test]
fn names_are_unique_per_module() {
    assert_eq!(
        errors(&with("function order_total(order: Order) -> money\n    return 0\n")),
        ["function 'order_total' is declared twice"]
    );
}

#[test]
fn modules_call_each_other_through_use() {
    let desk = "module desk
use sales

page Desk: window
    data order: Order
    button \"Approve\" name go at 1,1 span 2 on click: goClicked

code Desk
    handler goClicked
        approve(order)
";
    assert_eq!(problems_of(&[("sales.clarity", SALES), ("desk.clarity", desk)]), vec![]);
    let unused = desk.replace("use sales\n", "");
    let errors: Vec<String> = problems_of(&[("sales.clarity", SALES), ("desk.clarity", &unused)])
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect();
    assert!(errors.contains(&"unknown function 'approve'".to_string()), "{errors:?}");
}

#[test]
fn records_types_and_names_for_generators() {
    use clarity_spec::code::check::{NameKind, TypeInfo, types};
    let (file, _) = parse(SALES, "sales.clarity");
    let table = &types(std::slice::from_ref(&file))["sales.clarity"];
    let approve = &file.functions[1];
    let StmtKind::Assign { target, value } = &approve.body[0].kind else { panic!() };
    assert_eq!(table.type_of(target), TypeInfo::Enum("OrderStatus".into()));
    assert_eq!(table.name_of(value), Some(&NameKind::EnumValue("OrderStatus".into())));
    let clarity_spec::code::ExprKind::Member { object, .. } = &target.kind else { panic!() };
    assert_eq!(table.name_of(object), Some(&NameKind::Local));
    assert_eq!(table.type_of(object), TypeInfo::Record("Order".into()));
    let handler = &file.code[0].handlers[0];
    let StmtKind::Message { text, .. } = &handler.body[2].kind else { panic!() };
    assert_eq!(table.type_of(text), TypeInfo::Text);
    // A slot's type reaches the generators like any expression's.
    let clarity_spec::code::ExprKind::Text { parts } = &text.kind else { panic!() };
    let clarity_spec::code::TextPart::Slot { value } = &parts[1] else { panic!() };
    assert_eq!(table.type_of(value), TypeInfo::Money);
}
