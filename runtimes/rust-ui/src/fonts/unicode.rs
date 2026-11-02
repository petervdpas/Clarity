//! The few Unicode properties text shaping needs, without a table crate:
//! which characters are marks (they sit on the letter before them), which
//! are format characters (they draw nothing), which are currency signs, and
//! which need HarfBuzz to be drawn right. The ranges cover the scripts the
//! shaper handles; a character outside them counts as a plain letter or sign.

/// Whether `c` is in one of the inclusive `ranges`.
fn within(c: u32, ranges: &[(u32, u32)]) -> bool {
    ranges.iter().any(|(from, to)| c >= *from && c <= *to)
}

const MARKS: &[(u32, u32)] = &[
    (0x0300, 0x036F),
    (0x0483, 0x0489),
    // Hebrew points and accents.
    (0x0591, 0x05BD),
    (0x05BF, 0x05BF),
    (0x05C1, 0x05C2),
    (0x05C4, 0x05C5),
    (0x05C7, 0x05C7),
    // Arabic marks.
    (0x0610, 0x061A),
    (0x064B, 0x065F),
    (0x0670, 0x0670),
    (0x06D6, 0x06DC),
    (0x06DF, 0x06E4),
    (0x06E7, 0x06E8),
    (0x06EA, 0x06ED),
    // Syriac, Thaana, N'Ko, Samaritan, Mandaic, Arabic extended.
    (0x0711, 0x0711),
    (0x0730, 0x074A),
    (0x07A6, 0x07B0),
    (0x07EB, 0x07F3),
    (0x07FD, 0x07FD),
    (0x0816, 0x0819),
    (0x081B, 0x0823),
    (0x0825, 0x0827),
    (0x0829, 0x082D),
    (0x0859, 0x085B),
    (0x0898, 0x089F),
    (0x08CA, 0x08E1),
    (0x08E3, 0x08FF),
    // Sinhala.
    (0x0D81, 0x0D83),
    (0x0DCA, 0x0DCA),
    (0x0DCF, 0x0DDF),
    (0x0DF2, 0x0DF3),
    // Thai and Lao.
    (0x0E31, 0x0E31),
    (0x0E34, 0x0E3A),
    (0x0E47, 0x0E4E),
    (0x0EB1, 0x0EB1),
    (0x0EB4, 0x0EBC),
    (0x0EC8, 0x0ECE),
    // Tibetan.
    (0x0F18, 0x0F19),
    (0x0F35, 0x0F35),
    (0x0F37, 0x0F37),
    (0x0F39, 0x0F39),
    (0x0F3E, 0x0F3F),
    (0x0F71, 0x0F84),
    (0x0F86, 0x0F87),
    (0x0F8D, 0x0FBC),
    (0x0FC6, 0x0FC6),
    // Myanmar and Khmer.
    (0x102B, 0x103E),
    (0x1056, 0x1059),
    (0x105E, 0x1060),
    (0x1062, 0x1064),
    (0x1067, 0x106D),
    (0x1071, 0x1074),
    (0x1082, 0x108D),
    (0x108F, 0x108F),
    (0x109A, 0x109D),
    (0x17B4, 0x17D3),
    (0x17DD, 0x17DD),
    // Combining marks of every script.
    (0x1AB0, 0x1AFF),
    (0x1DC0, 0x1DFF),
    (0x20D0, 0x20FF),
    (0x2CEF, 0x2CF1),
    (0x2DE0, 0x2DFF),
    (0x302A, 0x302F),
    (0x3099, 0x309A),
    (0xA66F, 0xA672),
    (0xA674, 0xA67D),
    (0xFB1E, 0xFB1E),
    (0xFE00, 0xFE0F),
    (0xFE20, 0xFE2F),
    (0x1F3FB, 0x1F3FF),
    (0xE0100, 0xE01EF),
];

const FORMATS: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

const CURRENCIES: &[(u32, u32)] = &[
    (0x0024, 0x0024),
    (0x00A2, 0x00A5),
    (0x058F, 0x058F),
    (0x060B, 0x060B),
    (0x07FE, 0x07FF),
    (0x09F2, 0x09F3),
    (0x09FB, 0x09FB),
    (0x0AF1, 0x0AF1),
    (0x0BF9, 0x0BF9),
    (0x0E3F, 0x0E3F),
    (0x17DB, 0x17DB),
    (0x20A0, 0x20CF),
    (0xA838, 0xA838),
    (0xFDFC, 0xFDFC),
    (0xFE69, 0xFE69),
    (0xFF04, 0xFF04),
    (0xFFE0, 0xFFE1),
    (0xFFE5, 0xFFE6),
];

/// A mark of the Brahmic scripts (Devanagari to Malayalam): each block of
/// 128 lays its vowel signs, virama and nukta out at the same offsets.
fn brahmic_mark(c: u32) -> bool {
    if !(0x0900..=0x0D7F).contains(&c) {
        return false;
    }
    let at = c & 0x7F;
    matches!(at, 0x00..=0x03 | 0x3A..=0x3C | 0x3E..=0x4F | 0x51..=0x57 | 0x62..=0x63)
}

/// Whether `c` is a mark: a vowel point, an accent, a virama, a vowel sign; it joins the letter before it.
pub fn is_mark(c: char) -> bool {
    let c = c as u32;
    c >= 0x300 && (brahmic_mark(c) || within(c, MARKS))
}

/// Whether `c` is a format character (a joiner, a direction mark): it draws nothing.
pub fn is_format(c: char) -> bool {
    within(c as u32, FORMATS)
}

/// Whether `c` is a currency sign.
pub fn is_currency(c: char) -> bool {
    within(c as u32, CURRENCIES)
}

/// Whether a character needs HarfBuzz: a mark, a format character, or a
/// script whose letters change by their neighbours or run right to left.
pub fn char_needs_shaping(c: char) -> bool {
    let u = c as u32;
    if u < 0x300 {
        return false;
    }
    let scripts: &[(u32, u32)] = &[
        (0x0590, 0x08FF),
        (0x0900, 0x0DFF),
        (0x0E00, 0x0FFF),
        (0x1000, 0x109F),
        (0x1100, 0x11FF),
        (0x1700, 0x18AF),
        (0x1900, 0x1AFF),
        (0x1B00, 0x1C7F),
        (0xA800, 0xABFF),
        (0xD7B0, 0xD7FF),
        (0xFB1D, 0xFDFF),
        (0xFE70, 0xFEFF),
        (0x10800, 0x10FFF),
        (0x11000, 0x11FFF),
        (0x1E800, 0x1EFFF),
    ];
    within(u, scripts) || is_mark(c) || is_format(c)
}

/// Whether `text` needs HarfBuzz to be drawn right.
pub fn needs_shaping(text: &str) -> bool {
    text.chars().any(char_needs_shaping)
}

/// Which characters start a letter as a person sees it (a grapheme): a caret
/// goes before them. A mark, a joiner and what a joiner joins, and the line
/// feed of a CR LF belong to the letter before.
pub fn starts(chars: &[char]) -> Vec<bool> {
    let mut starts = vec![true; chars.len()];
    for i in 1..chars.len() {
        let c = chars[i];
        let before = chars[i - 1];
        if is_mark(c) || c == '\u{200D}' || c == '\u{200C}' || before == '\u{200D}' || (before == '\r' && c == '\n') {
            starts[i] = false;
        }
    }
    starts
}
