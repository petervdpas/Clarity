//! The sound part (DESIGN 5.22): heard, not seen, with no place in the
//! window. Code tells it what to play (`chime.play()`), as it tells a custom
//! part, through the reply's hears: play its source (a sound of the
//! program's resources), stop, a tone, a melody, a volume. Everything plays
//! through the output the host sets (`set_output`; rodio on the desktop),
//! nothing until then. It plays only while its window is on the screen:
//! what it plays stops as its window leaves (closed, or under a window
//! opened in place), and a sound told to play while it is off the screen is
//! not heard.
//!
//! Songs (a source that is no WAV: mp3, flac, ogg) play through Clarity.Audio
//! when the program carries it (`set_music`), one song at a time per part;
//! without it they go to the output as every sound does.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use serde_json::Value;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::melodies::{self, Note};
use crate::painter::Painter;
use crate::parts::{Part, PartBase, PartHost};

/// Something that plays; stopping it ends it.
pub trait Playing {
    fn stop(&mut self);

    /// Whether it played to its end: a sound that repeats never does.
    fn ended(&self) -> bool {
        false
    }
}

/// Where sounds play: the computer's speakers on the desktop.
pub trait SoundOutput {
    /// Plays `file` (WAV, or what the output reads) at `volume` (0 to 1), over and over when `repeat`.
    fn play(&self, file: Vec<u8>, volume: f32, repeat: bool) -> Box<dyn Playing>;
}

/// No output: what a program plays before its host sets one, and in tests.
pub struct SilentOutput;

struct Nothing;

impl Playing for Nothing {
    fn stop(&mut self) {}
}

impl SoundOutput for SilentOutput {
    fn play(&self, _file: Vec<u8>, _volume: f32, _repeat: bool) -> Box<dyn Playing> {
        Box::new(Nothing)
    }
}

thread_local! {
    static OUTPUT: RefCell<Option<Rc<dyn SoundOutput>>> = const { RefCell::new(None) };
}

/// Where every sound plays from now on: the host's output.
pub fn set_output(output: Rc<dyn SoundOutput>) {
    OUTPUT.with(|o| *o.borrow_mut() = Some(output));
}

/// Where sounds play: the output the host set, else none.
pub fn output() -> Rc<dyn SoundOutput> {
    OUTPUT.with(|o| o.borrow().clone()).unwrap_or_else(|| Rc::new(SilentOutput))
}

/// Why a song did not play or move: Clarity.Audio's translation key (`audioMissing`) and what it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicProblem {
    pub key: String,
    pub detail: String,
}

impl MusicProblem {
    /// The problem in the language shown, for the error output and `on fail`.
    pub fn shown(&self) -> String {
        let english = match self.key.as_str() {
            "audioMissing" => "no such file: {detail}",
            "audioUnreadable" => "cannot read: {detail}",
            "audioNotASound" => "not a sound: {detail}",
            "audioCannotSeek" => "cannot seek: {detail}",
            "audioNoSpeakers" => "no speakers: {detail}",
            "audioNoLibrary" => "{detail} needs Clarity.Audio to play, which this program does not carry",
            "audioNoDisk" => "a browser plays no file of the computer's disk: {detail}",
            _ => "{detail}",
        };
        crate::locale::fill(&crate::locale::translate(&self.key, english), "detail", &self.detail)
    }
}

/// What a song's file says of it: its title, artist, album and track number (0 when it does not say).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SongTags {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track: u32,
}

/// One song at a time, played by Clarity.Audio.
pub trait MusicPlayer {
    fn play_file(&mut self, path: &str) -> Result<(), MusicProblem>;
    /// Sound data in memory; `hint` is its kind (mp3, flac).
    fn play_bytes(&mut self, bytes: Vec<u8>, hint: Option<&str>) -> Result<(), MusicProblem>;
    fn pause(&mut self);
    fn resume(&mut self);
    fn seek(&mut self, at: Duration) -> Result<(), MusicProblem>;
    fn stop(&mut self);
    /// From 0 to 1.
    fn set_volume(&mut self, volume: f32);
    fn playing(&self) -> bool;
    fn paused(&self) -> bool;
    /// Played to its end; a stopped song has not.
    fn ended(&self) -> bool;
    fn position(&self) -> Duration;
    fn length(&self) -> Option<Duration>;
    /// The tags of the song that plays; none where they cannot be read.
    fn tags(&self) -> SongTags {
        SongTags::default()
    }
}

