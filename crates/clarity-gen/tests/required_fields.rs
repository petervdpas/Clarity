//! Required fields on every stack: each stored entity writes its
//! required fields (column and label as written) for the runtime's save to
//! check (conformance/required.json); settings are never saved, so they
//! write none.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};

const SHOP: &str = "module shop

entity Customer
    name: text(80), required, label @customerNameLabel
    city: text(80)

entity Order
    customer: ref Customer, required
    sheet: file, required
    paid: bool

settings
    nickname: text(40), required

page Card: window
    data order: Order
";

fn generated(backend: Backend) -> String {
    let project = clarity_spec::load_sources([("shop.clarity", SHOP)]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    // Rust and C# draw with Clarity.UI; Go, C and C++ keep the webview until theirs exists.
    settings.stack.ui = if matches!(backend, Backend::Rust | Backend::Csharp) { Ui::Clarity } else { Ui::Webview };
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "missing {expected:?}");
}

#[test]
fn go_records_give_their_required_fields() {
    let go = generated(Backend::Go);
    has(&go, "func (*Customer) Required() []clarity.RequiredField {\n\treturn []clarity.RequiredField{{Column: \"name\", Label: \"@customerNameLabel\"}}\n}");
    has(&go, "return []clarity.RequiredField{{Column: \"customer\", Label: \"Customer\"}, {Column: \"sheet\", Label: \"Sheet\"}}");
    assert!(!go.contains("Column: \"nickname\""), "a setting is never saved");
}

#[test]
fn c_entities_give_their_required_fields() {
    let c = generated(Backend::C);
    has(&c, "[] = {{\"name\", \"@customerNameLabel\"}};");
    has(&c, "[] = {{\"customer\", \"Customer\"}, {\"sheet\", \"Sheet\"}};");
    has(&c, "    .required_count = 2,");
    assert!(!c.contains("{\"nickname\", "), "a setting is never saved");
}

#[test]
fn cpp_records_give_their_required_fields() {
    let cpp = generated(Backend::Cpp);
    has(&cpp, "    static std::vector<clarity::Required> required();");
    has(&cpp, "std::vector<clarity::Required> Customer::required() {\n    return {{\"name\", \"@customerNameLabel\"}};\n}");
    has(&cpp, "std::vector<clarity::Required> Order::required() {\n    return {{\"customer\", \"Customer\"}, {\"sheet\", \"Sheet\"}};\n}");
    assert!(!cpp.contains("{\"nickname\", "), "a setting is never saved");
}

#[test]
fn rust_records_give_their_required_fields() {
    let rust = generated(Backend::Rust);
    has(&rust, "    const REQUIRED: &'static [(&'static str, &'static str)] = &[(\"name\", \"@customerNameLabel\")];\n");
    has(&rust, "    const REQUIRED: &'static [(&'static str, &'static str)] = &[(\"customer\", \"Customer\"), (\"sheet\", \"Sheet\")];\n");
    // The facts still list the setting's type; no REQUIRED list names it.
    let saved: Vec<&str> = rust.lines().filter(|l| l.contains("REQUIRED") && l.contains("(\"nickname\", ")).collect();
    assert!(saved.is_empty(), "a setting is never saved: {saved:#?}");
}

#[test]
fn csharp_records_give_their_required_fields() {
    let cs = generated(Backend::Csharp);
    has(&cs, "    public static IReadOnlyList<(string Column, string Label)> Required { get; } = [(\"name\", \"@customerNameLabel\")];\n");
    has(&cs, "    public static IReadOnlyList<(string Column, string Label)> Required { get; } = [(\"customer\", \"Customer\"), (\"sheet\", \"Sheet\")];\n");
    assert!(!cs.contains("(\"nickname\", "), "a setting is never saved");
}

#[test]
fn csharp_inputs_bound_to_a_required_field_show_why_a_save_refused_it() {
    let source = "module shop

entity Customer
    name: text(80), required, label @customerNameLabel
    city: text(80)

page Card: window
    data customer: Customer
    form name customerForm at 1,1 span 12 rows 2 bind customer
        textbox name nameBox at 1,1 span 6 bind customer.name
        textbox name cityBox at 1,2 span 6 bind customer.city
";
    let project = clarity_spec::load_sources([("shop.clarity", source)]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Clarity;
    let cs: String = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}")).iter().map(|f| f.text.clone()).collect();
    has(&cs, "public PartState NameBox { get; } = new(\"nameBox\") { Field = (\"customer\", \"name\") };");
    has(&cs, "public PartState CityBox { get; } = new(\"cityBox\");");
}
