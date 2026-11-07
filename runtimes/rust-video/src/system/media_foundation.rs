//! Windows: the video through Media Foundation, part of every Windows with
//! H.264 included. Its Source Reader reads the file and decodes it into
//! pixels itself (with its video processing on, so it turns them into
//! RGB32); Clarity.Video only copies them out as RGBA.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use windows::Win32::Media::MediaFoundation::{
    IMFAttributes, IMFSample, IMFSourceReader, MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE, MF_PD_DURATION, MF_SOURCE_READER_ALL_STREAMS, MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING,
    MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_SOURCE_READER_MEDIASOURCE, MF_SOURCE_READERF_ENDOFSTREAM, MF_VERSION, MFCreateAttributes, MFCreateMediaType, MFCreateSourceReaderFromURL, MFMediaType_Video, MFSTARTUP_FULL, MFStartup, MFVideoFormat_RGB32,
};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::Variant::VT_I8;
use windows::core::{GUID, HSTRING};

use crate::{Picture, Problem, Source};

const VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;

/// Media Foundation started once for the program; false where it cannot start.
fn started() -> bool {
    static STARTED: OnceLock<bool> = OnceLock::new();
    *STARTED.get_or_init(|| unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL).is_ok() })
}

/// A video read by Media Foundation's Source Reader.
pub struct MfSource {
    reader: IMFSourceReader,
    size: (u32, u32),
    /// Bytes from one row of pixels to the next, as Media Foundation lays them out.
    stride: i32,
    length: Duration,
    frame: Duration,
}

// The reader is free-threaded; the decoding thread alone uses it, one call at a time.
unsafe impl Send for MfSource {}

/// The video at `path` through Media Foundation; none when it cannot start or read it.
pub fn open(path: &Path) -> Option<Result<Box<dyn Source>, Problem>> {
    if !path.is_file() {
        return None;
    }
    unsafe { MfSource::open(path).ok().map(|s| Ok(Box::new(s) as Box<dyn Source>)) }
}

impl MfSource {
    unsafe fn open(path: &Path) -> windows::core::Result<MfSource> {
        unsafe {
            // The calling thread joins COM (a thread that has already is fine).
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            if !started() {
                return Err(windows::core::Error::empty());
            }
            let mut attributes: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attributes, 1)?;
            let attributes = attributes.ok_or_else(windows::core::Error::empty)?;
            attributes.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1)?;
            let reader = MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attributes)?;
            reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)?;
            reader.SetStreamSelection(VIDEO, true)?;
            let wanted = MFCreateMediaType()?;
            wanted.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            wanted.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
            reader.SetCurrentMediaType(VIDEO, None, &wanted)?;
            let current = reader.GetCurrentMediaType(VIDEO)?;
            let size = current.GetUINT64(&MF_MT_FRAME_SIZE)?;
            let (width, height) = ((size >> 32) as u32, size as u32);
            let stride = current.GetUINT32(&MF_MT_DEFAULT_STRIDE).map_or(width as i32 * 4, |s| s as i32);
            let rate = current.GetUINT64(&MF_MT_FRAME_RATE).unwrap_or(0);
            let (num, den) = ((rate >> 32) as u32, rate as u32);
            let frame = if num > 0 && den > 0 { Duration::from_secs_f64(den as f64 / num as f64) } else { Duration::from_millis(40) };
            let length = reader.GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION).map(|d| hundred_nanos(d.Anonymous.Anonymous.Anonymous.uhVal)).unwrap_or_default();
            Ok(MfSource { reader, size: (width, height), stride, length, frame })
        }
    }

    /// A sample's pixels as RGBA: Media Foundation's RGB32 is blue, green, red and an unset byte.
    unsafe fn picture(&self, sample: &IMFSample) -> windows::core::Result<Picture> {
        unsafe {
            let buffer = sample.ConvertToContiguousBuffer()?;
            let mut data = std::ptr::null_mut();
            let mut length = 0u32;
            buffer.Lock(&mut data, None, Some(&mut length))?;
            let (width, height) = self.size;
            let mut rgba = vec![0u8; width as usize * height as usize * 4];
            let row = width as usize * 4;
            let bytes = std::slice::from_raw_parts(data, length as usize);
            for y in 0..height as usize {
                // A negative stride is a picture stored bottom up.
                let from = if self.stride < 0 { (height as usize - 1 - y) * row } else { y * self.stride as usize };
                let Some(line) = bytes.get(from..from + row) else { break };
                for (to, px) in rgba[y * row..(y + 1) * row].chunks_exact_mut(4).zip(line.chunks_exact(4)) {
                    to.copy_from_slice(&[px[2], px[1], px[0], 255]);
                }
            }
            buffer.Unlock()?;
            Ok(Picture { width, height, rgba })
        }
    }

    fn read(&mut self) -> Result<Option<(Duration, Picture)>, Problem> {
        unsafe {
            let (mut flags, mut time) = (0u32, 0i64);
            let mut sample: Option<IMFSample> = None;
            self.reader.ReadSample(VIDEO, 0, None, Some(&mut flags), Some(&mut time), Some(&mut sample)).map_err(|why| Problem::Undecodable(why.message()))?;
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                return Ok(None);
            }
            match sample {
                Some(sample) => Ok(Some((hundred_nanos(time.max(0) as u64), self.picture(&sample).map_err(|why| Problem::Undecodable(why.message()))?))),
                // A gap in the stream: read on.
                None => self.read(),
            }
        }
    }
}

fn hundred_nanos(units: u64) -> Duration {
    Duration::from_nanos(units.saturating_mul(100))
}

impl Source for MfSource {
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
        self.read()
    }

    /// Media Foundation moves to the key frame before `at`; the frames up to `at` are read and passed.
    fn seek(&mut self, at: Duration) -> Result<Option<(Duration, Picture)>, Problem> {
        unsafe {
            let mut position = PROPVARIANT::default();
            (*position.Anonymous.Anonymous).vt = VT_I8;
            (*position.Anonymous.Anonymous).Anonymous.hVal = (at.as_nanos() / 100) as i64;
            self.reader.SetCurrentPosition(&GUID::zeroed(), &position).map_err(|why| Problem::Undecodable(why.message()))?;
        }
        let mut shown = None;
        while let Some((time, picture)) = self.read()? {
            let done = time + self.frame > at;
            shown = Some((time, picture));
            if done {
                break;
            }
        }
        Ok(shown)
    }

    fn decoder_name(&self) -> String {
        "media foundation".into()
    }
}
