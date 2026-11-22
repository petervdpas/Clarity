//! The every-part project used as a person uses it (tests/clarity_live):
//! built as Run builds it, its database migrated as Run migrates it,
//! started, signed in to, and driven with clicks, keys, typing and pointer
//! drags; after each step the test reads the screen and the database. The
//! same steps run on every stack: what a person can do is checked the same
//! everywhere.
//!
//! A web program is driven in a real browser (Chromium, headless) through
//! the page's own step channel (tests/live/relay.mjs); on `stack rust
//! desktop` (Clarity.UI) the steps run in the program itself, with no
//! display.
//!
//! Each test is a whole build, so they are ignored: run one at a time,
//! `cargo test -p clarity-gen --test gen -- --ignored --exact everything_live::<name>`.

use crate::clarity_live;
use clarity_gen_testing::everything_project;
use crate::ide_build;

use std::net::TcpListener;
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
    let (file, project, settings) = everything_project::prepare(&name, stack, |dir, project| {
        clarity_gen::parts::write_part_folders(dir, project).unwrap();
    });
    let (folder, program) = ide_build::build_for_run(&file, &project, &settings, clarity_project::Target::Web);
    let scratch = file.parent().unwrap().join("scratch");
    let data = scratch.join("data");
    std::fs::create_dir_all(&data).unwrap();
    ide_build::migrate_for_run(&file, &project, &settings, Some(&data));
    let port = free_port();
    let mut running = Running(
        Command::new(&program)
            .args(["--address", &format!("127.0.0.1:{port}")])
            .current_dir(&folder)
            .env("CLARITY_DATA", &data)
            .spawn()
            .unwrap_or_else(|e| panic!("{}: {e}", program.display())),
    );
    wait_for(port, &mut running);
    let failed = clarity_live::Driver::in_browser(&project, &format!("http://127.0.0.1:{port}/"), &folder, &data).run();
    drop(running);
    println!("{}", if failed.is_empty() { format!("\nall steps passed on {stack}") } else { format!("\n{} failed on {stack}: {}", failed.len(), failed.join("; ")) });
    assert!(failed.is_empty(), "{} steps failed on {stack}: {}", failed.len(), failed.join("; "));
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
#[ignore = "builds and drives the Go program; run with --ignored, one at a time"]
fn go_as_a_person_uses_it() {
    runs_as_a_person_uses_it("go web cli");
}

#[test]
#[ignore = "builds and drives the Rust program; run with --ignored, one at a time"]
fn rust_as_a_person_uses_it() {
    runs_as_a_person_uses_it("rust web cli");
}

#[test]
#[ignore = "builds and drives the C program; run with --ignored, one at a time"]
fn c_as_a_person_uses_it() {
    runs_as_a_person_uses_it("c web cli");
}

#[test]
#[ignore = "builds and drives the C++ program; run with --ignored, one at a time"]
fn cpp_as_a_person_uses_it() {
    runs_as_a_person_uses_it("cpp web cli");
}


#[test]
#[ignore = "builds and drives the Rust program on Clarity.UI; run with --ignored, one at a time"]
fn rust_clarity_as_a_person_uses_it() {
    let stack = "rust desktop cli";
    let (file, project, settings) = everything_project::prepare("everything-live-rust-clarity", stack, |dir, project| {
        clarity_gen::parts::write_part_folders(dir, project).unwrap();
    });
    let (folder, program) = ide_build::build_for_run(&file, &project, &settings, clarity_project::Target::Desktop);
    let scratch = file.parent().unwrap().join("scratch");
    let data = scratch.join("data");
    ide_build::migrate_for_run(&file, &project, &settings, Some(&data));
    let failed = clarity_live::Driver::start(&project, &program, &folder, &data, &scratch).run();
    println!("{}", if failed.is_empty() { format!("\nall steps passed on {stack}") } else { format!("\n{} failed on {stack}: {}", failed.len(), failed.join("; ")) });
    assert!(failed.is_empty(), "{} steps failed on {stack}: {}", failed.len(), failed.join("; "));
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}