/// Clarity.Audio, which the program carries when its sound parts play songs.
pub trait Music {
    /// A player of its own for a sound part.
    fn player(&self) -> Result<Box<dyn MusicPlayer>, MusicProblem>;
}

thread_local! {
    static MUSIC: RefCell<Option<Rc<dyn Music>>> = const { RefCell::new(None) };
}

/// Songs play through `music` from now on; none sends them to the output as every sound.
pub fn set_music(music: Option<Rc<dyn Music>>) {
    MUSIC.with(|m| *m.borrow_mut() = music);
}

/// Clarity.Audio, when the program carries it.
pub fn music() -> Option<Rc<dyn Music>> {
    MUSIC.with(|m| m.borrow().clone())
}

/// Whether `file` (named `source`) is WAV data, which Clarity.UI plays itself.
fn is_wav(source: &str, file: &[u8]) -> bool {
    (file.len() >= 12 && &file[0..4] == b"RIFF" && &file[8..12] == b"WAVE") || source.to_lowercase().ends_with(".wav")
}

pub struct SoundPart {
    base: PartBase,
    playing: Vec<Box<dyn Playing>>,
    /// Its song, through Clarity.Audio: made the first time it plays one.
    song: Option<Box<dyn MusicPlayer>>,
    /// Whether its song starts again at its end.
    song_repeats: bool,
    /// The path of the file its song plays; none for a resource.
    song_file: Option<String>,
    /// Whether `on end` ran for the song that ended.
    end_told: bool,
    /// Why its last song did not play; empty when it did.
    problem: String,
    /// The events to run next, out of laying out: `end`, `fail`.
    pending: Vec<&'static str>,
    /// How loud it plays, 0 to 100: its `volume`, until code sets another.
    pub volume: i64,
    /// Each thing code told it, as it heard it.
    pub heard: Vec<String>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let volume = spec.number("volume").map(|v| v as i64).unwrap_or(100).clamp(0, 100);
    Box::new(SoundPart { base: PartBase::new(spec, host), playing: Vec::new(), song: None, song_repeats: false, song_file: None, end_told: false, problem: String::new(), pending: Vec::new(), volume, heard: Vec::new() })
}

