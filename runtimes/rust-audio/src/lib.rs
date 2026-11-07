//! Clarity.Audio: the player behind the sound part of the programs Clarity
//! generates. It plays mp3, flac, wav and ogg files, read as they play, with
//! pause, resume, seek, a volume, the position and the length, and says when
//! a song has played to its end. It draws nothing: Clarity.UI's sound part
//! tells it what to play, and the browser plays with the page's own audio.
//!
//! A player plays on a mixer: the speakers' ([`Speakers::open`]) or a test's
//! own ([`mixer`]), whose samples the test takes as a sound card would.

use std::fmt;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// A mixer of its own, without speakers: what a test plays on.
pub use rodio::mixer::{mixer, Mixer, MixerSource};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Source};

/// Why something did not play or seek.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// No file at this path.
    Missing(String),
    /// The file is there but could not be read.
    Unreadable(String),
    /// The data is no sound Clarity.Audio can decode.
    NotASound(String),
    /// The song cannot be moved to that point.
    CannotSeek(String),
    /// The computer has no speakers, or they did not open.
    NoSpeakers(String),
}

impl Problem {
    /// The translation key a program shows the problem by; [`fmt::Display`] is its English text.
    pub fn key(&self) -> &'static str {
        match self {
            Problem::Missing(_) => "audioMissing",
            Problem::Unreadable(_) => "audioUnreadable",
            Problem::NotASound(_) => "audioNotASound",
            Problem::CannotSeek(_) => "audioCannotSeek",
            Problem::NoSpeakers(_) => "audioNoSpeakers",
        }
    }

    /// What the problem is about: the path, or the decoder's or system's reason.
    pub fn detail(&self) -> &str {
        match self {
            Problem::Missing(what) | Problem::Unreadable(what) | Problem::NotASound(what) | Problem::CannotSeek(what) | Problem::NoSpeakers(what) => what,
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Problem::Missing(_) => "no such file",
            Problem::Unreadable(_) => "cannot read",
            Problem::NotASound(_) => "not a sound",
            Problem::CannotSeek(_) => "cannot seek",
            Problem::NoSpeakers(_) => "no speakers",
        };
        write!(out, "{text}: {}", self.detail())
    }
}

impl std::error::Error for Problem {}

/// The system's default speakers, kept open as long as this lives.
pub struct Speakers(MixerDeviceSink);

impl Speakers {
    /// Opens the default output; a computer without one says why.
    pub fn open() -> Result<Speakers, Problem> {
        let mut sink = DeviceSinkBuilder::open_default_sink().map_err(|why| Problem::NoSpeakers(why.to_string()))?;
        sink.log_on_drop(false);
        Ok(Speakers(sink))
    }

    /// The mixer a [`Player`] plays on.
    pub fn mixer(&self) -> Mixer {
        self.0.mixer().clone()
    }
}

/// What a song's file says of it: ID3 in mp3, Vorbis comments in flac and ogg.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Its number on the album; 0 when the file does not say.
    pub track: u32,
}

/// The tags of the song at `path`; empty ones for a file without them, or no song.
pub fn tags(path: &Path) -> Tags {
    let hint = path.extension().map(|ext| ext.to_string_lossy().to_lowercase());
    File::open(path).map(|file| tags_of(Box::new(file), hint.as_deref())).unwrap_or_default()
}

/// The tags of sound data in memory; `hint` is its kind (mp3, flac).
pub fn tags_of_bytes(bytes: &[u8], hint: Option<&str>) -> Tags {
    tags_of(Box::new(Cursor::new(bytes.to_vec())), hint)
}

