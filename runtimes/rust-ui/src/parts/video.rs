//! The video part (docs/VIDEO-PLAN.md): a video in its place in the grid,
//! a file of the program's resources or one from disk, told by code what to
//! do as a sound is (play, pause, resume, seek, stop, volume). Its pictures
//! come from Clarity.Video when the program carries it (`set_videos`); its
//! sound plays through Clarity.Audio, whose song is the clock the pictures
//! follow, and a video without sound runs on a clock of its own. What code
//! reads of it (playing, position...) is kept in its values, as a sound's.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::parts::sound::{MusicPlayer, MusicProblem, music};
use crate::parts::{Part, PartBase, PartHost};

/// A decoded picture: `width` by `height` RGBA pixels, row after row.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<Vec<u8>>,
}

/// A video's pictures, decoded ahead for any time of its clock.
pub trait VideoFrames {
    fn size(&self) -> (u32, u32);
    fn length(&self) -> Duration;
    /// How long one frame shows; the part looks again after it.
    fn frame_length(&self) -> Duration;
    /// The picture to show at `at`; none before the first is decoded.
    fn picture_at(&self, at: Duration) -> Option<Frame>;
    /// Whether `at` is past the last picture.
    fn ended_at(&self, at: Duration) -> bool;
    /// Why the pictures stopped coming, when they did.
    fn problem(&self) -> Option<MusicProblem>;
    /// Whether it plays by itself, sound included (the browser's video
    /// element): the part then follows its clock and tells it what to do,
    /// and plays no sound of its own.
    fn plays_itself(&self) -> bool {
        false
    }
    /// Where it is, when it plays by itself.
    fn position(&self) -> Option<Duration> {
        None
    }
    fn pause(&self) {}
    fn resume(&self) {}
    fn seek(&self, _at: Duration) {}
    /// From 0 to 1.
    fn set_volume(&self, _volume: f32) {}
}

/// Clarity.Video, which the program carries when it has video parts.
pub trait Videos {
    fn open(&self, path: &str) -> Result<Box<dyn VideoFrames>, MusicProblem>;

    /// A video of the program's resources, named `name`: by default written
    /// to a file of its own, which `open` opens; the browser plays it as it is.
    fn open_bytes(&self, name: &str, bytes: Vec<u8>) -> Result<Box<dyn VideoFrames>, MusicProblem> {
        let path = std::env::temp_dir().join("clarity-videos").join(name);
        std::fs::create_dir_all(path.parent().unwrap_or(&path)).and_then(|()| std::fs::write(&path, bytes)).map_err(|why| MusicProblem { key: "videoUnreadable".into(), detail: format!("{name}: {why}") })?;
        self.open(&path.to_string_lossy())
    }
}

thread_local! {
    static VIDEOS: RefCell<Option<Rc<dyn Videos>>> = const { RefCell::new(None) };
}

/// Videos play through `videos` from now on; none leaves video parts saying so.
pub fn set_videos(videos: Option<Rc<dyn Videos>>) {
    VIDEOS.with(|v| *v.borrow_mut() = videos);
}

/// Clarity.Video, when the program carries it.
pub fn videos() -> Option<Rc<dyn Videos>> {
    VIDEOS.with(|v| v.borrow().clone())
}

/// The clock of a video without sound: when it started, from where, and whether it is held.
#[derive(Default)]
struct Clock {
    started: Option<Instant>,
    from: Duration,
    held: bool,
}

impl Clock {
    fn at(&self, now: Instant) -> Duration {
        match (self.started, self.held) {
            (Some(started), false) => self.from + now.saturating_duration_since(started),
            _ => self.from,
        }
    }

    fn start(&mut self, from: Duration, now: Instant) {
        *self = Clock { started: Some(now), from, held: false };
    }

    fn hold(&mut self, now: Instant) {
        self.from = self.at(now);
        self.held = true;
    }

    fn resume(&mut self, now: Instant) {
        if self.held {
            self.start(self.from, now);
        }
    }
}

pub struct VideoPart {
    base: PartBase,
    frames: Option<Box<dyn VideoFrames>>,
    /// Its sound through Clarity.Audio, when the video has some: the clock it follows.
    song: Option<Box<dyn MusicPlayer>>,
    clock: Clock,
    playing: bool,
    shown: Option<Frame>,
    file: Option<String>,
    problem: String,
    end_told: bool,
    pending: Vec<&'static str>,
    /// How loud it plays, 0 to 100.
    pub volume: i64,
    /// Each thing code told it, as it heard it.
    pub heard: Vec<String>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let volume = spec.number("volume").map(|v| v as i64).unwrap_or(100).clamp(0, 100);
    Box::new(VideoPart { base: PartBase::new(spec, host), frames: None, song: None, clock: Clock::default(), playing: false, shown: None, file: None, problem: String::new(), end_told: false, pending: Vec::new(), volume, heard: Vec::new() })
}

