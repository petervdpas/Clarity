//! Generates the shop example and builds it with cargo. Slow (it
//! compiles SQLite and Tauri the first time), so it only runs when asked:
//! `cargo test -p clarity-gen --test build -- --ignored`.

use std::path::Path;
use std::process::Command;

#[test]
#[ignore]
fn the_shop_example_builds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("examples/shop.clarity");
    let project = clarity_spec::load_dir(&clarity_project::module_dir(&file)).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the example has a project file");
    let files = clarity_gen::generate(&project, &settings).unwrap();

    let out = std::env::temp_dir().join(format!("clarity-gen-build-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["target"]).unwrap();

    // One target folder for every run, so SQLite compiles only once.
    let target = root.join("target/generated");
    let status = Command::new("cargo")
        .args(["build", "--quiet"])
        .current_dir(&out)
        .env("CARGO_TARGET_DIR", &target)
        .status()
        .unwrap();
    assert!(status.success(), "the generated project does not build; it is in {}", out.display());

    // The application opens a window, so it is built here, not started.
    assert!(target.join("debug/shop").exists());
    std::fs::remove_dir_all(&out).unwrap();
}

const SALES: &str = "module sales

enum OrderStatus
    Pending
    Approved

entity Customer
    name: text(80), required
    credit_limit: money, default 100

entity Order
    number: text(12), unique
    status: OrderStatus
    customer: ref Customer
    total: money

function within_credit(order: Order) -> bool
    return order.total <= order.customer.credit_limit

function label(order: Order) -> text
    return \"Order \" + order.number + \" for \" + order.customer.name

function orders_of(customer: Customer) -> list<Order>
    return find o in Order where o.customer == customer and o.total > 50 order by o.total desc

function named(part: text) -> list<Customer>
    return find c in Customer where c.name contains part order by c.name

action approve(order: Order)
    require order.status == Pending
    require within_credit(order)
    effects db.write
    order.status = Approved
    save order

page Orders: window
    data orders: list<Order>
    table name grid at 1,1 span 12 rows 4 bind orders columns number, total
";

/// Uses the generated functions the way a handler will.
const LOGIC_TEST: &str = r#"
#[path = "../src/sales.rs"]
mod sales;
#[path = "../src/schema.rs"]
mod schema;

use clarity_runtime::{Db, Decimal, Error};
use sales::*;

#[test]
fn approves_within_the_credit_limit() {
    let mut db = Db::memory().unwrap();
    db.migrate(schema::STEPS).unwrap();
    let mut ada = Customer { name: "Ada".into(), ..Customer::default() };
    db.save(&mut ada).unwrap();
    assert_eq!(ada.credit_limit, Decimal::new(100, 0));

    let mut order = Order { number: "ORD-1".into(), customer: Some(ada.id), total: Decimal::new(9950, 2), ..Order::default() };
    db.save(&mut order).unwrap();
    assert_eq!(label(&db, &order).unwrap(), "Order ORD-1 for Ada");
    approve(&db, &mut order).unwrap();
    assert_eq!(db.get::<Order>(order.id).unwrap().status, OrderStatus::Approved);

    // Approving twice breaks the first require; a big order the second.
    assert_eq!(approve(&db, &mut order), Err(Error::Contract("order.status == Pending".into())));
    let mut big = Order { customer: Some(ada.id), total: Decimal::new(500, 0), ..Order::default() };
    assert_eq!(approve(&db, &mut big), Err(Error::Contract("within_credit(order)".into())));

    // Queries: the orders of a customer over 50, biggest first.
    let mut grace = Customer { name: "Grace".into(), ..Customer::default() };
    db.save(&mut grace).unwrap();
    for (customer, total) in [(ada.id, 2000), (ada.id, 120000), (grace.id, 90000)] {
        let mut o = Order { customer: Some(customer), total: Decimal::new(total, 2), ..Order::default() };
        db.save(&mut o).unwrap();
    }
    let totals: Vec<String> = orders_of(&db, &ada).unwrap().iter().map(|o| o.total.to_string()).collect();
    assert_eq!(totals, ["1200.00", "99.50"]);
    let names: Vec<String> = named(&db, String::from("RAC")).unwrap().into_iter().map(|c| c.name).collect();
    assert_eq!(names, ["Grace"]);
}
"#;

#[test]
#[ignore]
fn generated_logic_runs() {
    let project = clarity_spec::load_sources([("sales.clarity", SALES)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = clarity_gen::generate(&project, &clarity_project::starter("Sales", "sales")).unwrap();
    let out = std::env::temp_dir().join(format!("clarity-gen-logic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["target"]).unwrap();
    std::fs::create_dir_all(out.join("tests")).unwrap();
    std::fs::write(out.join("tests/logic.rs"), LOGIC_TEST).unwrap();

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("cargo")
        .args(["test", "--quiet"])
        .current_dir(&out)
        .env("CARGO_TARGET_DIR", root.join("target/generated"))
        .output()
        .unwrap();
    let report = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{report}\nthe project is in {}", out.display());
    assert!(report.contains("1 passed") || report.contains("test result: ok"), "{report}");
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn write_out_keeps_unchanged_files_and_removes_stale_ones() {
    let dir = std::env::temp_dir().join(format!("clarity-write-out-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = |path: &str, text: &str| clarity_gen::OutFile {
        path: path.into(),
        text: text.into(),
        bytes: None,
    };
    let first = [file("a.txt", "a"), file("src/b.txt", "b")];
    assert_eq!(clarity_gen::write_out(&dir, &first, &["target"]).unwrap(), 2);
    std::fs::create_dir_all(dir.join("target")).unwrap();
    std::fs::write(dir.join("target/keep.txt"), "built").unwrap();

    let second = [file("a.txt", "a"), file("c.txt", "c")];
    assert_eq!(clarity_gen::write_out(&dir, &second, &["target"]).unwrap(), 1);
    assert!(!dir.join("src").exists(), "the stale folder is gone");
    assert!(dir.join("target/keep.txt").exists(), "kept folders stay");
    assert_eq!(std::fs::read_to_string(dir.join("c.txt")).unwrap(), "c");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn write_out_never_writes_into_a_folder_with_other_work() {
    let dir = std::env::temp_dir().join(format!("clarity-foreign-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("thesis.txt"), "years of work").unwrap();
    let files = [clarity_gen::OutFile {
        path: "a.txt".into(),
        text: "a".into(),
        bytes: None,
    }];
    let err = clarity_gen::write_out(&dir, &files, &[]).unwrap_err();
    assert!(err.to_string().contains("not made by Clarity"), "{err}");
    assert_eq!(std::fs::read_to_string(dir.join("thesis.txt")).unwrap(), "years of work");
    assert!(!dir.join("a.txt").exists());

    // An empty folder, and Clarity's own output, are fine.
    let empty = dir.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    clarity_gen::write_out(&empty, &files, &[]).unwrap();
    assert!(empty.join(clarity_gen::OUT_MARKER).exists());
    clarity_gen::write_out(&empty, &files, &[]).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}
