//! Generates the Rust examples on their own stack, and the test projects of
//! tests/fixtures and conformance as Rust, and builds them with cargo. Slow (it
//! compiles SQLite and Tauri the first time), so it only runs when asked,
//! one at a time:
//! `cargo test -p clarity-gen --test build -- --ignored --test-threads=1 --exact <name>`.
//!
//! The projects build on the runtime in runtimes/rust through a
//! `.cargo/config.toml` patch (the switch a developer of the runtime uses),
//! so a change to the runtime is tested before it is published; one test
//! builds on the crate from crates.io, as a project outside this repository
//! does.

use std::path::Path;
use std::process::Command;

use clarity_project::SourceKind;

mod served;

/// Where builds go: target/generated in the repository, on disk; never the
/// system's temporary folder, which may live in memory.
fn scratch() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated");
    std::fs::create_dir_all(&dir).expect("target/generated can be made");
    dir
}

/// Which runtime a generated project builds on.
#[derive(Clone, Copy, PartialEq)]
enum Runtime {
    /// The runtime in this repository, through a patch.
    Local,
    /// The published crate.
    Published,
}

/// Points the project in `out` at the runtime in this repository, as
/// `.cargo/config.toml` does for a developer of the runtime.
fn patch(out: &Path) {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust").canonicalize().unwrap();
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    std::fs::write(out.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", runtime.display().to_string())).unwrap();
}

/// Generates the example `name` (examples/<name>.clarity) as Rust with
/// Tauri and builds it with cargo, on `runtime`.
fn builds_with_cargo(name: &str, runtime: Runtime) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join(format!("examples/{name}.clarity"));
    let project = clarity_spec::load_dir(&clarity_project::module_dir(&file)).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the example has a project file");
    // An example builds on the stack its project file names, and no other.
    assert_eq!((settings.stack.backend, settings.stack.ui), (clarity_project::Backend::Rust, clarity_project::Ui::Web), "{name} is not a Rust example");
    let files = clarity_gen::generate(&project, &settings).unwrap();

    // The runtime is part of the name: the same example is built on both,
    // by two tests that run beside each other and may not share a folder.
    let on = if runtime == Runtime::Local { "local" } else { "published" };
    let out = scratch().join(format!("clarity-gen-build-{name}-{on}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["target"]).unwrap();
    if runtime == Runtime::Local {
        patch(&out);
    }

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
    let program = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    assert!(target.join("debug").join(&program).exists(), "no {program}");
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore]
fn the_shop_example_builds() {
    builds_with_cargo("shop", Runtime::Local);
}

/// The shop on the runtime crate from crates.io, at the version the
/// generator requires: what a project outside this repository builds on.
#[test]
#[ignore]
fn the_shop_example_builds_on_the_published_crate() {
    builds_with_cargo("shop", Runtime::Published);
}

/// The shop example on each server database: the generated project turns
/// on that database's driver in the runtime and still builds.
#[test]
#[ignore]
fn the_shop_example_builds_for_every_database() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("examples/shop.clarity");
    let project = clarity_spec::load_dir(&clarity_project::module_dir(&file)).unwrap();
    let target = root.join("target/generated");
    for (kind, feature) in [
        (SourceKind::Postgres, "\"postgres\""),
        (SourceKind::Mysql, "\"mysql\""),
        (SourceKind::Mssql, "\"mssql\""),
    ] {
        let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
        settings.datasources[0].kind = kind;
        settings.datasources[0].url = "env:SHOP_DATABASE".into();
        let files = clarity_gen::generate(&project, &settings).unwrap();
        let cargo = files.iter().find(|f| f.path == "Cargo.toml").unwrap();
        // The database's driver, beside the engines the shop has.
        assert!(cargo.text.contains(&format!("features = [\"engines\", {feature}]")), "{}", cargo.text);

        let out = scratch().join(format!("clarity-gen-build-{feature}-{}", std::process::id()).replace('"', ""));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        clarity_gen::write_out(&out, &files, &["target"]).unwrap();
        patch(&out);
        let status = Command::new("cargo")
            .args(["build", "--quiet"])
            .current_dir(&out)
            .env("CARGO_TARGET_DIR", &target)
            .status()
            .unwrap();
        assert!(status.success(), "the generated project does not build; it is in {}", out.display());
        std::fs::remove_dir_all(&out).unwrap();
    }
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

code Order
    before save
        if order.total < 0
            refuse \"An order cannot have a negative total\"

    before delete
        if order.status == Approved
            refuse \"Order {order.number} is approved\"
";

/// SALES as two steps: the first had a `limit`, an integer total and a note.
const STEPS: [(&str, &str); 2] = [
    (
        "0001.clarity",
        "migration \"First\"
    add entity Customer
        name: text(80)
        limit: money, default 100
    add entity Order
        number: text(12)
        status: OrderStatus
        customer: ref Customer
        total: integer
        note: text
",
    ),
    (
        "0002.clarity",
        "migration \"Tidy up\"
    rename field Customer.limit to credit_limit
    change field Order.total: money
    remove field Order.note
    add unique Order.number
",
    ),
];

/// Uses the generated functions the way a handler will.
const LOGIC_TEST: &str = r#"
#[path = "../src/modules/sales.rs"]
mod sales;
#[path = "../src/schema.rs"]
mod schema;

use clarity_runtime::{Db, Decimal, Error};
use sales::*;

#[test]
fn approves_within_the_credit_limit() {
    let mut db = Db::memory().unwrap();
    db.migrate(schema::SOURCES[0].1).unwrap();
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

    // Data events: an approved order stays, a negative total is never saved.
    // A refusal keeps its slots for the screen, and reads plainly elsewhere.
    let Err(Error::Refused(refusal)) = db.delete(&order) else { panic!("the approved order was deleted") };
    assert_eq!(refusal.text, "Order {order.number} is approved");
    assert_eq!(refusal.show(), "Order ORD-1 is approved");
    let mut negative = Order { customer: Some(ada.id), total: Decimal::new(-1, 0), ..Order::default() };
    assert_eq!(db.save(&mut negative), Err(Error::Refused("An order cannot have a negative total".into())));
    assert_eq!(negative.id, 0);

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
    // The database is built by the two steps, so the SQL of a rename, a
    // rebuild and a unique index runs on SQLite.
    let project = clarity_spec::load_with_migrations([("sales.clarity", SALES)], STEPS);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = clarity_gen::generate(&project, &clarity_project::starter("Sales", "sales")).unwrap();
    let out = scratch().join(format!("clarity-gen-logic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["target"]).unwrap();
    patch(&out);
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
    let dir = scratch().join(format!("clarity-write-out-{}", std::process::id()));
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
    let dir = scratch().join(format!("clarity-foreign-{}", std::process::id()));
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

/// A build leaves its tools' folders behind. Finding only those, without
/// the marker, Clarity writes: they are its build's, not somebody else's.
#[test]
fn write_out_writes_where_only_the_build_tools_left_something() {
    let dir = scratch().join(format!("clarity-leftover-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("target/debug")).unwrap();
    std::fs::write(dir.join("target/debug/shop"), "a program from before").unwrap();
    let files = [clarity_gen::OutFile {
        path: "Cargo.toml".into(),
        text: "[package]".into(),
        bytes: None,
    }];
    clarity_gen::write_out(&dir, &files, &["target"]).unwrap();
    assert!(dir.join("Cargo.toml").exists());
    // What the build tools made is still there.
    assert_eq!(std::fs::read_to_string(dir.join("target/debug/shop")).unwrap(), "a program from before");
    // A folder the project keeps counts the same way.
    let own = scratch().join(format!("clarity-keeps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&own);
    std::fs::create_dir_all(own.join("vendor")).unwrap();
    let err = clarity_gen::write_out(&own, &files, &[]).unwrap_err();
    assert!(err.to_string().contains("not made by Clarity"), "{err}");
    clarity_gen::write_out(&own, &files, &["vendor"]).unwrap();
    assert!(own.join("vendor").is_dir());
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&own).unwrap();
}

/// The output folder holds one stack's project: switching a project from
/// Rust to C# leaves none of Rust's build output behind, and C#'s own
/// build output and the application's data stay between generations.
#[test]
fn the_output_folder_holds_only_the_current_stacks_project() {
    use clarity_project::Backend;
    let dir = scratch().join(format!("clarity-switch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = |path: &str| clarity_gen::OutFile {
        path: path.into(),
        text: "x".into(),
        bytes: None,
    };
    clarity_gen::write_out(&dir, &[file("Cargo.toml"), file("src/main.rs")], clarity_gen::build_output(Backend::Rust)).unwrap();
    for built in ["target/debug/shop", "gen/schemas/x.json", "Cargo.lock", "data/shop.db"] {
        std::fs::create_dir_all(dir.join(built).parent().unwrap()).unwrap();
        std::fs::write(dir.join(built), "built").unwrap();
    }

    clarity_gen::write_out(&dir, &[file("Shop.csproj"), file("Program.cs")], clarity_gen::build_output(Backend::Csharp)).unwrap();
    for gone in ["target", "gen", "Cargo.lock", "Cargo.toml", "src"] {
        assert!(!dir.join(gone).exists(), "{gone} is Rust's and should be gone");
    }
    assert!(dir.join("data/shop.db").exists(), "the application's data stays");

    std::fs::create_dir_all(dir.join("bin/Debug")).unwrap();
    std::fs::create_dir_all(dir.join("obj")).unwrap();
    std::fs::write(dir.join("bin/Debug/Shop.dll"), "built").unwrap();
    clarity_gen::write_out(&dir, &[file("Shop.csproj"), file("Program.cs")], clarity_gen::build_output(Backend::Csharp)).unwrap();
    assert!(dir.join("bin/Debug/Shop.dll").exists(), "C#'s build output stays");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The test engine of conformance/engines.json in a generated application:
/// every function called from Clarity code, its values crossing as JSON.
const ENGINE_SHOP: &str = include_str!("../../../conformance/engines/shop.clarity");
const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

const ENGINE_TEST: &str = r#"
#[path = "../src/modules/shop.rs"]
mod shop;
#[path = "../src/engines.rs"]
mod engines;

use clarity_runtime::ui::{Ui, WindowState};
use clarity_runtime::{Db, Decimal, Error};
use shop::*;

#[test]
fn every_engine_function_answers_from_clarity_code() {
    let db = Db::memory().unwrap();
    engines::check().unwrap();
    assert_eq!(answer(&db, 2).unwrap(), 42);
    assert_eq!(greeting(&db, "Grüße".into()).unwrap(), "Hello, Grüße!");
    let lines = vec![Line { price: Decimal::new(995, 2), count: 3, ..Line::default() }, Line { price: Decimal::new(10, 2), count: 1, ..Line::default() }];
    assert_eq!(sum(&db, &lines).unwrap(), Decimal::new(2995, 2));
    assert_eq!(size_class(&db, 12).unwrap(), Size::Large);
    let mut order = Order { id: 7, number: "ORD-7".into(), total: Decimal::new(10005, 2) };
    let cheaper = cheaper(&db, &mut order).unwrap();
    assert_eq!((cheaper.id, cheaper.total), (7, Decimal::new(9005, 2)));
    // The engine's own error reads as a refusal, in its words.
    let Err(Error::Refused(why)) = refused(&db, "no stock".into()) else { panic!("not refused") };
    assert_eq!(why.show(), "no stock");
    // Bytes cross to an engine and back as they are.
    assert_eq!(backwards(&db, vec![1, 2, 3]).unwrap(), vec![3, 2, 1]);
    assert_eq!(measured(&db, vec![1, 2, 3]).unwrap(), 6);
    // An engine with state counts on from what its on open set.
    assert_eq!((counted(&db).unwrap(), counted(&db).unwrap()), (11, 12));
    // Each open window has an instance of its own of an engine with state
    // per window, which goes when the window closes.
    let (first, second) = (WindowState::new("Main", Default::default()), WindowState::new("Main", Default::default()));
    let (mut a, mut b) = (MainWindow::default(), MainWindow::default());
    let (mut in_a, mut in_b) = (Ui::from_state(&first), Ui::from_state(&second));
    main__moved(&db, &mut a, &mut in_a).unwrap();
    main__moved(&db, &mut b, &mut in_b).unwrap();
    main__moved(&db, &mut a, &mut in_a).unwrap();
    assert_eq!((a.moves, b.moves), (12, 11));
    engines::close_window(first.instance);
    main__moved(&db, &mut a, &mut in_a).unwrap();
    main__moved(&db, &mut b, &mut in_b).unwrap();
    assert_eq!((a.moves, b.moves), (11, 12));
}
"#;

#[test]
#[ignore]
fn an_engine_answers_in_the_generated_application() {
    let mut project = clarity_spec::load_sources([("shop.clarity", ENGINE_SHOP)]);
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "test.wasm".into(), bytes: TEST_ENGINE.to_vec() }]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = clarity_gen::generate(&project, &clarity_project::starter("Shop", "shop")).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-engine-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["target"]).unwrap();
    patch(&out);
    std::fs::create_dir_all(out.join("tests")).unwrap();
    std::fs::write(out.join("tests/engine.rs"), ENGINE_TEST).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("cargo").args(["test", "--quiet"]).current_dir(&out).env("CARGO_TARGET_DIR", root.join("target/generated")).output().unwrap();
    let report = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{report}\nthe project is in {}", out.display());
    std::fs::remove_dir_all(&out).unwrap();
}

/// The noughts example's Board engine, written in Clarity, as the Rust it
/// becomes: its module built on the runtime alone and played.
const BOARD_TEST: &str = r#"
use clarity_runtime::{Db, Error};
use noughts_board::modules::noughts::*;

#[test]
fn each_window_plays_a_game_of_its_own() {
    clarity_runtime::i18n::load(include_str!("../model.json"));
    let db = Db::memory().unwrap();
    assert_eq!(board_play(&db, 1, 1).unwrap(), "Your move");
    assert_eq!(board_mark(&db, 1, 1).unwrap(), 1);
    // The computer took the centre.
    assert_eq!(board_mark(&db, 1, 5).unwrap(), 2);
    let Err(Error::Refused(why)) = board_play(&db, 1, 5) else { panic!("the centre was free") };
    assert_eq!(why.show(), "That square is taken");
    let Err(Error::Refused(why)) = board_play(&db, 1, 10) else { panic!("square 10 was played") };
    assert_eq!(why.show(), "There are nine squares");
    // Another window has a board of its own, and a closed window's board goes.
    assert_eq!(board_mark(&db, 2, 1).unwrap(), 0);
    board_close(1);
    assert_eq!(board_mark(&db, 1, 1).unwrap(), 0);
    // The computer blocks your line and takes its own.
    board_play(&db, 3, 1).unwrap();
    board_play(&db, 3, 2).unwrap();
    assert_eq!(board_mark(&db, 3, 3).unwrap(), 2);
    let mut said = String::new();
    for square in [4, 6, 7, 8, 9] {
        if board_mark(&db, 3, square).unwrap() == 0 {
            said = board_play(&db, 3, square).unwrap();
            break;
        }
    }
    assert_ne!(said, "");
    assert_eq!(board_restart(&db, 3).unwrap(), "Your move");
    assert_eq!(board_taken(&db, 3).unwrap(), 0);
}
"#;

#[test]
#[ignore]
fn an_engine_written_in_clarity_runs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("crates/clarity-gen/tests/fixtures/board.clarity");
    let project = clarity_spec::load_dir(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the fixture has a project file");
    settings.stack.backend = clarity_project::Backend::Rust;
    settings.stack.ui = clarity_project::Ui::Web;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-board-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.join("src/modules")).unwrap();
    std::fs::create_dir_all(out.join("tests")).unwrap();
    let file_text = |path: &str| files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text.clone();
    std::fs::write(out.join("src/modules/noughts.rs"), file_text("src/modules/noughts.rs")).unwrap();
    std::fs::write(out.join("model.json"), file_text("src/model.json")).unwrap();
    std::fs::write(out.join("src/lib.rs"), "pub mod modules {\n    pub mod noughts;\n}\n").unwrap();
    let runtime = root.join("runtimes/rust").canonicalize().unwrap();
    std::fs::write(
        out.join("Cargo.toml"),
        format!("[package]\nname = \"noughts-board\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nclarity-runtime = {{ path = {:?} }}\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\n\n[workspace]\n", runtime.display().to_string()),
    )
    .unwrap();
    std::fs::write(out.join("tests/board.rs"), BOARD_TEST).unwrap();
    let output = Command::new("cargo").args(["test", "--quiet"]).current_dir(&out).env("CARGO_TARGET_DIR", root.join("target/generated")).output().unwrap();
    let report = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{report}\nthe project is in {}", out.display());
    std::fs::remove_dir_all(&out).unwrap();
}

/// A window that asks each kind of question (DESIGN 5.14) builds with
/// cargo, on the runtime in this repository.
#[test]
#[ignore]
fn questions_and_their_answers_build_in_rust() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("fixtures/asks.clarity"))], &[], "en");
    let mut settings = clarity_project::starter("Music", "music");
    settings.stack.backend = clarity_project::Backend::Rust;
    settings.stack.ui = clarity_project::Ui::Web;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-build-asks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["target"]).unwrap();
    patch(&out);
    let target = root.join("target/generated");
    let status = Command::new("cargo").args(["build", "--quiet"]).current_dir(&out).env("CARGO_TARGET_DIR", &target).status().unwrap();
    assert!(status.success(), "the generated project does not build; it is in {}", out.display());
    std::fs::remove_dir_all(&out).unwrap();
}

/// The web target (DESIGN 7.3): a project whose only target is the web
/// builds into a web server that serves the renderer and answers its
/// commands, with what the desktop commands call.
#[test]
#[ignore]
fn the_web_target_serves_in_rust() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("fixtures/asks.clarity"))], &[], "en");
    let mut settings = clarity_project::starter("Music", "music");
    settings.stack.targets = vec![clarity_project::Target::Web];
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-build-web-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["target"]).unwrap();
    patch(&out);
    let target = root.join("target/generated");
    let status = Command::new("cargo").args(["build", "--quiet"]).current_dir(&out).env("CARGO_TARGET_DIR", &target).status().unwrap();
    assert!(status.success(), "the generated project does not build; it is in {}", out.display());
    let program = target.join("debug").join(if cfg!(windows) { "music.exe" } else { "music" });
    served::serves(&out, &program, &[], "127.0.0.1:18734");
    std::fs::remove_dir_all(&out).unwrap();
}
