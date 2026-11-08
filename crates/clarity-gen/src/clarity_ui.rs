//! Clarity.UI's library for Go, C and C++ programs (DESIGN 7.9). Their
//! builds take it from the folder the environment variable `CLARITY_UI`
//! names: `include/clarity_ui.h` and, in `lib/`, the library for the
//! system. In order: the folder the person names; the one the installed
//! Clarity carries (its installer bundles the library for its own system);
//! else Clarity fetches the archive of its own library version from the
//! GitHub release the workflow release-clarity-ui.yml publishes, checks it
//! against the release's SHA256SUMS and unpacks it into the person's cache
//! folder, once per version. The decisions are plain functions here; the download itself is
//! the caller's (`provide` takes it as `get`).
//!
//! The browser files (round B: `clarity-ui.js`, `clarity-ui.wasm` and the
//! worker and page scripts, which every language's web server serves) come
//! the same way, as the release's `browser` archive: the folder the
//! environment variable `CLARITY_UI_BROWSER` names, else the one the
//! installed Clarity carries, else fetched into the cache folder
//! (`browser_library`).

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Where the releases' files are downloaded from: `<RELEASES>/<tag>/<file>`.
pub const RELEASES: &str = "https://github.com/petervdpas/Clarity/releases/download";

/// The release's name for the browser files, beside the systems' rids.
pub const BROWSER: &str = "browser";

/// The browser files, as a web server serves them side by side.
pub const BROWSER_FILES: [&str; 4] = ["clarity-ui.js", "clarity-ui.wasm", "clarity-ui-worker.js", "clarity-ui-page.js"];

/// The library's crate, whose version names the release.
const FFI_MANIFEST: &str = include_str!("../../../runtimes/rust-ui-ffi/Cargo.toml");

/// The library version this Clarity was built with: the ffi crate's.
pub fn version() -> &'static str {
    version_in(FFI_MANIFEST).unwrap_or("0.0.0")
}

/// The version of the `[package]` in a Cargo.toml.
pub fn version_in(manifest: &str) -> Option<&str> {
    let mut package = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            package = line == "[package]";
        } else if package && let Some((key, value)) = line.split_once('=')
            && key.trim() == "version"
        {
            return Some(value.trim().trim_matches('"'));
        }
    }
    None
}

/// The release's name for system `os` on `arch` (std::env::consts): `linux-x64`,
/// `win-arm64`, `osx-arm64`; none for a system the release has no archive for.
pub fn rid(os: &str, arch: &str) -> Option<String> {
    let os = match os {
        "linux" => "linux",
        "windows" => "win",
        "macos" => "osx",
        _ => return None,
    };
    let arch = match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return None,
    };
    Some(format!("{os}-{arch}"))
}

/// The tag of the library's release: `runtimes/rust-ui-ffi/v0.1.0`.
pub fn tag(version: &str) -> String {
    format!("runtimes/rust-ui-ffi/v{version}")
}

/// The folder an archive holds: `clarity-ui-0.1.0-linux-x64`.
pub fn folder_name(version: &str, rid: &str) -> String {
    format!("clarity-ui-{version}-{rid}")
}

/// The archive of one system: `clarity-ui-0.1.0-linux-x64.tar.gz`.
pub fn archive_name(version: &str, rid: &str) -> String {
    format!("{}.tar.gz", folder_name(version, rid))
}

/// Where the archive is, under `base` (RELEASES): GitHub keeps the tag's
/// slashes in the path.
pub fn archive_url(base: &str, version: &str, rid: &str) -> String {
    format!("{base}/{}/{}", tag(version), archive_name(version, rid))
}

/// The release's page, where people download an archive by hand.
pub fn release_page(version: &str) -> String {
    format!("https://github.com/petervdpas/Clarity/releases/tag/{}", tag(version))
}

/// Where the release's checksums are, one line per archive.
pub fn sums_url(base: &str, version: &str) -> String {
    format!("{base}/{}/SHA256SUMS", tag(version))
}

/// The person's cache folder on system `os`, with `env` giving the
/// environment: %LOCALAPPDATA% on Windows, ~/Library/Caches on macOS,
/// $XDG_CACHE_HOME or ~/.cache elsewhere.
pub fn cache_dir_in(os: &str, env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let set = |name: &str| env(name).filter(|v| !v.is_empty());
    match os {
        "windows" => set("LOCALAPPDATA").map(PathBuf::from),
        "macos" => set("HOME").map(|home| PathBuf::from(home).join("Library").join("Caches")),
        _ => set("XDG_CACHE_HOME").map(PathBuf::from).or_else(|| set("HOME").map(|home| PathBuf::from(home).join(".cache"))),
    }
}

