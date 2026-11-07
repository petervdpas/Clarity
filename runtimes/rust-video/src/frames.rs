//! A video's pictures for a player: decoded on a thread of their own, a few
//! frames ahead, and asked for by the time of the player's clock. The window
//! never waits on decoding, nor on the codec being fetched the first time:
//! until a picture is there it shows the one before. A clock that jumps (a
//! seek, back or far ahead) makes the thread seek too.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::{Picture, Problem, Source};

/// How many frames the thread keeps decoded ahead of the clock.
const AHEAD: usize = 8;

/// How far ahead of what is decoded the clock may be before the thread seeks there instead.
const FAR: Duration = Duration::from_secs(1);

#[derive(Default)]
struct State {
    /// The pictures decoded ahead, in time order.
    queue: VecDeque<(Duration, Arc<Picture>)>,
    /// The picture shown now: the last one at or before the clock.
    shown: Option<(Duration, Arc<Picture>)>,
    seek: Option<Duration>,
    /// The thread decoded the last frame.
    done: bool,
    problem: Option<Problem>,
    stop: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

/// A video's pictures, decoding ahead on their own thread while this lives.
pub struct Frames {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
    size: (u32, u32),
    length: Duration,
    frame: Duration,
    last: Duration,
}

impl Frames {
    /// Opens the video at `path` with the first decoder that takes it (`crate::open`) and starts decoding its first frames.
    pub fn open(path: &Path) -> Result<Frames, Problem> {
        Frames::of(crate::open(path)?)
    }

    /// The pictures of `movie`, decoded on their own thread.
    pub fn of(movie: Box<dyn Source>) -> Result<Frames, Problem> {
        let size = movie.size();
        let length = movie.length();
        // The last frame's time: what shows from then on.
        let frame = movie.frame_length();
        let last = length.saturating_sub(frame);
        let shared = Arc::new(Shared { state: Mutex::new(State::default()), wake: Condvar::new() });
        let decoding = shared.clone();
        let thread = std::thread::Builder::new()
            .name("clarity-video".into())
            .spawn(move || decode(movie, &decoding))
            .map_err(|why| Problem::Unreadable(why.to_string()))?;
        Ok(Frames { shared, thread: Some(thread), size, length, frame, last })
    }

    /// Width and height in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn length(&self) -> Duration {
        self.length
    }

    /// How long one frame shows.
    pub fn frame_length(&self) -> Duration {
        self.frame
    }

    /// Why the pictures stopped coming: the codec could not be had, a frame did not decode.
    pub fn problem(&self) -> Option<Problem> {
        self.lock().problem.clone()
    }

    /// Whether the clock at `at` is past the last frame.
    pub fn ended_at(&self, at: Duration) -> bool {
        at >= self.length || (at > self.last && self.lock().done)
    }

    /// The picture to show when the clock says `at`, and its own time: the
    /// last decoded frame at or before `at`; none before the first is there.
    pub fn picture_at(&self, at: Duration) -> Option<(Duration, Arc<Picture>)> {
        let mut state = self.lock();
        if state.seek.is_none() {
            while state.queue.front().is_some_and(|(time, _)| *time <= at) {
                state.shown = state.queue.pop_front();
            }
            let shown = state.shown.as_ref().map(|(time, _)| *time);
            let behind = shown.is_some_and(|time| at + Duration::from_millis(1) < time);
            let far = state.queue.is_empty() && !state.done && shown.is_some_and(|time| at > time + FAR);
            if behind || far {
                state.seek = Some(at);
                state.queue.clear();
            }
        }
        let shown = state.shown.clone();
        self.shared.wake.notify_all();
        shown
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.shared.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Drop for Frames {
    fn drop(&mut self) {
        self.lock().stop = true;
        self.shared.wake.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The thread's work: decode ahead, seek when asked, sleep when enough is decoded.
fn decode(mut movie: Box<dyn Source>, shared: &Shared) {
    loop {
        let seek = {
            let mut state = shared.state.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if state.stop {
                    return;
                }
                if state.seek.is_some() || (state.queue.len() < AHEAD && !state.done && state.problem.is_none()) {
                    break;
                }
                state = shared.wake.wait(state).unwrap_or_else(|e| e.into_inner());
            }
            state.seek
        };
        let decoded = match seek {
            Some(at) => movie.seek(at),
            None => movie.next_picture(),
        };
        let mut state = shared.state.lock().unwrap_or_else(|e| e.into_inner());
        match decoded {
            Ok(Some((time, picture))) => {
                let picture = (time, Arc::new(picture));
                if seek.is_some() {
                    // Asked again while seeking: the newer wish wins, this picture is let go.
                    if state.seek == seek {
                        state.seek = None;
                        state.done = false;
                        state.queue.clear();
                        state.shown = Some(picture);
                    }
                } else if state.seek.is_none() {
                    state.queue.push_back(picture);
                }
            }
            Ok(None) => {
                if state.seek == seek {
                    state.seek = None;
                }
                state.done = true;
            }
            Err(problem) => {
                state.seek = None;
                state.problem = Some(problem);
            }
        }
    }
}
