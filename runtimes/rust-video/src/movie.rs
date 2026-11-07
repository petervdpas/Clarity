//! A video file: its picture track read from an MP4 file, frame by frame,
//! decoded by the codec for its format. It keeps where it is, so the next
//! picture follows on; moving to a time decodes from the key frame before it.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use mp4::{MediaType, Mp4Reader, TrackType};

use crate::{Decoder, Picture, Problem, Setup, Source};

/// An opened video: its size, length and frames, and the decoder its codec gave.
pub struct Movie {
    reader: Mp4Reader<BufReader<File>>,
    track: u32,
    timescale: u32,
    frames: u32,
    setup: Setup,
    length: Duration,
    decoder: Option<Box<dyn Decoder>>,
    /// The next frame to decode, counted from 1 as MP4 counts them.
    next: u32,
}

impl Movie {
    /// Opens the video at `path`: its first picture track. The codec is asked for when the first picture is.
    pub fn open(path: &Path) -> Result<Movie, Problem> {
        let name = path.to_string_lossy().into_owned();
        let file = File::open(path).map_err(|why| match why.kind() {
            std::io::ErrorKind::NotFound => Problem::Missing(name.clone()),
            _ => Problem::Unreadable(format!("{name}: {why}")),
        })?;
        let size = file.metadata().map(|m| m.len()).map_err(|why| Problem::Unreadable(format!("{name}: {why}")))?;
        let reader = Mp4Reader::read_header(BufReader::new(file), size).map_err(|why| Problem::NotAVideo(format!("{name}: {why}")))?;
        let (track, found) = reader
            .tracks()
            .iter()
            .filter(|(_, t)| matches!(t.track_type(), Ok(TrackType::Video)))
            .min_by_key(|(id, _)| **id)
            .map(|(id, t)| (*id, t))
            .ok_or_else(|| Problem::NotAVideo(format!("{name}: no picture track")))?;
        let format = match found.media_type() {
            Ok(MediaType::H264) => "avc1".to_string(),
            Ok(other) => other.to_string(),
            Err(why) => return Err(Problem::NotAVideo(format!("{name}: {why}"))),
        };
        let parameters = match format.as_str() {
            "avc1" => [found.sequence_parameter_set(), found.picture_parameter_set()].into_iter().filter_map(|p| p.ok().map(<[u8]>::to_vec)).collect(),
            _ => Vec::new(),
        };
        let setup = Setup { format, width: found.width() as u32, height: found.height() as u32, parameters };
        let (timescale, frames, length) = (found.timescale().max(1), found.sample_count(), found.duration());
        Ok(Movie { reader, track, timescale, frames, setup, length, decoder: None, next: 1 })
    }

    /// What its codec is made for: the format, size and the codec's setup.
    pub fn setup(&self) -> &Setup {
        &self.setup
    }

    pub fn width(&self) -> u32 {
        self.setup.width
    }

    pub fn height(&self) -> u32 {
        self.setup.height
    }

    /// How long the video is.
    pub fn length(&self) -> Duration {
        self.length
    }

    /// How long one frame shows: the length over the frames.
    pub fn frame_length(&self) -> Duration {
        if self.frames == 0 { Duration::ZERO } else { self.length / self.frames }
    }

    /// How many frames it has.
    pub fn frames(&self) -> u32 {
        self.frames
    }

    /// The decoder of the codec for its format, made the first time a picture is asked for.
    fn decoder(&mut self) -> Result<&mut Box<dyn Decoder>, Problem> {
        if self.decoder.is_none() {
            let codec = crate::codec_for(&self.setup.format).ok_or_else(|| Problem::NoCodec(self.setup.format.clone()))?;
            self.decoder = Some(codec.decoder(&self.setup)?);
        }
        Ok(self.decoder.as_mut().expect("made above"))
    }

    /// The next picture and the time it shows at; none after the last frame.
    pub fn next_picture(&mut self) -> Result<Option<(Duration, Picture)>, Problem> {
        while self.next <= self.frames {
            let sample = self.reader.read_sample(self.track, self.next).map_err(|why| Problem::Unreadable(why.to_string()))?;
            self.next += 1;
            let Some(sample) = sample else { continue };
            let at = self.time(sample.start_time as i64 + sample.rendering_offset as i64);
            if let Some(picture) = self.decoder()?.decode(&sample.bytes)? {
                return Ok(Some((at, picture)));
            }
        }
        Ok(None)
    }

    /// Moves to `at`: decodes again from the key frame at or before the
    /// frame that shows at `at` up to that frame, and gives its picture; the
    /// next picture is the one after it.
    pub fn seek(&mut self, at: Duration) -> Result<Option<(Duration, Picture)>, Problem> {
        let wanted = (at.as_secs_f64() * self.timescale as f64).round() as u64;
        let (mut key, mut target) = (1, 1);
        for id in 1..=self.frames {
            let Some(sample) = self.reader.read_sample(self.track, id).map_err(|why| Problem::Unreadable(why.to_string()))? else { continue };
            if sample.start_time > wanted {
                break;
            }
            target = id;
            if sample.is_sync {
                key = id;
            }
        }
        // A fresh decoder: what it held belonged to the frames before.
        self.decoder = None;
        self.next = key;
        let mut shown = None;
        while self.next <= target {
            match self.next_picture()? {
                Some(picture) => shown = Some(picture),
                None => break,
            }
        }
        Ok(shown)
    }

    fn time(&self, units: i64) -> Duration {
        Duration::from_secs_f64(units.max(0) as f64 / self.timescale as f64)
    }
}

impl Source for Movie {
    fn size(&self) -> (u32, u32) {
        (self.width(), self.height())
    }
    fn length(&self) -> Duration {
        Movie::length(self)
    }
    fn frame_length(&self) -> Duration {
        Movie::frame_length(self)
    }
    fn next_picture(&mut self) -> Result<Option<(Duration, Picture)>, Problem> {
        Movie::next_picture(self)
    }
    fn seek(&mut self, at: Duration) -> Result<Option<(Duration, Picture)>, Problem> {
        Movie::seek(self, at)
    }
    fn decoder_name(&self) -> String {
        crate::codec_for(&self.setup.format).map_or_else(|| "none".to_string(), |c| c.name().to_string())
    }
}
