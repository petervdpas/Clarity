//! Every page of a web API (DESIGN 5.23, paging): `http get ... from shop
//! every page` calls the runtime's pages function of each stack with the
//! source's address, auth and paging, as the project file says them.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};
use clarity_spec::{Paging, PagingKind, Project, Severity, SourceRef, load_in};

const API: &str = "module api
source shop

entity Order
    number: text(20)

action list_orders(status: text) -> list<Order>
    effects net
    let reply = http get \"/orders\" from shop every page query \"status\": status
    if not reply.ok
        refuse \"shop answered {reply.status}: {reply.text}\"
    return read list<Order> from json reply.text
";

const SCREEN: &str = "module screen

page Main: window
    data shown: text
";

fn project() -> Project {
    let paging = Paging { kind: PagingKind::Page, param: "page".into(), size_param: "per_page".into(), size: 100, next: String::new(), items: "data".into() };
    let shop = SourceRef { paging: Some(paging), ..SourceRef::web("shop").at("https://shop.example.com/api", Some(("bearer", "env:SHOP_TOKEN"))) };
    let project = load_in([("api.clarity", API), ("screen.clarity", SCREEN)], [], &[SourceRef::sql("main"), shop]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let mut settings = starter("Api", "api");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    generate(&project(), &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn code(backend: Backend, ui: Ui, path: &str) -> String {
    let files = files(backend, ui);
    files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text.clone()
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn rust_reads_every_page() {
    let code = code(Backend::Rust, Ui::Web, "src/modules/api.rs");
    has(&code, "clarity_runtime::http::pages_at(\"https://shop.example.com/api\", &String::from(\"/orders\"), ");
    has(&code, "clarity_runtime::http::Auth::Bearer(\"env:SHOP_TOKEN\"), &clarity_runtime::http::Paging { kind: clarity_runtime::http::PagingKind::Page, param: \"page\", size_param: \"per_page\", size: 100, next: \"\", items: \"data\" })");
}

#[test]
fn go_reads_every_page() {
    let code = code(Backend::Go, Ui::Web, "api_module.go");
    has(&code, "clarity.HTTPPages(\"https://shop.example.com/api\", \"/orders\", ");
    has(&code, "clarity.HTTPAuth{Kind: \"bearer\", Secret: \"env:SHOP_TOKEN\"}, clarity.HTTPPaging{Kind: \"page\", Param: \"page\", SizeParam: \"per_page\", Size: 100, Next: \"\", Items: \"data\"})");
}

#[test]
fn csharp_reads_every_page() {
    let code = code(Backend::Csharp, Ui::Avalonia, "Modules/Api.cs");
    has(&code, "Http.Pages(\"https://shop.example.com/api\", \"/orders\", ");
    has(&code, "\"bearer\", \"env:SHOP_TOKEN\", new HttpPaging(\"page\", \"page\", \"per_page\", 100, \"\", \"data\"))");
}

#[test]
fn cpp_reads_every_page() {
    let code = code(Backend::Cpp, Ui::Web, "src/modules/api.cpp");
    has(&code, "clarity::http_pages(std::string(\"https://shop.example.com/api\"), ");
    has(&code, "clarity::HttpAuth::Bearer, std::string(\"env:SHOP_TOKEN\"), clarity::HttpPaging{clarity::PagingKind::Page, \"page\", \"per_page\", 100, \"\", \"data\"})");
}

#[test]
fn c_reads_every_page() {
    let code = code(Backend::C, Ui::Web, "src/modules/api.c");
    has(&code, "clarity_http_pages(\"https://shop.example.com/api\", ");
    has(&code, "CLARITY_AUTH_BEARER, \"env:SHOP_TOKEN\", (ClarityHttpPaging){CLARITY_PAGING_PAGE, \"page\", \"per_page\", 100, \"\", \"data\"})");
}
