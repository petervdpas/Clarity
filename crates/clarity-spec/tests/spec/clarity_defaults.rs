//! Clarity's defaults are the spec's (`clarity_spec::defaults`), written out
//! once in conformance/defaults.json, which each runtime that must carry a
//! copy checks its own against.

use clarity_spec::defaults;

fn case_file(name: &str) -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance").join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

#[test]
fn the_defaults_file_says_what_the_spec_owns() {
    let file = case_file("defaults.json");
    assert_eq!(file["signin"]["sessionMinutes"], defaults::SESSION_MINUTES);
    assert_eq!(file["signin"]["tries"], defaults::TRIES);
    assert_eq!(file["signin"]["triesMinutes"], defaults::TRIES_MINUTES);
    assert_eq!(file["undoSteps"], defaults::UNDO_STEPS);
    assert_eq!(file["keptBackups"], defaults::KEPT_BACKUPS);
    assert_eq!(file["httpSeconds"], defaults::HTTP_SECONDS);
    assert_eq!(file["ownersPerRead"], defaults::OWNERS_PER_READ);
    assert_eq!(file["editBusKeeps"], defaults::EDIT_BUS_KEEPS);
    assert_eq!(file["quietMinutes"], defaults::QUIET_MINUTES);
    assert_eq!(file["toastLifespan"]["default"], defaults::TOAST_LIFESPAN);
    assert_eq!(file["toastLifespan"]["least"], *defaults::TOAST_LIFESPANS.start());
    assert_eq!(file["toastLifespan"]["most"], *defaults::TOAST_LIFESPANS.end());
    // The edit bus's own cases keep as many as the defaults say.
    assert_eq!(case_file("edit-bus.json")["keeps"], defaults::EDIT_BUS_KEEPS);
}

#[test]
fn the_spec_s_older_names_are_the_defaults() {
    assert_eq!(clarity_spec::signin::DEFAULT_SESSION_MINUTES, defaults::SESSION_MINUTES);
    assert_eq!(clarity_spec::signin::DEFAULT_TRIES, defaults::TRIES);
    assert_eq!(clarity_spec::signin::DEFAULT_TRIES_MINUTES, defaults::TRIES_MINUTES);
    assert_eq!(clarity_spec::code::DEFAULT_TOAST_LIFESPAN, defaults::TOAST_LIFESPAN);
    assert_eq!(clarity_spec::code::TOAST_LIFESPANS, defaults::TOAST_LIFESPANS);
}
