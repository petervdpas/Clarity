//! The C interface of Clarity.Audio (include/clarity_audio.h): one table of
//! functions a program hands to Clarity.UI with `clarity_ui_use_audio`. The
//! speakers are opened once per thread, the first time a player is made, and
//! stay open while a player lives. No panic crosses the interface.

use std::cell::RefCell;
use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::time::Duration;

use audio::{Player, Problem, Speakers};

/// A mixer of its own, without speakers: what tests play on.
pub use audio::{mixer as audio_mixer, Mixer, MixerSource};

pub const INTERFACE: u32 = 2;

pub const TITLE: u32 = 0;
pub const ARTIST: u32 = 1;
pub const ALBUM: u32 = 2;
pub const TRACK: u32 = 3;

pub const OK: i32 = 0;
pub const MISSING: i32 = 1;
pub const UNREADABLE: i32 = 2;
pub const NOT_A_SOUND: i32 = 3;
pub const CANNOT_SEEK: i32 = 4;
pub const NO_SPEAKERS: i32 = 5;
pub const FAILED: i32 = 6;

pub const PLAYING: u32 = 1;
pub const PAUSED: u32 = 2;
pub const ENDED: u32 = 4;

/// A player and the speakers it keeps open.
pub struct ClarityAudioPlayer {
    player: Player,
    _speakers: Option<Rc<Speakers>>,
}

/// The table of include/clarity_audio.h, field for field.
#[repr(C)]
pub struct ClarityAudioApi {
    pub interface: u32,
    pub player_new: extern "C" fn() -> *mut ClarityAudioPlayer,
    pub player_free: extern "C" fn(*mut ClarityAudioPlayer),
    pub play_file: extern "C" fn(*mut ClarityAudioPlayer, *const c_char, usize) -> i32,
    pub play_bytes: extern "C" fn(*mut ClarityAudioPlayer, *const u8, usize, *const c_char, usize) -> i32,
    pub pause: extern "C" fn(*mut ClarityAudioPlayer),
    pub resume: extern "C" fn(*mut ClarityAudioPlayer),
    pub seek: extern "C" fn(*mut ClarityAudioPlayer, u64) -> i32,
    pub stop: extern "C" fn(*mut ClarityAudioPlayer),
    pub set_volume: extern "C" fn(*mut ClarityAudioPlayer, f32),
    pub state: extern "C" fn(*const ClarityAudioPlayer) -> u32,
    pub position_ms: extern "C" fn(*const ClarityAudioPlayer) -> u64,
    pub length_ms: extern "C" fn(*const ClarityAudioPlayer) -> i64,
    pub problem: extern "C" fn(*mut c_char, usize) -> usize,
    pub tag: extern "C" fn(*const ClarityAudioPlayer, u32, *mut c_char, usize) -> usize,
}

static API: ClarityAudioApi = ClarityAudioApi {
    interface: INTERFACE,
    player_new,
    player_free,
    play_file,
    play_bytes,
    pause,
    resume,
    seek,
    stop,
    set_volume,
    state,
    position_ms,
    length_ms,
    problem,
    tag,
};

/// The table, the same for the library's whole life.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_audio_api() -> *const ClarityAudioApi {
    &API
}

thread_local! {
    static SPEAKERS: RefCell<Option<Rc<Speakers>>> = const { RefCell::new(None) };
    static PROBLEM: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Keeps what went wrong for `problem`, and gives its code.
fn failed(code: i32, detail: &str) -> i32 {
    PROBLEM.with(|p| *p.borrow_mut() = detail.to_string());
    code
}

fn code(problem: Problem) -> i32 {
    let code = match problem {
        Problem::Missing(_) => MISSING,
        Problem::Unreadable(_) => UNREADABLE,
        Problem::NotASound(_) => NOT_A_SOUND,
        Problem::CannotSeek(_) => CANNOT_SEEK,
        Problem::NoSpeakers(_) => NO_SPEAKERS,
    };
    failed(code, problem.detail())
}

/// Runs `act`, a fault inside it becoming `fault`.
fn guarded<R>(fault: R, act: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(act)).unwrap_or_else(|_| {
        failed(FAILED, "a fault inside Clarity.Audio");
        fault
    })
}

/// Text of `len` bytes at `text`; none for NULL with 0, an error for text that is no UTF-8.
///
/// # Safety
/// `text` points at `len` readable bytes, or is NULL.
unsafe fn text<'a>(text: *const c_char, len: usize) -> Result<Option<&'a str>, i32> {
    if text.is_null() {
        return if len == 0 { Ok(None) } else { Err(failed(FAILED, "text is NULL")) };
    }
    let bytes = unsafe { std::slice::from_raw_parts(text.cast::<u8>(), len) };
    std::str::from_utf8(bytes).map(Some).map_err(|_| failed(FAILED, "text is no UTF-8"))
}

