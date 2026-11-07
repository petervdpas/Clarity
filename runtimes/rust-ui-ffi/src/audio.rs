//! Songs through Clarity.Audio's C table (clarity_audio.h): the program hands
//! it over with `clarity_ui_use_audio` before its first window, and every
//! sound part's song plays through it. The two libraries do not link each
//! other; the table is their whole contract.

use std::ffi::c_char;
use std::rc::Rc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::time::Duration;

use ui::parts::sound::{Music, MusicPlayer, MusicProblem, SongTags};

use crate::{guard, ClarityStatus, Failure};

/// A player of Clarity.Audio's, opaque here.
#[repr(C)]
pub struct ClarityAudioPlayer {
    _opaque: [u8; 0],
}

/// The table of clarity_audio.h, field for field.
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
    /// Interface 2 on: only read when the table says it has it.
    pub tag: extern "C" fn(*const ClarityAudioPlayer, u32, *mut c_char, usize) -> usize,
}

const PLAYING: u32 = 1;
const PAUSED: u32 = 2;
const ENDED: u32 = 4;

/// The translation key of each code of clarity_audio.h, from CLARITY_AUDIO_MISSING (1) on.
const KEYS: [&str; 6] = ["audioMissing", "audioUnreadable", "audioNotASound", "audioCannotSeek", "audioNoSpeakers", "audioFailed"];

/// Clarity.Audio behind its table.
struct Table(&'static ClarityAudioApi);

/// Clarity.Audio as the sound parts see it: the table, shared with each player.
struct TableMusic(Rc<Table>);

impl Table {
    fn problem(&self, code: i32) -> MusicProblem {
        let key = usize::try_from(code - 1).ok().and_then(|at| KEYS.get(at)).copied().unwrap_or("audioFailed");
        let mut into = vec![0 as c_char; 1024];
        let len = (self.0.problem)(into.as_mut_ptr(), into.len()).min(into.len() - 1);
        let detail = String::from_utf8_lossy(&into[..len].iter().map(|c| *c as u8).collect::<Vec<u8>>()).into_owned();
        MusicProblem { key: key.to_string(), detail }
    }

    fn done(&self, code: i32) -> Result<(), MusicProblem> {
        if code == 0 { Ok(()) } else { Err(self.problem(code)) }
    }
}

impl Music for TableMusic {
    fn player(&self) -> Result<Box<dyn MusicPlayer>, MusicProblem> {
        let player = (self.0.0.player_new)();
        if player.is_null() {
            return Err(self.0.problem(5));
        }
        Ok(Box::new(TablePlayer { table: self.0.clone(), player }))
    }
}

/// A sound part's song, freed with it.
struct TablePlayer {
    table: Rc<Table>,
    player: *mut ClarityAudioPlayer,
}

impl Drop for TablePlayer {
    fn drop(&mut self) {
        (self.table.0.player_free)(self.player);
    }
}

impl TablePlayer {
    fn state(&self) -> u32 {
        (self.table.0.state)(self.player)
    }
}

impl MusicPlayer for TablePlayer {
    fn play_file(&mut self, path: &str) -> Result<(), MusicProblem> {
        self.table.done((self.table.0.play_file)(self.player, path.as_ptr().cast(), path.len()))
    }
    fn play_bytes(&mut self, bytes: Vec<u8>, hint: Option<&str>) -> Result<(), MusicProblem> {
        let (hint, hint_len) = hint.map_or((std::ptr::null(), 0), |h| (h.as_ptr().cast(), h.len()));
        self.table.done((self.table.0.play_bytes)(self.player, bytes.as_ptr(), bytes.len(), hint, hint_len))
    }
    fn pause(&mut self) {
        (self.table.0.pause)(self.player);
    }
    fn resume(&mut self) {
        (self.table.0.resume)(self.player);
    }
    fn seek(&mut self, at: Duration) -> Result<(), MusicProblem> {
        self.table.done((self.table.0.seek)(self.player, at.as_millis() as u64))
    }
    fn stop(&mut self) {
        (self.table.0.stop)(self.player);
    }
    fn set_volume(&mut self, volume: f32) {
        (self.table.0.set_volume)(self.player, volume);
    }
    fn playing(&self) -> bool {
        self.state() & PLAYING != 0
    }
    fn paused(&self) -> bool {
        self.state() & PAUSED != 0
    }
    fn ended(&self) -> bool {
        self.state() & ENDED != 0
    }
    fn position(&self) -> Duration {
        Duration::from_millis((self.table.0.position_ms)(self.player))
    }
    fn length(&self) -> Option<Duration> {
        u64::try_from((self.table.0.length_ms)(self.player)).ok().map(Duration::from_millis)
    }
    /// From a table of interface 2 on; an older one has no tags to give.
    fn tags(&self) -> SongTags {
        if self.table.0.interface < 2 {
            return SongTags::default();
        }
        let tag = |which: u32| {
            let mut into = vec![0 as c_char; 512];
            let len = (self.table.0.tag)(self.player, which, into.as_mut_ptr(), into.len()).min(into.len() - 1);
            String::from_utf8_lossy(&into[..len].iter().map(|c| *c as u8).collect::<Vec<u8>>()).into_owned()
        };
        SongTags { title: tag(0), artist: tag(1), album: tag(2), track: tag(3).parse().unwrap_or(0) }
    }
}

/// The table handed over, kept for the process: the desktop takes it on its
/// own thread, which need not be the one that handed it over (Go).
static TABLE: AtomicPtr<ClarityAudioApi> = AtomicPtr::new(std::ptr::null_mut());

/// Songs on this thread play through the table handed over, if any.
pub fn take_on_this_thread() {
    let api = TABLE.load(Ordering::Acquire);
    // SAFETY: only a checked table that lives as long as the program is kept.
    let music = unsafe { api.as_ref() }.map(|api| Rc::new(TableMusic(Rc::new(Table(api)))) as Rc<dyn Music>);
    ui::parts::sound::set_music(music);
}

/// Songs play through `api` (clarity_audio_api()) from now on, on this
/// thread and on the desktop's; NULL sends them to the speakers as every
/// sound again.
///
/// # Safety
/// `api` is NULL or Clarity.Audio's table, which lives as long as the program.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_ui_use_audio(api: *const ClarityAudioApi) -> ClarityStatus {
    guard(|| {
        if let Some(table) = unsafe { api.as_ref() }
            && table.interface < 1
        {
            return Err(Failure::invalid("this is no Clarity.Audio table"));
        }
        TABLE.store(api.cast_mut(), Ordering::Release);
        take_on_this_thread();
        Ok(())
    })
}
