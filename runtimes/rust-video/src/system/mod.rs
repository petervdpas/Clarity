//! The system's own decoders: Media Foundation on Windows, AVFoundation on
//! macOS, GStreamer on Linux when it is installed. They read the file
//! themselves and work offline, with their royalties covered by the system.
//! Each is loaded or linked only on its own system.

use std::path::Path;

use crate::{Problem, Source};

#[cfg(target_os = "linux")]
pub mod gstreamer;
#[cfg(windows)]
pub mod media_foundation;
#[cfg(target_os = "macos")]
pub mod av_foundation;

/// The video at `path` through the system's decoder; none where the system has none (or it is switched off).
pub fn open(path: &Path) -> Option<Result<Box<dyn Source>, Problem>> {
    if std::env::var_os("CLARITY_VIDEO_NO_SYSTEM").is_some_and(|v| !v.is_empty()) {
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        gstreamer::open(path)
    }
    #[cfg(windows)]
    {
        media_foundation::open(path)
    }
    #[cfg(target_os = "macos")]
    {
        av_foundation::open(path)
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        let _ = path;
        None
    }
}