extern "C" fn player_new() -> *mut ClarityAudioPlayer {
    guarded(std::ptr::null_mut(), || {
        let speakers = match SPEAKERS.with(|s| s.borrow().clone()) {
            Some(speakers) => speakers,
            None => match Speakers::open() {
                Ok(opened) => {
                    let opened = Rc::new(opened);
                    SPEAKERS.with(|s| *s.borrow_mut() = Some(opened.clone()));
                    opened
                }
                Err(problem) => {
                    code(problem);
                    return std::ptr::null_mut();
                }
            },
        };
        Box::into_raw(Box::new(ClarityAudioPlayer { player: Player::on(speakers.mixer()), _speakers: Some(speakers) }))
    })
}

/// A player made of `mixer` instead of the speakers: what tests play on.
pub fn player_on(mixer: Mixer) -> *mut ClarityAudioPlayer {
    Box::into_raw(Box::new(ClarityAudioPlayer { player: Player::on(mixer), _speakers: None }))
}

extern "C" fn player_free(player: *mut ClarityAudioPlayer) {
    if !player.is_null() {
        guarded((), || drop(unsafe { Box::from_raw(player) }));
    }
}

/// Runs `act` with the player, or fails for NULL.
fn with<R>(player: *mut ClarityAudioPlayer, none: R, act: impl FnOnce(&mut Player) -> R) -> R {
    match unsafe { player.as_mut() } {
        Some(player) => guarded(none, || act(&mut player.player)),
        None => {
            failed(FAILED, "the player is NULL");
            none
        }
    }
}

fn read<R>(player: *const ClarityAudioPlayer, none: R, act: impl FnOnce(&Player) -> R) -> R {
    match unsafe { player.as_ref() } {
        Some(player) => guarded(none, || act(&player.player)),
        None => none,
    }
}

fn result(done: Result<(), Problem>) -> i32 {
    done.map_or_else(code, |()| OK)
}

extern "C" fn play_file(player: *mut ClarityAudioPlayer, path: *const c_char, path_len: usize) -> i32 {
    let path = match unsafe { text(path, path_len) } {
        Ok(Some(path)) => path,
        Ok(None) => return failed(FAILED, "no path"),
        Err(code) => return code,
    };
    with(player, FAILED, |p| result(p.play_file(std::path::Path::new(path))))
}

extern "C" fn play_bytes(player: *mut ClarityAudioPlayer, bytes: *const u8, len: usize, hint: *const c_char, hint_len: usize) -> i32 {
    if bytes.is_null() && len > 0 {
        return failed(FAILED, "the data is NULL");
    }
    let hint = match unsafe { text(hint, hint_len) } {
        Ok(hint) => hint,
        Err(code) => return code,
    };
    let data = if len == 0 { Vec::new() } else { unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec() };
    with(player, FAILED, |p| result(p.play_bytes(data, hint)))
}

extern "C" fn pause(player: *mut ClarityAudioPlayer) {
    with(player, (), Player::pause);
}

extern "C" fn resume(player: *mut ClarityAudioPlayer) {
    with(player, (), Player::resume);
}

extern "C" fn seek(player: *mut ClarityAudioPlayer, milliseconds: u64) -> i32 {
    with(player, FAILED, |p| result(p.seek(Duration::from_millis(milliseconds))))
}

extern "C" fn stop(player: *mut ClarityAudioPlayer) {
    with(player, (), Player::stop);
}

extern "C" fn set_volume(player: *mut ClarityAudioPlayer, volume: f32) {
    let volume = if volume.is_finite() { volume } else { 1.0 };
    with(player, (), |p| p.set_volume(volume));
}

extern "C" fn state(player: *const ClarityAudioPlayer) -> u32 {
    read(player, 0, |p| {
        let mut bits = 0;
        if p.playing() {
            bits |= PLAYING;
        }
        if p.paused() {
            bits |= PAUSED;
        }
        if p.ended() {
            bits |= ENDED;
        }
        bits
    })
}

extern "C" fn position_ms(player: *const ClarityAudioPlayer) -> u64 {
    read(player, 0, |p| p.position().as_millis() as u64)
}

extern "C" fn length_ms(player: *const ClarityAudioPlayer) -> i64 {
    read(player, -1, |p| p.length().map_or(-1, |l| l.as_millis() as i64))
}

/// Copies `text` into `into` (at most `size` bytes, ending with NUL); gives its whole length.
fn copy_out(text: &str, into: *mut c_char, size: usize) -> usize {
    if !into.is_null() && size > 0 {
        let n = text.len().min(size - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), into.cast::<u8>(), n);
            *into.add(n) = 0;
        }
    }
    text.len()
}

extern "C" fn problem(into: *mut c_char, size: usize) -> usize {
    PROBLEM.with(|p| copy_out(&p.borrow(), into, size))
}

extern "C" fn tag(player: *const ClarityAudioPlayer, which: u32, into: *mut c_char, size: usize) -> usize {
    let text = read(player, String::new(), |p| {
        let tags = p.tags();
        match which {
            TITLE => tags.title.clone(),
            ARTIST => tags.artist.clone(),
            ALBUM => tags.album.clone(),
            TRACK if tags.track > 0 => tags.track.to_string(),
            _ => String::new(),
        }
    });
    copy_out(&text, into, size)
}
