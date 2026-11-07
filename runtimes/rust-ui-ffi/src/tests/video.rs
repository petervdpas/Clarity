//! Clarity.Video's table handed over as a program does: its own, on the test video.

use std::time::{Duration, Instant};

use crate::video::{ClarityVideoApi, clarity_ui_use_video};
use crate::{CLARITY_INVALID, CLARITY_OK};
use ui::parts::video::videos;

#[test]
fn videos_play_through_clarity_videos_own_table() {
    let _process = super::GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
    let table = clarity_video::clarity_video_api().cast::<ClarityVideoApi>();
    assert_eq!(CLARITY_OK, unsafe { clarity_ui_use_video(table) });
    let library = videos().expect("Clarity.Video is in use");
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../rust-video/tests/videos/bar.mp4");
    let frames = library.open(path).unwrap();
    assert_eq!((64, 48), frames.size());
    assert_eq!(Duration::from_secs(2), frames.length());
    let until = Instant::now() + Duration::from_secs(1);
    let mut picture = None;
    while picture.is_none() && Instant::now() < until && frames.problem().is_none() {
        picture = frames.picture_at(Duration::ZERO);
        std::thread::sleep(Duration::from_millis(5));
    }
    if let Some(picture) = picture {
        assert_eq!((64, 48, 64 * 48 * 4), (picture.width, picture.height, picture.rgba.len()));
        // The same picture again is the one kept, not a copy.
        let again = frames.picture_at(Duration::ZERO).unwrap();
        assert!(std::sync::Arc::ptr_eq(&picture.rgba, &again.rgba));
    }
    assert_eq!("videoMissing", library.open("/videos/gone.mp4").err().unwrap().key);
    // Another thread, as the desktop's may be, takes the same table.
    assert!(std::thread::spawn(|| {
        crate::video::take_on_this_thread();
        videos().is_some()
    })
    .join()
    .unwrap());
    let wrong = ClarityVideoApi { interface: 0, ..unsafe { std::ptr::read(table) } };
    assert_eq!(CLARITY_INVALID, unsafe { clarity_ui_use_video(&wrong) });
    assert_eq!(CLARITY_OK, unsafe { clarity_ui_use_video(std::ptr::null()) });
    assert!(videos().is_none());
}

#[test]
fn this_table_is_clarity_videos_field_for_field() {
    let header = super::audio::fields(include_str!("../../../rust-video-ffi/include/clarity_video.h"), "typedef struct ClarityVideoApi {");
    assert_eq!(10, header.len());
    assert_eq!(header, super::audio::fields(include_str!("../video.rs"), "pub struct ClarityVideoApi {"));
}
