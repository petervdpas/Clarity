//! A save never overwrites someone else's change (docs/APP-CAPABILITIES-PLAN.md
//! step 5) on the C++ stack: a versioned entity names its version column and
//! carries the version, also through its JSON (a window's data travels as
//! JSON), an entity's `savedSince` text is filled from the record as stored
//! now (also `{current.changedBy.name}`, through the database), `save ... anyway` and `reload` of a record call the runtime, and an
//! entity whose versions step is not written yet carries nothing of versions.

use clarity_spec::{Project, Severity, SourceRef};

const SHOP: &str = "module shop

entity Person
    name: text(80)

entity Customer
    name: text(80)
    visits: integer
    changedBy: ref Person
    savedSince @customerChangedSince

action keep(customer: Customer)
    effects db.write
    save customer anyway
    reload customer

function fresh(customer: Customer) -> Customer
    let found = customer
    reload found
    return found

page Edit: window
    data customer: Customer
    textbox name name at 1,1 span 4 bind customer.name
    button \"Keep\" name keepButton at 1,2 span 2 on click: keepClicked

code Edit
    handler keepClicked
        save customer anyway
        reload customer
";

const ERP: &str = "module erp
source erp

entity Invoice
    table \"tblInvoice\"
    key \"InvoiceID\"
    version \"RowVersion\"
    number: text(20)

entity Supplier
    version none
    name: text(80)
";

const FIRST: &str = "migration \"Shop\"
    add entity Person
        name: text(80)
    add entity Customer
        name: text(80)
        visits: integer
        changedBy: ref Person
";

const VERSIONS: &str = "migration \"Versions\"
    add version Person
    add version Customer
";

fn project(steps: &[(&str, &str)]) -> Project {
    let sources = clarity_spec::converted([("shop/shop.clarity", SHOP), ("erp/erp.clarity", ERP)]);
    let project = clarity_spec::load_in(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), steps.iter().copied(), &[SourceRef::sql("main"), SourceRef::sql("erp").external()]);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

fn file(steps: &[(&str, &str)], path: &str) -> String {
    let settings = clarity_project::parse("project Shop\n    stack cpp desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external\n").unwrap();
    let files = clarity_gen::generate(&project(steps), &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text.clone()
}

fn versioned(path: &str) -> String {
    file(&[("migrations/main/0001.clarity", FIRST), ("migrations/main/0002.clarity", VERSIONS)], path)
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted:?} in:\n{text}");
}

/// The part of `text` from `start` up to the next `end` (or its end).
fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let from = &text[text.find(start).unwrap_or_else(|| panic!("no {start:?} in:\n{text}"))..];
    &from[..from[start.len()..].find(end).map_or(from.len(), |i| i + start.len())]
}

#[test]
fn a_versioned_entity_names_its_column_and_carries_the_version() {
    let header = versioned("src/records.hpp");
    let customer = between(&header, "struct Customer {", "};");
    has(customer, "    std::optional<int64_t> changedBy;\n    /// The version it was read or last saved with (conformance/versions.json); 0 for a new one.\n    int64_t version = 0;\n");
    has(customer, "    static constexpr const char* version_column = \"row_version\";\n");
    let invoice = between(&header, "struct Invoice {", "};");
    has(invoice, "    static constexpr const char* version_column = \"RowVersion\";\n");
    // `version none`: saves are not checked, nothing of versions.
    let supplier = between(&header, "struct Supplier {", "};");
    assert!(!supplier.contains("version"), "{supplier}");

    // A window's data travels as JSON, so the version travels with it.
    let source = versioned("src/records.cpp");
    has(between(&source, "void to_json(nlohmann::json& out, const Customer& r)", "}\n\n"), "    out[\"rowVersion\"] = r.version;\n");
    has(
        between(&source, "void from_json(const nlohmann::json& in, Customer& r)", "}\n\n"),
        "    if (in.contains(\"rowVersion\") && in[\"rowVersion\"].is_number()) r.version = in[\"rowVersion\"].get<int64_t>();\n",
    );
    assert!(!between(&source, "void to_json(nlohmann::json& out, const Supplier& r)", "}\n\n").contains("rowVersion"));
}

