//! The program file's resources on Windows: its icon
//! and its version
//!. Clarity converts the picture the project
//! names (`icon "app.png"`: PNG, JPEG, WebP or SVG) itself: it draws every
//! size Explorer and the taskbar pick from and writes them as a Windows icon
//! (`windows/app.ico`, which C# takes as its ApplicationIcon). It wraps the
//! icon, and the version the Properties of the file show, in a COFF object
//! with a `.rsrc` section per processor (`windows/resources-amd64.obj` and
//! its kin), which Go (as a `.syso`), Rust and CMake link as they are. C#
//! writes its version from its `<Version>`. No stack needs a resource
//! compiler or any other tool.

use crate::OutFile;

/// The sizes a Windows icon carries, smallest first; 256 is the largest Windows shows.
pub const SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

/// The processors a resource object is written for: Clarity's name, the COFF machine and its relocation of an image-relative address.
pub const MACHINES: [(&str, u16, u16); 3] = [("amd64", 0x8664, 0x0003), ("arm64", 0xAA64, 0x0002), ("386", 0x014C, 0x0007)];

/// The picture `bytes` (named `name`, for its kind) drawn square at `size` pixels, as PNG; none when it is no picture Clarity reads.
fn drawn(name: &str, bytes: &[u8], size: u32) -> Option<Vec<u8>> {
    let image = if name.to_ascii_lowercase().ends_with(".svg") {
        let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
        let (w, h) = (tree.size().width(), tree.size().height());
        // Fitted in the square, kept in proportion and centred.
        let scale = (size as f32 / w).min(size as f32 / h);
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale).post_translate((size as f32 - w * scale) / 2.0, (size as f32 - h * scale) / 2.0);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        // tiny-skia keeps premultiplied pixels; a PNG holds straight ones.
        let straight: Vec<u8> = pixmap.pixels().iter().flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        }).collect();
        image::RgbaImage::from_raw(size, size, straight)?
    } else {
        let picture = image::load_from_memory(bytes).ok()?.to_rgba8();
        // Fitted in the square, kept in proportion and centred on a clear ground.
        let (w, h) = picture.dimensions();
        let scale = (size as f32 / w as f32).min(size as f32 / h as f32);
        let (fw, fh) = (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1));
        let fitted = image::imageops::resize(&picture, fw, fh, image::imageops::FilterType::Lanczos3);
        let mut square = image::RgbaImage::new(size, size);
        image::imageops::overlay(&mut square, &fitted, ((size - fw) / 2) as i64, ((size - fh) / 2) as i64);
        square
    };
    let mut png = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
    Some(png)
}

/// The picture at every size of SIZES, as PNG; none when it is no picture Clarity reads.
pub fn images(name: &str, bytes: &[u8]) -> Option<Vec<(u32, Vec<u8>)>> {
    SIZES.iter().map(|s| drawn(name, bytes, *s).map(|png| (*s, png))).collect()
}

/// A Windows icon file of `images` (each a size and its PNG): an ICONDIR and an entry per size, then the PNGs.
pub fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in images {
        // 256 is written as 0, as the format has one byte for it.
        out.push(if *size >= 256 { 0 } else { *size as u8 });
        out.push(if *size >= 256 { 0 } else { *size as u8 });
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    out
}

/// The icon group a program carries (RT_GROUP_ICON): the entries of the icon, each naming its RT_ICON by number (1, 2, ...).
fn group(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    for (i, (size, png)) in images.iter().enumerate() {
        out.push(if *size >= 256 { 0 } else { *size as u8 });
        out.push(if *size >= 256 { 0 } else { *size as u8 });
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&(i as u16 + 1).to_le_bytes());
    }
    out
}

const RT_ICON: u32 = 3;
const RT_GROUP_ICON: u32 = 14;
/// The language of the resources: neutral.
const LANGUAGE: u32 = 0;