/// Where a fetched library is kept: `<cache>/clarity/clarity-ui/<version>/<rid>`,
/// the folder CLARITY_UI then names.
pub fn library_dir(cache: &Path, version: &str, rid: &str) -> PathBuf {
    cache.join("clarity").join("clarity-ui").join(version).join(rid)
}

/// The shared library's file in `lib/` on system `os`.
pub fn library_file(os: &str) -> &'static str {
    match os {
        "windows" => "clarity_ui.dll",
        "macos" => "libclarity_ui.dylib",
        _ => "libclarity_ui.so",
    }
}

/// Clarity.Audio's shared library in `lib/` on system `os`, beside Clarity.UI's.
pub fn audio_file(os: &str) -> &'static str {
    match os {
        "windows" => "clarity_audio.dll",
        "macos" => "libclarity_audio.dylib",
        _ => "libclarity_audio.so",
    }
}

/// Cisco's openh264 for a system and processor (as std names them), which
/// every app with video carries beside its program (docs/VIDEO-PLAN.md);
/// the names of runtimes/rust-video/openh264.sha256.
pub fn openh264_file(os: &str, arch: &str) -> Option<&'static str> {
    Some(match (os, arch) {
        ("linux", "x86_64") => "libopenh264-2.6.0-linux64.8.so",
        ("linux", "aarch64") => "libopenh264-2.6.0-linux-arm64.8.so",
        ("macos", "x86_64") => "libopenh264-2.6.0-mac-x64.dylib",
        ("macos", "aarch64") => "libopenh264-2.6.0-mac-arm64.dylib",
        ("windows", "x86_64") => "openh264-2.6.0-win64.dll",
        ("windows", "aarch64") => "openh264-2.6.0-win-arm64.dll",
        _ => return None,
    })
}

/// The SHA-256 of each Cisco binary Clarity.Video's openh264 knows, by file name.
const OPENH264_HASHES: &str = include_str!("../../../runtimes/rust-video/openh264.sha256");

/// Where Cisco serves its binaries, compressed with bzip2.
pub const OPENH264_RELEASES: &str = "https://ciscobinary.openh264.org";

/// The known SHA-256 of Cisco's file `name`.
pub fn openh264_hash(name: &str) -> Option<&'static str> {
    OPENH264_HASHES.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        (parts.next()?.trim_start_matches('*') == name).then_some(hash)
    })
}

/// Makes sure `folder/lib` holds Cisco's openh264 for this system, as every
/// video app carries it (docs/VIDEO-PLAN.md): when it lacks it, or holds one
/// whose hash Clarity does not know, the binary is fetched from Cisco with
/// `get`, unpacked and checked, then put there. None where Cisco has no
/// binary for the system; the path otherwise.
pub fn ensure_openh264(folder: &Path, os: &str, arch: &str, get: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Result<Option<PathBuf>, String> {
    let Some(name) = openh264_file(os, arch) else { return Ok(None) };
    let wanted = openh264_hash(name).ok_or_else(|| format!("Clarity knows no hash for {name}"))?;
    let path = folder.join("lib").join(name);
    if std::fs::read(&path).is_ok_and(|bytes| sha256_hex(&bytes) == wanted) {
        return Ok(Some(path));
    }
    let packed = get(&format!("{OPENH264_RELEASES}/{name}.bz2"))?;
    let mut binary = Vec::new();
    std::io::Read::read_to_end(&mut bzip2::read::BzDecoder::new(packed.as_slice()), &mut binary).map_err(|why| format!("{name}: {why}"))?;
    if sha256_hex(&binary) != wanted {
        return Err(format!("{name} from Cisco is not the one Clarity knows"));
    }
    std::fs::create_dir_all(folder.join("lib")).map_err(|why| why.to_string())?;
    let fresh = folder.join("lib").join(format!("{name}.part"));
    std::fs::write(&fresh, &binary).map_err(|why| why.to_string())?;
    std::fs::rename(&fresh, &path).map_err(|why| why.to_string())?;
    Ok(Some(path))
}

/// Whether `folder` holds the header and the library for system `os`.
pub fn complete(folder: &Path, os: &str) -> bool {
    folder.join("include").join("clarity_ui.h").is_file() && folder.join("lib").join(library_file(os)).is_file()
}

/// Whether `folder` holds the browser files.
pub fn browser_complete(folder: &Path) -> bool {
    BROWSER_FILES.iter().all(|file| folder.join(file).is_file())
}

/// The file an unpacked archive of `rid` must hold, below its folder.
fn proof(rid: &str) -> &'static str {
    if rid == BROWSER { "clarity-ui.wasm" } else { "include/clarity_ui.h" }
}

