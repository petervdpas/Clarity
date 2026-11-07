//! Linux: the video through GStreamer, when it is installed (GNOME, KDE and
//! most desktops have it). Its libraries are loaded at run time, so
//! Clarity.Video needs nothing of GStreamer to build and loads fine where it
//! is missing; then openh264 plays instead. One pipeline per video:
//! `filesrc ! decodebin ! videoconvert ! RGBA ! appsink`, pulled a frame at a
//! time by the decoding thread.

use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use libloading::Library;

use crate::{Picture, Problem, Source};

type Element = *mut c_void;

const STATE_NULL: c_int = 1;
const STATE_PAUSED: c_int = 3;
const STATE_PLAYING: c_int = 4;
const CHANGE_FAILURE: c_int = 0;
const FORMAT_TIME: c_int = 3;
const SEEK_FLUSH: c_int = 1;
const SEEK_ACCURATE: c_int = 2;
const MAP_READ: c_int = 1;
/// A GstBuffer's presentation time, in nanoseconds: after the 64 bytes of
/// its GstMiniObject and its pool pointer (GStreamer 1's stable layout).
const BUFFER_PTS_OFFSET: usize = 72;
const NONE_TIME: u64 = u64::MAX;

/// GstMapInfo, as GStreamer 1 lays it out.
#[repr(C)]
struct MapInfo {
    memory: *mut c_void,
    flags: c_int,
    data: *mut u8,
    size: usize,
    maxsize: usize,
    user_data: [*mut c_void; 4],
    reserved: [*mut c_void; 4],
}

/// The functions of GStreamer, its app library and GObject that are used.
struct Gst {
    _libraries: Vec<Library>,
    parse_launch: unsafe extern "C" fn(*const c_char, *mut *mut c_void) -> Element,
    bin_get_by_name: unsafe extern "C" fn(Element, *const c_char) -> Element,
    object_set: unsafe extern "C" fn(*mut c_void, *const c_char, ...),
    set_state: unsafe extern "C" fn(Element, c_int) -> c_int,
    get_state: unsafe extern "C" fn(Element, *mut c_int, *mut c_int, u64) -> c_int,
    query_duration: unsafe extern "C" fn(Element, c_int, *mut i64) -> c_int,
    seek_simple: unsafe extern "C" fn(Element, c_int, c_int, i64) -> c_int,
    object_unref: unsafe extern "C" fn(*mut c_void),
    mini_object_unref: unsafe extern "C" fn(*mut c_void),
    sample_get_buffer: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    sample_get_caps: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    caps_get_structure: unsafe extern "C" fn(*mut c_void, c_uint) -> *mut c_void,
    structure_get_int: unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_int) -> c_int,
    structure_get_fraction: unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_int, *mut c_int) -> c_int,
    buffer_map: unsafe extern "C" fn(*mut c_void, *mut MapInfo, c_int) -> c_int,
    buffer_unmap: unsafe extern "C" fn(*mut c_void, *mut MapInfo),
    try_pull_sample: unsafe extern "C" fn(Element, u64) -> *mut c_void,
    try_pull_preroll: unsafe extern "C" fn(Element, u64) -> *mut c_void,
}

// The function pointers are plain C functions GStreamer makes safe to call from any thread.
unsafe impl Send for Gst {}
unsafe impl Sync for Gst {}

/// GStreamer, loaded and started once; none where it is not installed.
fn gst() -> Option<&'static Gst> {
    static GST: OnceLock<Option<Gst>> = OnceLock::new();
    GST.get_or_init(|| unsafe { load().ok() }).as_ref()
}

