//! The sound part (DESIGN 5.22): a part of a window without a place in its
//! grid, which plays the project's sound files and makes tones and melodies,
//! told by code as a custom part is told.

use clarity_spec::code::StmtKind;
use clarity_spec::parser::parse;
use clarity_spec::resources::{self, ResourceFile};
use clarity_spec::widgets::{WidgetKind, widget_line};
use clarity_spec::{Project, Severity, load_sources};

const TABLE: &str = "module m

page Table: window
    sound name rollSound source \"roll.wav\" volume 80 repeat
    sound name tune
    button \"Roll\" name roll at 1,1 span 2 on click: rollClicked

code Table
    handler rollClicked
        rollSound.play()
        tune.melody(\"C4 E4 G4 C5*2\", 140)
        tune.tone(440, 0.5)
        rollSound.volume(50)
        rollSound.stop()
";

fn with_sounds(source: &str, names: &[&str]) -> Project {
    let mut project = load_sources([("m.clarity", source)]);
    resources::attach(&mut project, names.iter().map(|n| ResourceFile { name: n.to_string(), bytes: b"RIFF".to_vec() }).collect());
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn has(errors: &[String], part: &str) -> bool {
    errors.iter().any(|e| e.contains(part))
}

#[test]
fn a_sound_has_a_source_a_volume_and_repeats() {
    let (file, diagnostics) = parse(TABLE, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let roll = &file.pages[0].widgets[0].props;
    assert_eq!(roll.kind, WidgetKind::Sound);
    assert_eq!(roll.name.as_deref(), Some("rollSound"));
    assert_eq!(roll.source.as_deref(), Some("roll.wav"));
    assert_eq!(roll.volume, Some(80));
    assert!(roll.repeat);
    let tune = &file.pages[0].widgets[1].props;
    assert_eq!((tune.volume, tune.repeat), (None, false));
}

#[test]
fn a_sound_writes_back_without_a_place() {
    let (file, _) = parse(TABLE, "m.clarity");
    assert_eq!(widget_line(&file.pages[0].widgets[0].props), "sound name rollSound source \"roll.wav\" volume 80 repeat");
    assert_eq!(widget_line(&file.pages[0].widgets[1].props), "sound name tune");
}

#[test]
fn a_sound_has_no_place_in_the_grid() {
    let (_, diagnostics) = parse("module m\n\npage P: window\n    sound name s at 1,1\n", "m.clarity");
    assert!(diagnostics.iter().any(|d| d.message.contains("a sound has no place in the window")), "{diagnostics:?}");
}

#[test]
fn code_tells_a_sound_what_to_play() {
    let project = with_sounds(TABLE, &["roll.wav"]);
    assert_eq!(errors(&project), Vec::<String>::new());
    let body = &project.files[0].code[0].handlers[0].body;
    assert!(body.iter().all(|s| matches!(s.kind, StmtKind::Hear { .. })), "{body:?}");
}

#[test]
fn what_code_tells_a_sound_is_checked() {
    let e = errors(&with_sounds(&TABLE.replace("rollSound.stop()", "rollSound.jump()"), &["roll.wav"]));
    assert!(has(&e, "a sound hears play, stop, tone, melody or volume, not 'jump'"), "{e:?}");
    let e = errors(&with_sounds(&TABLE.replace("tune.tone(440, 0.5)", "tune.tone(\"high\", 0.5)"), &["roll.wav"]));
    assert!(has(&e, "value 'hertz' of tune.tone is integer"), "{e:?}");
    let e = errors(&with_sounds(&TABLE.replace("rollSound.play()", "rollSound.play(1)"), &["roll.wav"]));
    assert!(has(&e, "rollSound.play takes 0 values, not 1"), "{e:?}");
}

#[test]
fn a_melody_in_the_code_is_checked() {
    let e = errors(&with_sounds(&TABLE.replace("C4 E4 G4 C5*2", "C4 H2 G4"), &["roll.wav"]));
    assert!(has(&e, "'H2' in the melody is not a note"), "{e:?}");
}

#[test]
fn the_volume_is_0_to_100() {
    let e = errors(&with_sounds(&TABLE.replace("volume 80", "volume 120"), &["roll.wav"]));
    assert!(has(&e, "a sound's volume is 0 to 100"), "{e:?}");
}

#[test]
fn the_source_is_a_sound_of_the_resources() {
    let e = errors(&with_sounds(TABLE, &["dice.mp3"]));
    assert_eq!(e, ["sound 'roll.wav' is not in the resources folder; it has dice.mp3"]);
    let e = errors(&with_sounds(TABLE, &[]));
    assert_eq!(e, ["sound 'roll.wav' is not in the resources folder, which has no sounds yet"]);
    let e = errors(&with_sounds(&TABLE.replace("roll.wav", "roll.ogg"), &["roll.ogg"]));
    assert_eq!(e, ["'roll.ogg' is not a sound every stack can play; use a .wav, .mp3 or .flac file"]);
}

#[test]
fn melodies_read_alike_on_every_stack() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/melody.json")).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let notes = case["notes"].as_str().unwrap();
        let (plays, skips) = clarity_spec::sound::melody(notes, case["tempo"].as_i64().unwrap());
        let plays: Vec<serde_json::Value> = plays
            .iter()
            .map(|n| serde_json::json!({ "hertz": n.hertz.map(|h| (h * 100.0).round() / 100.0), "seconds": (n.seconds * 10000.0).round() / 10000.0 }))
            .collect();
        assert_eq!(serde_json::Value::from(plays), case["plays"], "{notes}");
        assert_eq!(serde_json::to_value(&skips).unwrap(), case["skips"], "{notes}");
    }
}
