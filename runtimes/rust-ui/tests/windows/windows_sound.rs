//! The sound part beyond what core_parts.rs checks (C# SoundPartTests):
//! play plays its source at full volume unless code set another, a melody
//! leaves out a note it cannot read and plays the rest, and stop stops
//! everything it plays.

use std::cell::RefCell;
use std::rc::Rc;

use clarity_runtime::ui::Hear;
use clarity_ui::parts::sound::{self, Playing, SoundOutput, SoundPart};
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::{melodies, Screen};
use serde_json::{json, Value};

#[derive(Default)]
struct Recorded {
    played: RefCell<Vec<(usize, f32, bool)>>,
    stopped: Rc<RefCell<usize>>,
}

struct Sounding(Rc<RefCell<usize>>);

impl Playing for Sounding {
    fn stop(&mut self) {
        *self.0.borrow_mut() += 1;
    }
}

impl SoundOutput for Recorded {
    fn play(&self, file: Vec<u8>, volume: f32, repeat: bool) -> Box<dyn Playing> {
        self.played.borrow_mut().push((file.len(), volume, repeat));
        Box::new(Sounding(self.stopped.clone()))
    }
}

fn hear(host: &TestHost, screen: &mut Screen, hear: &str, values: Vec<Value>) {
    host.hears.borrow_mut().push(Hear { part: "rollSound".into(), hear: hear.into(), values });
    host.change();
    Shot::of(screen, 200, 100, 1.0);
}

fn part(screen: &Screen) -> &SoundPart {
    screen.element(part_named(screen, "rollSound")).unwrap().as_any().downcast_ref::<SoundPart>().unwrap()
}

#[test]
fn play_plays_its_source_and_stop_stops_everything() {
    let output = Rc::new(Recorded::default());
    sound::set_output(output.clone());
    let host = TestHost::new();
    host.resources.borrow_mut().insert("roll.wav".into(), vec![0; 100]);
    let mut screen = window_of(r#"<Sound name="rollSound" at="1,1" source="roll.wav" repeat="true" />"#, host.clone());
    Shot::of(&mut screen, 200, 100, 1.0);
    assert_eq!(100, part(&screen).volume);
    hear(&host, &mut screen, "play", vec![]);
    hear(&host, &mut screen, "melody", vec![json!("C4 E4 nope"), json!(120)]);
    // Two notes of half a second at 120 beats a minute; the note it cannot read is left out.
    let melody = 44 + 2 * (0.5 * melodies::SAMPLE_RATE as f64) as usize * 2;
    assert_eq!(vec![(100, 1.0, true), (melody, 1.0, false)], output.played.borrow().clone());
    assert_eq!(2, part(&screen).playing());
    hear(&host, &mut screen, "stop", vec![]);
    assert_eq!(2, *output.stopped.borrow());
    assert_eq!(0, part(&screen).playing());
}

#[test]
fn a_tone_plays_at_the_volume_code_set() {
    let output = Rc::new(Recorded::default());
    sound::set_output(output.clone());
    let host = TestHost::new();
    let mut screen = window_of(r#"<Sound name="rollSound" at="1,1" />"#, host.clone());
    Shot::of(&mut screen, 200, 100, 1.0);
    hear(&host, &mut screen, "volume", vec![json!(40)]);
    hear(&host, &mut screen, "tone", vec![json!(440), json!(0.25)]);
    let tone = 44 + 2 * (0.25 * melodies::SAMPLE_RATE as f64) as usize;
    assert_eq!(vec![(tone, 0.4, false)], output.played.borrow().clone());
    assert_eq!(vec!["volume", "tone"], part(&screen).heard);
}

/// C# SoundPart.Hear plays when it is told, also while its window is off
/// the screen (parked under another): the window stack lets a parked
/// window's sounds hear at once.
#[test]
fn a_sound_told_to_play_while_its_window_is_parked_plays_at_once() {
    let output = Rc::new(Recorded::default());
    sound::set_output(output.clone());
    let host = TestHost::new();
    let mut screen = window_of(r#"<Sound name="rollSound" at="1,1" />"#, host.clone());
    Shot::of(&mut screen, 200, 100, 1.0);
    let id = part_named(&screen, "rollSound");
    screen.dispose();
    host.hears.borrow_mut().push(Hear { part: "rollSound".into(), hear: "tone".into(), values: vec![json!(440), json!(0.25)] });
    screen.element_mut(id).unwrap().as_any_mut().downcast_mut::<SoundPart>().unwrap().hear_now();
    assert_eq!(1, output.played.borrow().len());
}
