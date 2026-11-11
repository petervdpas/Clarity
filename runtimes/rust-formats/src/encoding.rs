//! The encodings text formats read and write besides UTF-8: windows-1252
//! and iso-8859-1, one byte a character.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Windows1252,
    Latin1,
}

impl Encoding {
    /// The encoding a name says, in any case; none for one not read.
    pub fn named(name: &str) -> Option<Encoding> {
        match name.to_ascii_lowercase().as_str() {
            "utf-8" | "utf8" => Some(Encoding::Utf8),
            "windows-1252" | "cp1252" => Some(Encoding::Windows1252),
            "iso-8859-1" | "latin1" | "latin-1" => Some(Encoding::Latin1),
            _ => None,
        }
    }
}

/// The characters windows-1252 has at 0x80 to 0x9F where iso-8859-1 has control characters.
const WINDOWS_1252: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{8D}', '\u{017D}', '\u{8F}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{9D}', '\u{017E}', '\u{0178}',
];

/// The bytes as text; a UTF-8 byte order mark dropped; none when they are not UTF-8.
pub fn decode(bytes: &[u8], encoding: Encoding) -> Option<String> {
    match encoding {
        Encoding::Utf8 => std::str::from_utf8(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes)).ok().map(str::to_string),
        Encoding::Latin1 => Some(bytes.iter().map(|&b| b as char).collect()),
        Encoding::Windows1252 => Some(bytes.iter().map(|&b| if (0x80..0xA0).contains(&b) { WINDOWS_1252[(b - 0x80) as usize] } else { b as char }).collect()),
    }
}

/// One character's byte, or none when the encoding cannot hold it.
fn byte(c: char, encoding: Encoding) -> Option<u8> {
    let code = c as u32;
    if code < 0x80 || (encoding == Encoding::Latin1 && code < 0x100) || (encoding == Encoding::Windows1252 && (0xA0..0x100).contains(&code)) {
        return Some(code as u8);
    }
    WINDOWS_1252.iter().position(|w| *w == c).filter(|_| encoding == Encoding::Windows1252).map(|at| 0x80 + at as u8)
}

fn name(encoding: Encoding) -> &'static str {
    if encoding == Encoding::Latin1 { "iso-8859-1" } else { "windows-1252" }
}

/// The text as bytes; why not, when a character cannot be written.
pub fn encode(text: &str, encoding: Encoding) -> Result<Vec<u8>, String> {
    if encoding == Encoding::Utf8 {
        return Ok(text.as_bytes().to_vec());
    }
    text.chars().map(|c| byte(c, encoding).ok_or_else(|| format!("'{c}' cannot be written in {}", name(encoding)))).collect()
}

/// The text as bytes, a character the encoding cannot hold written by `instead` (a character reference).
pub fn encode_or(text: &str, encoding: Encoding, instead: impl Fn(char) -> String) -> Vec<u8> {
    if encoding == Encoding::Utf8 {
        return text.as_bytes().to_vec();
    }
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match byte(c, encoding) {
            Some(b) => out.push(b),
            None => out.extend(instead(c).into_bytes()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_decoded_from_the_encoding_named() {
        assert_eq!(decode(b"\xEF\xBB\xBFname", Encoding::Utf8).unwrap(), "name", "the byte order mark is dropped");
        assert_eq!(decode(b"caf\xE9 \x80", Encoding::Windows1252).unwrap(), "café €");
        assert_eq!(encode("café €", Encoding::Windows1252).unwrap(), b"caf\xE9 \x80");
        assert!(encode("€", Encoding::Latin1).is_err());
        assert_eq!(encode_or("a€", Encoding::Latin1, |c| format!("&#{};", c as u32)), b"a&#8364;");
    }
}
