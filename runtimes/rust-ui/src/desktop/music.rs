//! Songs through Clarity.Audio (the `audio` feature): one player per sound
//! part, all on the speakers, which open the first time a song plays. When
//! they do not open, each song says why and the window goes on.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use clarity_audio::{Player, Problem, Speakers};

use crate::parts::sound::{Music, MusicPlayer, MusicProblem, SongTags};

/// Clarity.Audio on the system's speakers.
#[derive(Default)]
pub struct ClarityAudio {
    speakers: RefCell<Option<Rc<Speakers>>>,
}

fn problem(problem: Problem) -> MusicProblem {
    MusicProblem { key: problem.key().to_string(), detail: problem.detail().to_string() }
}

impl Music for ClarityAudio {
    fn player(&self) -> Result<Box<dyn MusicPlayer>, MusicProblem> {
        let speakers = match self.speakers.borrow().clone() {
            Some(speakers) => speakers,
            None => {
                let opened = Rc::new(Speakers::open().map_err(problem)?);
                *self.speakers.borrow_mut() = Some(opened.clone());
                opened
            }
        };
        Ok(Box::new(Song { player: Player::on(speakers.mixer()), _speakers: speakers }))
    }
}

/// A sound part's song; it keeps the speakers open while it lives.
struct Song {
    player: Player,
    _speakers: Rc<Speakers>,
}

impl MusicPlayer for Song {
    fn play_file(&mut self, path: &str) -> Result<(), MusicProblem> {
        self.player.play_file(std::path::Path::new(path)).map_err(problem)
    }
    fn play_bytes(&mut self, bytes: Vec<u8>, hint: Option<&str>) -> Result<(), MusicProblem> {
        self.player.play_bytes(bytes, hint).map_err(problem)
    }
    fn pause(&mut self) {
        self.player.pause();
    }
    fn resume(&mut self) {
        self.player.resume();
    }
    fn seek(&mut self, at: Duration) -> Result<(), MusicProblem> {
        self.player.seek(at).map_err(problem)
    }
    fn stop(&mut self) {
        self.player.stop();
    }
    fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume);
    }
    fn playing(&self) -> bool {
        self.player.playing()
    }
    fn paused(&self) -> bool {
        self.player.paused()
    }
    fn ended(&self) -> bool {
        self.player.ended()
    }
    fn position(&self) -> Duration {
        self.player.position()
    }
    fn length(&self) -> Option<Duration> {
        self.player.length()
    }
    fn tags(&self) -> SongTags {
        let tags = self.player.tags();
        SongTags { title: tags.title.clone(), artist: tags.artist.clone(), album: tags.album.clone(), track: tags.track }
    }
}