unsafe fn load() -> Result<Gst, libloading::Error> {
    unsafe {
        let core = Library::new("libgstreamer-1.0.so.0")?;
        let app = Library::new("libgstapp-1.0.so.0")?;
        let gobject = Library::new("libgobject-2.0.so.0")?;
        let init: libloading::Symbol<unsafe extern "C" fn(*mut c_int, *mut *mut *mut c_char)> = core.get(b"gst_init\0")?;
        init(std::ptr::null_mut(), std::ptr::null_mut());
        macro_rules! from {
            ($lib:expr, $name:literal) => {
                *$lib.get(concat!($name, "\0").as_bytes())?
            };
        }
        let gst = Gst {
            parse_launch: from!(core, "gst_parse_launch"),
            bin_get_by_name: from!(core, "gst_bin_get_by_name"),
            object_set: from!(gobject, "g_object_set"),
            set_state: from!(core, "gst_element_set_state"),
            get_state: from!(core, "gst_element_get_state"),
            query_duration: from!(core, "gst_element_query_duration"),
            seek_simple: from!(core, "gst_element_seek_simple"),
            object_unref: from!(core, "gst_object_unref"),
            mini_object_unref: from!(core, "gst_mini_object_unref"),
            sample_get_buffer: from!(core, "gst_sample_get_buffer"),
            sample_get_caps: from!(core, "gst_sample_get_caps"),
            caps_get_structure: from!(core, "gst_caps_get_structure"),
            structure_get_int: from!(core, "gst_structure_get_int"),
            structure_get_fraction: from!(core, "gst_structure_get_fraction"),
            buffer_map: from!(core, "gst_buffer_map"),
            buffer_unmap: from!(core, "gst_buffer_unmap"),
            try_pull_sample: from!(app, "gst_app_sink_try_pull_sample"),
            try_pull_preroll: from!(app, "gst_app_sink_try_pull_preroll"),
            _libraries: vec![core, app, gobject],
        };
        Ok(gst)
    }
}

/// How long a pull may wait for the next frame before the video counts as stuck.
const PULL_WAIT: u64 = 5_000_000_000;

/// A video playing through a GStreamer pipeline.
pub struct GstSource {
    gst: &'static Gst,
    pipeline: Element,
    sink: Element,
    size: (u32, u32),
    length: Duration,
    frame: Duration,
    /// The first picture, pulled while the pipeline was getting ready; given first.
    first: Option<(Duration, Picture)>,
    /// The time of the last picture given: the pipeline gives its first one again as it starts playing.
    given: Option<Duration>,
}

// The pipeline is used from the decoding thread only, one call at a time.
unsafe impl Send for GstSource {}

/// The video at `path` through GStreamer; none where GStreamer is not installed or cannot decode it.
pub fn open(path: &Path) -> Option<Result<Box<dyn Source>, Problem>> {
    let gst = gst()?;
    if !path.is_file() {
        return None;
    }
    unsafe { GstSource::open(gst, path).map(|s| Ok(Box::new(s) as Box<dyn Source>)) }
}

impl GstSource {
    unsafe fn open(gst: &'static Gst, path: &Path) -> Option<GstSource> {
        unsafe {
            let description = c"filesrc name=file ! decodebin ! videoconvert ! video/x-raw,format=RGBA ! appsink name=sink sync=false max-buffers=4";
            let pipeline = (gst.parse_launch)(description.as_ptr(), std::ptr::null_mut());
            if pipeline.is_null() {
                return None;
            }
            let file = (gst.bin_get_by_name)(pipeline, c"file".as_ptr());
            let sink = (gst.bin_get_by_name)(pipeline, c"sink".as_ptr());
            let location = CString::new(path.to_string_lossy().as_bytes()).ok();
            if file.is_null() || sink.is_null() || location.is_none() {
                (gst.object_unref)(pipeline);
                return None;
            }
            let location = location.unwrap_or_default();
            (gst.object_set)(file, c"location".as_ptr(), location.as_ptr(), std::ptr::null::<c_char>());
            (gst.object_unref)(file);
            let mut source = GstSource { gst, pipeline, sink, size: (0, 0), length: Duration::ZERO, frame: Duration::from_millis(40), first: None, given: None };
            // Ready when the first picture is decoded: a file it cannot decode fails here, and openh264 plays instead.
            (gst.set_state)(pipeline, STATE_PAUSED);
            let (mut state, mut pending) = (0, 0);
            if (gst.get_state)(pipeline, &mut state, &mut pending, PULL_WAIT) == CHANGE_FAILURE || state != STATE_PAUSED {
                return None;
            }
            let preroll = (gst.try_pull_preroll)(sink, PULL_WAIT);
            source.first = source.picture(preroll);
            source.first.as_ref()?;
            let mut nanos = 0i64;
            if (gst.query_duration)(pipeline, FORMAT_TIME, &mut nanos) != 0 && nanos > 0 {
                source.length = Duration::from_nanos(nanos as u64);
            }
            (gst.set_state)(pipeline, STATE_PLAYING);
            Some(source)
        }
    }

