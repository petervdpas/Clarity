//! What only the page can do, asked by telling it: the worker the library
//! runs in has no clipboard, no speakers and no downloads, so each of these
//! sends the page a message (`Tell`, JSON) and the page does it. What the
//! page tells back (pasted text, a sound that ended) comes in through the
//! host.
//!
//! The messages, by their `tell`:
//! - `title` {title}: the tab's title.
//! - `cursor` {cursor}: the pointer's CSS shape over the canvas.
//! - `caret` {at: [x, y, width, height] | null}: where an input method writes, in CSS pixels of the canvas.
//! - `copy` {text}: the text goes on the system's clipboard.
//! - `open` {address}: a link opens in a new tab.
//! - `download` {name, data}: a file to save (base64), downloaded under its name.
//! - `show` {name, data, type}: a file opened in a tab of its own, as its media type; only
//!   kinds that run nothing in the page's origin (`shown_type`), others are a `download`.
//! - `play` {id, data, volume, repeat}: a sound (WAV, base64) plays; `stop` {id} stops it.
//! - `songPlay` {id, data, kind, volume}: a song (mp3, flac, ogg, base64) plays in an audio
//!   element; `songDo` {id, do, value} pauses, resumes, seeks (seconds), sets the volume or
//!   stops it. The page says how it plays with each change (`song_said`).
//! - `videoOpen` {id, data, kind, volume}: a video (base64) plays in a video element, sound
//!   and all; `videoDo` {id, do, value} pauses, resumes, seeks (seconds), sets the volume or
//!   stops it. The page sends each new picture (`video_frame`) and how it plays (`video_said`).
//! - `choices` {choices}: the person's choices (language, theme, columns), all of them, for the page to keep.
//! - `ended` {quit}: the program's windows closed.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use serde_json::{Value, json};

use crate::files::{ChosenFile, FileFilter, FilePicker};
use crate::overlay::Clipboard;
use crate::parts::sound::{Music, MusicPlayer, MusicProblem, Playing, SoundOutput};
use crate::parts::video::{Frame, VideoFrames, Videos};

/// What sends the page a message.
pub type Tell = Arc<dyn Fn(Value) + Send + Sync>;

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// The clipboard as the page has it: what was copied goes to the page, and
/// what the page last pasted (or this screen copied) is what a paste reads.
pub struct PageClipboard {
    tell: Tell,
    known: Rc<RefCell<Option<String>>>,
}

impl PageClipboard {
    /// The clipboard over the page; `known` is what the page told it holds.
    pub fn new(tell: Tell, known: Rc<RefCell<Option<String>>>) -> PageClipboard {
        PageClipboard { tell, known }
    }
}

impl Clipboard for PageClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.known.borrow().clone()
    }

    fn set_text(&mut self, text: &str) {
        *self.known.borrow_mut() = Some(text.to_string());
        (self.tell)(json!({ "tell": "copy", "text": text }));
    }
}

/// The sounds the page plays, by number, and those it told ended.
#[derive(Default)]
pub struct Sounds {
    next: u64,
    ended: BTreeSet<u64>,
}

/// Sound through the page's Web Audio.
pub struct PageSound {
    tell: Tell,
    sounds: Rc<RefCell<Sounds>>,
}

impl PageSound {
    pub fn new(tell: Tell, sounds: Rc<RefCell<Sounds>>) -> PageSound {
        PageSound { tell, sounds }
    }
}

struct PagePlaying {
    id: u64,
    tell: Tell,
    sounds: Rc<RefCell<Sounds>>,
    stopped: bool,
}

impl Playing for PagePlaying {
    fn stop(&mut self) {
        if !self.stopped {
            self.stopped = true;
            (self.tell)(json!({ "tell": "stop", "id": self.id }));
        }
    }

    fn ended(&self) -> bool {
        self.stopped || self.sounds.borrow().ended.contains(&self.id)
    }
}

