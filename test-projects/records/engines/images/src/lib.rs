//! The Images engine: the size of a picture in pixels, read from the first
//! bytes of its file (PNG or JPEG); 0 for a file that is neither.

use clarity_engine::Bytes;

clarity_engine::interface!();

clarity_engine::function!(width, |(picture,): (Bytes,)| Ok(size(&picture.0).map_or(0, |(w, _)| w)));
clarity_engine::function!(height, |(picture,): (Bytes,)| Ok(size(&picture.0).map_or(0, |(_, h)| h)));

/// Width and height, from a PNG's header or a JPEG's frame.
fn size(bytes: &[u8]) -> Option<(u32, u32)> {
    let be16 = |at: usize| Some(u16::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]) as u32);
    let be32 = |at: usize| Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        // The header chunk comes first: width and height after its length and name.
        return Some((be32(16)?, be32(20)?));
    }
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    // A JPEG is a row of segments; a start of frame holds the size.
    let mut at = 2;
    while at + 9 < bytes.len() {
        if bytes[at] != 0xFF {
            return None;
        }
        let marker = bytes[at + 1];
        let length = be16(at + 2)? as usize;
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return Some((be16(at + 7)?, be16(at + 5)?));
        }
        at += 2 + length;
    }
    None
}