fn tags_of(source: Box<dyn symphonia::core::io::MediaSource>, hint: Option<&str>) -> Tags {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::{MetadataOptions, StandardTagKey, Tag};
    use symphonia::core::probe::Hint;
    let mut wanted = Hint::new();
    if let Some(hint) = hint {
        wanted.with_extension(hint);
    }
    let stream = MediaSourceStream::new(source, Default::default());
    let Ok(mut probed) = symphonia::default::get_probe().format(&wanted, stream, &FormatOptions::default(), &MetadataOptions::default()) else {
        return Tags::default();
    };
    // Tags before the container (ID3 of an mp3), then the container's own.
    let mut found: Vec<Tag> = Vec::new();
    if let Some(revision) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
        found.extend(revision.tags().iter().cloned());
    }
    if let Some(revision) = probed.format.metadata().current() {
        found.extend(revision.tags().iter().cloned());
    }
    let mut tags = Tags::default();
    for tag in found {
        let value = tag.value.to_string().trim().to_string();
        match tag.std_key {
            Some(StandardTagKey::TrackTitle) if tags.title.is_empty() => tags.title = value,
            Some(StandardTagKey::Artist) if tags.artist.is_empty() => tags.artist = value,
            Some(StandardTagKey::Album) if tags.album.is_empty() => tags.album = value,
            // "7" or "7/12": the number before the slash.
            Some(StandardTagKey::TrackNumber) if tags.track == 0 => tags.track = value.split('/').next().and_then(|n| n.trim().parse().ok()).unwrap_or(0),
            _ => {}
        }
    }
    tags
}

/// Where a song comes from, kept so seeking can open it again at another point.
#[derive(Clone)]
enum Origin {
    File(PathBuf),
    Bytes(Arc<[u8]>, Option<String>),
}

/// Plays one song at a time; a new one takes the place of the one playing.
///
/// Seeking opens the song again at the new point instead of asking the audio
/// thread to move, so it never waits on the sound card.
pub struct Player {
    mixer: Mixer,
    playing: Option<rodio::Player>,
    origin: Option<Origin>,
    /// Where the song started playing: zero, or the point of the last seek.
    from: Duration,
    length: Option<Duration>,
    volume: f32,
    /// What the song's file says of it.
    tags: Tags,
}

impl Player {
    /// A player on `mixer`, with nothing playing, at full volume.
    pub fn on(mixer: Mixer) -> Player {
        Player { mixer, playing: None, origin: None, from: Duration::ZERO, length: None, volume: 1.0, tags: Tags::default() }
    }

    /// Plays the file at `path`, read from disk as it plays.
    pub fn play_file(&mut self, path: &Path) -> Result<(), Problem> {
        self.start(Origin::File(path.to_path_buf()), Duration::ZERO, false)
    }

    /// Plays sound data held in memory, such as a program's resource; `hint` is its kind (mp3, flac).
    pub fn play_bytes(&mut self, bytes: Vec<u8>, hint: Option<&str>) -> Result<(), Problem> {
        self.start(Origin::Bytes(bytes.into(), hint.map(str::to_string)), Duration::ZERO, false)
    }

    /// Plays `origin` from `at`, paused when `paused`, in the place of what played.
    fn start(&mut self, origin: Origin, at: Duration, paused: bool) -> Result<(), Problem> {
        let player = rodio::Player::connect_new(&self.mixer);
        player.set_volume(self.volume);
        if paused {
            player.pause();
        }
        let length = match &origin {
            Origin::File(path) => {
                let name = path.to_string_lossy().into_owned();
                let file = File::open(path).map_err(|why| match why.kind() {
                    std::io::ErrorKind::NotFound => Problem::Missing(name.clone()),
                    _ => Problem::Unreadable(format!("{name}: {why}")),
                })?;
                let size = file.metadata().map(|meta| meta.len()).map_err(|why| Problem::Unreadable(format!("{name}: {why}")))?;
                let hint = path.extension().map(|ext| ext.to_string_lossy().to_lowercase());
                append(&player, decode(BufReader::new(file), size, hint.as_deref())?, at)?
            }
            Origin::Bytes(bytes, hint) => append(&player, decode(Cursor::new(bytes.clone()), bytes.len() as u64, hint.as_deref())?, at)?,
        };
        let tags = if at.is_zero() || self.origin.is_none() {
            match &origin {
                Origin::File(path) => tags(path),
                Origin::Bytes(bytes, hint) => tags_of_bytes(bytes, hint.as_deref()),
            }
        } else {
            std::mem::take(&mut self.tags)
        };
        self.stop();
        self.playing = Some(player);
        self.origin = Some(origin);
        self.from = at;
        self.length = length;
        self.tags = tags;
        Ok(())
    }