impl SoundOutput for PageSound {
    fn play(&self, file: Vec<u8>, volume: f32, repeat: bool) -> Box<dyn Playing> {
        let id = {
            let mut sounds = self.sounds.borrow_mut();
            sounds.next += 1;
            sounds.next
        };
        (self.tell)(json!({ "tell": "play", "id": id, "data": base64(&file), "volume": volume, "repeat": repeat }));
        Box::new(PagePlaying { id, tell: self.tell.clone(), sounds: self.sounds.clone(), stopped: false })
    }
}

impl Sounds {
    /// The page told sound `id` played to its end.
    pub fn ended(&mut self, id: u64) {
        self.ended.insert(id);
    }
}

/// What the page said of a song as it plays.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SongSaid {
    pub playing: bool,
    pub paused: bool,
    pub ended: bool,
    /// Seconds played, and its length in seconds (0 while the page does not know).
    pub position: f64,
    pub length: f64,
}

/// The songs the page plays, by number, and what it said of each.
#[derive(Default)]
pub struct Songs {
    next: u64,
    said: BTreeMap<u64, SongSaid>,
}

impl Songs {
    /// The page said how song `id` plays.
    pub fn said(&mut self, id: u64, said: SongSaid) {
        if let Some(kept) = self.said.get_mut(&id) {
            *kept = said;
        }
    }
}

/// Songs through the page's audio elements: in a browser the page plays them, no Clarity.Audio.
pub struct PageMusic {
    tell: Tell,
    songs: Rc<RefCell<Songs>>,
}

impl PageMusic {
    pub fn new(tell: Tell, songs: Rc<RefCell<Songs>>) -> PageMusic {
        PageMusic { tell, songs }
    }
}

impl Music for PageMusic {
    fn player(&self) -> Result<Box<dyn MusicPlayer>, MusicProblem> {
        let id = {
            let mut songs = self.songs.borrow_mut();
            songs.next += 1;
            songs.next
        };
        Ok(Box::new(PageSong { id, tell: self.tell.clone(), songs: self.songs.clone(), volume: 1.0 }))
    }
}

/// A sound part's song in the page.
struct PageSong {
    id: u64,
    tell: Tell,
    songs: Rc<RefCell<Songs>>,
    volume: f32,
}

impl PageSong {
    fn said(&self) -> SongSaid {
        self.songs.borrow().said.get(&self.id).copied().unwrap_or_default()
    }

    fn act(&self, act: &str, value: f64) {
        (self.tell)(json!({ "tell": "songDo", "id": self.id, "do": act, "value": value }));
    }
}

impl Drop for PageSong {
    fn drop(&mut self) {
        self.act("stop", 0.0);
        self.songs.borrow_mut().said.remove(&self.id);
    }
}

impl MusicPlayer for PageSong {
    /// A page has no files of the computer's disk to play.
    fn play_file(&mut self, path: &str) -> Result<(), MusicProblem> {
        Err(MusicProblem { key: "audioNoDisk".into(), detail: path.to_string() })
    }
    fn play_bytes(&mut self, bytes: Vec<u8>, hint: Option<&str>) -> Result<(), MusicProblem> {
        self.songs.borrow_mut().said.insert(self.id, SongSaid { playing: true, ..SongSaid::default() });
        (self.tell)(json!({ "tell": "songPlay", "id": self.id, "data": base64(&bytes), "kind": hint.unwrap_or_default(), "volume": self.volume }));
        Ok(())
    }
    fn pause(&mut self) {
        self.act("pause", 0.0);
    }
    fn resume(&mut self) {
        self.act("resume", 0.0);
    }
    fn seek(&mut self, at: Duration) -> Result<(), MusicProblem> {
        self.act("seek", at.as_secs_f64());
        Ok(())
    }
    fn stop(&mut self) {
        self.act("stop", 0.0);
        self.songs.borrow_mut().said.remove(&self.id);
    }
    fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
        self.act("volume", volume as f64);
    }
    fn playing(&self) -> bool {
        self.said().playing
    }
    fn paused(&self) -> bool {
        self.said().paused
    }
    fn ended(&self) -> bool {
        self.said().ended
    }
    fn position(&self) -> Duration {
        Duration::from_secs_f64(self.said().position.max(0.0))
    }
    fn length(&self) -> Option<Duration> {
        let length = self.said().length;
        (length > 0.0 && length.is_finite()).then(|| Duration::from_secs_f64(length))
    }
}

