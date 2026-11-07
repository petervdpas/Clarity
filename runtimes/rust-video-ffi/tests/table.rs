//! The table as a C program uses it, on the test video, and the header
//! against the Rust table: the same fields in the same order.

use std::ffi::c_char;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clarity_video::{ClarityVideoApi, MISSING, NEW_PICTURE, NO_PICTURE, SAME_PICTURE, clarity_video_api};

fn api() -> &'static ClarityVideoApi {
    unsafe { &*clarity_video_api() }
}

fn video(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rust-video/tests/videos").join(name).to_string_lossy().into_owned()
}

#[test]
fn a_video_opens_and_gives_its_pictures_through_the_table() {
    let path = video("bar.mp4");
    let frames = (api().open)(path.as_ptr().cast(), path.len());
    assert!(!frames.is_null());
    let (mut width, mut height) = (0, 0);
    (api().size)(frames, &mut width, &mut height);
    assert_eq!((64, 48), (width, height));
    assert_eq!(2000, (api().length_ms)(frames));
    let mut pixels = vec![0u8; 64 * 48 * 4];
    let mut time = 0u64;
    // The decoding thread has the first picture soon; a second at most, or the decoder is missing here.
    let until = Instant::now() + Duration::from_secs(1);
    let mut got = NO_PICTURE;
    while got == NO_PICTURE && Instant::now() < until {
        got = (api().picture_at)(frames, 0, u64::MAX, pixels.as_mut_ptr(), pixels.len(), &mut time);
        std::thread::sleep(Duration::from_millis(5));
    }
    if got == NO_PICTURE {
        let mut why = [0 as c_char; 256];
        eprintln!("no decoder here (code {}): left out", (api().problem)(frames, why.as_mut_ptr(), why.len()));
    } else {
        assert_eq!((NEW_PICTURE, 0), (got, time));
        assert_eq!(SAME_PICTURE, (api().picture_at)(frames, 0, time, pixels.as_mut_ptr(), pixels.len(), &mut time));
    }
    assert_eq!(1, (api().ended_at)(frames, 5000));
    assert_eq!(0, (api().ended_at)(frames, 500));
    (api().free)(frames);
}

#[test]
fn what_cannot_open_gives_its_code_and_says_about_what() {
    let gone = video("gone.mp4");
    assert!((api().open)(gone.as_ptr().cast(), gone.len()).is_null());
    let mut why = [0 as c_char; 512];
    assert_eq!(MISSING, (api().problem)(std::ptr::null(), why.as_mut_ptr(), why.len()));
    let detail: String = why.iter().take_while(|c| **c != 0).map(|c| *c as u8 as char).collect();
    assert_eq!(gone, detail);
    (api().free)(std::ptr::null_mut());
    assert_eq!(0, (api().length_ms)(std::ptr::null()));
    // A codec library that is not there says so.
    let codec = "libnot-there.so";
    assert_eq!(clarity_video::CODEC_MISSING, (api().add_codec)(codec.as_ptr().cast(), codec.len()));
}

/// The field names of the struct after `start` in `source`, in order: C or Rust.
pub fn fields(source: &str, start: &str) -> Vec<String> {
    let body = source.split(start).nth(1).unwrap().split('}').next().unwrap();
    body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("/*") && !l.starts_with('*') && !l.starts_with("//"))
        .map(|l| {
            if let Some(rest) = l.strip_prefix("pub ") {
                rest.split(':').next().unwrap().to_string()
            } else if let Some(at) = l.find("(*") {
                l[at + 2..].split(')').next().unwrap().to_string()
            } else {
                l.trim_end_matches(';').rsplit(' ').next().unwrap().to_string()
            }
        })
        .collect()
}

#[test]
fn the_header_and_the_table_have_the_same_fields_in_order() {
    let c = fields(include_str!("../include/clarity_video.h"), "typedef struct ClarityVideoApi {");
    assert_eq!(10, c.len(), "{c:?}");
    assert_eq!(c, fields(include_str!("../src/lib.rs"), "pub struct ClarityVideoApi {"));
}