/// The checksum SHA256SUMS (as `sha256sum` writes it) gives for file `name`.
pub fn sum_for<'a>(sums: &'a str, name: &str) -> Option<&'a str> {
    sums.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        let (hash, file) = (words.next()?, words.next()?);
        (file.trim_start_matches('*') == name).then_some(hash)
    })
}

/// The SHA-256 of `bytes`, in lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether `archive`, the file `name`, is what the release's SHA256SUMS says.
pub fn check_archive(archive: &[u8], sums: &str, name: &str) -> Result<(), String> {
    let expected = sum_for(sums, name).ok_or_else(|| format!("the release's SHA256SUMS names no {name}"))?;
    let actual = sha256_hex(archive);
    if actual.eq_ignore_ascii_case(expected) { Ok(()) } else { Err(format!("{name} is not the release's: its SHA-256 is {actual}, SHA256SUMS says {expected}")) }
}

/// Unpacks `archive` (a .tar.gz holding the folder `folder_name(version,
/// rid)`) so that its contents are `into`: unpacked beside it first, then
/// moved there whole, so a stopped fetch leaves no half folder behind.
pub fn unpack(archive: &[u8], version: &str, rid: &str, into: &Path) -> Result<(), String> {
    let parent = into.parent().ok_or_else(|| format!("{} has no parent folder", into.display()))?;
    std::fs::create_dir_all(parent).map_err(|err| format!("cannot make {}: {err}", parent.display()))?;
    let partial = parent.join(format!(".{rid}.partial-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&partial);
    let done = (|| {
        std::fs::create_dir_all(&partial).map_err(|err| format!("cannot make {}: {err}", partial.display()))?;
        // tar leaves out entries that would land outside the folder (`..`, absolute paths).
        tar::Archive::new(flate2::read::GzDecoder::new(archive)).unpack(&partial).map_err(|err| format!("cannot unpack the archive: {err}"))?;
        let top = partial.join(folder_name(version, rid));
        if !top.join(proof(rid)).is_file() {
            return Err(format!("the archive has no {}/{}", folder_name(version, rid), proof(rid)));
        }
        if into.exists() {
            std::fs::remove_dir_all(into).map_err(|err| format!("cannot replace {}: {err}", into.display()))?;
        }
        std::fs::rename(&top, into).map_err(|err| format!("cannot move the library to {}: {err}", into.display()))
    })();
    let _ = std::fs::remove_dir_all(&partial);
    done
}

/// Where a build takes Clarity.UI's library from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Library {
    /// The folder the person named with CLARITY_UI, which always wins.
    Named(PathBuf),
    /// The folder the installed Clarity carries.
    Bundled(PathBuf),
    /// Fetched before, in the cache folder.
    Fetched(PathBuf),
    /// Fetched into `folder` on the first build.
    ToFetch { folder: PathBuf, version: String, rid: String },
    /// Neither named nor fetchable: no archive for this system, or no cache folder.
    Unavailable,
}

/// Where the library comes from, given what CLARITY_UI says (`named`), the
/// folder the installed Clarity carries (`bundled`, taken when it is
/// complete), the cache folder, this system's `rid` and `os`, and the
/// library `version`.
pub fn library_in(named: Option<&OsStr>, bundled: Option<&Path>, cache: Option<&Path>, rid: Option<&str>, version: &str, os: &str) -> Library {
    found_in(named, bundled, cache, rid, version, &|folder| complete(folder, os))
}

/// Where the browser files come from, given what CLARITY_UI_BROWSER says
/// (`named`), the folder the installed Clarity carries (`bundled`), the
/// cache folder and the library `version`: as `library_in` decides.
pub fn browser_library_in(named: Option<&OsStr>, bundled: Option<&Path>, cache: Option<&Path>, version: &str) -> Library {
    found_in(named, bundled, cache, Some(BROWSER), version, &browser_complete)
}

fn found_in(named: Option<&OsStr>, bundled: Option<&Path>, cache: Option<&Path>, rid: Option<&str>, version: &str, ready: &dyn Fn(&Path) -> bool) -> Library {
    if let Some(named) = named.filter(|n| !n.is_empty()) {
        return Library::Named(PathBuf::from(named));
    }
    if let Some(bundled) = bundled.filter(|b| ready(b)) {
        return Library::Bundled(bundled.to_path_buf());
    }
    let (Some(cache), Some(rid)) = (cache, rid) else { return Library::Unavailable };
    let folder = library_dir(cache, version, rid);
    if ready(&folder) { Library::Fetched(folder) } else { Library::ToFetch { folder, version: version.to_string(), rid: rid.to_string() } }
}

/// The folder the installed Clarity carries, which the IDE names when it
/// starts (`set_bundled`); none in the command-line tool.
static BUNDLED: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Names the folder of the library the installed Clarity carries: its
/// resource folder's `clarity-ui`.
pub fn set_bundled(folder: PathBuf) {
    let _ = BUNDLED.set(folder);
}

/// The IDE's product name (tauri.conf.json's productName), which names its
/// resource folder on Linux.
pub const PRODUCT_NAME: &str = "Clarity IDE";

/// The resource folder of the installed Clarity, for a program in `exe_dir`
/// (the IDE or the command-line tool beside it) on `os`: the package's
/// `lib/Clarity IDE` on Linux, the program's own folder on Windows, the
/// bundle's Resources on macOS. It holds `clarity-ui` and `clarity-ui-browser`.
pub fn installed_resources(exe_dir: &Path, os: &str) -> PathBuf {
    match os {
        "windows" => exe_dir.to_path_buf(),
        "macos" => exe_dir.join("..").join("Resources"),
        _ => exe_dir.join("..").join("lib").join(PRODUCT_NAME),
    }
}

/// Where the library comes from on this machine.
pub fn library() -> Library {
    let os = std::env::consts::OS;
    let named = std::env::var_os("CLARITY_UI");
    let cache = cache_dir_in(os, |name| std::env::var(name).ok());
    library_in(named.as_deref(), BUNDLED.get().map(PathBuf::as_path), cache.as_deref(), rid(os, std::env::consts::ARCH).as_deref(), version(), os)
}

/// The browser files the installed Clarity carries, which the IDE names when it starts.
static BUNDLED_BROWSER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Names the folder of the browser files the installed Clarity carries: its resource folder's `clarity-ui-browser`.
pub fn set_bundled_browser(folder: PathBuf) {
    let _ = BUNDLED_BROWSER.set(folder);
}

/// Where the browser files come from on this machine.
pub fn browser_library() -> Library {
    let named = std::env::var_os("CLARITY_UI_BROWSER");
    let cache = cache_dir_in(std::env::consts::OS, |name| std::env::var(name).ok());
    browser_library_in(named.as_deref(), BUNDLED_BROWSER.get().map(PathBuf::as_path), cache.as_deref(), version())
}

/// The browser files Clarity's repository built (runtimes/rust-ui-browser/build.sh
/// writes them to `<repo>/target/clarity-ui-browser`); none when they are not built.
pub fn repository_browser(repo: &Path) -> Option<PathBuf> {
    let folder = repo.join("target").join("clarity-ui-browser");
    browser_complete(&folder).then_some(folder)
}

/// The folder Clarity's repository makes of the library it built (a
/// Clarity run from source, which has no installer's folder):
/// `<repo>/target/clarity-ui`, with the header from runtimes/rust-ui-ffi
/// and the library from target/release copied in when they changed, and
/// Clarity.Audio's beside them when it is built. None when Clarity.UI's
/// library is not built.
pub fn repository_library(repo: &Path, os: &str) -> Option<PathBuf> {
    let header = repo.join("runtimes").join("rust-ui-ffi").join("include").join("clarity_ui.h");
    let release = repo.join("target").join("release");
    let mut files = vec![(header, PathBuf::from("include").join("clarity_ui.h")), (release.join(library_file(os)), PathBuf::from("lib").join(library_file(os)))];
    if os == "windows" {
        files.push((release.join("clarity_ui.dll.lib"), PathBuf::from("lib").join("clarity_ui.dll.lib")));
    }
    if !files.iter().all(|(from, _)| from.is_file()) {
        return None;
    }
    let audio_header = repo.join("runtimes").join("rust-audio-ffi").join("include").join("clarity_audio.h");
    let mut audio = vec![(audio_header, PathBuf::from("include").join("clarity_audio.h")), (release.join(audio_file(os)), PathBuf::from("lib").join(audio_file(os)))];
    if os == "windows" {
        audio.push((release.join("clarity_audio.dll.lib"), PathBuf::from("lib").join("clarity_audio.dll.lib")));
    }
    if audio.iter().all(|(from, _)| from.is_file()) {
        files.extend(audio);
    }
    let image_header = repo.join("runtimes").join("rust-image-ffi").join("include").join("clarity_image.h");
    let image_file = library_file(os).replace("clarity_ui", "clarity_image");
    let mut image = vec![(image_header, PathBuf::from("include").join("clarity_image.h")), (release.join(&image_file), PathBuf::from("lib").join(&image_file))];
    if os == "windows" {
        image.push((release.join("clarity_image.dll.lib"), PathBuf::from("lib").join("clarity_image.dll.lib")));
    }
    if image.iter().all(|(from, _)| from.is_file()) {
        files.extend(image);
    }
    let video_header = repo.join("runtimes").join("rust-video-ffi").join("include").join("clarity_video.h");
    let video_file = library_file(os).replace("clarity_ui", "clarity_video");
    let mut video = vec![(video_header, PathBuf::from("include").join("clarity_video.h")), (release.join(&video_file), PathBuf::from("lib").join(&video_file))];
    if os == "windows" {
        video.push((release.join("clarity_video.dll.lib"), PathBuf::from("lib").join("clarity_video.dll.lib")));
    }
    if video.iter().all(|(from, _)| from.is_file()) {
        files.extend(video);
    }
    // Cisco's openh264, from the cache Clarity.Video's tests fetch it into, when it is there.
    if let Some(name) = openh264_file(os, std::env::consts::ARCH) {
        let home = std::env::var("XDG_CACHE_HOME").ok().filter(|v| !v.is_empty()).map(PathBuf::from).or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".cache")));
        let cached = home.map(|c| c.join("clarity").join("openh264").join("2.6.0").join(name));
        if let Some(cached) = cached.filter(|c| c.is_file()) {
            files.push((cached, PathBuf::from("lib").join(name)));
        }
    }
    let folder = repo.join("target").join("clarity-ui");
    for (from, to) in files {
        let to = folder.join(to);
        let same = |a: &Path, b: &Path| -> Option<bool> {
            let (a, b) = (std::fs::metadata(a).ok()?, std::fs::metadata(b).ok()?);
            Some(a.len() == b.len() && a.modified().ok()? <= b.modified().ok()?)
        };
        if same(&from, &to) != Some(true) {
            std::fs::create_dir_all(to.parent()?).ok()?;
            std::fs::copy(&from, &to).ok()?;
        }
    }
    Some(folder)
}