/// What the page said of a video: how it plays, and its last picture.
#[derive(Default)]
pub struct VideoSaid {
    pub playing: bool,
    pub paused: bool,
    pub ended: bool,
    pub failed: bool,
    pub position: f64,
    pub length: f64,
    pub frame: Option<Frame>,
}

/// The videos the page plays, by number.
#[derive(Default)]
pub struct PageVideoState {
    next: u64,
    said: BTreeMap<u64, VideoSaid>,
}

impl PageVideoState {
    /// The page said how video `id` plays.
    pub fn said(&mut self, id: u64, playing: bool, paused: bool, ended: bool, failed: bool, position: f64, length: f64) {
        if let Some(kept) = self.said.get_mut(&id) {
            (kept.playing, kept.paused, kept.ended, kept.failed, kept.position, kept.length) = (playing, paused, ended, failed, position, length);
        }
    }

    /// The page sent video `id`'s next picture.
    pub fn frame(&mut self, id: u64, width: u32, height: u32, rgba: Vec<u8>) {
        if let Some(kept) = self.said.get_mut(&id)
            && rgba.len() == width as usize * height as usize * 4
        {
            kept.frame = Some(Frame { width, height, rgba: std::sync::Arc::new(rgba) });
        }
    }
}

/// Videos through the page's video elements: in a browser the page plays them, sound and all.
pub struct PageVideos {
    tell: Tell,
    state: Rc<RefCell<PageVideoState>>,
}

impl PageVideos {
    pub fn new(tell: Tell, state: Rc<RefCell<PageVideoState>>) -> PageVideos {
        PageVideos { tell, state }
    }
}

impl Videos for PageVideos {
    /// A page has no files of the computer's disk to play.
    fn open(&self, path: &str) -> Result<Box<dyn VideoFrames>, MusicProblem> {
        Err(MusicProblem { key: "audioNoDisk".into(), detail: path.to_string() })
    }

    fn open_bytes(&self, name: &str, bytes: Vec<u8>) -> Result<Box<dyn VideoFrames>, MusicProblem> {
        let id = {
            let mut state = self.state.borrow_mut();
            state.next += 1;
            let id = state.next;
            state.said.insert(id, VideoSaid { playing: true, ..VideoSaid::default() });
            id
        };
        let kind = name.rsplit_once('.').map(|(_, k)| k.to_lowercase()).unwrap_or_default();
        (self.tell)(json!({ "tell": "videoOpen", "id": id, "data": base64(&bytes), "kind": kind, "volume": 1.0 }));
        Ok(Box::new(PageVideo { id, tell: self.tell.clone(), state: self.state.clone() }))
    }
}

/// A video part's video in the page.
struct PageVideo {
    id: u64,
    tell: Tell,
    state: Rc<RefCell<PageVideoState>>,
}

impl PageVideo {
    fn act(&self, act: &str, value: f64) {
        (self.tell)(json!({ "tell": "videoDo", "id": self.id, "do": act, "value": value }));
    }

    fn with<R>(&self, read: impl FnOnce(&VideoSaid) -> R) -> Option<R> {
        self.state.borrow().said.get(&self.id).map(read)
    }
}

impl Drop for PageVideo {
    fn drop(&mut self) {
        self.act("stop", 0.0);
        self.state.borrow_mut().said.remove(&self.id);
    }
}

