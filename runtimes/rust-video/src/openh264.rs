//! openh264: H.264 through Cisco's prebuilt library, loaded at run time.
//! An app carries the binary beside its program, so it plays offline;
//! nothing is downloaded while it runs. Clarity's release workflow and its
//! tests take the binary from Cisco ([`fetch`]); it is loaded only when its
//! hash is one openh264's crate knows.

use std::path::{Path, PathBuf};

use openh264::OpenH264API;
use openh264::decoder::{Decoder as H264Decoder, DecoderConfig};
use openh264::formats::YUVSource;

use crate::{Codec, Decoder, Picture, Problem, Setup};

/// The openh264 release the crate loads.
pub const VERSION: &str = "2.6.0";

/// The formats the base codec takes, as MP4 names them.
pub const FORMATS: [&str; 1] = ["avc1"];

/// Where Cisco keeps its binaries.
pub const RELEASES: &str = "https://ciscobinary.openh264.org";

/// Cisco's file for a system (`linux`, `macos`, `windows`) and processor
/// (`x86_64`, `aarch64`), as std names them; none where Cisco has no binary.
pub fn binary_name(os: &str, arch: &str) -> Option<String> {
    let name = match (os, arch) {
        ("linux", "x86_64") => "libopenh264-{v}-linux64.8.so",
        ("linux", "aarch64") => "libopenh264-{v}-linux-arm64.8.so",
        ("macos", "x86_64") => "libopenh264-{v}-mac-x64.dylib",
        ("macos", "aarch64") => "libopenh264-{v}-mac-arm64.dylib",
        ("windows", "x86_64") => "openh264-{v}-win64.dll",
        ("windows", "aarch64") => "openh264-{v}-win-arm64.dll",
        _ => return None,
    };
    Some(name.replace("{v}", VERSION))
}

/// Where Cisco serves `name`, compressed with bzip2 (Clarity's release workflow takes it from there).
pub fn binary_url(name: &str) -> String {
    format!("{RELEASES}/{name}.bz2")
}

/// The person's cache folder for Clarity.Video's codecs on system `os`:
/// `$XDG_CACHE_HOME` or `~/.cache` on Linux, `~/Library/Caches` on macOS,
/// `%LOCALAPPDATA%` on Windows; then `clarity/openh264/<version>`.
pub fn cache_dir_in(os: &str, env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let some = |name: &str| env(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = match os {
        "windows" => some("LOCALAPPDATA")?,
        "macos" => some("HOME")?.join("Library").join("Caches"),
        _ => some("XDG_CACHE_HOME").or_else(|| some("HOME").map(|h| h.join(".cache")))?,
    };
    Some(base.join("clarity").join("openh264").join(VERSION))
}

/// The binary an app carries, when it does: beside the program (C, C++,
/// Rust), in .NET's `runtimes/<rid>/native` beside it (C#), or beside
/// Clarity.Video's own library (Go, and any program that finds its
/// libraries in one folder).
pub fn included() -> Option<PathBuf> {
    let name = binary_name(std::env::consts::OS, std::env::consts::ARCH)?;
    let program = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let mut places = Vec::new();
    if let Some(program) = &program {
        places.push(program.clone());
        if let Some(rid) = dotnet_rid(std::env::consts::OS, std::env::consts::ARCH) {
            places.push(program.join("runtimes").join(rid).join("native"));
        }
    }
    places.extend(own_folder());
    places.into_iter().map(|folder| folder.join(&name)).find(|path| path.is_file())
}

/// .NET's name for this system and processor.
pub(crate) fn dotnet_rid_here() -> Option<&'static str> {
    dotnet_rid(std::env::consts::OS, std::env::consts::ARCH)
}

/// The folder of Clarity.Video's own library, when it is one.
pub(crate) fn own_folder_here() -> Option<PathBuf> {
    own_folder()
}

/// .NET's name for a system and processor, as its packages lay native libraries out.
fn dotnet_rid(os: &str, arch: &str) -> Option<&'static str> {
    Some(match (os, arch) {
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "x86_64") => "osx-x64",
        ("macos", "aarch64") => "osx-arm64",
        ("windows", "x86_64") => "win-x64",
        ("windows", "aarch64") => "win-arm64",
        _ => return None,
    })
}

/// The folder of the library this code is in (libclarity_video), when it is one.
#[cfg(unix)]
fn own_folder() -> Option<PathBuf> {
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    let here = own_folder as *const ();
    if unsafe { libc::dladdr(here.cast(), &mut info) } == 0 || info.dli_fname.is_null() {
        return None;
    }
    let file = unsafe { std::ffi::CStr::from_ptr(info.dli_fname) }.to_string_lossy().into_owned();
    Path::new(&file).parent().map(Path::to_path_buf)
}

