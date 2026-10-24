//! HTTP calls from code (DESIGN 5.23) on every stack: each calls its
//! runtime's client with the method, the url, the body and the headers, and
//! reads the reply's status, ok and text. Only a project that calls out
//! brings the client along.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};
use clarity_spec::{Project, load_sources};

const API: &str = "module api

entity Order
    summary: text(200)

action send_order(order: Order, token: text) -> integer
    effects net
    let reply = http post \"https://api.example.com/orders\" with order.summary header \"Authorization\": \"Bearer {token}\" header \"Accept\": \"text/plain\"
    if not reply.ok
        refuse \"not sent\"
    return reply.status

page Status: window
    data shown: text
    button \"Refresh\" name refresh at 1,1 span 2 on click: refreshClicked

code Status
    handler refreshClicked
        let reply = http get \"https://status.example.com/api/now\"
        shown = reply.text
";

const PLAIN: &str = "module plain

page Main: window
    data shown: text
";

fn project(source: &str) -> Project {
    let project = load_sources([("api.clarity", source)]);
    assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::Severity::Error), "{:#?}", project.diagnostics);
    project
}

fn files(source: &str, backend: Backend, ui: Ui) -> Vec<OutFile> {
    let mut settings = starter("Api", "api");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    generate(&project(source), &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn text<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn rust_calls_through_the_runtime() {
    let files = files(API, Backend::Rust, Ui::Web);
    let code = text(&files, "src/modules/api.rs");
    has(code, "clarity_runtime::http::call(clarity_runtime::http::Method::Post, ");
    has(code, "&String::from(\"https://api.example.com/orders\"), Some((order.summary.clone()).as_str()), &[(\"Authorization\", ");
    has(code, "(\"Accept\", (String::from(\"text/plain\")).as_str())]);");
    has(code, "Method::Get, &String::from(\"https://status.example.com/api/now\"), None, &[]);");
    has(code, "reply.ok");
    has(code, "reply.status");
    has(text(&files, "src/modules/api.rs"), "clarity_runtime::http::call(clarity_runtime::http::Method::Get, ");
    has(text(&files, "Cargo.toml"), "\"http\"");
    assert!(!text(&self::files(PLAIN, Backend::Rust, Ui::Web), "Cargo.toml").contains("\"http\""));
}

#[test]
fn go_calls_through_the_runtime() {
    let files = files(API, Backend::Go, Ui::Web);
    let code = text(&files, "api_module.go");
    has(code, "clarity.HTTP(\"POST\", \"https://api.example.com/orders\", order.Summary, []clarity.Header{{Name: \"Authorization\", Value: ");
    has(code, "{Name: \"Accept\", Value: \"text/plain\"}})");
    has(code, "reply.Ok");
    has(code, "clarity.HTTP(\"GET\", \"https://status.example.com/api/now\", \"\", nil)");
}

#[test]
fn csharp_calls_through_the_runtime() {
    let files = files(API, Backend::Csharp, Ui::Avalonia);
    let code = text(&files, "Modules/Api.cs");
    has(code, "Http.Post(\"https://api.example.com/orders\", order.Summary, [(\"Authorization\", ");
    has(code, "(\"Accept\", \"text/plain\")])");
    has(code, "reply.Ok");
    has(code, "reply.Status");
    let windows: String = files.iter().filter(|f| f.path.ends_with(".cs")).map(|f| f.text.as_str()).collect();
    has(&windows, "Http.Get(\"https://status.example.com/api/now\", [])");
}

#[test]
fn cpp_calls_through_the_runtime_and_links_libcurl() {
    let files = files(API, Backend::Cpp, Ui::Web);
    let code = text(&files, "src/modules/api.cpp");
    has(code, "clarity::http_call(clarity::HttpMethod::Post, ");
    has(code, "{\"Accept\", std::string(\"text/plain\")}}");
    has(code, "reply.ok");
    has(text(&files, "CMakeLists.txt"), " clarity_http");
    assert!(!text(&self::files(PLAIN, Backend::Cpp, Ui::Web), "CMakeLists.txt").contains("clarity_http"));
}

#[test]
fn c_calls_through_the_runtime_and_links_libcurl() {
    let files = files(API, Backend::C, Ui::Web);
    let code = text(&files, "src/modules/api.c");
    has(code, "clarity_http_call(CLARITY_HTTP_POST, ");
    has(code, "(ClarityHttpHeader[]){{\"Authorization\", ");
    has(code, "{\"Accept\", \"text/plain\"}}, 2)");
    has(code, "clarity_http_call(CLARITY_HTTP_GET, \"https://status.example.com/api/now\", NULL, NULL, 0)");
    has(code, ".ok");
    has(text(&files, "CMakeLists.txt"), " clarity_http");
    assert!(!text(&self::files(PLAIN, Backend::C, Ui::Web), "CMakeLists.txt").contains("clarity_http"));
}

/// The README of a C or C++ project that calls out names libcurl, which
/// its build links; one that does not, and every other stack, needs none.
#[test]
fn a_readme_names_libcurl_where_the_build_needs_it() {
    for backend in [Backend::C, Backend::Cpp] {
        assert!(text(&files(API, backend, Ui::Web), "README.md").contains("libcurl"), "{backend:?}");
        assert!(!text(&files(PLAIN, backend, Ui::Web), "README.md").contains("libcurl"), "{backend:?}");
    }
    assert!(!text(&files(API, Backend::Rust, Ui::Web), "README.md").contains("libcurl"));
}
