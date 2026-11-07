//! Clarity.Audio's table handed over as a program does; a table of the
//! test's own stands in for it where a real one would open the speakers.

use std::ffi::c_char;
use std::time::Duration;

use crate::audio::{clarity_ui_use_audio, ClarityAudioApi, ClarityAudioPlayer};
use crate::{CLARITY_INVALID, CLARITY_OK};
use ui::parts::sound::music;

extern "C" fn player_new() -> *mut ClarityAudioPlayer {
    std::ptr::NonNull::dangling().as_ptr()
}
extern "C" fn player_free(_: *mut ClarityAudioPlayer) {}
extern "C" fn play_file(_: *mut ClarityAudioPlayer, _: *const c_char, _: usize) -> i32 {
    1
}
extern "C" fn play_bytes(_: *mut ClarityAudioPlayer, _: *const u8, len: usize, _: *const c_char, _: usize) -> i32 {
    if len == 0 { 3 } else { 0 }
}
extern "C" fn nothing(_: *mut ClarityAudioPlayer) {}
extern "C" fn seek(_: *mut ClarityAudioPlayer, _: u64) -> i32 {
    0
}
extern "C" fn set_volume(_: *mut ClarityAudioPlayer, _: f32) {}
extern "C" fn state(_: *const ClarityAudioPlayer) -> u32 {
    2 | 4
}
extern "C" fn position_ms(_: *const ClarityAudioPlayer) -> u64 {
    1500
}
extern "C" fn length_ms(_: *const ClarityAudioPlayer) -> i64 {
    -1
}
extern "C" fn problem(into: *mut c_char, size: usize) -> usize {
    let text = b"song.mp3";
    let n = text.len().min(size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), into.cast::<u8>(), n);
        *into.add(n) = 0;
    }
    text.len()
}

extern "C" fn tag(_: *const ClarityAudioPlayer, which: u32, into: *mut c_char, size: usize) -> usize {
    let text: &[u8] = match which {
        0 => b"Morning Tone",
        3 => b"7",
        _ => b"",
    };
    let n = text.len().min(size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), into.cast::<u8>(), n);
        *into.add(n) = 0;
    }
    text.len()
}

static FAKE: ClarityAudioApi = ClarityAudioApi {
    interface: 2,
    player_new,
    player_free,
    play_file,
    play_bytes,
    pause: nothing,
    resume: nothing,
    seek,
    stop: nothing,
    set_volume,
    state,
    position_ms,
    length_ms,
    problem,
    tag,
};

#[test]
fn songs_play_through_the_table_handed_over() {
    let _process = super::GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(CLARITY_OK, unsafe { clarity_ui_use_audio(&FAKE) });
    let mut player = music().expect("Clarity.Audio is in use").player().unwrap();
    let missing = player.play_file("song.mp3").unwrap_err();
    assert_eq!(("audioMissing", "song.mp3"), (missing.key.as_str(), missing.detail.as_str()));
    assert_eq!("audioNotASound", player.play_bytes(Vec::new(), Some("mp3")).unwrap_err().key);
    assert!(player.play_bytes(vec![1], None).is_ok());
    assert!(player.paused() && player.ended() && !player.playing());
    assert_eq!(Duration::from_millis(1500), player.position());
    assert_eq!(None, player.length());
    assert_eq!(("Morning Tone", "", 7), (player.tags().title.as_str(), player.tags().artist.as_str(), player.tags().track));
    drop(player);
    assert_eq!(CLARITY_OK, unsafe { clarity_ui_use_audio(std::ptr::null()) });
    assert!(music().is_none());
}

#[test]
fn clarity_audios_own_table_is_taken_and_a_wrong_one_is_not() {
    let _process = super::GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
    let table = clarity_audio::clarity_audio_api().cast::<ClarityAudioApi>();
    assert_eq!(CLARITY_OK, unsafe { clarity_ui_use_audio(table) });
    assert!(music().is_some());
    // Another thread, as the desktop's may be, takes the same table.
    assert!(std::thread::spawn(|| {
        crate::audio::take_on_this_thread();
        music().is_some()
    })
    .join()
    .unwrap());
    let wrong = ClarityAudioApi { interface: 0, ..FAKE };
    assert_eq!(CLARITY_INVALID, unsafe { clarity_ui_use_audio(&wrong) });
    assert_eq!(CLARITY_OK, unsafe { clarity_ui_use_audio(std::ptr::null()) });
}

/// The field names of the struct after `start` in `source`, in order: C or Rust.
fn fields(source: &str, start: &str) -> Vec<String> {
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
fn this_table_is_clarity_audios_field_for_field() {
    let header = fields(include_str!("../../../rust-audio-ffi/include/clarity_audio.h"), "typedef struct ClarityAudioApi {");
    assert_eq!(15, header.len());
    assert_eq!(header, fields(include_str!("../audio.rs"), "pub struct ClarityAudioApi {"));
}
