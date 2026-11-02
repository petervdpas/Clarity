//! The every-part project used as a person uses it (tests/live/drive.mjs):
//! built for the web as Run builds it, started, and driven in a real browser
//! (Chromium, headless) with key presses, typing and pointer drags; after
//! each step the test reads the screen and the database. The same steps run
//! on every stack: what a person can do is checked the same everywhere.
//!
//! Each test is a whole build, so they are ignored: run one at a time,
//! `cargo test -p clarity-gen --test everything_live -- --ignored --exact <name>`.

mod everything_project;
mod ide_build;

use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// Stops the program however the test ends.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn wait_for(port: u16, program: &mut Running) {
    let end = Instant::now() + Duration::from_secs(60);
    while Instant::now() < end {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        if let Ok(Some(status)) = program.0.try_wait() {
            panic!("the program ended before it served: {status}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("nothing served on {port}");
}

/// Builds the every-part project for `stack`, serves it, and runs the steps.
fn runs_as_a_person_uses_it(stack: &str) {
    let name = format!("everything-live-{}", stack.replace(' ', "-"));
    let (file, project, settings) = everything_project::prepare(&name, stack);
    let (folder, program) = ide_build::build_for_run(&file, &project, &settings, clarity_project::Target::Web);
    let port = free_port();
    let mut running = Running(
        Command::new(&program)
            .args(["--address", &format!("127.0.0.1:{port}")])
            .current_dir(&folder)
            .spawn()
            .unwrap_or_else(|e| panic!("{}: {e}", program.display())),
    );
    wait_for(port, &mut running);
    let drive = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/live/drive.mjs");
    let ran = Command::new("node")
        .arg(&drive)
        .arg(format!("http://127.0.0.1:{port}/"))
        .arg(&folder)
        .arg(stack)
        .arg(clarity_project::module_dir(&file).join("data/everything.db"))
        .env("CLARITY_FIXTURE", Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/everything"))
        .output()
        .unwrap();
    drop(running);
    let said = format!("{}{}", String::from_utf8_lossy(&ran.stdout), String::from_utf8_lossy(&ran.stderr));
    println!("{said}");
    assert!(ran.status.success(), "{stack}:\n{said}");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
#[ignore = "builds and drives the Go program; run with --ignored, one at a time"]
fn go_as_a_person_uses_it() {
    runs_as_a_person_uses_it("go webview web cli");
}

#[test]
#[ignore = "builds and drives the Rust program; run with --ignored, one at a time"]
fn rust_as_a_person_uses_it() {
    runs_as_a_person_uses_it("rust webview web cli");
}

#[test]
#[ignore = "builds and drives the C program; run with --ignored, one at a time"]
fn c_as_a_person_uses_it() {
    runs_as_a_person_uses_it("c webview web cli");
}

#[test]
#[ignore = "builds and drives the C++ program; run with --ignored, one at a time"]
fn cpp_as_a_person_uses_it() {
    runs_as_a_person_uses_it("cpp webview web cli");
}

