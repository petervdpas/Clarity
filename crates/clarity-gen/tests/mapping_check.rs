//! A mapping's `check "orders.xsd"` (docs/FORMATS-PLAN.md, step 4): the
//! schema file is read with the project's folder and its text travels in
//! the options object every stack passes to Clarity.Formats, so no stack
//! reads files for the library. In code and on the file source alike.

use clarity_project::Backend;

const FEED: &str = "module feed\nsource feed\n\nentity Order\n    number: text(20), label @orderNumber\n    customer: text(100), label @orderCustomer\n\nmapping OrderFeed of Order from xml\n    at \"shop/orders/order\"\n    number at \"@number\"\n    customer at \"customer/name\"\n    check \"schemas/orders.xsd\"\n\nfunction ordersIn(text: text) -> list<Order>\n    return read list<Order> from text using OrderFeed\n";
const MAIN: &str = "module main\nuse feed\n\npage Orders: window\n    data orders: list<Order>\n    table name list at 1,1 span 4 bind orders\n";

/// A schema with what each stack's literal escapes (quotes, a line break,
/// `"#` that ends a Rust raw string) and long enough for C's pieces.
fn schema() -> String {
    let long = "a long note on the orders. ".repeat(800);
    format!("<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n<xs:annotation><xs:documentation>{long}</xs:documentation></xs:annotation>\n<xs:element name=\"shop\" id=\"#shop\"/>\n</xs:schema>\n")
}

fn project_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("mapping-check").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("schemas")).unwrap();
    for (path, text) in clarity_spec::converted([("main.clarity", MAIN), ("feed.clarity", FEED)]) {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), "translation en\n    orderNumber \"Number\"\n    orderCustomer \"Customer\"\n").unwrap();
    std::fs::write(dir.join("schemas/orders.xsd"), schema()).unwrap();
    dir
}

fn files(dir: &std::path::Path, backend: Backend) -> String {
    let sources = [clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef { external: true, mapping: Some("OrderFeed".into()), ..clarity_spec::SourceRef::sql("feed") }];
    let project = clarity_spec::load_dir_in(dir, &sources).unwrap();
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = clarity_project::parse("project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource feed file \"imports/orders.xml\" external using OrderFeed\n").unwrap();
    settings.stack.backend = backend;
    let out = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    out.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

#[test]
fn the_schema_text_is_in_the_options_on_every_stack() {
    let dir = project_dir("every-stack");
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
        let text = files(&dir, backend);
        // In code (`read ... using OrderFeed`) and on the file source: the schema's text, not its name.
        let plain = text.matches(r#""check":"<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n<xs:annotation>"#).count();
        let escaped = text.matches(r#"\"check\":\"<xs:schema xmlns:xs=\\\"http://www.w3.org/2001/XMLSchema\\\">\\n<xs:annotation>"#).count();
        assert!(plain + escaped >= 2, "{backend:?}: the schema's text in the options, in code and on the file source");
        assert!(!text.contains(r#"check\":\"schemas/orders.xsd"#) && !text.contains(r#""check":"schemas/orders.xsd"#), "{backend:?}: the schema's file name as the check");
        if matches!(backend, Backend::C | Backend::Cpp) {
            assert!(text.lines().all(|l| l.split("\" \"").all(|piece| piece.len() < 16380)), "{backend:?}: a literal longer than MSVC takes");
        }
    }
}

#[test]
fn without_the_folder_the_check_is_left_to_the_checker() {
    let sources = [clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef { external: true, mapping: Some("OrderFeed".into()), ..clarity_spec::SourceRef::sql("feed") }];
    let project = clarity_spec::load_in(clarity_spec::converted([("shop/main.clarity", MAIN), ("shop/feed.clarity", FEED)]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &sources);
    assert_eq!(project.mapping("OrderFeed").unwrap().schema, None);
    let dir = project_dir("no-file");
    std::fs::remove_file(dir.join("schemas/orders.xsd")).unwrap();
    let project = clarity_spec::load_dir_in(&dir, &sources).unwrap();
    assert!(project.diagnostics.iter().any(|d| d.message == "mapping OrderFeed checks against \"schemas/orders.xsd\", which is not in the project's folder"), "{:#?}", project.diagnostics);
}