#[cfg(windows)]
fn own_folder() -> Option<PathBuf> {
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleFileNameW, GetModuleHandleExW};
    let mut module = HMODULE::default();
    let here = own_folder as *const ();
    unsafe { GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, windows::core::PCWSTR(here.cast()), &mut module) }.ok()?;
    let mut name = vec![0u16; 1024];
    let len = unsafe { GetModuleFileNameW(Some(module), &mut name) } as usize;
    let file = String::from_utf16_lossy(&name[..len]);
    Path::new(&file).parent().map(Path::to_path_buf)
}

#[cfg(not(any(unix, windows)))]
fn own_folder() -> Option<PathBuf> {
    None
}

/// The cached binary for this system, when it is there.
pub fn cached() -> Option<PathBuf> {
    let name = binary_name(std::env::consts::OS, std::env::consts::ARCH)?;
    let path = cache_dir_in(std::env::consts::OS, |n| std::env::var(n).ok())?.join(name);
    path.is_file().then_some(path)
}

/// The codec on the binary at a path; by default the one beside the program, else Clarity's cache (where Clarity's own runs find it).
#[derive(Default)]
pub struct OpenH264 {
    binary: Option<PathBuf>,
}

impl OpenH264 {
    /// The codec on the binary at `path` (a test's).
    pub fn at(path: &Path) -> OpenH264 {
        OpenH264 { binary: Some(path.to_path_buf()) }
    }

    fn api(&self) -> Result<OpenH264API, Problem> {
        // Carried beside the program, else in Clarity's cache; never fetched while the program runs.
        let path = match &self.binary {
            Some(path) => path.clone(),
            None => included().or_else(cached).ok_or_else(|| {
                Problem::CodecMissing(format!("openh264 is not beside the program ({})", binary_name(std::env::consts::OS, std::env::consts::ARCH).unwrap_or_default()))
            })?,
        };
        OpenH264API::from_blob_path(&path).map_err(|why| Problem::CodecMissing(format!("{}: {why}", path.display())))
    }
}

impl Codec for OpenH264 {
    fn name(&self) -> &str {
        "openh264"
    }

    fn formats(&self) -> Vec<String> {
        FORMATS.iter().map(|f| f.to_string()).collect()
    }

    fn decoder(&self, setup: &Setup) -> Result<Box<dyn Decoder>, Problem> {
        let decoder = H264Decoder::with_api_config(self.api()?, DecoderConfig::new()).map_err(|why| Problem::CodecMissing(why.to_string()))?;
        let mut decoder = H264 { decoder, header: annex_b(&setup.parameters), started: false };
        decoder.started = decoder.header.is_empty();
        Ok(Box::new(decoder))
    }
}

/// NAL units with Annex B's start codes before each, as openh264 reads them.
pub fn annex_b(units: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for unit in units {
        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(unit);
    }
    out
}

/// An MP4 frame (NAL units each led by its length in four bytes) as Annex B; none for a frame that does not read so.
pub fn length_prefixed_to_annex_b(frame: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(frame.len() + 16);
    let mut at = 0;
    while at < frame.len() {
        let len = u32::from_be_bytes(frame.get(at..at + 4)?.try_into().ok()?) as usize;
        let unit = frame.get(at + 4..at + 4 + len)?;
        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(unit);
        at += 4 + len;
    }
    Some(out)
}

struct H264 {
    decoder: H264Decoder,
    /// The parameter sets, given with the first frame.
    header: Vec<u8>,
    started: bool,
}

impl Decoder for H264 {
    fn decode(&mut self, frame: &[u8]) -> Result<Option<Picture>, Problem> {
        let units = length_prefixed_to_annex_b(frame).ok_or_else(|| Problem::Undecodable("a frame whose units do not read".into()))?;
        let packet = if self.started {
            units
        } else {
            self.started = true;
            let mut first = std::mem::take(&mut self.header);
            first.extend_from_slice(&units);
            first
        };
        let decoded = self.decoder.decode(&packet).map_err(|why| Problem::Undecodable(why.to_string()))?;
        Ok(decoded.map(|yuv| {
            let (width, height) = yuv.dimensions();
            let mut rgba = vec![0; width * height * 4];
            yuv.write_rgba8(&mut rgba);
            Picture { width: width as u32, height: height as u32, rgba }
        }))
    }
}
