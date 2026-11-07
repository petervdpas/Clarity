//! macOS: the video through AVFoundation, part of every macOS with H.264
//! included. An asset reader reads the picture track and decodes it into
//! BGRA pixel buffers; Clarity.Video copies them out as RGBA. A reader
//! cannot move, so seeking makes a new one that starts at the time asked.

use std::path::Path;
use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_av_foundation::{AVAssetReader, AVAssetReaderOutput, AVAssetReaderTrackOutput, AVAssetTrack, AVMediaTypeVideo, AVURLAsset};
use objc2_core_foundation::CFString;
use objc2_core_media::{CMSampleBuffer, CMTime, CMTimeRange, kCMTimePositiveInfinity};
use objc2_core_video::{CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferGetHeight, CVPixelBufferGetWidth, CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress, kCVPixelBufferPixelFormatTypeKey, kCVPixelFormatType_32BGRA};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

use crate::{Picture, Problem, Source};

/// A video read by AVFoundation's asset reader.
pub struct AvSource {
    asset: Retained<AVURLAsset>,
    track: Retained<AVAssetTrack>,
    reader: Retained<AVAssetReader>,
    output: Retained<AVAssetReaderTrackOutput>,
    size: (u32, u32),
    length: Duration,
    frame: Duration,
}

// The reader and its objects are used from the decoding thread alone, one call at a time.
unsafe impl Send for AvSource {}

/// The video at `path` through AVFoundation; none when it cannot read it.
pub fn open(path: &Path) -> Option<Result<Box<dyn Source>, Problem>> {
    if !path.is_file() {
        return None;
    }
    unsafe { AvSource::open(path).map(|s| Ok(Box::new(s) as Box<dyn Source>)) }
}

/// The settings that ask the reader for BGRA pixels.
unsafe fn bgra() -> Retained<NSDictionary<NSString, AnyObject>> {
    unsafe {
        // CFString and NSString are the same object (toll-free bridged).
        let key: &NSString = &*(kCVPixelBufferPixelFormatTypeKey as *const CFString).cast::<NSString>();
        let format = NSNumber::new_u32(kCVPixelFormatType_32BGRA);
        let value: &AnyObject = &format;
        NSDictionary::from_slices(&[key], &[value])
    }
}

/// A reader of `track` from `start` to the end, reading.
unsafe fn reader_from(asset: &AVURLAsset, track: &AVAssetTrack, start: Duration) -> Option<(Retained<AVAssetReader>, Retained<AVAssetReaderTrackOutput>)> {
    unsafe {
        let reader = AVAssetReader::assetReaderWithAsset_error(asset).ok()?;
        let output = AVAssetReaderTrackOutput::assetReaderTrackOutputWithTrack_outputSettings(track, Some(&bgra()));
        output.setAlwaysCopiesSampleData(false);
        if !reader.canAddOutput(&output) {
            return None;
        }
        reader.addOutput(&output);
        if !start.is_zero() {
            reader.setTimeRange(CMTimeRange { start: CMTime::with_seconds(start.as_secs_f64(), 90_000), duration: kCMTimePositiveInfinity });
        }
        reader.startReading().then_some((reader, output))
    }
}

impl AvSource {
    unsafe fn open(path: &Path) -> Option<AvSource> {
        unsafe {
            let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
            let asset = AVURLAsset::URLAssetWithURL_options(&url, None);
            #[allow(deprecated)]
            let tracks = asset.tracksWithMediaType(AVMediaTypeVideo?);
            let track = tracks.firstObject()?;
            let size = track.naturalSize();
            let rate = track.nominalFrameRate();
            let frame = if rate > 0.0 { Duration::from_secs_f64(1.0 / rate as f64) } else { Duration::from_millis(40) };
            let seconds = asset.duration().seconds();
            let length = if seconds.is_finite() && seconds > 0.0 { Duration::from_secs_f64(seconds) } else { Duration::ZERO };
            let (reader, output) = reader_from(&asset, &track, Duration::ZERO)?;
            Some(AvSource { asset, track, reader, output, size: (size.width.round() as u32, size.height.round() as u32), length, frame })
        }
    }

    /// A sample buffer's pixels as RGBA, with its time.
    unsafe fn picture(sample: &CMSampleBuffer) -> Option<(Duration, Picture)> {
        unsafe {
            let seconds = sample.presentation_time_stamp().seconds();
            let at = if seconds.is_finite() && seconds > 0.0 { Duration::from_secs_f64(seconds) } else { Duration::ZERO };
            let pixels = sample.image_buffer()?;
            CVPixelBufferLockBaseAddress(&pixels, CVPixelBufferLockFlags::ReadOnly);
            let (width, height) = (CVPixelBufferGetWidth(&pixels), CVPixelBufferGetHeight(&pixels));
            let (base, per_row) = (CVPixelBufferGetBaseAddress(&pixels).cast::<u8>(), CVPixelBufferGetBytesPerRow(&pixels));
            let mut rgba = vec![0u8; width * height * 4];
            if !base.is_null() {
                for y in 0..height {
                    let line = std::slice::from_raw_parts(base.add(y * per_row), width * 4);
                    for (to, px) in rgba[y * width * 4..(y + 1) * width * 4].chunks_exact_mut(4).zip(line.chunks_exact(4)) {
                        to.copy_from_slice(&[px[2], px[1], px[0], 255]);
                    }
                }
            }
            CVPixelBufferUnlockBaseAddress(&pixels, CVPixelBufferLockFlags::ReadOnly);
            Some((at, Picture { width: width as u32, height: height as u32, rgba }))
        }
    }
}

impl Source for AvSource {
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
        unsafe {
            let output: &AVAssetReaderOutput = &self.output;
            while let Some(sample) = output.copyNextSampleBuffer() {
                if let Some(picture) = AvSource::picture(&sample) {
                    return Ok(Some(picture));
                }
            }
        }
        Ok(None)
    }

    /// A new reader from `at`: AVFoundation starts it at the frame showing then.
    fn seek(&mut self, at: Duration) -> Result<Option<(Duration, Picture)>, Problem> {
        let (reader, output) = unsafe { reader_from(&self.asset, &self.track, at) }.ok_or_else(|| Problem::Undecodable(format!("AVFoundation could not move to {at:?}")))?;
        self.reader = reader;
        self.output = output;
        self.next_picture()
    }

    fn decoder_name(&self) -> String {
        "av foundation".into()
    }
}
