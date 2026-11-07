//! Reading a video and decoding its pictures. The container and the codec
//! interface are tested with a codec of the test's own; openh264 itself
//! decodes when Cisco's binary is in the cache (fetched by the ignored
//! `fetch_openh264`), and says it was left out otherwise.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clarity_video::{Codec, Decoder, Movie, Picture, Problem, Setup, openh264};

fn video(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/videos").join(name)
}

/// The registry is the process's: tests that add codecs take turns.
static TURN: Mutex<()> = Mutex::new(());

/// A codec that takes H.264 and counts the frames it is given: a grey picture each.
struct Counting {
    given: Arc<Mutex<Vec<usize>>>,
    setups: Arc<Mutex<Vec<Setup>>>,
}

impl Codec for Counting {
    fn name(&self) -> &str {
        "counting"
    }
    fn formats(&self) -> Vec<String> {
        vec!["avc1".into()]
    }
    fn decoder(&self, setup: &Setup) -> Result<Box<dyn Decoder>, Problem> {
        self.setups.lock().unwrap().push(setup.clone());
        Ok(Box::new(CountingDecoder { given: self.given.clone(), size: (setup.width, setup.height) }))
    }
}

struct CountingDecoder {
    given: Arc<Mutex<Vec<usize>>>,
    size: (u32, u32),
}

impl Decoder for CountingDecoder {
    fn decode(&mut self, frame: &[u8]) -> Result<Option<Picture>, Problem> {
        self.given.lock().unwrap().push(frame.len());
        Ok(Some(Picture { width: self.size.0, height: self.size.1, rgba: vec![128; (self.size.0 * self.size.1 * 4) as usize] }))
    }
}

/// What a counting codec was given: the frames' sizes, and the setups of its decoders.
type Counted = (Arc<Mutex<Vec<usize>>>, Arc<Mutex<Vec<Setup>>>);

fn counting() -> Counted {
    let given = Arc::new(Mutex::new(Vec::new()));
    let setups = Arc::new(Mutex::new(Vec::new()));
    clarity_video::register(Arc::new(Counting { given: given.clone(), setups: setups.clone() }));
    (given, setups)
}

#[test]
fn a_video_says_its_size_length_and_frames() {
    let movie = Movie::open(&video("bar.mp4")).unwrap();
    assert_eq!((64, 48), (movie.width(), movie.height()));
    assert_eq!(20, movie.frames());
    assert_eq!(Duration::from_secs(2), movie.length());
    let setup = movie.setup();
    assert_eq!("avc1", setup.format);
    // The sequence and picture parameter sets, from the file.
    assert_eq!(2, setup.parameters.len());
    assert_eq!(7, setup.parameters[0][0] & 0x1f);
    assert_eq!(8, setup.parameters[1][0] & 0x1f);
}

#[test]
fn a_codec_added_takes_the_frames_in_order() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let (given, setups) = counting();
    let mut movie = Movie::open(&video("bar.mp4")).unwrap();
    let mut times = Vec::new();
    while let Some((at, picture)) = movie.next_picture().unwrap() {
        assert_eq!((64, 48), (picture.width, picture.height));
        times.push(at.as_millis());
    }
    assert_eq!((0..20).map(|i| i * 100).collect::<Vec<u128>>(), times);
    assert_eq!(20, given.lock().unwrap().len());
    assert_eq!(1, setups.lock().unwrap().len(), "one decoder for the whole video");
    clarity_video::forget_added();
}

#[test]
fn seeking_decodes_from_the_key_frame_before() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let (given, setups) = counting();
    let mut movie = Movie::open(&video("bar.mp4")).unwrap();
    // Key frames every 10: frame 16 (at 1.5 s) is decoded from frame 11.
    let (at, _) = movie.seek(Duration::from_millis(1550)).unwrap().unwrap();
    assert_eq!(Duration::from_millis(1500), at);
    assert_eq!(6, given.lock().unwrap().len());
    assert_eq!(1, setups.lock().unwrap().len());
    // And it goes on after it.
    assert_eq!(Duration::from_millis(1600), movie.next_picture().unwrap().unwrap().0);
    clarity_video::forget_added();
}

#[test]
fn what_cannot_open_says_why() {
    assert!(matches!(Movie::open(&video("gone.mp4")), Err(Problem::Missing(_))));
    assert!(matches!(Movie::open(&video("not-a-video.mp4")), Err(Problem::NotAVideo(_))));
    assert_eq!("videoNoCodec", Problem::NoCodec("vp09".into()).key());
    assert_eq!("no codec plays: vp09", Problem::NoCodec("vp09".into()).to_string());
}

#[test]
fn the_base_codec_takes_h264_and_one_added_wins() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    clarity_video::forget_added();
    assert_eq!(Some("openh264".to_string()), clarity_video::codec_for("avc1").map(|c| c.name().to_string()));
    assert!(clarity_video::codec_for("vp09").is_none());
    let _ = counting();
    assert_eq!(Some("counting".to_string()), clarity_video::codec_for("avc1").map(|c| c.name().to_string()));
    clarity_video::forget_added();
}

