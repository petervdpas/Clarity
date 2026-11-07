//! `zip(paths, texts)` (conformance/zip.json): a zip archive of one file per
//! path holding the text of the same place, stored as it is, in order, so an
//! EPUB's mimetype can come first. The same files give the same bytes.

/// The CRC-32 a zip keeps of each file.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// The archive's bytes; paths and texts pair up, what the longer list has more is left out.
pub fn stored<P: AsRef<str>, T: AsRef<str>>(paths: &[P], texts: &[T]) -> Vec<u8> {
    let (mut out, mut central, mut count) = (Vec::new(), Vec::new(), 0u16);
    let u16s = |v: &mut Vec<u8>, xs: &[u16]| xs.iter().for_each(|x| v.extend_from_slice(&x.to_le_bytes()));
    let u32s = |v: &mut Vec<u8>, xs: &[u32]| xs.iter().for_each(|x| v.extend_from_slice(&x.to_le_bytes()));
    for (path, text) in paths.iter().zip(texts) {
        let (name, data) = (path.as_ref().as_bytes(), text.as_ref().as_bytes());
        let (crc, size, name_len, offset) = (crc32(data), data.len() as u32, name.len() as u16, out.len() as u32);
        // Version 20, UTF-8 names, stored, 1980-01-01 00:00.
        u32s(&mut out, &[0x0403_4b50]);
        u16s(&mut out, &[20, 0x0800, 0, 0, 0x21]);
        u32s(&mut out, &[crc, size, size]);
        u16s(&mut out, &[name_len, 0]);
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        u32s(&mut central, &[0x0201_4b50]);
        u16s(&mut central, &[20, 20, 0x0800, 0, 0, 0x21]);
        u32s(&mut central, &[crc, size, size]);
        u16s(&mut central, &[name_len, 0, 0, 0, 0]);
        u32s(&mut central, &[0, offset]);
        central.extend_from_slice(name);
        count += 1;
    }
    let (central_size, central_at) = (central.len() as u32, out.len() as u32);
    out.extend_from_slice(&central);
    u32s(&mut out, &[0x0605_4b50]);
    u16s(&mut out, &[0, 0, count, count]);
    u32s(&mut out, &[central_size, central_at]);
    u16s(&mut out, &[0]);
    out
}
