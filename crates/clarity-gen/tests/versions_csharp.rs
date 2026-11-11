//! A save never overwrites someone else's change (docs/APP-CAPABILITIES-PLAN.md
//! step 5) on the C# stack: a versioned entity names its version column to
//! the runtime and carries the version, an entity's `savedSince` text is
//! built from the record as stored now, `save ... anyway` writes over the
//! version and `reload` of a record variable reads it again.

use clarity_project::Backend;
use clarity_spec::SourceRef;
use clarity_spec::model::Locale;

const SHOP: &str = "module shop

entity Person
    name: text(80), label @personName

entity Customer
    name: text(80), label @customerName
    text: text(200), label @customerText
    changedBy: ref Person, label @customerChangedBy
    savedSince @customerChangedSince

entity Note
    text: text(200), label @noteText

action keep(customer: Customer)
    effects db.write
    save customer anyway
    reload customer
    save customer

action keepAll()
    effects db.write
    for c in find x in Customer
        reload c
        save c

app Shop
    title @shopTitle
    main Main

page Main: window
    data customer: Customer
    label @hello name hello at 1,1 span 4
    button @keep name keepButton at 1,2 span 2 on click: keepClicked

code Main
    handler keepClicked
        let other = customer
        reload other
        reload customer
";

const ERP: &str = "module erp
source erp

entity Invoice
    table \"tblInvoice\"
    key \"InvoiceID\"
    version \"RowVersion\"
    number: text(20), label @invoiceNumber

entity Supplier
    version none
    name: text(80), label @supplierName
";

const PROJECT: &str = "project Shop
    stack csharp desktop
    datasource main sqlite \"data/shop.db\"
    datasource erp sqlite \"data/erp.db\" external
";

const FIRST: &str = "migration \"Shop\"
    add entity Person
        name: text(80)
    add entity Customer
        name: text(80)
        text: text(200)
        changedBy: ref Person
    add entity Note
        text: text(200)
";

const VERSIONS: &str = "migration \"Versions\"
    add version Customer
";

const EN: &str = "translation en
    customerName \"Name\"
    customerText \"Text\"
    personName \"Name\"
    customerChangedBy \"Changed by\"
    noteText \"Text\"
    shopTitle \"Shop\"
    hello \"Hello\"
    keep \"Keep\"
    invoiceNumber \"Number\"
    supplierName \"Name\"
    customerChangedSince \"{current.name} ({current.id}) was changed since you opened it by {current.changedBy.name}: {current.text}\"
";

fn files(steps: &[(&str, &str)]) -> Vec<clarity_gen::OutFile> {
    let mut settings = clarity_project::parse(PROJECT).unwrap();
    settings.stack.backend = Backend::Csharp;
    let converted = clarity_spec::converted([("shop/shop.clarity", SHOP), ("erp/erp.clarity", ERP)]);
    let mut project =
        clarity_spec::load_in(converted.iter().map(|(p, t)| (p.as_str(), t.as_str())), steps.iter().copied(), &[SourceRef::sql("main"), SourceRef::sql("erp").external()]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    clarity_spec::translations::attach(&mut project, [("translations/en.clarity", EN)]);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn versioned() -> Vec<clarity_gen::OutFile> {
    files(&[("migrations/main/0001.clarity", FIRST), ("migrations/main/0002.clarity", VERSIONS)])
}

fn file<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}: {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted} in\n{text}");
}

/// The class of `entity` in a module's file.
fn class<'a>(text: &'a str, entity: &str) -> &'a str {
    let start = text.find(&format!("public sealed partial class {entity} ")).unwrap_or_else(|| panic!("no class {entity} in\n{text}"));
    let rest = &text[start..];
    &rest[..rest.find("\n}\n").map_or(rest.len(), |end| end + 3)]
}

// On the wire as rowVersion, as C and Go name it.
const VERSION: &str = "    [System.Text.Json.Serialization.JsonPropertyName(\"rowVersion\")]\n    public long Version { get; set; }\n";