    /// Holds the song where it is.
    pub fn pause(&mut self) {
        if let Some(player) = &self.playing {
            player.pause();
        }
    }

    /// Plays on from where the song was paused.
    pub fn resume(&mut self) {
        if let Some(player) = &self.playing {
            player.play();
        }
    }

    /// Moves the song to `at` from its start.
    pub fn seek(&mut self, at: Duration) -> Result<(), Problem> {
        let Some(origin) = self.origin.clone() else { return Err(Problem::CannotSeek("nothing plays".into())) };
        match self.length {
            // At or past the end: the song has played to its end.
            Some(length) if at >= length => {
                if let Some(player) = self.playing.replace(rodio::Player::connect_new(&self.mixer)) {
                    player.stop();
                }
                self.from = length;
                Ok(())
            }
            _ => self.start(origin, at, self.paused()),
        }
    }

    /// Stops the song: it has not ended, and nothing plays.
    pub fn stop(&mut self) {
        if let Some(player) = self.playing.take() {
            player.stop();
        }
        self.origin = None;
        self.from = Duration::ZERO;
        self.length = None;
        self.tags = Tags::default();
    }

    /// The volume from 0 to 1, kept for the next songs.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(player) = &self.playing {
            player.set_volume(self.volume);
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// A song plays now: not paused, not ended, not stopped.
    pub fn playing(&self) -> bool {
        self.playing.as_ref().is_some_and(|player| !player.is_paused() && !player.empty())
    }

    /// A song is held by [`Player::pause`].
    pub fn paused(&self) -> bool {
        self.playing.as_ref().is_some_and(|player| player.is_paused() && !player.empty())
    }

    /// The song played to its end (a stopped one has not).
    pub fn ended(&self) -> bool {
        self.playing.as_ref().is_some_and(rodio::Player::empty)
    }

    /// How far the song has played.
    pub fn position(&self) -> Duration {
        self.playing.as_ref().map_or(Duration::ZERO, |player| self.from + player.get_pos())
    }

    /// What the song's file says of it; empty when nothing plays.
    pub fn tags(&self) -> &Tags {
        &self.tags
    }

    /// How long the song is, when its file says so.
    pub fn length(&self) -> Option<Duration> {
        self.length
    }

    /// The path of the file that plays; none for data in memory or when nothing plays.
    pub fn file(&self) -> Option<String> {
        match &self.origin {
            Some(Origin::File(path)) => Some(path.to_string_lossy().into_owned()),
            _ => None,
        }
    }
}

/// Appends `source` to `player` from `at`; gives the song's whole length.
fn append<S: Source + Send + 'static>(player: &rodio::Player, mut source: S, at: Duration) -> Result<Option<Duration>, Problem> {
    let length = source.total_duration();
    if !at.is_zero() {
        source.try_seek(at).map_err(|why| Problem::CannotSeek(why.to_string()))?;
    }
    player.append(source);
    Ok(length)
}

/// A decoder that reads `data` as it plays and can seek in it.
fn decode<R: Read + Seek + Send + Sync + 'static>(data: R, size: u64, hint: Option<&str>) -> Result<Decoder<R>, Problem> {
    let mut builder = Decoder::builder().with_data(data).with_byte_len(size).with_seekable(true);
    if let Some(hint) = hint {
        builder = builder.with_hint(hint);
    }
    builder.build().map_err(|why| Problem::NotASound(why.to_string()))
}
