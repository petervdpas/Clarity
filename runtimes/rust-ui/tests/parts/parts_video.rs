//! The video part with a Clarity.Video of the test's own: pictures that say
//! their time, no sound, so the part runs on its own clock.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use clarity_runtime::ui::Hear;
use clarity_ui::geometry::Rect;
use clarity_ui::parts::PartHost;
use clarity_ui::parts::sound::{self, MusicProblem};
use clarity_ui::parts::video::{self, Frame, VideoFrames, VideoPart, Videos, letterboxed};
use clarity_ui::testing::{Shot, TestHost, part_named, window_of};
use clarity_ui::Screen;
use serde_json::{Value, json};

/// A second of pictures, a tenth each; the first byte of each says which tenth it is.
struct Tenths;

impl VideoFrames for Tenths {
    fn size(&self) -> (u32, u32) {
        (4, 2)
    }
    fn length(&self) -> Duration {
        Duration::from_secs(1)
    }
    fn frame_length(&self) -> Duration {
        Duration::from_millis(100)
    }
    fn picture_at(&self, at: Duration) -> Option<Frame> {
        let mut rgba = vec![0; 4 * 2 * 4];
        rgba[0] = (at.as_millis().min(999) / 100) as u8;
        Some(Frame { width: 4, height: 2, rgba: Arc::new(rgba) })
    }
    fn ended_at(&self, at: Duration) -> bool {
        at >= Duration::from_secs(1)
    }
    fn problem(&self) -> Option<MusicProblem> {
        None
    }
}

#[derive(Default)]
struct Library {
    opened: RefCell<Vec<String>>,
}

impl Videos for Library {
    fn open(&self, path: &str) -> Result<Box<dyn VideoFrames>, MusicProblem> {
        self.opened.borrow_mut().push(path.to_string());
        if path.contains("gone") {
            return Err(MusicProblem { key: "videoMissing".into(), detail: path.into() });
        }
        Ok(Box::new(Tenths))
    }
}

const SCREEN: &str = r#"<Video name="film" at="1,1" span="8" rows="4" onend="filmEnded" onfail="filmFailed" />"#;

fn hear(host: &TestHost, hear: &str, values: Vec<Value>) {
    host.hears.borrow_mut().push(Hear { part: "film".into(), hear: hear.into(), values });
    host.change();
}

fn said(host: &TestHost) -> serde_json::Map<String, Value> {
    host.state(Some("film")).and_then(|s| s.values).unwrap_or_default()
}

fn wake(screen: &mut Screen) {
    screen.wake_due(std::time::Instant::now() + Duration::from_secs(1));
}

fn part(screen: &Screen) -> &VideoPart {
    screen.element(part_named(screen, "film")).unwrap().as_any().downcast_ref::<VideoPart>().unwrap()
}

#[test]
fn a_video_plays_shows_its_pictures_and_says_what_plays() {
    sound::set_music(None);
    let library = Rc::new(Library::default());
    video::set_videos(Some(library.clone()));
    let host = TestHost::new();
    let mut screen = window_of(SCREEN, host.clone());
    hear(&host, "play", vec![json!("/videos/holiday.mp4")]);
    Shot::of(&mut screen, 400, 300, 1.0);
    assert_eq!(vec!["/videos/holiday.mp4"], *library.opened.borrow());
    assert!(part(&screen).shown().is_some());
    let values = said(&host);
    assert_eq!((Some(&json!(true)), Some(&json!(1)), Some(&json!("/videos/holiday.mp4"))), (values.get("playing"), values.get("length"), values.get("file")));
    hear(&host, "pause", vec![]);
    Shot::of(&mut screen, 400, 300, 1.0);
    assert_eq!((Some(&json!(false)), Some(&json!(true))), (said(&host).get("playing"), said(&host).get("paused")));
    hear(&host, "seek", vec![json!(0.55)]);
    Shot::of(&mut screen, 400, 300, 1.0);
    // Held at the tenth it was moved to.
    assert_eq!(5, part(&screen).shown().unwrap().rgba[0]);
    video::set_videos(None);
}

#[test]
fn a_video_that_played_to_its_end_runs_on_end_once() {
    sound::set_music(None);
    video::set_videos(Some(Rc::new(Library::default())));
    let host = TestHost::new();
    let mut screen = window_of(SCREEN, host.clone());
    hear(&host, "play", vec![json!("/videos/holiday.mp4")]);
    hear(&host, "seek", vec![json!(1.0)]);
    Shot::of(&mut screen, 400, 300, 1.0);
    wake(&mut screen);
    host.change();
    Shot::of(&mut screen, 400, 300, 1.0);
    wake(&mut screen);
    let ends: Vec<String> = host.logged().into_iter().filter(|l| l.contains(" end ")).collect();
    assert_eq!(vec!["run film end filmEnded"], ends);
    assert_eq!(Some(&json!(false)), said(&host).get("playing"));
    video::set_videos(None);
}

#[test]
fn a_video_that_cannot_play_runs_on_fail_with_why() {
    sound::set_music(None);
    video::set_videos(Some(Rc::new(Library::default())));
    let host = TestHost::new();
    let mut screen = window_of(SCREEN, host.clone());
    hear(&host, "play", vec![json!("/videos/gone.mp4")]);
    Shot::of(&mut screen, 400, 300, 1.0);
    wake(&mut screen);
    assert!(host.logged().iter().any(|l| l == "run film fail filmFailed"), "{:?}", host.logged());
    assert_eq!(Some(&json!("no such video: /videos/gone.mp4")), said(&host).get("problem"));
    // Without Clarity.Video it says so.
    video::set_videos(None);
    let host = TestHost::new();
    let mut screen = window_of(SCREEN, host.clone());
    hear(&host, "play", vec![json!("/videos/holiday.mp4")]);
    Shot::of(&mut screen, 400, 300, 1.0);
    let problem = said(&host).get("problem").and_then(|p| p.as_str().map(str::to_string)).unwrap_or_default();
    assert!(problem.contains("Clarity.Video"), "{problem}");
}

#[test]
fn a_video_of_the_resources_is_opened_from_a_file_of_its_own() {
    sound::set_music(None);
    let library = Rc::new(Library::default());
    video::set_videos(Some(library.clone()));
    let host = TestHost::new();
    host.resources.borrow_mut().insert("intro.mp4".to_string(), vec![0, 0, 0, 24]);
    let mut screen = window_of(r#"<Video name="film" at="1,1" span="8" rows="4" source="intro.mp4" />"#, host.clone());
    hear(&host, "play", vec![]);
    Shot::of(&mut screen, 400, 300, 1.0);
    let opened = library.opened.borrow().clone();
    assert!(opened.len() == 1 && opened[0].ends_with("intro.mp4"), "{opened:?}");
    // A resource plays, but it is no file of the person's.
    assert_eq!(Some(&json!("")), said(&host).get("file"));
    video::set_videos(None);
}

#[test]
fn a_picture_shows_as_large_as_fits_centred() {
    let area = Rect::new(0.0, 0.0, 400.0, 300.0);
    let wide = letterboxed(area, (1920, 1080));
    assert_eq!((0.0, 400.0), (wide.x, wide.width));
    assert!((wide.height - 225.0).abs() < 0.01 && (wide.y - 37.5).abs() < 0.01);
    let tall = letterboxed(area, (100, 200));
    assert_eq!((150.0, 300.0), (tall.width, tall.height));
    assert_eq!(125.0, tall.x);
}