#[test]
fn cisco_has_a_binary_for_each_system_clarity_runs_on() {
    assert_eq!(Some("libopenh264-2.6.0-linux64.8.so".to_string()), openh264::binary_name("linux", "x86_64"));
    assert_eq!(Some("openh264-2.6.0-win64.dll".to_string()), openh264::binary_name("windows", "x86_64"));
    assert_eq!(Some("libopenh264-2.6.0-mac-arm64.dylib".to_string()), openh264::binary_name("macos", "aarch64"));
    assert_eq!(None, openh264::binary_name("freebsd", "x86_64"));
    assert_eq!("https://ciscobinary.openh264.org/openh264-2.6.0-win64.dll.bz2", openh264::binary_url("openh264-2.6.0-win64.dll"));
    let env = |name: &str| match name {
        "HOME" => Some("/home/ann".to_string()),
        "LOCALAPPDATA" => Some("C:/Users/ann/AppData/Local".to_string()),
        _ => None,
    };
    assert_eq!(Some(PathBuf::from("/home/ann/.cache/clarity/openh264/2.6.0")), openh264::cache_dir_in("linux", env));
    assert_eq!(Some(PathBuf::from("/home/ann/Library/Caches/clarity/openh264/2.6.0")), openh264::cache_dir_in("macos", env));
    assert_eq!(Some(PathBuf::from("C:/Users/ann/AppData/Local/clarity/openh264/2.6.0")), openh264::cache_dir_in("windows", env));
}

#[test]
fn mp4_frames_become_annex_b() {
    let frame = [0, 0, 0, 2, 0x65, 0xAA, 0, 0, 0, 1, 0x41];
    assert_eq!(Some(vec![0, 0, 0, 1, 0x65, 0xAA, 0, 0, 0, 1, 0x41]), openh264::length_prefixed_to_annex_b(&frame));
    assert_eq!(None, openh264::length_prefixed_to_annex_b(&[0, 0, 0, 9, 1]));
}

/// openh264 decodes the bar: white on black, a little further right each frame.
#[test]
fn openh264_decodes_the_pictures() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    clarity_video::forget_added();
    if openh264::cached().is_none() {
        eprintln!("openh264 is not in the cache: run fetch_openh264 to decode for real");
        return;
    }
    let mut movie = Movie::open(&video("bar.mp4")).unwrap();
    let mut bars = Vec::new();
    while let Some((_, picture)) = movie.next_picture().unwrap() {
        assert_eq!((64, 48), (picture.width, picture.height));
        // Where the bar is on the middle row: the first bright pixel.
        let row = &picture.rgba[(24 * 64 * 4)..(25 * 64 * 4)];
        bars.push(row.chunks(4).position(|px| px[0] > 160).unwrap_or(99));
    }
    assert_eq!(20, bars.len());
    assert!(bars.windows(2).all(|w| w[1] >= w[0]), "{bars:?}");
    assert!(bars[0] <= 2 && bars[19] >= 36, "{bars:?}");
    // Seeking lands on the right picture.
    let (_, picture) = movie.seek(Duration::from_millis(1000)).unwrap().unwrap();
    let row = &picture.rgba[(24 * 64 * 4)..(25 * 64 * 4)];
    assert_eq!(bars[10], row.chunks(4).position(|px| px[0] > 160).unwrap_or(99));
}

/// Fetches Cisco's binary into Clarity's cache (network), as the release
/// workflow takes it for the archives: unpacked, and kept only when openh264
/// knows its hash.
#[test]
#[ignore = "fetches Cisco's openh264 binary"]
fn fetch_openh264() {
    use std::io::Read;
    let name = openh264::binary_name(std::env::consts::OS, std::env::consts::ARCH).unwrap();
    let folder = openh264::cache_dir_in(std::env::consts::OS, |n| std::env::var(n).ok()).unwrap();
    let mut reply = ureq::get(&openh264::binary_url(&name)).call().unwrap();
    let packed = reply.body_mut().with_config().limit(32 * 1024 * 1024).read_to_vec().unwrap();
    let mut binary = Vec::new();
    bzip2::read::BzDecoder::new(packed.as_slice()).read_to_end(&mut binary).unwrap();
    std::fs::create_dir_all(&folder).unwrap();
    let fresh = folder.join(format!("{name}.part"));
    std::fs::write(&fresh, &binary).unwrap();
    ::openh264::OpenH264API::from_blob_path(&fresh).expect("Cisco's binary, as openh264 knows it");
    std::fs::rename(&fresh, folder.join(&name)).unwrap();
    assert!(openh264::cached().is_some());
}