fn number(given: &[Value], index: usize) -> f64 {
    match given.get(index) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

impl VideoPart {
    /// A video of the program's resources, which `play()` plays.
    pub fn source(&self) -> Option<&str> {
        self.base.spec.text("source").filter(|s| !s.is_empty())
    }

    fn repeats(&self) -> bool {
        self.base.spec.flag("repeat")
    }

    /// Where the video is: its own player's when it plays by itself, its
    /// song's position when it has sound, else its own clock.
    pub fn position(&self) -> Duration {
        if let Some(at) = self.frames.as_ref().and_then(|f| f.position()) {
            return at;
        }
        match &self.song {
            Some(song) if song.playing() || song.paused() => song.position(),
            _ => self.clock.at(Instant::now()),
        }
    }

    /// The picture it shows now.
    pub fn shown(&self) -> Option<&Frame> {
        self.shown.as_ref()
    }

    fn hear_all(&mut self) {
        let Some(name) = self.base.name().map(str::to_string) else { return };
        for hear in self.base.host.hears(&name) {
            self.hear(&hear.hear, &hear.values);
        }
    }

    fn hear(&mut self, hear: &str, given: &[Value]) {
        self.heard.push(hear.to_string());
        let now = Instant::now();
        match hear {
            "play" => match given.first().and_then(Value::as_str) {
                Some(path) => self.play(path.to_string(), true),
                None => self.play_source(),
            },
            "pause" => {
                if let Some(song) = &mut self.song {
                    song.pause();
                }
                if let Some(frames) = &self.frames {
                    frames.pause();
                }
                self.clock.hold(now);
            }
            "resume" => {
                if let Some(song) = &mut self.song {
                    song.resume();
                }
                if let Some(frames) = &self.frames {
                    frames.resume();
                }
                self.clock.resume(now);
            }
            "seek" => {
                let at = Duration::from_secs_f64(number(given, 0).max(0.0));
                if let Some(song) = &mut self.song {
                    let _ = song.seek(at);
                }
                if let Some(frames) = &self.frames {
                    frames.seek(at);
                }
                let held = self.clock.held;
                self.clock.start(at, now);
                if held {
                    self.clock.hold(now);
                }
                self.end_told = false;
            }
            "stop" => self.stop(),
            "volume" => {
                self.volume = (number(given, 0) as i64).clamp(0, 100);
                if let Some(song) = &mut self.song {
                    song.set_volume(self.volume as f32 / 100.0);
                }
                if let Some(frames) = &self.frames {
                    frames.set_volume(self.volume as f32 / 100.0);
                }
            }
            _ => {}
        }
    }

    /// Plays its source, a video of the program's resources.
    fn play_source(&mut self) {
        let Some(source) = self.source().map(str::to_string) else { return };
        let Some(bytes) = self.base.host.resource(&source) else {
            return self.fail(MusicProblem { key: "videoMissing".into(), detail: source });
        };
        self.stop();
        let Some(videos) = videos() else {
            return self.fail(MusicProblem { key: "videoNoLibrary".into(), detail: source });
        };
        match videos.open_bytes(&source, bytes) {
            Ok(frames) => self.started(frames, None),
            Err(problem) => self.fail(problem),
        }
    }

    /// Plays the video at `path` from its start: pictures from Clarity.Video, sound from Clarity.Audio.
    fn play(&mut self, path: String, from_disk: bool) {
        self.stop();
        let Some(videos) = videos() else {
            return self.fail(MusicProblem { key: "videoNoLibrary".into(), detail: path });
        };
        match videos.open(&path) {
            Ok(frames) => self.started(frames, Some(path).filter(|_| from_disk)),
            Err(problem) => self.fail(problem),
        }
    }

    /// A video opened: its sound plays through Clarity.Audio (unless it plays by itself), and its clock starts.
    fn started(&mut self, frames: Box<dyn VideoFrames>, file: Option<String>) {
        frames.set_volume(self.volume as f32 / 100.0);
        let plays_itself = frames.plays_itself();
        // Its sound, when it has some and the program carries Clarity.Audio; without, its own clock.
        if !plays_itself
            && let Some(path) = &file.clone().or_else(|| self.opened_path())
            && let Some(music) = music()
            && let Ok(mut song) = music.player()
        {
            song.set_volume(self.volume as f32 / 100.0);
            if song.play_file(path).is_ok() {
                self.song = Some(song);
            }
        }
        self.frames = Some(frames);
        self.clock.start(Duration::ZERO, Instant::now());
        self.playing = true;
        self.end_told = false;
        self.problem.clear();
        self.file = file;
    }

    /// Where its source was written to play, for its sound.
    fn opened_path(&self) -> Option<String> {
        let source = self.source()?;
        Some(std::env::temp_dir().join("clarity-videos").join(source).to_string_lossy().into_owned()).filter(|p| std::path::Path::new(p).is_file())
    }

    /// Stops it: no pictures, no sound; it has not ended.
    pub fn stop(&mut self) {
        if let Some(song) = &mut self.song {
            song.stop();
        }
        self.song = None;
        self.frames = None;
        self.shown = None;
        self.playing = false;
        self.clock = Clock::default();
        self.file = None;
        self.end_told = true;
    }

    fn fail(&mut self, problem: MusicProblem) {
        self.problem = video_shown(&problem);
        eprintln!("{}", self.problem);
        self.playing = false;
        self.pending.push("fail");
    }

    /// Whether it is held by `pause`.
    fn paused(&self) -> bool {
        self.playing && self.clock.held
    }

    /// Brings the picture up to the clock; at the end, starts again or runs `on end`.
    fn follow(&mut self) {
        let Some(frames) = &self.frames else { return };
        let at = self.position();
        if let Some(frame) = frames.picture_at(at) {
            self.shown = Some(frame);
        }
        if let Some(problem) = frames.problem() {
            self.frames = None;
            return self.fail(problem);
        }
        if self.playing && !self.paused() && frames.ended_at(at) {
            if self.repeats() {
                frames.seek(Duration::ZERO);
                if let Some(song) = &mut self.song {
                    let _ = song.seek(Duration::ZERO);
                }
                self.clock.start(Duration::ZERO, Instant::now());
            } else if !self.end_told {
                self.end_told = true;
                self.playing = false;
                if let Some(song) = &mut self.song {
                    song.stop();
                }
                self.pending.push("end");
            }
        }
    }

    /// What code reads of it, in its values: set when it changed.
    fn say(&self) {
        let Some(name) = self.base.name() else { return };
        let mut said = serde_json::Map::new();
        said.insert("playing".into(), Value::Bool(self.playing && !self.paused()));
        said.insert("paused".into(), Value::Bool(self.paused()));
        let position = if self.frames.is_some() { self.position().as_secs() } else { 0 };
        said.insert("position".into(), Value::from(position));
        said.insert("length".into(), Value::from(self.frames.as_ref().map_or(0, |f| f.length().as_secs())));
        said.insert("file".into(), Value::String(self.file.clone().unwrap_or_default()));
        said.insert("problem".into(), Value::String(self.problem.clone()));
        let mut state = self.base.host.state(Some(name)).unwrap_or_default();
        let values = state.values.get_or_insert_with(serde_json::Map::new);
        if said.iter().any(|(key, value)| values.get(key) != Some(value)) {
            values.extend(said);
            self.base.host.set_state(name, state);
        }
    }

    fn run_pending(&mut self) {
        for event in std::mem::take(&mut self.pending) {
            self.base.fire(event);
        }
    }

    /// While it plays it looks again after a frame, so the picture moves on.
    fn keep_up(&mut self, cx: &mut Cx) {
        let now = cx.now();
        if !self.pending.is_empty() {
            cx.wake(self.base.node.id(), now);
        } else if self.playing {
            let frame = self.frames.as_ref().map_or(Duration::from_millis(40), |f| f.frame_length()).clamp(Duration::from_millis(10), Duration::from_millis(100));
            cx.wake(self.base.node.id(), now + frame);
        }
    }
}

/// A video's problem in the language shown.
fn video_shown(problem: &MusicProblem) -> String {
    let english = match problem.key.as_str() {
        "videoMissing" => "no such video: {detail}",
        "videoUnreadable" => "cannot read the video: {detail}",
        "videoNotAVideo" => "not a video: {detail}",
        "videoNoCodec" => "no codec plays {detail}",
        "videoCodecMissing" => "the video's codec is missing: {detail}",
        "videoUndecodable" => "cannot decode the video: {detail}",
        "videoNoLibrary" => "{detail} needs Clarity.Video to play, which this program does not carry",
        _ => return problem.shown(),
    };
    crate::locale::fill(&crate::locale::translate(&problem.key, english), "detail", &problem.detail)
}

/// Where a picture of `size` shows in `area`, as large as fits, centred.
pub fn letterboxed(area: Rect, size: (u32, u32)) -> Rect {
    if size.0 == 0 || size.1 == 0 || area.width <= 0.0 || area.height <= 0.0 {
        return area;
    }
    let scale = (area.width / size.0 as f64).min(area.height / size.1 as f64);
    let (width, height) = (size.0 as f64 * scale, size.1 as f64 * scale);
    Rect::new(area.x + (area.width - width) / 2.0, area.y + (area.height - height) / 2.0, width, height)
}

impl Element for VideoPart {
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

    fn is_shown(&self) -> bool {
        self.base.shown()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.hear_all();
        self.follow();
        self.say();
        self.keep_up(cx);
    }

    /// Black where no picture shows, the picture as large as fits.
    fn draw(&self, p: &Painter) {
        let area = self.base.node.bounds;
        p.fill(area, skia_safe::Color::BLACK, 0.0);
        if let Some(frame) = &self.shown {
            p.pixels(&frame.rgba, frame.width, frame.height, frame.width as usize * 4, letterboxed(area, (frame.width, frame.height)));
        }
    }

    fn on_attached(&mut self, _cx: &mut Cx) {
        self.hear_all();
    }

    /// Its window left the screen: it stops.
    fn on_detached(&mut self, _cx: &mut Cx) {
        self.stop();
    }

    fn on_wake(&mut self, cx: &mut Cx) {
        if !self.base.node.attached() {
            return;
        }
        self.follow();
        self.say();
        self.run_pending();
        cx.invalidate();
        self.keep_up(cx);
    }
}

impl Part for VideoPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
