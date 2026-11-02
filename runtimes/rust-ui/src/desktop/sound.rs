//! Sound on the desktop through rodio: the computer's speakers, opened the
//! first time something plays. A sound comes as WAV data (Clarity makes its
//! tones and melodies so); a program's own sound files are read as WAV too.
//! A sound that cannot be read, or a computer with no speakers, plays
//! nothing and says why on the error output, and the window goes on.

use std::cell::RefCell;
use std::num::NonZero;

use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source};

use crate::parts::sound::{Playing, SoundOutput};

/// What opens the speakers: the system's default output, or a test's.
pub type OpenSpeakers = Box<dyn Fn() -> Result<MixerDeviceSink, String>>;

/// The speakers, once opened: made, not opened, so no device is asked for until a sound plays.
pub struct Speakers {
    open: OpenSpeakers,
    sink: RefCell<Option<MixerDeviceSink>>,
    failed: RefCell<bool>,
}

impl Default for Speakers {
    fn default() -> Speakers {
        Speakers::with(Box::new(|| DeviceSinkBuilder::open_default_sink().map_err(|why| why.to_string())))
    }
}

struct Played(Option<Player>);

impl Playing for Played {
    fn stop(&mut self) {
        if let Some(player) = self.0.take() {
            player.stop();
        }
    }
}

/// WAV data as its channels, rate and samples between -1 and 1; none for data that is no PCM WAV.
pub fn read_wav(file: &[u8]) -> Option<(u16, u32, Vec<f32>)> {
    if file.len() < 12 || &file[0..4] != b"RIFF" || &file[8..12] != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let mut format: Option<(u16, u16, u32, u16)> = None;
    while at + 8 <= file.len() {
        let id = &file[at..at + 4];
        let size = u32::from_le_bytes(file[at + 4..at + 8].try_into().ok()?) as usize;
        let body = file.get(at + 8..(at + 8 + size).min(file.len()))?;
        if id == b"fmt " && body.len() >= 16 {
            let tag = u16::from_le_bytes([body[0], body[1]]);
            let channels = u16::from_le_bytes([body[2], body[3]]);
            let rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
            let bits = u16::from_le_bytes([body[14], body[15]]);
            format = Some((tag, channels, rate, bits));
        } else if id == b"data" {
            let (tag, channels, rate, bits) = format?;
            let samples: Vec<f32> = match (tag, bits) {
                (1, 8) => body.iter().map(|b| (*b as f32 - 128.0) / 128.0).collect(),
                (1, 16) => body.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect(),
                (1, 24) => body.chunks_exact(3).map(|c| (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0).collect(),
                (1, 32) => body.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0).collect(),
                (3, 32) => body.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(),
                _ => return None,
            };
            return (channels > 0 && rate > 0).then_some((channels, rate, samples));
        }
        at += 8 + size + (size & 1);
    }
    None
}

impl Speakers {
    /// Speakers that `open` opens the first time a sound plays.
    pub fn with(open: OpenSpeakers) -> Speakers {
        Speakers { open, sink: RefCell::new(None), failed: RefCell::new(false) }
    }

    /// Whether the speakers did not open: sounds play nothing, and they are not asked for again.
    pub fn failed(&self) -> bool {
        *self.failed.borrow()
    }

    /// Runs `act` with the speakers, opening them the first time; none when the computer has none.
    fn with_sink<R>(&self, act: impl FnOnce(&MixerDeviceSink) -> R) -> Option<R> {
        if self.sink.borrow().is_none() && !*self.failed.borrow() {
            match (self.open)() {
                Ok(mut sink) => {
                    sink.log_on_drop(false);
                    *self.sink.borrow_mut() = Some(sink);
                }
                Err(why) => {
                    *self.failed.borrow_mut() = true;
                    eprintln!("{}", crate::locale::fill(&crate::locale::translate("soundNoSpeakers", "sounds cannot play: {why}"), "why", &why));
                }
            }
        }
        self.sink.borrow().as_ref().map(act)
    }
}

impl SoundOutput for Speakers {
    fn play(&self, file: Vec<u8>, volume: f32, repeat: bool) -> Box<dyn Playing> {
        let Some((channels, rate, samples)) = read_wav(&file) else {
            eprintln!("{}", crate::locale::translate("soundNotWav", "a sound that is no WAV data plays nothing"));
            return Box::new(Played(None));
        };
        let (Some(channels), Some(rate)) = (NonZero::new(channels), NonZero::new(rate)) else { return Box::new(Played(None)) };
        let player = self.with_sink(|sink| {
            let player = Player::connect_new(sink.mixer());
            let buffer = SamplesBuffer::new(channels, rate, samples);
            if repeat {
                player.append(buffer.repeat_infinite());
            } else {
                player.append(buffer);
            }
            player.set_volume(volume.clamp(0.0, 1.0));
            player
        });
        Box::new(Played(player))
    }
}
