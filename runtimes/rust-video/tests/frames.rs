//! The player's frames: decoded on a thread of their own, a few ahead,
//! asked for by the clock's time. A codec of the test's own stands in for
//! openh264, so the test needs no binary.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clarity_video::{Codec, Decoder, Frames, Picture, Problem, Setup};

fn video(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/videos").join(name)
}

/// A codec whose pictures say which frame they are: the first byte counts them.
struct Numbered;

impl Codec for Numbered {
    fn name(&self) -> &str {
        "numbered"
    }
    fn formats(&self) -> Vec<String> {
        vec!["avc1".into()]
    }
    fn decoder(&self, setup: &Setup) -> Result<Box<dyn Decoder>, Problem> {
        Ok(Box::new(NumberedDecoder { at: 0, size: (setup.width, setup.height) }))
    }
}

struct NumberedDecoder {
    at: u8,
    size: (u32, u32),
}

impl Decoder for NumberedDecoder {
    fn decode(&mut self, _frame: &[u8]) -> Result<Option<Picture>, Problem> {
        let mut rgba = vec![0; (self.size.0 * self.size.1 * 4) as usize];
        rgba[0] = self.at;
        self.at += 1;
        Ok(Some(Picture { width: self.size.0, height: self.size.1, rgba }))
    }
}

static TURN: Mutex<()> = Mutex::new(());

/// Asks for the picture at `at` until the decoding thread has it, a second at most.
fn picture_at(frames: &Frames, at: Duration) -> Option<(Duration, Arc<Picture>)> {
    let until = Instant::now() + Duration::from_secs(1);
    loop {
        let shown = frames.picture_at(at);
        if shown.as_ref().is_some_and(|(time, _)| *time <= at && *time + Duration::from_millis(100) > at) || Instant::now() > until {
            return shown;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn the_picture_for_a_time_is_the_last_frame_at_or_before_it() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    clarity_video::register(Arc::new(Numbered));
    let frames = Frames::open(&video("bar.mp4")).unwrap();
    assert_eq!((64, 48), frames.size());
    assert_eq!(Duration::from_secs(2), frames.length());
    let (at, picture) = picture_at(&frames, Duration::ZERO).unwrap();
    assert_eq!((Duration::ZERO, 0), (at, picture.rgba[0]));
    let (at, picture) = picture_at(&frames, Duration::from_millis(350)).unwrap();
    assert_eq!((Duration::from_millis(300), 3), (at, picture.rgba[0]));
    // Far ahead: it seeks there instead of decoding every frame between (from the key frame at 1 s).
    let (at, _) = picture_at(&frames, Duration::from_millis(1720)).unwrap();
    assert_eq!(Duration::from_millis(1700), at);
    // Back: it seeks back.
    let (at, _) = picture_at(&frames, Duration::from_millis(200)).unwrap();
    assert_eq!(Duration::from_millis(200), at);
    // After the last frame: the last picture stays, and the frames have ended.
    let until = Instant::now() + Duration::from_secs(1);
    while frames.picture_at(Duration::from_secs(5)).is_none_or(|(at, _)| at < Duration::from_millis(1900)) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(Duration::from_millis(1900), frames.picture_at(Duration::from_secs(5)).unwrap().0);
    assert!(frames.ended_at(Duration::from_secs(5)));
    assert!(!frames.ended_at(Duration::from_millis(500)));
    clarity_video::forget_added();
}

#[test]
fn a_video_without_its_codec_says_why_through_the_frames() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    struct Failing;
    impl Codec for Failing {
        fn name(&self) -> &str {
            "failing"
        }
        fn formats(&self) -> Vec<String> {
            vec!["avc1".into()]
        }
        fn decoder(&self, _: &Setup) -> Result<Box<dyn Decoder>, Problem> {
            Err(Problem::CodecMissing("offline".into()))
        }
    }
    clarity_video::register(Arc::new(Failing));
    let frames = Frames::open(&video("bar.mp4")).unwrap();
    let until = Instant::now() + Duration::from_secs(1);
    while frames.problem().is_none() && Instant::now() < until {
        let _ = frames.picture_at(Duration::ZERO);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(Some(Problem::CodecMissing("offline".into())), frames.problem());
    assert!(frames.picture_at(Duration::ZERO).is_none());
    clarity_video::forget_added();
    assert!(matches!(Frames::open(&video("gone.mp4")), Err(Problem::Missing(_))));
}
