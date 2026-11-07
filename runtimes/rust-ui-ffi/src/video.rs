//! Videos through Clarity.Video's C table (clarity_video.h): the program
//! hands it over with `clarity_ui_use_video` before its first window, and
//! every video part plays through it. The two libraries do not link each
//! other; the table is their whole contract.

use std::cell::RefCell;
use std::ffi::c_char;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::time::Duration;

use ui::parts::sound::MusicProblem;
use ui::parts::video::{Frame, VideoFrames, Videos};

use crate::{ClarityStatus, Failure, guard};

/// A video's frames in Clarity.Video, opaque here.
#[repr(C)]
pub struct ClarityVideoFrames {
    _opaque: [u8; 0],
}

/// The table of clarity_video.h, field for field.
#[repr(C)]
pub struct ClarityVideoApi {
    pub interface: u32,
    pub open: extern "C" fn(*const c_char, usize) -> *mut ClarityVideoFrames,
    pub free: extern "C" fn(*mut ClarityVideoFrames),
    pub size: extern "C" fn(*const ClarityVideoFrames, *mut u32, *mut u32),
    pub length_ms: extern "C" fn(*const ClarityVideoFrames) -> u64,
    pub frame_ms: extern "C" fn(*const ClarityVideoFrames) -> u64,
    pub picture_at: extern "C" fn(*const ClarityVideoFrames, u64, u64, *mut u8, usize, *mut u64) -> i32,
    pub ended_at: extern "C" fn(*const ClarityVideoFrames, u64) -> i32,
    pub problem: extern "C" fn(*const ClarityVideoFrames, *mut c_char, usize) -> i32,
    /// Interface 2 on: the generated program calls it, Clarity.UI does not.
    pub add_codec: extern "C" fn(*const c_char, usize) -> i32,
}

const NEW_PICTURE: i32 = 2;
const SAME_PICTURE: i32 = 1;

/// The translation key of each code of clarity_video.h, from CLARITY_VIDEO_MISSING (1) on.
const KEYS: [&str; 7] = ["videoMissing", "videoUnreadable", "videoNotAVideo", "videoNoCodec", "videoCodecMissing", "videoUndecodable", "videoFailed"];

struct Table(&'static ClarityVideoApi);

impl Table {
    /// The problem of `frames` (NULL: of the last open on this thread), or none.
    fn problem(&self, frames: *const ClarityVideoFrames) -> Option<MusicProblem> {
        let mut into = vec![0 as c_char; 1024];
        let code = (self.0.problem)(frames, into.as_mut_ptr(), into.len());
        if code == 0 {
            return None;
        }
        let detail: Vec<u8> = into.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
        let key = usize::try_from(code - 1).ok().and_then(|at| KEYS.get(at)).copied().unwrap_or("videoFailed");
        Some(MusicProblem { key: key.to_string(), detail: String::from_utf8_lossy(&detail).into_owned() })
    }
}

/// Clarity.Video as the video parts see it.
struct TableVideos(Rc<Table>);

impl Videos for TableVideos {
    fn open(&self, path: &str) -> Result<Box<dyn VideoFrames>, MusicProblem> {
        let frames = (self.0.0.open)(path.as_ptr().cast(), path.len());
        if frames.is_null() {
            return Err(self.0.problem(std::ptr::null()).unwrap_or(MusicProblem { key: "videoFailed".into(), detail: path.to_string() }));
        }
        let (mut width, mut height) = (0, 0);
        (self.0.0.size)(frames, &mut width, &mut height);
        Ok(Box::new(TableFrames { table: self.0.clone(), frames, size: (width, height), last: RefCell::new(None) }))
    }
}

/// A video part's frames, freed with it; the last picture is kept, so an unchanged one is not copied again.
struct TableFrames {
    table: Rc<Table>,
    frames: *mut ClarityVideoFrames,
    size: (u32, u32),
    last: RefCell<Option<(u64, Frame)>>,
}

impl Drop for TableFrames {
    fn drop(&mut self) {
        (self.table.0.free)(self.frames);
    }
}

impl VideoFrames for TableFrames {
    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn length(&self) -> Duration {
        Duration::from_millis((self.table.0.length_ms)(self.frames))
    }
    fn frame_length(&self) -> Duration {
        Duration::from_millis((self.table.0.frame_ms)(self.frames))
    }
    fn picture_at(&self, at: Duration) -> Option<Frame> {
        let mut last = self.last.borrow_mut();
        let known = last.as_ref().map_or(u64::MAX, |(time, _)| *time);
        let (width, height) = self.size;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let mut time = 0u64;
        match (self.table.0.picture_at)(self.frames, at.as_millis() as u64, known, pixels.as_mut_ptr(), pixels.len(), &mut time) {
            NEW_PICTURE => {
                let frame = Frame { width, height, rgba: Arc::new(pixels) };
                *last = Some((time, frame.clone()));
                Some(frame)
            }
            SAME_PICTURE => last.as_ref().map(|(_, frame)| frame.clone()),
            _ => None,
        }
    }
    fn ended_at(&self, at: Duration) -> bool {
        (self.table.0.ended_at)(self.frames, at.as_millis() as u64) != 0
    }
    fn problem(&self) -> Option<MusicProblem> {
        self.table.problem(self.frames)
    }
}

/// The table handed over, kept for the process: the desktop takes it on its own thread.
static TABLE: AtomicPtr<ClarityVideoApi> = AtomicPtr::new(std::ptr::null_mut());

/// Videos on this thread play through the table handed over, if any.
pub fn take_on_this_thread() {
    let api = TABLE.load(Ordering::Acquire);
    // SAFETY: only a checked table that lives as long as the program is kept.
    let videos = unsafe { api.as_ref() }.map(|api| Rc::new(TableVideos(Rc::new(Table(api)))) as Rc<dyn Videos>);
    ui::parts::video::set_videos(videos);
}

/// Videos play through `api` (clarity_video_api()) from now on, on this
/// thread and on the desktop's; NULL leaves video parts saying Clarity.Video is missing.
///
/// # Safety
/// `api` is NULL or Clarity.Video's table, which lives as long as the program.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_ui_use_video(api: *const ClarityVideoApi) -> ClarityStatus {
    guard(|| {
        if let Some(table) = unsafe { api.as_ref() }
            && table.interface < 1
        {
            return Err(Failure::invalid("this is no Clarity.Video table"));
        }
        TABLE.store(api.cast_mut(), Ordering::Release);
        take_on_this_thread();
        Ok(())
    })
}
