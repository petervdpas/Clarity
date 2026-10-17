use clarity_spec::code::{BinaryOp, ExprKind, MessageLevel, StmtKind};
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_sources};

const BASE: &str = "module shop

enum CustomerKind
    Private
    Business

entity Customer
    name: text(80), required
    kind: CustomerKind
    vat_number: text(14)
    credit_limit: money
    active: bool
    since: date

entity Order
    number: text(10)
    customer: ref Customer
    total: money
    lines: integer

page CustomerForm: window
    data customer: Customer
    dropdown name kind at 1,1 span 4 bind customer.kind on change: kind_changed
    textbox name vat_number at 1,2 span 4 bind customer.vat_number
    button \"Save\" name save at 1,3 span 2 on click: save_clicked

page Orders: window
    data orders: list<Order>
    table name grid at 1,1 span 12 rows 6 bind orders columns number, total on open: grid_opened
";

const CODE: &str = "
code CustomerForm
    handler kind_changed
        vat_number.visible = customer.kind == Business

    handler save_clicked
        if customer.name == \"\"
            message error \"Name is required\"
            return
        else if customer.credit_limit < 0
            message warning \"The credit limit is negative\"
        else
            customer.active = true
        let doubled = customer.credit_limit * 2
        customer.credit_limit = doubled + 1
        save customer
        message \"Saved \" + customer.name
        close

code Orders
    handler grid_opened
        let order = grid.selected
        open CustomerForm with customer = order.customer
";

/// All problems of BASE plus `code`, as (severity, message).
fn problems(code: &str) -> Vec<(Severity, String)> {
    let source = format!("{BASE}{code}");
    load_sources([("shop.clarity", source.as_str())])
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

fn errors(code: &str) -> Vec<String> {
    problems(code)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect()
}

fn warnings(code: &str) -> Vec<String> {
    problems(code)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Warning)
        .map(|(_, m)| m)
        .collect()
}

/// One handler for CustomerForm with `body` (indented by 8) plus the other
/// wired handlers, so only the problems of `body` remain.
fn form(body: &str) -> String {
    let body: String = body.lines().map(|l| format!("        {l}\n")).collect();
    format!(
        "
code CustomerForm
    handler kind_changed
{body}
    handler save_clicked
        close

code Orders
    handler grid_opened
        close
"
    )
}

#[test]
fn valid_code_has_no_problems() {
    assert_eq!(problems(CODE), vec![]);
}

#[test]
fn parses_handlers_and_statements() {
    let source = format!("{BASE}{CODE}");
    let (file, diags) = parse(&source, "shop.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(file.code.len(), 2);
    let form = &file.code[0];
    assert_eq!(form.name.name, "CustomerForm");
    let names: Vec<_> = form.handlers.iter().map(|h| h.name.name.as_str()).collect();
    assert_eq!(names, ["kind_changed", "save_clicked"]);

    let body = &form.handlers[1].body;
    let StmtKind::If { branches, otherwise } = &body[0].kind else {
        panic!("expected if, got {:?}", body[0].kind)
    };
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0].body.len(), 2);
    assert!(matches!(branches[0].body[0].kind, StmtKind::Message { level: MessageLevel::Error, .. }));
    assert!(matches!(branches[0].body[1].kind, StmtKind::Return { value: None }));
    assert_eq!(otherwise.as_ref().map(Vec::len), Some(1));
    assert!(matches!(body[1].kind, StmtKind::Let { .. }));
    assert!(matches!(body[2].kind, StmtKind::Assign { .. }));
    assert!(matches!(body[3].kind, StmtKind::Save { .. }));
    assert!(matches!(body[4].kind, StmtKind::Message { level: MessageLevel::Info, .. }));
    assert!(matches!(body[5].kind, StmtKind::Close));

    let StmtKind::Open { window, with } = &file.code[1].handlers[0].body[1].kind else {
        panic!("expected open")
    };
    assert_eq!(window.name, "CustomerForm");
    assert_eq!(with[0].name.name, "customer");
}