/// Writes tests/videos/bar.mp4 with openh264's encoder: 64 by 48, 20 frames
/// at 10 a second, a key frame every 10, a white bar moving right on black.
#[test]
#[ignore = "writes the test video; needs openh264 in the cache"]
fn make_bar_video() {
    use mp4::{AvcConfig, FourCC, MediaConfig, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig, TrackType};
    use ::openh264::encoder::{Encoder, EncoderConfig};
    use ::openh264::formats::{RgbSliceU8, YUVBuffer};
    let api = ::openh264::OpenH264API::from_blob_path(openh264::cached().expect("run fetch_openh264 first")).unwrap();
    let mut encoder = Encoder::with_api_config(api, EncoderConfig::new()).unwrap();
    let file = std::fs::File::create(video("bar.mp4")).unwrap();
    let config = Mp4Config { major_brand: FourCC::from(*b"isom"), minor_version: 512, compatible_brands: vec![FourCC::from(*b"isom"), FourCC::from(*b"avc1")], timescale: 1000 };
    let mut writer = Mp4Writer::write_start(file, &config).unwrap();
    let (mut sps, mut pps) = (Vec::new(), Vec::new());
    let mut samples = Vec::new();
    for i in 0..20u64 {
        let mut rgb = vec![0u8; 64 * 48 * 3];
        for y in 0..48 {
            for x in (i as usize * 2)..(i as usize * 2 + 6) {
                rgb[(y * 64 + x) * 3..(y * 64 + x) * 3 + 3].copy_from_slice(&[255, 255, 255]);
            }
        }
        if i % 10 == 0 {
            encoder.force_intra_frame();
        }
        let yuv = YUVBuffer::from_rgb_source(RgbSliceU8::new(&rgb, (64, 48)));
        let stream = encoder.encode(&yuv).unwrap().to_vec();
        // Annex B into NAL units; the parameter sets go into the track's setup.
        let mut units = Vec::new();
        let mut at = 0;
        let starts: Vec<usize> = (0..stream.len().saturating_sub(3)).filter(|&k| stream[k..k + 4] == [0, 0, 0, 1]).collect();
        for (n, start) in starts.iter().enumerate() {
            let end = starts.get(n + 1).copied().unwrap_or(stream.len());
            let unit = &stream[start + 4..end];
            match unit[0] & 0x1f {
                7 => sps = unit.to_vec(),
                8 => pps = unit.to_vec(),
                _ => {
                    units.extend_from_slice(&(unit.len() as u32).to_be_bytes());
                    units.extend_from_slice(unit);
                }
            }
            at = end;
        }
        let _ = at;
        samples.push(Mp4Sample { start_time: i * 100, duration: 100, rendering_offset: 0, is_sync: i % 10 == 0, bytes: units.into() });
    }
    writer.add_track(&TrackConfig { track_type: TrackType::Video, timescale: 1000, language: "und".into(), media_conf: MediaConfig::AvcConfig(AvcConfig { width: 64, height: 48, seq_param_set: sps, pic_param_set: pps }) }).unwrap();
    for sample in samples {
        writer.write_sample(1, &sample).unwrap();
    }
    writer.write_end().unwrap();
}

/// The system's decoder plays first where there is one (GStreamer here, when installed).
#[test]
fn the_systems_decoder_plays_first() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    clarity_video::forget_added();
    let Some(Ok(mut system)) = clarity_video::system::open(&video("bar.mp4")) else {
        eprintln!("no system decoder here: left out");
        return;
    };
    assert_eq!((64, 48), system.size());
    assert_eq!(Duration::from_secs(2), system.length());
    let mut times = Vec::new();
    while let Some((at, picture)) = system.next_picture().unwrap() {
        assert_eq!((64, 48), (picture.width, picture.height));
        times.push(at.as_millis());
    }
    assert_eq!((0..20).map(|i| i * 100).collect::<Vec<u128>>(), times);
    let (at, _) = system.seek(Duration::from_millis(1500)).unwrap().unwrap();
    assert_eq!(Duration::from_millis(1500), at);
    // And `open` takes it, unless a codec was added for the format.
    assert_eq!(system.decoder_name(), clarity_video::open(&video("bar.mp4")).unwrap().decoder_name());
    let _ = counting();
    assert_eq!("counting", clarity_video::open(&video("bar.mp4")).unwrap().decoder_name());
    clarity_video::forget_added();
}

/// The hashes Clarity's release checks Cisco's binaries against name the
/// files Clarity.Video looks for beside the program, one per system.
#[test]
fn the_release_knows_each_systems_binary() {
    let known = include_str!("../openh264.sha256");
    for (os, arch) in [("linux", "x86_64"), ("linux", "aarch64"), ("macos", "x86_64"), ("macos", "aarch64"), ("windows", "x86_64"), ("windows", "aarch64")] {
        let name = openh264::binary_name(os, arch).unwrap();
        assert!(known.lines().any(|l| l.split_whitespace().nth(1) == Some(name.as_str()) && l.split_whitespace().next().is_some_and(|h| h.len() == 64)), "{name}");
    }
    let script = include_str!("../../../.github/scripts/fetch_openh264.py");
    assert!(script.contains(&format!("VERSION = \"{}\"", openh264::VERSION)));
}
