//! A stored record's list (order.lines) loads with the record on every
//! stack: its type holds the list, the stack's loading reads it one level
//! deep, and code reads it (docs/ledger/runtime-data.md, step 2).

mod ide_build;

const SHOP: &str = "module shop

entity Order
    number: text, label @orderNumber
    lines: list<Line>, owned, label @orderLines

entity Line
    order: ref Order, label @lineOrder
    product: text, label @lineProduct

page Orders: window
    title @ordersTitle
    data current: Order
    data count: integer
    button @countCaption name countButton at 1,1 span 2 on click: countClicked

code Orders
    handler countClicked
        count = 0
        for line in current.lines
            count = count + 1
";

const TRANSLATIONS: &str = "translation en\n    orderNumber \"Number\"\n    orderLines \"Lines\"\n    lineOrder \"Order\"\n    lineProduct \"Product\"\n    ordersTitle \"Orders\"\n    countCaption \"Count\"\n";

fn settings(stack: &str) -> clarity_project::Settings {
    clarity_project::parse(&format!("project Shop\n    stack {stack}\n    datasource main sqlite \"data/shop.db\"\n")).unwrap()
}

#[test]
fn every_stack_loads_a_stored_record_s_list_with_it() {
    let project = clarity_spec::load_converted([("shop/shop.clarity", SHOP)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    for (stack, field, loads) in [
        ("rust desktop", "lines: Vec<Line>", "fn load_lists(&mut self, db: &Db)"),
        ("go desktop", "Lines []Line", "func (r *Order) LoadLists(db *clarity.Db) error"),
        ("csharp desktop", "List<Line> Lines", "public void LoadLists(Db db)"),
        ("c desktop", "LineList lines;", "static bool order_load_lists(ClarityDb* db, Order* record)"),
        ("cpp desktop", "std::vector<Line> lines", "void Order::load_lists(clarity::Db& db)"),
    ] {
        let files: Vec<clarity_gen::OutFile> = clarity_gen::programs(&project, &settings(stack)).unwrap_or_else(|e| panic!("{stack}: {e:#?}")).into_iter().flat_map(|p| p.files).collect();
        // Runs of spaces as one, as Go lines up its struct's columns.
        let all: String = files.iter().map(|f| f.text.split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n");
        assert!(all.contains(field), "{stack}: the record holds no {field}");
        assert!(all.contains(loads), "{stack}: no {loads}");
    }
}

/// The project, written out and built whole on `stack`.
fn builds_without_warnings(stack: &str) {
    let name = format!("held-lists-{}", stack.split(' ').next().unwrap());
    let root = ide_build::repo().join("target/generated").join(&name);
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    for folder in ["translations", "migrations/main"] {
        std::fs::create_dir_all(dir.join(folder)).unwrap();
    }
    std::fs::write(dir.join("migrations/main/0001.clarity"), "migration \"Orders\"\n    add entity Order\n        number: text\n    add entity Line\n        order: ref Order\n        product: text\n").unwrap();
    for (path, text) in clarity_spec::converted([("shop.clarity", SHOP)]) {
        let at = dir.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
    }
    std::fs::write(dir.join("translations/en.clarity"), TRANSLATIONS).unwrap();
    let file = root.join("shop.clarity");
    std::fs::write(&file, format!("project Shop\n    title \"Shop\"\n    stack {stack}\n    output \"out\"\n    language \"en\"\n    datasource main sqlite \"data/shop.db\"\n")).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a held-lists project on rust desktop; run with --ignored, one at a time"]
fn a_rust_held_lists_project_builds_without_warnings() {
    builds_without_warnings("rust desktop");
}

#[test]
#[ignore = "builds a held-lists project on go desktop; run with --ignored, one at a time"]
fn a_go_held_lists_project_builds_without_warnings() {
    builds_without_warnings("go desktop");
}

#[test]
#[ignore = "builds a held-lists project on csharp desktop; run with --ignored, one at a time"]
fn a_csharp_held_lists_project_builds_without_warnings() {
    builds_without_warnings("csharp desktop");
}

#[test]
#[ignore = "builds a held-lists project on c desktop; run with --ignored, one at a time"]
fn a_c_held_lists_project_builds_without_warnings() {
    builds_without_warnings("c desktop");
}

#[test]
#[ignore = "builds a held-lists project on cpp desktop; run with --ignored, one at a time"]
fn a_cpp_held_lists_project_builds_without_warnings() {
    builds_without_warnings("cpp desktop");
}