fn let_value(code: &str) -> ExprKind {
    let source = format!("{BASE}{}", form(code));
    let (file, diags) = parse(&source, "shop.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    match &file.code[0].handlers[0].body[0].kind {
        StmtKind::Let { value, .. } => value.kind.clone(),
        other => panic!("expected let, got {other:?}"),
    }
}

fn binary(kind: &ExprKind) -> (BinaryOp, &ExprKind, &ExprKind) {
    match kind {
        ExprKind::Binary { op, left, right } => (*op, &left.kind, &right.kind),
        other => panic!("expected a binary expression, got {other:?}"),
    }
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    let value = let_value("let x = 1 + 2 * 3");
    let (op, left, right) = binary(&value);
    assert_eq!(op, BinaryOp::Add);
    assert!(matches!(left, ExprKind::Number { .. }));
    assert_eq!(binary(right).0, BinaryOp::Mul);
}

#[test]
fn comparison_binds_tighter_than_and_and_or() {
    let value = let_value("let x = 1 < 2 and 3 > 2 or false");
    let (op, left, _) = binary(&value);
    assert_eq!(op, BinaryOp::Or);
    let (op, left, right) = binary(left);
    assert_eq!(op, BinaryOp::And);
    assert_eq!(binary(left).0, BinaryOp::Lt);
    assert_eq!(binary(right).0, BinaryOp::Gt);
}

#[test]
fn minus_after_a_value_subtracts() {
    for code in ["let x = 5 -1", "let x = 5 - 1", "let x = 5-1"] {
        let value = let_value(code);
        assert_eq!(binary(&value).0, BinaryOp::Sub, "{code}");
    }
    assert!(matches!(let_value("let x = -1"), ExprKind::Number { .. }));
    assert!(errors(&form("let x = 2 * -1\nlet y = -x")).is_empty());
}

#[test]
fn parentheses_group() {
    let value = let_value("let x = (1 + 2) * 3");
    let (op, left, _) = binary(&value);
    assert_eq!(op, BinaryOp::Mul);
    assert_eq!(binary(left).0, BinaryOp::Add);
}

#[test]
fn reports_syntax_errors() {
    assert_eq!(errors(&form("else\n    close")), ["'else' without 'if'"]);
    assert_eq!(errors(&form("let = 1")), ["expected a name"]);
    assert_eq!(errors(&form("let x = 1 +")), ["expected a value"]);
    assert_eq!(errors(&form("customer.name")), ["expected a statement, like 'x = value'"]);
    assert_eq!(errors(&form("1 = 2")), ["only a local, a data field or a part property can be set"]);
    assert_eq!(errors(&form("close\n    return")), ["unexpected indented block"]);
    assert_eq!(errors(&form("if true")), ["'if' needs an indented block"]);
}

#[test]
fn reports_unknown_names_and_fields() {
    assert_eq!(errors(&form("custmer.name = \"x\"")), ["unknown name 'custmer'"]);
    assert_eq!(errors(&form("customer.nme = \"x\"")), ["Customer has no field 'nme'"]);
    assert_eq!(errors(&form("save.colour = \"x\"")), ["button 'save' has no property 'colour'"]);
    assert_eq!(errors(&form("let x = y")), ["unknown name 'y'"]);
}

#[test]
fn locals_are_scoped_to_their_block() {
    assert_eq!(errors(&form("if true\n    let x = 1\nlet y = x")), ["unknown name 'x'"]);
    assert_eq!(errors(&form("let x = 1\nlet x = 2")), ["'x' is already defined"]);
    assert_eq!(errors(&form("let customer = 1")), ["'customer' is already a name in this window"]);
}