impl VideoFrames for PageVideo {
    fn size(&self) -> (u32, u32) {
        self.with(|s| s.frame.as_ref().map_or((0, 0), |f| (f.width, f.height))).unwrap_or_default()
    }
    fn length(&self) -> Duration {
        Duration::from_secs_f64(self.with(|s| s.length).unwrap_or_default().max(0.0))
    }
    fn frame_length(&self) -> Duration {
        Duration::from_millis(33)
    }
    fn picture_at(&self, _at: Duration) -> Option<Frame> {
        self.with(|s| s.frame.clone()).flatten()
    }
    fn ended_at(&self, _at: Duration) -> bool {
        self.with(|s| s.ended).unwrap_or(true)
    }
    fn problem(&self) -> Option<MusicProblem> {
        self.with(|s| s.failed).unwrap_or_default().then(|| MusicProblem { key: "videoUndecodable".into(), detail: "the browser cannot play this video".into() })
    }
    fn plays_itself(&self) -> bool {
        true
    }
    fn position(&self) -> Option<Duration> {
        self.with(|s| Duration::from_secs_f64(s.position.max(0.0)))
    }
    fn pause(&self) {
        self.act("pause", 0.0);
    }
    fn resume(&self) {
        self.act("resume", 0.0);
    }
    fn seek(&self, at: Duration) {
        self.act("seek", at.as_secs_f64());
    }
    fn set_volume(&self, volume: f32) {
        self.act("volume", volume as f64);
    }
}

/// The media type a file named `name` shows in a tab as: only kinds that
/// run nothing (pictures, PDF, plain text, sound, video). HTML, SVG, XML
/// and anything unknown could run script in the page's origin: none, and
/// such a file is downloaded instead.
pub fn shown_type(name: &str) -> Option<&'static str> {
    let extension = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    Some(match extension.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "txt" | "log" | "md" | "csv" | "json" => "text/plain; charset=utf-8",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => return None,
    })
}

/// Files through the page: a saved file is a download, a shown one a tab
/// of its own. A browser chooses no folder (one is typed there). Choosing a
/// file to open needs the page's file input, which answers later than a
/// picker may wait: it chooses none here, and the file the page gives is
/// taken by the next open (`given`).
pub struct PageFiles {
    tell: Tell,
    given: Mutex<Option<ChosenFile>>,
}

impl PageFiles {
    pub fn new(tell: Tell) -> PageFiles {
        PageFiles { tell, given: Mutex::new(None) }
    }

    /// The file the person chose in the page, for the next open.
    pub fn give(&self, file: ChosenFile) {
        if let Ok(mut given) = self.given.lock() {
            *given = Some(file);
        }
    }
}

impl FilePicker for PageFiles {
    fn picks_folders(&self) -> bool {
        false
    }

    fn open_file(&self, title: &str, filter: Option<&FileFilter>) -> Option<ChosenFile> {
        if let Some(file) = self.given.lock().ok().and_then(|mut g| g.take()) {
            return Some(file);
        }
        let accept: Vec<String> = filter.map(|f| f.patterns.iter().map(|p| p.trim_start_matches('*').to_string()).collect()).unwrap_or_default();
        (self.tell)(json!({ "tell": "pick", "title": title, "accept": accept }));
        None
    }

    fn save_file(&self, _title: &str, name: &str, bytes: &[u8]) -> Option<String> {
        let name = crate::files::safe_name(name);
        (self.tell)(json!({ "tell": "download", "name": name, "data": base64(bytes) }));
        Some(name)
    }

    fn choose_folder(&self, _title: &str) -> Option<String> {
        None
    }

    fn show(&self, name: &str, bytes: &[u8]) {
        let name = crate::files::safe_name(name);
        match shown_type(&name) {
            Some(kind) => (self.tell)(json!({ "tell": "show", "name": name, "data": base64(bytes), "type": kind })),
            None => (self.tell)(json!({ "tell": "download", "name": name, "data": base64(bytes) })),
        }
    }
}
