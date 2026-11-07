//! Clarity.Video: the player behind the `video` part of the programs Clarity
//! generates (docs/VIDEO-PLAN.md). [`open`] gives a video's pictures from
//! the first of: a codec the project added for its format ([`register`]),
//! the system's own decoder ([`system`]), or openh264, Cisco's binary
//! ([`openh264`]) under the MP4 reader ([`Movie`]). It draws nothing:
//! Clarity.UI's video part shows the pictures it gives.

use std::fmt;
use std::sync::{Arc, Mutex};

pub mod frames;
pub mod library;
pub mod movie;
pub mod openh264;
pub mod system;

pub use frames::Frames;
pub use movie::Movie;

/// A decoded picture: `width` by `height` pixels, four bytes each (red, green, blue, alpha), row after row.
#[derive(Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl fmt::Debug for Picture {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "Picture({}x{})", self.width, self.height)
    }
}

/// Why something did not open, decode or play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// No file at this path.
    Missing(String),
    /// The file is there but could not be read.
    Unreadable(String),
    /// The file is no video Clarity.Video reads (no MP4, no picture track).
    NotAVideo(String),
    /// No codec takes the video's format; the format is named.
    NoCodec(String),
    /// The codec could not be had: not fetched, not loaded, or no release for this system.
    CodecMissing(String),
    /// A frame could not be decoded.
    Undecodable(String),
}

impl Problem {
    /// The translation key a program shows the problem by; [`fmt::Display`] is its English text.
    pub fn key(&self) -> &'static str {
        match self {
            Problem::Missing(_) => "videoMissing",
            Problem::Unreadable(_) => "videoUnreadable",
            Problem::NotAVideo(_) => "videoNotAVideo",
            Problem::NoCodec(_) => "videoNoCodec",
            Problem::CodecMissing(_) => "videoCodecMissing",
            Problem::Undecodable(_) => "videoUndecodable",
        }
    }

    /// What the problem is about: a path, a format or the reason.
    pub fn detail(&self) -> &str {
        match self {
            Problem::Missing(what) | Problem::Unreadable(what) | Problem::NotAVideo(what) | Problem::NoCodec(what) | Problem::CodecMissing(what) | Problem::Undecodable(what) => what,
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Problem::Missing(_) => "no such file",
            Problem::Unreadable(_) => "cannot read",
            Problem::NotAVideo(_) => "not a video",
            Problem::NoCodec(_) => "no codec plays",
            Problem::CodecMissing(_) => "the codec is missing",
            Problem::Undecodable(_) => "cannot decode",
        };
        write!(out, "{text}: {}", self.detail())
    }
}

impl std::error::Error for Problem {}

/// What a decoder is made for: the track's format (`avc1` for H.264), its
/// size, and the codec's own setup from the file (for H.264 the sequence
/// and picture parameter sets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub parameters: Vec<Vec<u8>>,
}

/// Turns a track's compressed frames into pictures, one frame at a time, in decoding order.
pub trait Decoder: Send {
    /// Decodes one frame as the file holds it; none while the codec waits for more.
    fn decode(&mut self, frame: &[u8]) -> Result<Option<Picture>, Problem>;
}

/// Where a video's pictures come from, in time order: the MP4 reader with a
/// codec, or a system decoder that reads the file itself.
pub trait Source: Send {
    /// Width and height in pixels.
    fn size(&self) -> (u32, u32);
    fn length(&self) -> std::time::Duration;
    /// How long one frame shows.
    fn frame_length(&self) -> std::time::Duration;
    /// The next picture and its time; none after the last.
    fn next_picture(&mut self) -> Result<Option<(std::time::Duration, Picture)>, Problem>;
    /// Moves to `at` and gives the picture there; the next one follows it.
    fn seek(&mut self, at: std::time::Duration) -> Result<Option<(std::time::Duration, Picture)>, Problem>;
    /// What plays it, for the error output and tests: `openh264`, `gstreamer`.
    fn decoder_name(&self) -> String;
}

/// Opens the video at `path` with the first decoder that takes it: a codec
/// the project added for its format, the system's own, else openh264.
pub fn open(path: &std::path::Path) -> Result<Box<dyn Source>, Problem> {
    let movie = Movie::open(path);
    let added = movie.as_ref().is_ok_and(|m| added_for(&m.setup().format));
    if !added && let Some(Ok(system)) = system::open(path) {
        return Ok(system);
    }
    movie.map(|m| Box::new(m) as Box<dyn Source>)
}

/// A codec: the formats it takes, and a decoder for a track of one.
pub trait Codec: Send + Sync {
    /// Its name, as a project file names it (`openh264`, `vp9`).
    fn name(&self) -> &str;
    /// The formats it decodes, as MP4 names them (`avc1`).
    fn formats(&self) -> Vec<String>;
    fn decoder(&self, setup: &Setup) -> Result<Box<dyn Decoder>, Problem>;
}

static CODECS: Mutex<Vec<Arc<dyn Codec>>> = Mutex::new(Vec::new());

/// Adds a codec: for the formats it takes, it wins over the base codec and those added before it.
pub fn register(codec: Arc<dyn Codec>) {
    CODECS.lock().unwrap_or_else(|e| e.into_inner()).push(codec);
}

/// Takes the codecs a project added away again; the base codec stays.
pub fn forget_added() {
    CODECS.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Whether a codec the project added takes `format`.
pub fn added_for(format: &str) -> bool {
    CODECS.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|c| c.formats().iter().any(|f| f == format))
}

/// The codec for `format`: the last one added that takes it, else the base codec for H.264.
pub fn codec_for(format: &str) -> Option<Arc<dyn Codec>> {
    let added = CODECS.lock().unwrap_or_else(|e| e.into_inner()).iter().rev().find(|c| c.formats().iter().any(|f| f == format)).cloned();
    added.or_else(|| openh264::FORMATS.contains(&format).then(|| Arc::new(openh264::OpenH264::default()) as Arc<dyn Codec>))
}