/// The folder a build's CLARITY_UI names: the named or fetched one, else
/// fetched now from `base` (RELEASES) with `get` (a URL's bytes), checked
/// against SHA256SUMS and unpacked. `say` tells what happens, a line at a time.
pub fn provide(library: Library, base: &str, get: &dyn Fn(&str) -> Result<Vec<u8>, String>, say: &dyn Fn(String)) -> Result<PathBuf, String> {
    match library {
        Library::Named(folder) | Library::Bundled(folder) | Library::Fetched(folder) => Ok(folder),
        Library::Unavailable => Err(UNAVAILABLE.to_string()),
        Library::ToFetch { folder, version, rid } => {
            let name = archive_name(&version, &rid);
            let url = archive_url(base, &version, &rid);
            say(format!("Clarity.UI's library {version} for {rid}: fetching {url}"));
            let sums = get(&sums_url(base, &version))?;
            let sums = String::from_utf8_lossy(&sums);
            let archive = get(&url)?;
            check_archive(&archive, &sums, &name)?;
            unpack(&archive, &version, &rid, &folder)?;
            say(format!("Clarity.UI's library is in {}", folder.display()));
            Ok(folder)
        }
    }
}

/// The lines a generated C or C++ CMakeLists.txt names the library's
/// version with, before it includes the runtime: a build that names no
/// CLARITY_UI fetches that version (clarity-runtime/clarity_ui.cmake).
pub fn cmake_version_lines() -> String {
    format!(
        "\n# The version of Clarity.UI's library the program is generated for: a build\n# with no CLARITY_UI fetches it from its release (clarity-runtime/clarity_ui.cmake).\nset(CLARITY_UI_VERSION {})\n",
        version()
    )
}

/// Why the library cannot be fetched here.
pub const UNAVAILABLE: &str = "Clarity.UI's library has no release for this system (or there is no cache folder to keep it in): set the environment variable CLARITY_UI to a folder with include/clarity_ui.h and lib/";

/// Reads all of `reader`, at most `limit` bytes: what a download hands `provide`.
pub fn read_limited(reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes).map_err(|err| format!("the download stopped: {err}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!("the download is larger than {limit} bytes"));
    }
    Ok(bytes)
}
