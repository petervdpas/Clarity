//! Every example, built on its own stack as the IDE builds it, without a
//! single warning (tests/ide_build). One test per example; each is a whole
//! build, so they are ignored: run one at a time,
//! `cargo test -p clarity-gen --test examples_build -- --ignored --exact <name>`.

mod ide_build;

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

const EXAMPLES: [&str; 15] = ["calculator", "dial", "dice", "invaders", "ledger", "modelling", "noughts", "orbs", "pills", "records", "shop", "stock", "todo", "walker", "waves"];

#[test]
#[ignore = "builds the calculator example (go web desktop); run with --ignored, one at a time"]
fn the_calculator_example_builds_without_warnings() {
    builds_without_warnings("calculator");
}

#[test]
#[ignore = "builds the dial example (go web desktop); run with --ignored, one at a time"]
fn the_dial_example_builds_without_warnings() {
    builds_without_warnings("dial");
}

#[test]
#[ignore = "builds the dice example (rust web desktop); run with --ignored, one at a time"]
fn the_dice_example_builds_without_warnings() {
    builds_without_warnings("dice");
}

#[test]
#[ignore = "builds the invaders example (go web web); run with --ignored, one at a time"]
fn the_invaders_example_builds_without_warnings() {
    builds_without_warnings("invaders");
}

#[test]
#[ignore = "builds the ledger example (csharp cli); run with --ignored, one at a time"]
fn the_ledger_example_builds_without_warnings() {
    builds_without_warnings("ledger");
}

#[test]
#[ignore = "builds the modelling example (csharp avalonia desktop); run with --ignored, one at a time"]
fn the_modelling_example_builds_without_warnings() {
    builds_without_warnings("modelling");
}

#[test]
#[ignore = "builds the noughts example (csharp avalonia desktop); run with --ignored, one at a time"]
fn the_noughts_example_builds_without_warnings() {
    builds_without_warnings("noughts");
}

#[test]
#[ignore = "builds the orbs example (cpp web desktop); run with --ignored, one at a time"]
fn the_orbs_example_builds_without_warnings() {
    builds_without_warnings("orbs");
}

#[test]
#[ignore = "builds the pills example (go web desktop); run with --ignored, one at a time"]
fn the_pills_example_builds_without_warnings() {
    builds_without_warnings("pills");
}

#[test]
#[ignore = "builds the records example (go web desktop); run with --ignored, one at a time"]
fn the_records_example_builds_without_warnings() {
    builds_without_warnings("records");
}

#[test]
#[ignore = "builds the shop example (rust web desktop); run with --ignored, one at a time"]
fn the_shop_example_builds_without_warnings() {
    builds_without_warnings("shop");
}

#[test]
#[ignore = "builds the stock example (go web desktop cli); run with --ignored, one at a time"]
fn the_stock_example_builds_without_warnings() {
    builds_without_warnings("stock");
}

#[test]
#[ignore = "builds the todo example (csharp avalonia desktop); run with --ignored, one at a time"]
fn the_todo_example_builds_without_warnings() {
    builds_without_warnings("todo");
}

#[test]
#[ignore = "builds the walker example (csharp avalonia web); run with --ignored, one at a time"]
fn the_walker_example_builds_without_warnings() {
    builds_without_warnings("walker");
}

#[test]
#[ignore = "builds the waves example (go web desktop); run with --ignored, one at a time"]
fn the_waves_example_builds_without_warnings() {
    builds_without_warnings("waves");
}
