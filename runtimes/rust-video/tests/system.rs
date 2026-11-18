//! The system's own decoder on Windows (Media Foundation). A program opens
//! its first video on its UI thread, before the window: the window's library
//! (winit) then joins that thread to a single-threaded COM apartment for drag
//! and drop, and panics when the thread was already made multithreaded
//! ("OleInitialize failed! RPC_E_CHANGED_MODE"), so the window never opened.
//! Opening a video leaves the caller's thread as it found it.

#![cfg(windows)]

use std::path::Path;

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

#[test]
fn opening_a_video_leaves_the_threads_apartment_to_the_window() {
    let video = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/videos/bar.mp4");
    std::thread::spawn(move || {
        let mut source = clarity_video::system::open(&video).expect("Media Foundation is on every Windows").expect("bar.mp4 opens");
        assert!(source.next_picture().unwrap().is_some(), "it decodes on the thread that opened it");
        // What winit does next on the UI thread: a single-threaded apartment.
        let joined = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        assert!(joined.is_ok(), "the thread is still free for the window: {joined:?}");
        // The decoding thread, which joins no apartment of its own, reads on.
        std::thread::spawn(move || assert!(source.next_picture().unwrap().is_some(), "it decodes on another thread")).join().unwrap();
    })
    .join()
    .unwrap();
}