/// A told value as a number, however code gave it.
fn number(given: &[Value], index: usize) -> f64 {
    match given.get(index) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

impl SoundPart {
    /// A sound of the program's resources, which `play` plays.
    pub fn source(&self) -> Option<&str> {
        self.base.spec.text("source").filter(|s| !s.is_empty())
    }

    /// Whether `play` plays its source over and over until `stop`.
    pub fn repeats(&self) -> bool {
        self.base.spec.flag("repeat")
    }

    /// How many sounds it plays now.
    pub fn playing(&self) -> usize {
        self.playing.len()
    }

    /// What code told it since it last heard, at once: the window stack asks
    /// this of a parked window's sounds, so a sound told to play while its
    /// window is covered plays then, as on every stack.
    pub fn hear_now(&mut self) {
        self.hear_all();
    }

    /// What code told it since it last heard: each in order.
    fn hear_all(&mut self) {
        let Some(name) = self.base.name().map(str::to_string) else { return };
        for hear in self.base.host.hears(&name) {
            self.hear(&hear.hear, &hear.values);
        }
    }

    fn hear(&mut self, hear: &str, given: &[Value]) {
        self.heard.push(hear.to_string());
        match hear {
            // A song from disk (docs/AUDIO-PLAN.md).
            "play" if matches!(given.first(), Some(Value::String(_))) => {
                let path = given[0].as_str().unwrap_or_default().to_string();
                self.play_path(&path);
            }
            "pause" => {
                if let Some(song) = &mut self.song {
                    song.pause();
                }
            }
            "resume" => {
                if let Some(song) = &mut self.song {
                    song.resume();
                }
            }
            "seek" => {
                let at = Duration::from_secs_f64(number(given, 0).max(0.0));
                if let Some(song) = &mut self.song
                    && let Err(problem) = song.seek(at)
                {
                    self.fail(problem);
                }
            }
            "play" => {
                let Some(source) = self.source().map(str::to_string) else { return };
                if let Some(file) = self.base.host.resource(&source) {
                    let repeat = self.repeats();
                    match music().filter(|_| !is_wav(&source, &file)) {
                        Some(music) => self.play_song(&music, &source, file, repeat),
                        None => self.start(file, repeat),
                    }
                }
            }
            "stop" => self.stop(),
            "tone" => {
                let note = Note { hertz: Some(number(given, 0)), seconds: number(given, 1) };
                self.start(melodies::wav(&[note]), false);
            }
            "melody" => {
                let notes = given.first().map(|n| n.as_str().map(str::to_string).unwrap_or_else(|| n.to_string())).unwrap_or_default();
                let (plays, _) = melodies::read(&notes, number(given, 1) as i64);
                self.start(melodies::wav(&plays), false);
            }
            "volume" => {
                self.volume = (number(given, 0) as i64).clamp(0, 100);
                if let Some(song) = &mut self.song {
                    song.set_volume(self.volume as f32 / 100.0);
                }
            }
            _ => {}
        }
    }

    /// Lets go of the sounds that played to their end, on the screen's
    /// thread (C# SoundFlowOutput): one that repeats goes on.
    pub fn let_go_ended(&mut self) {
        self.playing.retain(|p| !p.ended());
        let ended = self.song.as_ref().is_some_and(|song| song.ended());
        if ended && self.song_repeats {
            if let Some(Err(problem)) = self.song.as_mut().map(|song| song.seek(Duration::ZERO)) {
                self.song_repeats = false;
                self.fail(problem);
            }
        } else if ended && !self.end_told {
            self.end_told = true;
            self.pending.push("end");
        }
    }

    /// Plays the file at `path` through Clarity.Audio; without it, says so through `on fail`.
    fn play_path(&mut self, path: &str) {
        let Some(music) = music() else {
            return self.fail(MusicProblem { key: "audioNoLibrary".into(), detail: path.to_string() });
        };
        if !self.make_song(&music) {
            return;
        }
        let Some(song) = &mut self.song else { return };
        song.set_volume(self.volume as f32 / 100.0);
        match song.play_file(path) {
            Ok(()) => self.started(Some(path.to_string()), false),
            Err(problem) => self.fail(problem),
        }
    }

    /// Its player, made the first time a song plays; false when Clarity.Audio could not make one.
    fn make_song(&mut self, music: &Rc<dyn Music>) -> bool {
        if self.song.is_none() {
            match music.player() {
                Ok(player) => self.song = Some(player),
                Err(problem) => {
                    self.fail(problem);
                    return false;
                }
            }
        }
        true
    }

    /// A song started: from `file` (none for a resource), again at its end when `repeat`.
    fn started(&mut self, file: Option<String>, repeat: bool) {
        self.song_file = file;
        self.song_repeats = repeat;
        self.end_told = false;
        self.problem.clear();
    }

    /// A song did not play: why goes to the error output and `problem`, and `on fail` runs.
    fn fail(&mut self, problem: MusicProblem) {
        self.problem = problem.shown();
        self.song_repeats = false;
        eprintln!("{}", self.problem);
        self.pending.push("fail");
    }

    /// What code reads of it (docs/AUDIO-PLAN.md), in its values: set when it changed.
    fn say(&self) {
        let Some(name) = self.base.name() else { return };
        let song = self.song.as_deref();
        let mut said = serde_json::Map::new();
        said.insert("playing".into(), Value::Bool(song.is_some_and(|s| s.playing())));
        said.insert("paused".into(), Value::Bool(song.is_some_and(|s| s.paused())));
        said.insert("position".into(), Value::from(song.map_or(0, |s| s.position().as_secs())));
        said.insert("length".into(), Value::from(song.and_then(|s| s.length()).map_or(0, |l| l.as_secs())));
        let file = if song.is_some_and(|s| s.playing() || s.paused()) { self.song_file.clone().unwrap_or_default() } else { String::new() };
        said.insert("file".into(), Value::String(file));
        said.insert("problem".into(), Value::String(self.problem.clone()));
        // The tags of a song that plays or is held; none once it ended or stopped.
        let tags = song.filter(|s| s.playing() || s.paused()).map(|s| s.tags()).unwrap_or_default();
        said.insert("title".into(), Value::String(tags.title));
        said.insert("artist".into(), Value::String(tags.artist));
        said.insert("album".into(), Value::String(tags.album));
        said.insert("track".into(), Value::from(tags.track));
        let mut state = self.base.host.state(Some(name)).unwrap_or_default();
        let values = state.values.get_or_insert_with(serde_json::Map::new);
        if said.iter().any(|(key, value)| values.get(key) != Some(value)) {
            values.extend(said);
            self.base.host.set_state(name, state);
        }
    }

    /// Runs the events that came up while it heard and laid out.
    fn run_pending(&mut self) {
        for event in std::mem::take(&mut self.pending) {
            self.base.fire(event);
        }
    }

    /// While a song plays, it looks again in a quarter of a second, so its position stays fresh.
    fn keep_up(&mut self, cx: &mut Cx) {
        if !self.pending.is_empty() {
            let now = cx.now();
            cx.wake(self.base.node.id(), now);
        } else if self.song.as_ref().is_some_and(|s| s.playing()) {
            let at = cx.now() + Duration::from_millis(250);
            cx.wake(self.base.node.id(), at);
        }
    }

    /// Plays the song `file` (named `source`) through Clarity.Audio, in the place of its last song.
    fn play_song(&mut self, music: &Rc<dyn Music>, source: &str, file: Vec<u8>, repeat: bool) {
        if !self.make_song(music) {
            return;
        }
        let Some(song) = &mut self.song else { return };
        song.set_volume(self.volume as f32 / 100.0);
        let hint = source.rsplit_once('.').map(|(_, kind)| kind.to_lowercase());
        match song.play_bytes(file, hint.as_deref()) {
            Ok(()) => self.started(None, repeat),
            Err(problem) => self.fail(problem),
        }
    }

    /// Its song, when one plays or played: what code reads of it.
    pub fn song(&self) -> Option<&dyn MusicPlayer> {
        self.song.as_deref()
    }

    /// Plays, also while its window is parked: what was told plays when it was told.
    fn start(&mut self, file: Vec<u8>, repeat: bool) {
        self.let_go_ended();
        self.playing.push(output().play(file, self.volume as f32 / 100.0, repeat));
    }

    /// Stops everything it plays.
    pub fn stop(&mut self) {
        for mut playing in self.playing.drain(..) {
            playing.stop();
        }
        if let Some(song) = &mut self.song {
            song.stop();
        }
        self.song_repeats = false;
        self.end_told = true;
    }
}

impl Element for SoundPart {
    fn node(&self) -> &crate::element::Node {
        &self.base.node
    }
    fn node_mut(&mut self) -> &mut crate::element::Node {
        &mut self.base.node
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_part(&self) -> Option<&dyn Part> {
        Some(self)
    }
    fn as_part_mut(&mut self) -> Option<&mut dyn Part> {
        Some(self)
    }

    /// Never seen: it takes no pointer, no focus and draws nothing.
    fn is_shown(&self) -> bool {
        false
    }

    /// Laid out with its window, it takes no room; what code told it is heard then, after every reply.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = Rect::new(area.x, area.y, 0.0, 0.0);
        self.let_go_ended();
        self.hear_all();
        self.say();
        self.keep_up(cx);
    }

    /// Its time came: what code reads is brought up to date, and the events that came up run.
    fn on_wake(&mut self, cx: &mut Cx) {
        if !self.base.node.attached() {
            return;
        }
        self.let_go_ended();
        self.say();
        self.run_pending();
        self.keep_up(cx);
    }

    fn draw(&self, _p: &Painter) {}

    fn on_attached(&mut self, _cx: &mut Cx) {
        self.hear_all();
    }

    /// Its window left the screen: what it plays stops.
    fn on_detached(&mut self, _cx: &mut Cx) {
        self.stop();
    }
}

impl Part for SoundPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