#[test]
fn a_field_named_version_keeps_clear_of_the_runtimes_member() {
    let names = SHOP.replace("    name: text(80)\n    visits", "    name: text(80)\n    version: integer\n    visits");
    let sources = clarity_spec::converted([("shop/shop.clarity", names.as_str())]);
    let first = FIRST.replace("        visits", "        version: integer\n        visits");
    let project = clarity_spec::load_in(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", first.as_str()), ("migrations/main/0002.clarity", VERSIONS)], &[SourceRef::sql("main")]);
    let settings = clarity_project::parse("project Shop\n    stack cpp desktop\n    datasource main sqlite \"data/shop.db\"\n").unwrap();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let header = &files.iter().find(|f| f.path == "src/records.hpp").unwrap().text;
    let customer = between(header, "struct Customer {", "};");
    has(customer, "    int64_t version_ = 0;\n");
    has(customer, "    int64_t version = 0;\n");
}

#[test]
fn saved_since_fills_the_entity_text_from_the_stored_record() {
    let header = versioned("src/records.hpp");
    has(between(&header, "struct Customer {", "};"), "    static clarity::Text saved_since(clarity::Db& db, const Customer& current);\n");
    // Every slot the language offers the text is filled, `{current.changedBy.name}` among them.
    let project = project(&[("migrations/main/0001.clarity", FIRST), ("migrations/main/0002.clarity", VERSIONS)]);
    let customer = project.entity("Customer").unwrap();
    let slots = clarity_spec::versions::saved_since_slots(&project, customer);
    assert!(slots.iter().any(|s| s == "current.changedBy.name"), "{slots:?}");
    let source = versioned("src/records.cpp");
    for slot in slots {
        has(&source, &format!(".with(\"{slot}\", "));
    }
    // Without a savedSince line, the runtime's own savedSince.
    assert!(!between(&header, "struct Person {", "};").contains("saved_since"));
    let source = versioned("src/records.cpp");
    has(
        &source,
        concat!(
            "clarity::Text Customer::saved_since(clarity::Db& db, const Customer& current) {\n",
            "    auto text = clarity::Text::key(\"customerChangedSince\")\n",
            "        .with(\"current.id\", clarity::slot_integer(current.id, \"\"))\n",
            "        .with(\"current.name\", clarity::slot_text(current.name))\n",
            "        .with(\"current.visits\", clarity::slot_integer(current.visits, \"\"))\n",
            "        .with(\"current.changedBy\", (current.changedBy ? clarity::slot_integer(*current.changedBy, \"\") : clarity::slot_text(\"\")))\n",
            "        .with(\"current.changedBy.id\", (current.changedBy ? clarity::slot_integer(*current.changedBy, \"\") : clarity::slot_text(\"\")));\n",
            "    // The record changedBy refers to, as stored now.\n",
            "    text = text\n",
            "        .with(\"current.changedBy.name\", clarity::slot_text(\"\"));\n",
            "    if (current.changedBy) {\n",
            "        try {\n",
            "            const auto found = db.get<Person>(*current.changedBy);\n",
            "            text = text\n",
            "                .with(\"current.changedBy.name\", clarity::slot_text(found.name));\n",
            "        } catch (const std::exception&) {\n",
            "            // Gone since: its slots stay empty.\n",
            "        }\n",
            "    }\n",
            "    return text;\n",
            "}\n",
        ),
    );
}

#[test]
fn save_anyway_and_reload_of_a_record_call_the_runtime() {
    let shop = versioned("src/modules/shop.cpp");
    has(&shop, "    db.save_anyway(customer);\n    db.reload(customer);\n");
    has(&shop, "    db.reload(found);\n");
    // The window's data is read again as before; its handler saves anyway.
    let edit = between(&shop, "void EditWindow::keep_clicked(", "}\n\n");
    has(edit, "db.save_anyway(customer);\n");
    has(edit, "if (customer.id != 0) customer = db.get<Customer>(customer.id);\n");
}

#[test]
fn without_the_versions_step_nothing_of_versions_is_emitted() {
    let steps = [("migrations/main/0001.clarity", FIRST)];
    let header = file(&steps, "src/records.hpp");
    for entity in ["Person", "Customer"] {
        let record = between(&header, &format!("struct {entity} {{"), "};");
        assert!(!record.contains("version") && !record.contains("saved_since"), "{record}");
    }
    let source = file(&steps, "src/records.cpp");
    let shop = between(&source, "const std::vector<std::string>& Person::columns()", "const std::vector<std::string>& Invoice::columns()");
    assert!(!shop.contains("rowVersion") && !shop.contains("saved_since"), "{shop}");
    // `save ... anyway` of an unchecked record is a save, which the runtime writes as one.
    has(&file(&steps, "src/modules/shop.cpp"), "db.save_anyway(customer);\n");
}

/// The shop's records and module compile against the runtime in this
/// repository (`savedSince`, `save ... anyway` and `reload` of a record are
/// in no example). Needs a C++20 compiler and nlohmann/json, so it only runs
/// when asked: `cargo test -p clarity-gen --test versions_cpp -- --ignored`.
#[test]
#[ignore]
fn the_shop_compiles_against_the_cpp_runtime() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join("target/generated").join(format!("clarity-gen-versions-cpp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let settings = clarity_project::parse("project Shop\n    stack cpp desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external\n").unwrap();
    let steps = [("migrations/main/0001.clarity", FIRST), ("migrations/main/0002.clarity", VERSIONS)];
    let files = clarity_gen::generate(&project(&steps), &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    clarity_gen::write_out(&out, &files, &["build", "data"]).unwrap();
    for source in ["src/records.cpp", "src/modules/shop.cpp"] {
        let done = std::process::Command::new("c++")
            .args(["-std=c++20", "-fsyntax-only", "-Wall", "-Iclarity-runtime", "-Isrc", source])
            .current_dir(&out)
            .output()
            .expect("a C++ compiler");
        let said = format!("{}{}", String::from_utf8_lossy(&done.stdout), String::from_utf8_lossy(&done.stderr));
        assert!(done.status.success() && !said.contains("warning"), "{source} in {}:\n{said}", out.display());
    }
    let _ = std::fs::remove_dir_all(&out);
}