#[test]
fn checks_types() {
    assert_eq!(errors(&form("if customer.name\n    close")), ["the condition must be bool, not text"]);
    assert_eq!(errors(&form("customer.credit_limit = \"lots\"")), ["cannot set money to text"]);
    assert_eq!(errors(&form("customer.active = 1")), ["cannot set bool to integer"]);
    assert_eq!(errors(&form("let x = customer.name * 2")), ["'*' needs numbers, not text and integer"]);
    assert_eq!(errors(&form("let x = customer.active and 1")), ["'and' needs bool, not integer"]);
    assert_eq!(errors(&form("let x = customer.name < 3")), ["cannot compare text with integer"]);
    // Integers widen to decimals, and text joins any value.
    assert_eq!(errors(&form("customer.credit_limit = 5\nlet t = \"Limit: \" + customer.credit_limit")), Vec::<String>::new());
}

#[test]
fn enum_values_take_their_type_from_the_other_side() {
    assert!(errors(&form("customer.kind = Business\nlet k = CustomerKind.Private")).is_empty());
    assert_eq!(errors(&form("customer.kind = Wholesale")), ["CustomerKind has no value 'Wholesale'"]);
    assert_eq!(errors(&form("let k = Business")), ["unknown name 'Business'"]);
    assert!(errors(&form("let ok = customer.kind in [Private, Business]")).is_empty());
}

#[test]
fn checks_part_properties() {
    assert!(errors(&form("save.caption = \"Store\"\nsave.enabled = customer.active")).is_empty());
    assert_eq!(errors(&form("vat_number.caption = \"x\"")), ["textbox 'vat_number' has no property 'caption'"]);
    let orders = "
code CustomerForm
    handler kind_changed
        close
    handler save_clicked
        close

code Orders
    handler grid_opened
        grid.selected = grid.selected
        let n = count(orders) + grid.selected.lines
";
    assert_eq!(errors(orders), ["'selected' of table 'grid' can only be read"]);
}

#[test]
fn checks_open_and_records() {
    assert_eq!(errors(&form("open Nowhere")), ["unknown window 'Nowhere'"]);
    assert_eq!(errors(&form("open Orders with customer = customer")), ["Orders has no data 'customer'"]);
    assert_eq!(
        errors(&form("open CustomerForm with customer = customer.name")),
        ["'customer' of CustomerForm is a Customer, not text"]
    );
    assert_eq!(errors(&form("save customer.name")), ["'save' needs a record, not text"]);
    assert_eq!(errors(&form("message customer")), ["a message is text or a value, not Customer"]);
}

#[test]
fn wiring_and_code_must_match() {
    let missing = "
code CustomerForm
    handler kind_changed
        close
";
    assert_eq!(
        warnings(missing),
        ["handler 'save_clicked' is wired but not written yet", "handler 'grid_opened' is wired but not written yet"]
    );
    let extra = format!("{CODE}\ncode Orders\n    handler nobody\n        close\n");
    assert_eq!(errors(&extra), ["there is already a code block for Orders"]);
    let unwired = CODE.replace("code Orders\n", "code Orders\n    handler nobody\n        close\n");
    assert_eq!(warnings(&unwired), ["handler 'nobody' is not wired to any event"]);
    let twice = CODE.replace("    handler grid_opened\n", "    handler grid_opened\n        close\n    handler grid_opened\n");
    assert_eq!(errors(&twice), ["handler 'grid_opened' is written twice"]);
    assert_eq!(errors("\ncode Missing\n    handler x\n        close\n"), ["there is no window, app or entity 'Missing' in this file"]);
}

#[test]
fn a_part_and_data_may_not_share_a_name() {
    let source = format!(
        "{}{CODE}",
        BASE.replace("textbox name vat_number", "textbox name customer")
            .replace("vat_number.visible", "customer.visible")
    );
    let errors: Vec<String> = load_sources([("shop.clarity", source.as_str())])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect();
    assert!(errors.contains(&"'customer' is both data and a part of CustomerForm".to_string()), "{errors:?}");
}

#[test]
fn app_code_handles_its_frame_commands() {
    let app = "
app Shop
    main CustomerForm
    menu
        item \"Quit\" name quit on click: quit_clicked

code Shop
    handler quit_clicked
        quit.enabled = false
        close
";
    let code = format!("{app}{}", CODE);
    assert_eq!(problems(&code), vec![]);
}
