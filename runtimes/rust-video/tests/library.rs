//! A codec library people add (include/clarity_codec.h): the test codec in
//! tests/codec/grey.c, built with the system's C compiler, loaded and
//! registered, wins over every other decoder for the format it takes.

#![cfg(unix)]

use std::path::PathBuf;
use std::sync::Arc;

use clarity_video::{Codec, Movie, Problem, library};

fn here(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

/// The test codec built into a library of its own; none without a C compiler.
fn grey() -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("clarity-grey-{}", std::process::id()));
    std::fs::create_dir_all(&out).ok()?;
    let library = out.join(if cfg!(target_os = "macos") { "libgrey.dylib" } else { "libgrey.so" });
    let built = std::process::Command::new("cc").args(["-shared", "-fPIC", "-o"]).arg(&library).arg(here("tests/codec/grey.c")).status().ok()?;
    built.success().then_some(library)
}

#[test]
fn a_codec_library_is_loaded_registered_and_decodes() {
    let Some(path) = grey() else {
        eprintln!("no C compiler here: left out");
        return;
    };
    let codec = library::load(&path).unwrap();
    assert_eq!(("grey", vec!["avc1".to_string()]), (codec.name(), codec.formats()));
    clarity_video::register(Arc::new(codec));
    // `open` takes it before the system's decoder and openh264.
    assert_eq!("grey", clarity_video::open(&here("tests/videos/bar.mp4")).unwrap().decoder_name());
    let mut movie = Movie::open(&here("tests/videos/bar.mp4")).unwrap();
    let mut counted = Vec::new();
    while let Some((_, picture)) = movie.next_picture().unwrap() {
        assert_eq!((64, 48), (picture.width, picture.height));
        counted.push(picture.rgba[0]);
    }
    assert_eq!((0..20).collect::<Vec<u8>>(), counted);
    clarity_video::forget_added();
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn what_is_no_codec_library_says_why() {
    assert!(matches!(library::load(&here("tests/videos/bar.mp4")), Err(Problem::CodecMissing(_))));
    assert!(matches!(library::add("libnot-there.so"), Err(Problem::CodecMissing(why)) if why.contains("libnot-there.so")));
}
