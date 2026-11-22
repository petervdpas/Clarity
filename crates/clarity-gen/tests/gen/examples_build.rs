//! Every example, built on its own stack as the IDE builds it, without a
//! single warning (tests/ide_build). One test per example; each is a whole
//! build, so they are ignored: run one at a time,
//! `cargo test -p clarity-gen --test gen -- --ignored --exact examples_build::<name>`.

use crate::ide_build;

fn builds_without_warnings(name: &str) {
    let (file, project, settings) = ide_build::example(name);
    ide_build::builds_without_warnings(&file, &project, &settings);
}

/// Each example in the examples folder has its test here.
#[test]
fn every_example_is_here() {
    let mut found: Vec<String> = std::fs::read_dir(ide_build::repo().join("examples"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".clarity").map(String::from))
        .collect();
    found.sort();
    assert_eq!(found, EXAMPLES, "an example without a build test here, or a test without its example");
}

const EXAMPLES: [&str; 17] = clarity_devtools::release::EXAMPLES;

/// Examples the checks on GitHub do not build yet, each with why. invoices:
/// the user, 2026-11-20, "invoices can go in the checks if it also has a way
/// of editing the report in the app".
const NOT_IN_THE_CHECKS_YET: [&str; 1] = ["invoices"];

/// The checks on GitHub build every example too: their list (checks.yml,
/// the examples job's matrix) is EXAMPLES but for NOT_IN_THE_CHECKS_YET, so
/// an example is never left out unnoticed.
#[test]
fn the_checks_build_every_example() {
    let checks = std::fs::read_to_string(ide_build::repo().join(".github/workflows/checks.yml")).unwrap();
    let line = checks.lines().find(|l| l.trim_start().starts_with("example: ${{ fromJSON(")).expect("the examples job's matrix line");
    let list = line.rsplit("|| '").next().and_then(|l| l.split("')").next()).expect("the matrix's list of examples");
    let mut listed: Vec<String> = serde_json::from_str(list).unwrap();
    listed.sort();
    let expected: Vec<&str> = EXAMPLES.iter().copied().filter(|e| !NOT_IN_THE_CHECKS_YET.contains(e)).collect();
    assert_eq!(listed, expected, "checks.yml's examples matrix differs from EXAMPLES (crates/clarity-gen/src/tools/devtools/release.rs) but for NOT_IN_THE_CHECKS_YET");
    for later in NOT_IN_THE_CHECKS_YET {
        assert!(EXAMPLES.contains(&later), "{later} is no example");
    }
}

#[test]
#[ignore = "builds the calculator example (go desktop); run with --ignored, one at a time"]
fn the_calculator_example_builds_without_warnings() {
    builds_without_warnings("calculator");
}

#[test]
#[ignore = "builds the dashboard example (rust desktop); run with --ignored, one at a time"]
fn the_dashboard_example_builds_without_warnings() {
    builds_without_warnings("dashboard");
}

#[test]
#[ignore = "builds the dial example (go desktop); run with --ignored, one at a time"]
fn the_dial_example_builds_without_warnings() {
    builds_without_warnings("dial");
}

#[test]
#[ignore = "builds the invaders example (go web); run with --ignored, one at a time"]
fn the_invaders_example_builds_without_warnings() {
    builds_without_warnings("invaders");
}

#[test]
#[ignore = "builds the invoices example (go desktop); run with --ignored, one at a time"]
fn the_invoices_example_builds_without_warnings() {
    builds_without_warnings("invoices");
}

#[test]
#[ignore = "builds the kitchen example (c desktop); run with --ignored, one at a time"]
fn the_kitchen_example_builds_without_warnings() {
    builds_without_warnings("kitchen");
}

#[test]
#[ignore = "builds the ledger example (csharp cli); run with --ignored, one at a time"]
fn the_ledger_example_builds_without_warnings() {
    builds_without_warnings("ledger");
}

#[test]
#[ignore = "builds the modelling example (rust desktop); run with --ignored, one at a time"]
fn the_modelling_example_builds_without_warnings() {
    builds_without_warnings("modelling");
}

#[test]
#[ignore = "builds the mp3player example (csharp desktop); run with --ignored, one at a time"]
fn the_mp3player_example_builds_without_warnings() {
    builds_without_warnings("mp3player");
}

#[test]
#[ignore = "builds the network example (rust desktop); run with --ignored, one at a time"]
fn the_network_example_builds_without_warnings() {
    builds_without_warnings("network");
}

#[test]
#[ignore = "builds the noughts example (csharp desktop); run with --ignored, one at a time"]
fn the_noughts_example_builds_without_warnings() {
    builds_without_warnings("noughts");
}

#[test]
#[ignore = "builds the orbs example (cpp desktop); run with --ignored, one at a time"]
fn the_orbs_example_builds_without_warnings() {
    builds_without_warnings("orbs");
}

#[test]
#[ignore = "builds the records example (go desktop); run with --ignored, one at a time"]
fn the_records_example_builds_without_warnings() {
    builds_without_warnings("records");
}

#[test]
#[ignore = "builds the todo example (csharp desktop); run with --ignored, one at a time"]
fn the_todo_example_builds_without_warnings() {
    builds_without_warnings("todo");
}

#[test]
#[ignore = "builds the walker example (csharp web); run with --ignored, one at a time"]
fn the_walker_example_builds_without_warnings() {
    builds_without_warnings("walker");
}

#[test]
#[ignore = "builds the videoplayer example (go desktop); run with --ignored, one at a time"]
fn the_videoplayer_example_builds_without_warnings() {
    builds_without_warnings("videoplayer");
}

#[test]
#[ignore = "builds the novel example (cpp desktop); run with --ignored, one at a time"]
fn the_novel_example_builds_without_warnings() {
    builds_without_warnings("novel");
}
