//! HTTP calls (DESIGN 5.23) in a built program on every stack: the command
//! line program of tests/fixtures/calls.clarity calls a small server of the
//! test's own, with each method and a header, and prints what came back.
//! Slow (each stack's build), so it runs only when asked:
//! `cargo test -p clarity-gen --test http_build -- --ignored --exact <name>`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

use clarity_project::Backend;

const FIXTURE: &str = include_str!("fixtures/calls.clarity");
const STORE: &str = include_str!("fixtures/store.clarity");

/// The program on `backend`, its web API data source `shop` at `server`,
/// signed with a bearer token from CALLS_TOKEN.
fn files(backend: Backend, server: &str) -> Vec<clarity_gen::OutFile> {
    let sources = [
        clarity_spec::SourceRef::sql("main"),
        clarity_spec::SourceRef::web("shop").at(server, Some(("bearer", "env:CALLS_TOKEN"))),
        // Signed with a secret setting, from the system's protected store (DESIGN 5.27).
        clarity_spec::SourceRef::web("vault").at(server, Some(("bearer", "setting:apiToken"))),
        // Hands out its lists two at a time (DESIGN 5.23, paging).
        clarity_spec::SourceRef {
            paging: Some(clarity_spec::Paging {
                kind: clarity_spec::PagingKind::Page,
                param: "page".into(),
                size_param: "size".into(),
                size: 2,
                next: String::new(),
                items: String::new(),
            }),
            ..clarity_spec::SourceRef::web("paged").at(server, None)
        },
    ];
    // Order lives in the database and has its step; Item lives in the web API and needs none.
    let step = "migration \"Orders\"\n    add entity Order\n        number: text(20)\n        total: money\n        lines: integer\n        paid: bool\n        kind: Kind\n        placed: date\n        tags: list<text>\n";
    let project = clarity_spec::load_in([("calls/calls.clarity", FIXTURE), ("calls/store.clarity", STORE)], [("migrations/main/0001.clarity", step)], &sources);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = clarity_project::parse("project Calls\n    stack rust cli\n    datasource main sqlite \"data/calls.db\"\n").unwrap();
    settings.stack.backend = backend;
    if backend == Backend::Csharp {
        settings.stack.ui = clarity_project::Ui::Avalonia;
    }
    clarity_gen::generate_cli(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

/// Where builds go: target/generated in the repository, on disk; never the
/// system's temporary folder, which may live in memory.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The routes of conformance/http.json, on a free port; gives its address.
fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let mut words = line.split_whitespace();
            let (method, target) = (words.next().unwrap_or_default().to_string(), words.next().unwrap_or_default().to_string());
            let (path, query) = target.split_once('?').map_or((target.clone(), String::new()), |(p, q)| (p.to_string(), q.to_string()));
            let (mut length, mut test, mut authorization) = (0, String::new(), String::new());
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).is_err() {
                    break;
                }
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                let (name, value) = header.split_once(':').unwrap_or((header, ""));
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => length = value.trim().parse().unwrap_or(0),
                    "x-test" => test = value.trim().to_string(),
                    "authorization" => authorization = value.trim().to_string(),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);
            let (status, text) = match path.as_str() {
                "/hello" => (200, "hello".to_string()),
                // The numbers 1 to 5, `size` to a page, as conformance/http.json's /paged.
                "/paged" => {
                    let number = |name: &str, otherwise: i64| query.split('&').find_map(|p| p.strip_prefix(&format!("{name}="))).and_then(|v| v.parse().ok()).unwrap_or(otherwise);
                    let (page, size) = (number("page", 1), number("size", 5));
                    let shown: Vec<String> = ((page - 1) * size + 1..=(page * size).min(5)).map(|n| n.to_string()).collect();
                    (200, format!("[{}]", shown.join(",")))
                }
                "/echo" => (200, format!("{method} {}", String::from_utf8_lossy(&body))),
                "/header" => (200, test),
                "/query" => (200, query.clone()),
                "/auth" => (200, authorization.clone()),
                "/items" => (200, r#"{"items": [{"name": "Pen", "price": 1.5}, {"Name": "Ink", "PRICE": "12"}]}"#.to_string()),
                "/basket" => (200, r#"{"title": "Week", "buyer": {"Name": "Ann", "id": 7}, "lines": [{"name": "Pen", "price": 1.5}, {"name": "Ink", "price": "12"}]}"#.to_string()),
                // A service's list: its own names, a number for money, a string for an integer.
                "/orders" => (
                    200,
                    r#"{"data": [{"number": "A-1", "total": 19.99, "lines": "3", "paid": true, "kind": "person", "placed": "2026-10-25T08:00:00Z", "tags": ["x", "y"], "extra": 1}, {"Number": "A-2", "TOTAL": "5", "kind": "BUSINESS"}]}"#.to_string(),
                ),
                // What was sent, as JSON with its names sorted, so every stack's `json of` compares alike.
                "/json" => (200, serde_json::from_slice::<serde_json::Value>(&body).map(|v| v.to_string()).unwrap_or_else(|e| format!("no JSON: {e}"))),
                _ => (404, "no such page".to_string()),
            };
            let reason = if status == 200 { "OK" } else { "Not Found" };
            let mut stream = reader.into_inner();
            let _ = stream.write_all(
                format!("HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len())
                    .as_bytes(),
            );
        }
    });
    address
}

