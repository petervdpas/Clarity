//! The every-part project (tests/fixtures/everything.clarity) built on each
//! stack as the IDE builds it: the same `toolchain` commands Run (debug) and
//! Build (release) use, every program of the stack (the desktop or web
//! program and the command-line one), on the runtimes in this repository.
//! No line of the build may be a warning, of the generated code or of
//! Clarity's runtimes; only code fetched from elsewhere is left out.
//!
//! The building is tests/ide_build's, which the examples' builds share.
//!
//! Each test is a whole build, so they are ignored: run one at a time,
//! `cargo test -p clarity-gen --test everything_build -- --ignored --exact <name>`.

mod everything_project;
mod ide_build;

/// Builds the every-part project on `stack` as the IDE does, without a warning.
fn builds_without_warnings(stack: &str) {
    let (file, project, settings) = everything_project::prepare(&format!("everything-build-{}", stack.replace(' ', "-")), stack);
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a C# program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_csharp() {
    builds_without_warnings("csharp avalonia desktop cli");
}

#[test]
#[ignore = "builds a C# web program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_csharp_on_the_web() {
    builds_without_warnings("csharp avalonia web cli");
}

#[test]
#[ignore = "builds a Rust program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_rust() {
    builds_without_warnings("rust web desktop cli");
}

#[test]
#[ignore = "builds a Rust web program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_rust_on_the_web() {
    builds_without_warnings("rust web web cli");
}

#[test]
#[ignore = "builds a Go program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_go() {
    builds_without_warnings("go web desktop cli");
}

#[test]
#[ignore = "builds a Go web program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_go_on_the_web() {
    builds_without_warnings("go web web cli");
}

#[test]
#[ignore = "builds a C program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_c() {
    builds_without_warnings("c web desktop cli");
}

#[test]
#[ignore = "builds a C web program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_c_on_the_web() {
    builds_without_warnings("c web web cli");
}

#[test]
#[ignore = "builds a C++ program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_cpp() {
    builds_without_warnings("cpp web desktop cli");
}

#[test]
#[ignore = "builds a C++ web program with every part; run with --ignored, one at a time"]
fn every_part_builds_without_warnings_in_cpp_on_the_web() {
    builds_without_warnings("cpp web web cli");
}
