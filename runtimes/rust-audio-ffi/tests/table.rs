//! The table as a C program uses it, on a mixer of the test's own, and the
//! header against the Rust table: the same fields in the same order.

use std::ffi::c_char;
use std::num::NonZero;
use std::path::PathBuf;

use clarity_audio::{clarity_audio_api, player_on, ClarityAudioApi, ENDED, MISSING, NOT_A_SOUND, OK, PAUSED, PLAYING};

const RATE: u32 = 22050;

fn sound(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rust-audio/tests/sounds").join(name).to_string_lossy().into_owned()
}

fn api() -> &'static ClarityAudioApi {
    unsafe { &*clarity_audio_api() }
}

fn problem() -> String {
    let mut into = [0 as c_char; 256];
    let len = (api().problem)(into.as_mut_ptr(), into.len());
    let text: Vec<u8> = into[..len.min(255)].iter().map(|c| *c as u8).collect();
    String::from_utf8(text).unwrap()
}

#[test]
fn a_song_plays_pauses_seeks_and_ends_through_the_table() {
    let (mixer, mut speakers) = clarity_audio::audio_mixer(NonZero::new(2).unwrap(), NonZero::new(RATE).unwrap());
    let player = player_on(mixer);
    let api = api();
    assert_eq!(2, api.interface);
    let path = sound("tone.flac");
    assert_eq!(OK, (api.play_file)(player, path.as_ptr().cast(), path.len()));
    assert_eq!(PLAYING, (api.state)(player));
    let length = (api.length_ms)(player);
    assert!((1900..2200).contains(&length), "{length}");
    (api.pause)(player);
    assert_eq!(PAUSED, (api.state)(player));
    assert_eq!(OK, (api.seek)(player, 1000));
    assert_eq!(1000, (api.position_ms)(player));
    (api.resume)(player);
    for _ in 0..(RATE * 2 * 2) {
        speakers.next();
    }
    assert_eq!(ENDED, (api.state)(player));
    (api.player_free)(player);
}

#[test]
fn bytes_play_with_their_kind() {
    let (mixer, _speakers) = clarity_audio::audio_mixer(NonZero::new(2).unwrap(), NonZero::new(RATE).unwrap());
    let player = player_on(mixer);
    let data = std::fs::read(sound("tone.mp3")).unwrap();
    assert_eq!(OK, (api().play_bytes)(player, data.as_ptr(), data.len(), c"mp3".as_ptr(), 3));
    assert_eq!(PLAYING, (api().state)(player));
    (api().set_volume)(player, f32::NAN);
    (api().player_free)(player);
}

#[test]
fn what_cannot_play_gives_its_code_and_says_about_what() {
    let (mixer, _speakers) = clarity_audio::audio_mixer(NonZero::new(2).unwrap(), NonZero::new(RATE).unwrap());
    let player = player_on(mixer);
    let gone = sound("gone.mp3");
    assert_eq!(MISSING, (api().play_file)(player, gone.as_ptr().cast(), gone.len()));
    assert_eq!(gone, problem());
    let text = sound("not-a-sound.mp3");
    assert_eq!(NOT_A_SOUND, (api().play_file)(player, text.as_ptr().cast(), text.len()));
    assert_eq!(0, (api().state)(player));
    (api().player_free)(player);
    // A NULL player fails, and reads as nothing playing.
    assert_eq!(clarity_audio::FAILED, (api().seek)(std::ptr::null_mut(), 0));
    assert_eq!(0, (api().state)(std::ptr::null()));
    assert_eq!(-1, (api().length_ms)(std::ptr::null()));
    (api().player_free)(std::ptr::null_mut());
}

/// The field names of `struct` in `source`, in order: C (`(*name)(` or `type name;`) or Rust (`pub name:`).
pub fn fields(source: &str, start: &str) -> Vec<String> {
    let body = source.split(start).nth(1).unwrap().split('}').next().unwrap();
    body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("/*") && !l.starts_with('*') && !l.starts_with("//"))
        .map(|l| {
            if let Some(rest) = l.strip_prefix("pub ") {
                rest.split(':').next().unwrap().to_string()
            } else if let Some(at) = l.find("(*") {
                l[at + 2..].split(')').next().unwrap().to_string()
            } else {
                l.trim_end_matches(';').rsplit(' ').next().unwrap().to_string()
            }
        })
        .collect()
}

#[test]
fn the_header_and_the_table_have_the_same_fields_in_order() {
    let header = include_str!("../include/clarity_audio.h");
    let rust = include_str!("../src/lib.rs");
    let c = fields(header, "typedef struct ClarityAudioApi {");
    assert_eq!(15, c.len(), "{c:?}");
    assert_eq!(c, fields(rust, "pub struct ClarityAudioApi {"));
}

#[test]
fn the_tags_of_what_plays_come_through_the_table() {
    let (mixer, _speakers) = clarity_audio::audio_mixer(NonZero::new(2).unwrap(), NonZero::new(RATE).unwrap());
    let player = player_on(mixer);
    let path = sound("tagged.ogg");
    assert_eq!(OK, (api().play_file)(player, path.as_ptr().cast(), path.len()));
    let tag = |which: u32| {
        let mut into = [0 as c_char; 64];
        let len = (api().tag)(player, which, into.as_mut_ptr(), into.len());
        String::from_utf8(into[..len].iter().map(|c| *c as u8).collect()).unwrap()
    };
    assert_eq!(("Morning Tone".to_string(), "Clarity Band".to_string(), "Test Album".to_string(), "7".to_string()), (tag(clarity_audio::TITLE), tag(clarity_audio::ARTIST), tag(clarity_audio::ALBUM), tag(clarity_audio::TRACK)));
    assert_eq!(0, (api().tag)(std::ptr::null(), clarity_audio::TITLE, std::ptr::null_mut(), 0));
    (api().player_free)(player);
}