/// What the program printed for `args`.
fn printed(program: &Path, dir: &Path, args: &[&str]) -> String {
    let out = Command::new(program).args(args).current_dir(dir).env("XDG_CONFIG_HOME", dir.join("config")).env("CALLS_TOKEN", "t0k3n").output().unwrap();
    assert!(out.status.success(), "{args:?}: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim_end().to_string()
}

/// The program's calls, against the server, as the cases of conformance/http.json.
fn the_calls_come_back(program: &Path, dir: &Path, server: &str) {
    assert_eq!(printed(program, dir, &["fetch", &format!("{server}/hello")]), "200 yes hello");
    assert_eq!(printed(program, dir, &["fetch", &format!("{server}/missing")]), "404 no no such page");
    assert_eq!(printed(program, dir, &["fetch", &format!("{server}/header")]), "200 yes blue");
    assert_eq!(printed(program, dir, &["send", &format!("{server}/echo"), "done"]), "200");
    assert_eq!(printed(program, dir, &["replace", &format!("{server}/echo"), "twice"]), "PUT twice");
    assert_eq!(printed(program, dir, &["remove", &format!("{server}/echo")]), "DELETE");
    assert_eq!(printed(program, dir, &["amend", &format!("{server}/echo"), "half"]), "PATCH half");
    assert_eq!(printed(program, dir, &["numbers"]), "[1,2,3,4,5]");
    let refused = printed(program, dir, &["fetch", "http://127.0.0.1:1/"]);
    assert!(refused.starts_with("0 no cannot reach 127.0.0.1: "), "{refused}");
    // JSON (DESIGN 5.23): a service's list read into records, a record sent as Clarity's JSON.
    assert_eq!(printed(program, dir, &["orders", &format!("{server}/orders")]), "A-1 19.99 3 yes Person 2026-10-25 2\nA-2 5 0 no Business  0");
    assert_eq!(
        printed(program, dir, &["order", &format!("{server}/json")]),
        r#"{"id":0,"kind":"Business","lines":2,"number":"A-9","paid":true,"placed":null,"tags":[],"total":"12.5"}"#
    );
    let broken = Command::new(program).args(["broken", &format!("{server}/missing")]).current_dir(dir).env("XDG_CONFIG_HOME", dir.join("config")).output().unwrap();
    assert!(!broken.status.success());
    assert!(String::from_utf8_lossy(&broken.stderr).contains("the text is no JSON"), "{}", String::from_utf8_lossy(&broken.stderr));
    // A web API data source (DESIGN 5.23): its entities read from its paths, its query encoded, its calls signed.
    // A secret setting (DESIGN 5.27): kept in the system's protected store, never in the
    // settings file, and signing the calls of the web API that names it. Needs a store,
    // such as a desktop session's Secret Service; CLARITY_SECRET_STORE=1 says there is one.
    if std::env::var("CLARITY_SECRET_STORE").as_deref() == Ok("1") {
        assert_eq!(printed(program, dir, &["token", "s3cr3t"]), "");
        assert_eq!(printed(program, dir, &["vaulted", "auth"]), "Bearer s3cr3t");
        let kept = std::fs::read_dir(dir.join("config")).into_iter().flatten().flatten().filter_map(|e| std::fs::read_to_string(e.path().join("settings.json")).ok()).collect::<String>();
        assert!(!kept.contains("s3cr3t"), "the secret is in the settings file: {kept}");
        assert_eq!(printed(program, dir, &["token", ""]), "");
        assert_eq!(printed(program, dir, &["vaulted", "auth"]), "Bearer");
    }
    assert_eq!(printed(program, dir, &["store", "open & paid"]), "Pen 1.5\nInk 12\nstatus=open%20%26%20paid\nstatus=open%20%26%20paid&page=2\nstatus=open%20%26%20paid&page=3&sort=name\nBearer t0k3n\nWeek for Ann\nPen 1.5\nInk 12");
}

#[test]
#[ignore = "builds the Rust program with cargo"]
fn a_rust_program_calls_out() {
    let out = scratch("http-rust");
    let server = serve();
    clarity_gen::write_out(&out, &files(Backend::Rust, &server), &["target", "data", ".cargo"]).unwrap();
    // On the runtime in this repository.
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust").canonicalize().unwrap();
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    std::fs::write(out.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", clarity_project::slashed(&runtime))).unwrap();
    let status = Command::new("cargo").args(["build", "--quiet"]).current_dir(&out).env("CARGO_BUILD_JOBS", "2").status().unwrap();
    assert!(status.success());
    the_calls_come_back(&out.join("target/debug/calls"), &out, &server);
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the Go program with go"]
fn a_go_program_calls_out() {
    let out = scratch("http-go");
    let server = serve();
    clarity_gen::write_out(&out, &files(Backend::Go, &server), &["bin", "data", "go.work"]).unwrap();
    // On the runtime in this repository, through a workspace.
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/go").canonicalize().unwrap();
    let go = include_str!("../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
    std::fs::write(out.join("go.work"), format!("{go}\n\nuse .\n\nreplace github.com/petervdpas/Clarity/runtimes/go => {}\n", clarity_project::slashed(&runtime))).unwrap();
    let gofmt = Command::new("gofmt").args(["-l", "."]).current_dir(&out).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&gofmt.stdout), "", "gofmt would change these files");
    let vet = Command::new("go").args(["vet", "./..."]).current_dir(&out).output().unwrap();
    assert!(vet.status.success(), "{}", String::from_utf8_lossy(&vet.stderr));
    let build = Command::new("go").args(["build", "-o", "bin/calls", "."]).current_dir(&out).output().unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    the_calls_come_back(&out.join("bin/calls"), &out, &server);
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the C# program with dotnet"]
fn a_csharp_program_calls_out() {
    let out = scratch("http-csharp");
    let server = serve();
    clarity_gen::write_out(&out, &files(Backend::Csharp, &server), &["bin", "obj", "data"]).unwrap();
    // On the runtime projects in this repository.
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp").canonicalize().unwrap();
    let build = Command::new("dotnet")
        .args(["build", "--nologo", "-tl:off", "-v", "quiet", &format!("-p:ClarityRuntime={}", clarity_project::slashed(&runtime))])
        .current_dir(&out)
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stdout));
    the_calls_come_back(&out.join("bin/Debug/net10.0/calls"), &out, &server);
    std::fs::remove_dir_all(&out).unwrap();
}

/// Builds a generated CMake project in `out` and gives its program.
fn cmake_build(out: &Path, program: &str) -> PathBuf {
    let configure = Command::new("cmake").args(["-S", ".", "-B", "build/debug", "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Debug"]).current_dir(out).output().unwrap();
    assert!(configure.status.success(), "{}{}", String::from_utf8_lossy(&configure.stdout), String::from_utf8_lossy(&configure.stderr));
    let build = Command::new("cmake").args(["--build", "build/debug", "-j2"]).current_dir(out).output().unwrap();
    assert!(build.status.success(), "{}{}", String::from_utf8_lossy(&build.stdout), String::from_utf8_lossy(&build.stderr));
    out.join("build/debug").join(program)
}

#[test]
#[ignore = "builds the C program with CMake"]
fn a_c_program_calls_out() {
    let out = scratch("http-c");
    let server = serve();
    clarity_gen::write_out(&out, &files(Backend::C, &server), &["build", "data"]).unwrap();
    let program = cmake_build(&out, "calls");
    the_calls_come_back(&program, &out, &server);
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "builds the C++ program with CMake"]
fn a_cpp_program_calls_out() {
    let out = scratch("http-cpp");
    let server = serve();
    clarity_gen::write_out(&out, &files(Backend::Cpp, &server), &["build", "data"]).unwrap();
    let program = cmake_build(&out, "calls");
    the_calls_come_back(&program, &out, &server);
    std::fs::remove_dir_all(&out).unwrap();
}