#[test]
fn a_versioned_entity_names_its_column_and_carries_its_version() {
    let out = versioned();
    let shop = file(&out, "Modules/Shop.cs");
    let customer = class(shop, "Customer");
    has(customer, "    public static string? VersionColumn => \"row_version\";\n");
    has(customer, VERSION);
    let erp = file(&out, "Modules/Erp.cs");
    let invoice = class(erp, "Invoice");
    has(invoice, "    public static string? VersionColumn => \"RowVersion\";\n");
    has(invoice, VERSION);
    // `version none`, and an owned table whose step adds none: nothing of versions.
    for (text, entity) in [(erp, "Supplier"), (shop, "Note")] {
        let class = class(text, entity);
        assert!(!class.contains("Version"), "{class}");
    }
}

#[test]
fn saved_since_is_built_from_the_record_as_stored_now() {
    let out = versioned();
    let customer = class(file(&out, "Modules/Shop.cs"), "Customer");
    has(
        customer,
        "    public static Clarity.Runtime.Text SavedSince(Db db, Customer current)
    {
        Person? changedBy_ = null;
        try
        {
            changedBy_ = db.Load<Person>(current.ChangedBy);
        }
        catch (ClarityException)
        {
        }
        return Clarity.Runtime.Text.Of(\"customerChangedSince\").With(\"current.id\", Clarity.Runtime.Slot.OfInteger(current.Id, null)).With(\"current.name\", Clarity.Runtime.Slot.OfText(current.Name)).With(\"current.text\", Clarity.Runtime.Slot.OfText(current.Text)).With(\"current.changedBy.name\", changedBy_ is null ? Clarity.Runtime.Slot.OfText(\"\") : Clarity.Runtime.Slot.OfText(changedBy_.Name));
    }
",
    );
    // Without the line, the runtime's own savedSince.
    let invoice = class(file(&out, "Modules/Erp.cs"), "Invoice");
    assert!(!invoice.contains("SavedSince"), "{invoice}");
}

#[test]
fn save_anyway_and_reload_of_a_record_variable() {
    let out = versioned();
    let shop = file(&out, "Modules/Shop.cs");
    has(shop, "        db.SaveAnyway(customer);\n        customer = db.Reload(customer);\n        db.Save(customer);\n");
    // A `for` variable is read again into a copy C# may assign.
    has(shop, "        foreach (var c_ in ");
    has(shop, "            var c = c_;\n            c = db.Reload(c);\n            db.Save(c);\n");
    // In a handler a local is the record read again; any other name is the window's data.
    let main = file(&out, "Windows/MainViewModel.cs");
    has(main, "        var other = Customer;\n        other = Db.Reload(other);\n        if (Customer.Id != 0)\n            Customer = Db.Get<Customer>(Customer.Id);\n");
}

#[test]
fn without_the_versions_step_nothing_of_versions_is_written() {
    let out = files(&[("migrations/main/0001.clarity", FIRST)]);
    let customer = class(file(&out, "Modules/Shop.cs"), "Customer");
    assert!(!customer.contains("Version") && !customer.contains("SavedSince"), "{customer}");
    // `anyway` and `reload` still read and write the record; the runtime checks no version.
    has(file(&out, "Modules/Shop.cs"), "        db.SaveAnyway(customer);\n        customer = db.Reload(customer);\n");
}

/// The versioned program builds on the C# runtime.
#[test]
#[ignore]
fn builds_on_the_runtime() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated/versions-csharp");
    let _ = std::fs::remove_dir_all(&out);
    for f in versioned() {
        let path = out.join(&f.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        match &f.bytes {
            Some(bytes) => std::fs::write(&path, bytes).unwrap(),
            None => std::fs::write(&path, &f.text).unwrap(),
        }
    }
    let built = std::process::Command::new("dotnet")
        .args(["build", "-nologo", "-v", "q"])
        .current_dir(&out)
        .env("CLARITY_RUNTIME", std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp"))
        .env("MSBUILDDISABLENODEREUSE", "1")
        .output()
        .expect("dotnet runs");
    let said = String::from_utf8_lossy(&built.stdout).to_string() + &String::from_utf8_lossy(&built.stderr);
    assert!(built.status.success(), "{said}");
    let _ = std::fs::remove_dir_all(&out);
}