fn u16le(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn u32le(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// A resource directory's header with `ids` entries, each an id and the offset it points at (a subdirectory has the high bit set).
fn directory(out: &mut Vec<u8>, ids: &[(u32, u32)]) {
    u32le(out, 0);
    u32le(out, 0);
    u16le(out, 0);
    u16le(out, 0);
    u16le(out, 0);
    u16le(out, ids.len() as u16);
    for (id, offset) in ids {
        u32le(out, *id);
        u32le(out, *offset);
    }
}

/// One resource of the program: its type (RT_ICON, RT_GROUP_ICON, RT_VERSION), its id and its data.
pub struct Resource {
    pub kind: u32,
    pub id: u32,
    pub data: Vec<u8>,
}

/// The resources of the icon `images`: each picture (RT_ICON 1..n) and the group that lists them (RT_GROUP_ICON 1).
pub fn icon_resources(images: &[(u32, Vec<u8>)]) -> Vec<Resource> {
    let mut out: Vec<Resource> = images.iter().enumerate().map(|(i, (_, png))| Resource { kind: RT_ICON, id: i as u32 + 1, data: png.clone() }).collect();
    out.push(Resource { kind: RT_GROUP_ICON, id: 1, data: group(images) });
    out
}

/// The `.rsrc` section of `resources`: the resource tree (type, name,
/// language, each level in ascending order as Windows reads it), its data
/// entries and the data, with the offsets inside the section of each data
/// entry's address, which the linker relocates.
fn section(resources: &[Resource]) -> (Vec<u8>, Vec<u32>) {
    let mut sorted: Vec<&Resource> = resources.iter().collect();
    sorted.sort_by_key(|r| (r.kind, r.id));
    let mut kinds: Vec<u32> = sorted.iter().map(|r| r.kind).collect();
    kinds.dedup();
    let dir = |entries: usize| 16 + 8 * entries as u32;
    // Where each part of the tree goes: the root, a directory per type, one per resource (its language), the data entries, the data.
    let mut at = dir(kinds.len());
    let kind_dirs: Vec<u32> = kinds
        .iter()
        .map(|k| {
            let here = at;
            at += dir(sorted.iter().filter(|r| r.kind == *k).count());
            here
        })
        .collect();
    let language_dirs: Vec<u32> = sorted
        .iter()
        .map(|_| {
            let here = at;
            at += dir(1);
            here
        })
        .collect();
    let entries_at = at;
    let mut data_at = entries_at + 16 * sorted.len() as u32;
    let mut placed = Vec::new();
    for resource in &sorted {
        data_at = data_at.next_multiple_of(8);
        placed.push(data_at);
        data_at += resource.data.len() as u32;
    }
    let sub = 0x8000_0000;
    let mut out = Vec::new();
    directory(&mut out, &kinds.iter().zip(&kind_dirs).map(|(k, d)| (*k, sub | d)).collect::<Vec<_>>());
    for kind in &kinds {
        let ids: Vec<(u32, u32)> = sorted.iter().zip(&language_dirs).filter(|(r, _)| r.kind == *kind).map(|(r, d)| (r.id, sub | d)).collect();
        directory(&mut out, &ids);
    }
    for i in 0..sorted.len() {
        directory(&mut out, &[(LANGUAGE, entries_at + 16 * i as u32)]);
    }
    let mut relocated = Vec::new();
    for (resource, at) in sorted.iter().zip(&placed) {
        relocated.push(out.len() as u32);
        u32le(&mut out, *at);
        u32le(&mut out, resource.data.len() as u32);
        u32le(&mut out, 0);
        u32le(&mut out, 0);
    }
    for (resource, at) in sorted.iter().zip(&placed) {
        out.resize(*at as usize, 0);
        out.extend_from_slice(&resource.data);
    }
    out.resize(out.len().next_multiple_of(8), 0);
    (out, relocated)
}

const RT_VERSION: u32 = 16;

/// UTF-16, little-endian, with its terminating zero.
fn wide(text: &str) -> Vec<u8> {
    text.encode_utf16().chain(std::iter::once(0)).flat_map(|c| c.to_le_bytes()).collect()
}

/// One block of a version resource: its length, its value's length, its
/// type (1 for text), its key, its value and its children, each on a
/// 4-byte boundary, as Windows' resource compiler writes them.
fn version_block(key: &str, kind: u16, value: &[u8], value_length: u16, children: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![0, 0];
    u16le(&mut out, value_length);
    u16le(&mut out, kind);
    out.extend(wide(key));
    out.resize(out.len().next_multiple_of(4), 0);
    out.extend_from_slice(value);
    for child in children {
        out.resize(out.len().next_multiple_of(4), 0);
        out.extend_from_slice(child);
    }
    let length = out.len() as u16;
    out[..2].copy_from_slice(&length.to_le_bytes());
    out
}

/// The program's version resource (VS_VERSIONINFO): its numbers for
/// Windows to compare, and the texts its file's Properties show: the
/// title as the description and product, and the version as written.
pub fn version_info(version: &str, title: &str, program: &str) -> Vec<u8> {
    let (numbers, pre) = match version.split_once('-') {
        Some((numbers, _)) => (numbers, true),
        None => (version, false),
    };
    let part = |i: usize| numbers.split('.').nth(i).and_then(|n| n.parse::<u32>().ok()).unwrap_or(0).min(0xFFFF);
    let (major, minor, patch) = (part(0), part(1), part(2));
    let mut fixed = Vec::new();
    u32le(&mut fixed, 0xFEEF_04BD);
    u32le(&mut fixed, 0x0001_0000);
    u32le(&mut fixed, (major << 16) | minor);
    u32le(&mut fixed, patch << 16);
    u32le(&mut fixed, (major << 16) | minor);
    u32le(&mut fixed, patch << 16);
    u32le(&mut fixed, 0x3F);
    // A pre-release (`2.0.0-beta.1`) is marked so.
    u32le(&mut fixed, if pre { 0x2 } else { 0 });
    u32le(&mut fixed, 0x0004_0004);
    u32le(&mut fixed, 1);
    u32le(&mut fixed, 0);
    u32le(&mut fixed, 0);
    u32le(&mut fixed, 0);
    let string = |key: &str, value: &str| version_block(key, 1, &wide(value), value.encode_utf16().count() as u16 + 1, &[]);
    let file = format!("{program}.exe");
    let strings = [
        string("FileDescription", title),
        string("FileVersion", version),
        string("InternalName", program),
        string("OriginalFilename", &file),
        string("ProductName", title),
        string("ProductVersion", version),
    ];
    // US English, Unicode: the language and code page the strings are in.
    let table = version_block("040904B0", 1, &[], 0, &strings);
    let string_info = version_block("StringFileInfo", 1, &[], 0, &[table]);
    let translation = version_block("Translation", 0, &[0x09, 0x04, 0xB0, 0x04], 4, &[]);
    let var_info = version_block("VarFileInfo", 1, &[], 0, &[translation]);
    version_block("VS_VERSION_INFO", 0, &fixed, fixed.len() as u16, &[string_info, var_info])
}

/// A COFF object holding `images` as the program's icon in a `.rsrc`
/// section, for the processor `machine` with its relocation `relocation` (MACHINES).
pub fn resource_object(images: &[(u32, Vec<u8>)], machine: u16, relocation: u16) -> Vec<u8> {
    resources_object(&icon_resources(images), machine, relocation)
}

/// A COFF object holding `resources` in a `.rsrc` section, for the
/// processor `machine` with its relocation `relocation` (MACHINES); the
/// linker places the section and fixes each data entry's address.
pub fn resources_object(resources: &[Resource], machine: u16, relocation: u16) -> Vec<u8> {
    let (data, relocated) = section(resources);
    let header = 20u32;
    let section_header = 40u32;
    let data_at = header + section_header;
    let relocations_at = data_at + data.len() as u32;
    let symbols_at = relocations_at + 10 * relocated.len() as u32;
    let mut out = Vec::new();
    // The file header.
    u16le(&mut out, machine);
    u16le(&mut out, 1);
    u32le(&mut out, 0);
    u32le(&mut out, symbols_at);
    u32le(&mut out, 2);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    // The section header: initialized data, read only, marked as Windows' own resource converter marks it.
    out.extend_from_slice(b".rsrc\0\0\0");
    u32le(&mut out, 0);
    u32le(&mut out, 0);
    u32le(&mut out, data.len() as u32);
    u32le(&mut out, data_at);
    u32le(&mut out, relocations_at);
    u32le(&mut out, 0);
    u16le(&mut out, relocated.len() as u16);
    u16le(&mut out, 0);
    u32le(&mut out, 0x0000_0040 | 0x4000_0000);
    out.extend_from_slice(&data);
    // Each data entry's address is relative to the section's symbol (0), made image-relative by the linker.
    for at in &relocated {
        u32le(&mut out, *at);
        u32le(&mut out, 0);
        u16le(&mut out, relocation);
    }
    // The section's symbol, with its auxiliary record.
    out.extend_from_slice(b".rsrc\0\0\0");
    u32le(&mut out, 0);
    u16le(&mut out, 1);
    u16le(&mut out, 0);
    out.push(3);
    out.push(1);
    u32le(&mut out, data.len() as u32);
    u16le(&mut out, relocated.len() as u16);
    u16le(&mut out, 0);
    u32le(&mut out, 0);
    u16le(&mut out, 0);
    out.push(0);
    out.extend_from_slice(&[0, 0, 0]);
    // An empty string table.
    u32le(&mut out, 4);
    out
}

/// The project's icon at every size; none when it names no icon or it is no picture Clarity reads.
fn project_images(project: &clarity_spec::Project) -> Option<Vec<(u32, Vec<u8>)>> {
    let name = project.icon.as_ref()?;
    let bytes = project.resources.as_ref()?.files.iter().find(|f| &f.name == name).map(|f| &f.bytes)?;
    images(name, bytes)
}

fn binary(path: String, bytes: Vec<u8>) -> OutFile {
    OutFile { path, text: String::new(), bytes: Some(std::borrow::Cow::Owned(bytes)) }
}

/// Where C# finds the icon file, from its project's folder.
pub const ICO_PATH: &str = "windows/app.ico";

/// The icon file C# takes as its ApplicationIcon (ICO_PATH); none without an icon.
pub fn ico_file(project: &clarity_spec::Project) -> Option<OutFile> {
    project_images(project).map(|images| binary(ICO_PATH.into(), ico(&images)))
}

/// The program's resources on Windows: its icon, and its version with
/// `title` and the program's name `program`.
pub fn program_resources(project: &clarity_spec::Project, title: &str, program: &str) -> Vec<Resource> {
    let mut out = project_images(project).map(|images| icon_resources(&images)).unwrap_or_default();
    if let Some(version) = &project.version {
        out.push(Resource { kind: RT_VERSION, id: 1, data: version_info(version, title, program) });
    }
    out
}

/// The resource object of each processor, at the path `at` gives for its
/// name (`amd64`, `arm64`, `386`): the icon and the version; none with neither.
pub fn object_files(project: &clarity_spec::Project, title: &str, program: &str, at: impl Fn(&str) -> String) -> Vec<OutFile> {
    let resources = program_resources(project, title, program);
    if resources.is_empty() {
        return Vec::new();
    }
    MACHINES.iter().map(|(arch, machine, relocation)| binary(at(arch), resources_object(&resources, *machine, *relocation))).collect()
}

/// Where Rust, C and C++ find a processor's resource object, from the project's folder.
pub fn object_path(arch: &str) -> String {
    format!("windows/resources-{arch}.obj")
}

/// Where Go finds a processor's resource object: beside `main.go`, which links each `.syso` for its own system and processor.
pub fn syso_path(arch: &str) -> String {
    format!("resources_windows_{arch}.syso")
}

/// A Rust program's `build.rs` that links its icon on Windows.
pub const RUST_BUILD: &str = "// Generated by Clarity: links the program's icon and version on Windows (windows/resources-<processor>.obj).
fn main() {
    println!(\"cargo:rerun-if-changed=windows\");
    if std::env::var(\"CARGO_CFG_TARGET_OS\").as_deref() != Ok(\"windows\") {
        return;
    }
    let arch = match std::env::var(\"CARGO_CFG_TARGET_ARCH\").as_deref() {
        Ok(\"x86_64\") => \"amd64\",
        Ok(\"aarch64\") => \"arm64\",
        Ok(\"x86\") => \"386\",
        _ => return,
    };
    let dir = std::env::var(\"CARGO_MANIFEST_DIR\").unwrap_or_default();
    println!(\"cargo:rustc-link-arg-bins={dir}/windows/resources-{arch}.obj\");
}
";

/// The lines of a CMakeLists.txt that link the program `program`'s icon and version on Windows.
pub fn cmake_lines(program: &str) -> String {
    format!(
        "
# The program's icon and version on Windows (windows/resources-<processor>.obj), written by Clarity.
if(WIN32)
  if(CMAKE_SIZEOF_VOID_P EQUAL 4)
    set(CLARITY_RESOURCES_CPU 386)
  elseif(CMAKE_SYSTEM_PROCESSOR MATCHES \"^(ARM64|arm64|aarch64)$\" OR CMAKE_GENERATOR_PLATFORM MATCHES \"ARM64\")
    set(CLARITY_RESOURCES_CPU arm64)
  else()
    set(CLARITY_RESOURCES_CPU amd64)
  endif()
  target_sources({program} PRIVATE ${{CMAKE_CURRENT_SOURCE_DIR}}/windows/resources-${{CLARITY_RESOURCES_CPU}}.obj)
endif()
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]));
        let mut out = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    #[test]
    fn a_picture_is_drawn_square_at_every_size() {
        let drawn = images("logo.png", &png(300, 150)).unwrap();
        assert_eq!(SIZES.to_vec(), drawn.iter().map(|(s, _)| *s).collect::<Vec<_>>());
        for (size, bytes) in &drawn {
            let image = image::load_from_memory(bytes).unwrap();
            assert_eq!((*size, *size), (image.width(), image.height()));
        }
        // A wide picture keeps its proportion: clear above and below, red in the middle.
        let big = image::load_from_memory(&drawn[6].1).unwrap().to_rgba8();
        assert_eq!(0, big.get_pixel(128, 2)[3]);
        assert_eq!([200, 30, 30, 255], big.get_pixel(128, 128).0);
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="24" height="24" fill="#1e64c8"/></svg>"##;
        let from_svg = images("logo.svg", svg).unwrap();
        assert_eq!([30, 100, 200, 255], image::load_from_memory(&from_svg[3].1).unwrap().to_rgba8().get_pixel(24, 24).0);
        assert!(images("notes.png", b"no picture").is_none());
    }

    #[test]
    fn an_icon_file_lists_each_size_and_holds_its_png() {
        let drawn = images("logo.png", &png(64, 64)).unwrap();
        let file = ico(&drawn);
        assert_eq!(&file[0..6], &[0, 0, 1, 0, 7, 0]);
        // The 256 entry is written as 0 by 0, and points at a PNG.
        let last = 6 + 16 * 6;
        assert_eq!(&file[last..last + 2], &[0, 0]);
        let offset = u32::from_le_bytes(file[last + 12..last + 16].try_into().unwrap()) as usize;
        assert_eq!(&file[offset..offset + 8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn a_resource_object_has_one_rsrc_section_and_a_relocation_per_data_entry() {
        let drawn = images("logo.png", &png(32, 32)).unwrap();
        for (_, machine, relocation) in MACHINES {
            let object = resource_object(&drawn, machine, relocation);
            assert_eq!(u16::from_le_bytes([object[0], object[1]]), machine);
            assert_eq!(&object[20..25], b".rsrc");
            let relocations = u16::from_le_bytes([object[52], object[53]]) as usize;
            assert_eq!(relocations, SIZES.len() + 1, "each icon and the group");
        }
    }
}
