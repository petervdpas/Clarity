//! Codecs people add as native libraries (include/clarity_codec.h): a
//! project names one in its project file, Clarity carries it beside the
//! program, and the program loads and registers it as it starts. For the
//! formats it takes it wins over the system's decoder and openh264.

use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libloading::Library;

use crate::{Codec, Decoder, Picture, Problem, Setup};

/// The table of include/clarity_codec.h, field for field.
#[repr(C)]
pub struct ClarityCodecApi {
    pub interface: u32,
    pub name: *const c_char,
    pub formats: *const c_char,
    pub decoder_new: extern "C" fn(*const c_char, u32, u32, *const u8, usize) -> *mut std::ffi::c_void,
    pub decoder_free: extern "C" fn(*mut std::ffi::c_void),
    pub decode: extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut u32, *mut u32) -> i32,
}

const PICTURE: i32 = 1;
const ERROR: i32 = -1;

/// A codec in a library, kept loaded while the codec lives.
pub struct LibraryCodec {
    _library: Arc<Library>,
    api: &'static ClarityCodecApi,
    name: String,
    formats: Vec<String>,
}

// The table is plain C functions and constant text, which the library's author makes safe to call from any thread.
unsafe impl Send for LibraryCodec {}
unsafe impl Sync for LibraryCodec {}

/// Loads the codec library at `path` and gives its codec.
pub fn load(path: &Path) -> Result<LibraryCodec, Problem> {
    let shown = path.display().to_string();
    let library = unsafe { Library::new(path) }.map_err(|why| Problem::CodecMissing(format!("{shown}: {why}")))?;
    let api = unsafe {
        let get: libloading::Symbol<extern "C" fn() -> *const ClarityCodecApi> = library.get(b"clarity_codec_api\0").map_err(|why| Problem::CodecMissing(format!("{shown}: {why}")))?;
        get().as_ref()
    }
    .ok_or_else(|| Problem::CodecMissing(format!("{shown} gives no codec table")))?;
    if api.interface < 1 || api.name.is_null() || api.formats.is_null() {
        return Err(Problem::CodecMissing(format!("{shown} gives no codec table Clarity.Video reads")));
    }
    // SAFETY: the library stays loaded as long as the codec (and so its table) lives.
    let api: &'static ClarityCodecApi = unsafe { &*(api as *const ClarityCodecApi) };
    let text = |p: *const c_char| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
    let formats = text(api.formats).split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect();
    Ok(LibraryCodec { _library: Arc::new(library), api, name: text(api.name), formats })
}

/// Finds the codec library `file` where Clarity carries it: beside the
/// program or in its `codecs` folder, in .NET's native folder, beside
/// Clarity.Video's own library, or in `codecs` of the folder the program
/// runs in (a program run from its project's output).
pub fn find(file: &str) -> Option<PathBuf> {
    let mut places = Vec::new();
    if let Some(program) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        places.push(program.join("codecs"));
        places.push(program.clone());
        places.push(program.join("runtimes").join(crate::openh264::dotnet_rid_here().unwrap_or_default()).join("native"));
    }
    places.extend(crate::openh264::own_folder_here());
    if let Ok(here) = std::env::current_dir() {
        places.push(here.join("codecs"));
    }
    places.into_iter().map(|folder| folder.join(file)).find(|path| path.is_file())
}

/// Finds, loads and registers the codec library `file`, so it plays the formats it takes.
pub fn add(file: &str) -> Result<String, Problem> {
    let path = find(file).ok_or_else(|| Problem::CodecMissing(format!("the codec library {file} is not beside the program")))?;
    let codec = load(&path)?;
    let name = codec.name.clone();
    crate::register(Arc::new(codec));
    Ok(name)
}

impl Codec for LibraryCodec {
    fn name(&self) -> &str {
        &self.name
    }

    fn formats(&self) -> Vec<String> {
        self.formats.clone()
    }

    fn decoder(&self, setup: &Setup) -> Result<Box<dyn Decoder>, Problem> {
        let format = CString::new(setup.format.as_str()).map_err(|_| Problem::NoCodec(setup.format.clone()))?;
        let mut packed = Vec::new();
        for unit in &setup.parameters {
            packed.extend_from_slice(&(unit.len() as u32).to_be_bytes());
            packed.extend_from_slice(unit);
        }
        let decoder = (self.api.decoder_new)(format.as_ptr(), setup.width, setup.height, if packed.is_empty() { std::ptr::null() } else { packed.as_ptr() }, packed.len());
        if decoder.is_null() {
            return Err(Problem::CodecMissing(format!("{} made no decoder for {}", self.name, setup.format)));
        }
        Ok(Box::new(LibraryDecoder { codec: self.api, _library: self._library.clone(), decoder, size: (setup.width, setup.height) }))
    }
}

struct LibraryDecoder {
    codec: &'static ClarityCodecApi,
    _library: Arc<Library>,
    decoder: *mut std::ffi::c_void,
    size: (u32, u32),
}

// A decoder is used from the decoding thread alone, as the header says.
unsafe impl Send for LibraryDecoder {}

impl Drop for LibraryDecoder {
    fn drop(&mut self) {
        (self.codec.decoder_free)(self.decoder);
    }
}

impl Decoder for LibraryDecoder {
    fn decode(&mut self, frame: &[u8]) -> Result<Option<Picture>, Problem> {
        let mut rgba = vec![0u8; self.size.0 as usize * self.size.1 as usize * 4];
        let (mut width, mut height) = (0, 0);
        match (self.codec.decode)(self.decoder, frame.as_ptr(), frame.len(), rgba.as_mut_ptr(), rgba.len(), &mut width, &mut height) {
            PICTURE if width as usize * height as usize * 4 <= rgba.len() => {
                rgba.truncate(width as usize * height as usize * 4);
                Ok(Some(Picture { width, height, rgba }))
            }
            ERROR => Err(Problem::Undecodable("the codec library could not decode a frame".into())),
            _ => Ok(None),
        }
    }
}
