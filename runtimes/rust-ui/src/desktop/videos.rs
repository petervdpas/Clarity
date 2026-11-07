//! Videos through Clarity.Video (the `video` feature): each video part's
//! pictures decoded on a thread of their own, by the first decoder that
//! takes the file (a codec the program added, the system's, openh264).

use std::time::Duration;

use clarity_video::{Frames, Problem};

use crate::parts::sound::MusicProblem;
use crate::parts::video::{Frame, VideoFrames, Videos};

/// Clarity.Video on this computer.
#[derive(Default)]
pub struct ClarityVideo;

fn problem(problem: Problem) -> MusicProblem {
    MusicProblem { key: problem.key().to_string(), detail: problem.detail().to_string() }
}

impl Videos for ClarityVideo {
    fn open(&self, path: &str) -> Result<Box<dyn VideoFrames>, MusicProblem> {
        Frames::open(std::path::Path::new(path)).map(|frames| Box::new(Pictures { frames, last: Default::default() }) as Box<dyn VideoFrames>).map_err(problem)
    }
}

/// A video's frames, and the last picture handed out by its time: a frame is copied out once, not at every look.
struct Pictures {
    frames: Frames,
    last: std::cell::RefCell<Option<(Duration, Frame)>>,
}

impl VideoFrames for Pictures {
    fn size(&self) -> (u32, u32) {
        self.frames.size()
    }
    fn length(&self) -> Duration {
        self.frames.length()
    }
    fn frame_length(&self) -> Duration {
        Duration::from_millis(33)
    }
    fn picture_at(&self, at: Duration) -> Option<Frame> {
        let (time, picture) = self.frames.picture_at(at)?;
        let mut last = self.last.borrow_mut();
        if let Some((kept, frame)) = last.as_ref()
            && *kept == time
        {
            return Some(frame.clone());
        }
        let frame = Frame { width: picture.width, height: picture.height, rgba: std::sync::Arc::new(picture.rgba.clone()) };
        *last = Some((time, frame.clone()));
        Some(frame)
    }
    fn ended_at(&self, at: Duration) -> bool {
        self.frames.ended_at(at)
    }
    fn problem(&self) -> Option<MusicProblem> {
        self.frames.problem().map(problem)
    }
}