    /// A sample as a picture with its time, and the sample let go; none for none.
    unsafe fn picture(&mut self, sample: *mut c_void) -> Option<(Duration, Picture)> {
        unsafe {
            if sample.is_null() {
                return None;
            }
            let gst = self.gst;
            let caps = (gst.sample_get_caps)(sample);
            let structure = if caps.is_null() { std::ptr::null_mut() } else { (gst.caps_get_structure)(caps, 0) };
            if !structure.is_null() {
                let (mut width, mut height, mut num, mut den) = (0, 0, 0, 0);
                (gst.structure_get_int)(structure, c"width".as_ptr(), &mut width);
                (gst.structure_get_int)(structure, c"height".as_ptr(), &mut height);
                self.size = (width.max(0) as u32, height.max(0) as u32);
                if (gst.structure_get_fraction)(structure, c"framerate".as_ptr(), &mut num, &mut den) != 0 && num > 0 && den > 0 {
                    self.frame = Duration::from_secs_f64(den as f64 / num as f64);
                }
            }
            let buffer = (gst.sample_get_buffer)(sample);
            let mut picture = None;
            if !buffer.is_null() {
                let pts = *(buffer.cast::<u8>().add(BUFFER_PTS_OFFSET).cast::<u64>());
                let at = if pts == NONE_TIME { Duration::ZERO } else { Duration::from_nanos(pts) };
                let mut map: MapInfo = std::mem::zeroed();
                if (gst.buffer_map)(buffer, &mut map, MAP_READ) != 0 {
                    let (width, height) = self.size;
                    let wanted = width as usize * height as usize * 4;
                    if map.size >= wanted && wanted > 0 {
                        let rgba = std::slice::from_raw_parts(map.data, wanted).to_vec();
                        picture = Some((at, Picture { width, height, rgba }));
                    }
                    (gst.buffer_unmap)(buffer, &mut map);
                }
            }
            (gst.mini_object_unref)(sample);
            picture
        }
    }
}

impl Source for GstSource {
    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn length(&self) -> Duration {
        self.length
    }

    fn frame_length(&self) -> Duration {
        self.frame
    }

    fn next_picture(&mut self) -> Result<Option<(Duration, Picture)>, Problem> {
        if let Some(first) = self.first.take() {
            self.given = Some(first.0);
            return Ok(Some(first));
        }
        loop {
            let pulled = unsafe {
                let sample = (self.gst.try_pull_sample)(self.sink, PULL_WAIT);
                self.picture(sample)
            };
            match pulled {
                Some((at, _)) if self.given.is_some_and(|given| at <= given) => continue,
                Some(picture) => {
                    self.given = Some(picture.0);
                    return Ok(Some(picture));
                }
                None => return Ok(None),
            }
        }
    }

    fn seek(&mut self, at: Duration) -> Result<Option<(Duration, Picture)>, Problem> {
        self.first = None;
        self.given = None;
        unsafe {
            if (self.gst.seek_simple)(self.pipeline, FORMAT_TIME, SEEK_FLUSH | SEEK_ACCURATE, at.as_nanos() as i64) == 0 {
                return Err(Problem::Undecodable(format!("GStreamer could not move to {at:?}")));
            }
        }
        self.next_picture()
    }

    fn decoder_name(&self) -> String {
        "gstreamer".into()
    }
}

impl Drop for GstSource {
    fn drop(&mut self) {
        unsafe {
            (self.gst.set_state)(self.pipeline, STATE_NULL);
            (self.gst.object_unref)(self.sink);
            (self.gst.object_unref)(self.pipeline);
        }
    }
}
