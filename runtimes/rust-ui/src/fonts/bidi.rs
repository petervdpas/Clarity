//! The Unicode bidirectional algorithm (UAX #9) for one line of text, as
//! the C# version runs it: the paragraph's direction from its first strong
//! letter, the implicit levels of every character (rules W1 to W7, N1, N2,
//! I1, I2), the line's end reset (L1), and the visual order of runs of one
//! level (L2). Hebrew and Arabic run right to left, Latin and numbers left
//! to right inside them. Explicit embedding and isolate marks are left out
//! (rule X9), so they change nothing; the marks LRM, RLM and ALM count as
//! strong letters. Places are characters (Unicode scalar values).

use super::unicode::{is_currency, is_format, is_mark};

/// The bidirectional class of a character (UAX #9, table 4), as far as Clarity.UI needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidiClass {
    L,
    R,
    AL,
    EN,
    ES,
    ET,
    AN,
    CS,
    NSM,
    BN,
    B,
    S,
    WS,
    ON,
}

/// Arabic, Syriac, Thaana and their presentation forms: the letters rule W2 reads digits after as Arabic.
fn is_arabic_letter(c: u32) -> bool {
    matches!(c, 0x0600..=0x07BF | 0x0860..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF | 0x1EE00..=0x1EEFF)
}

/// The blocks of scripts written right to left: Hebrew, N'Ko, Samaritan, Mandaic, and the old ones beyond the first plane.
fn is_right_to_left(c: u32) -> bool {
    matches!(c, 0x0590..=0x08FF | 0xFB1D..=0xFB4F | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF)
}

/// The bidirectional class of `character`.
pub fn class_of(character: char) -> BidiClass {
    use BidiClass::*;
    let c = character as u32;
    match c {
        0x200E => return L,
        0x200F => return R,
        0x061C => return AL,
        0x0A | 0x0D | 0x1C | 0x1D | 0x1E | 0x85 | 0x2029 => return B,
        0x09 | 0x0B | 0x1F => return S,
        0x20 | 0x0C | 0x1680 | 0x2028 | 0x205F | 0x3000 | 0x2000..=0x200A => return WS,
        0x30..=0x39 | 0x06F0..=0x06F9 | 0xFF10..=0xFF19 | 0xB2 | 0xB3 | 0xB9 | 0x2070 | 0x2074..=0x2079 | 0x2080..=0x2089 => {
            return EN;
        }
        0x0600..=0x0605 | 0x0660..=0x0669 | 0x066B | 0x066C | 0x06DD | 0x0890 | 0x0891 | 0x08E2 => return AN,
        0x2B | 0x2D | 0x207A | 0x207B | 0x208A | 0x208B | 0x2212 | 0xFB29 | 0xFE62 | 0xFE63 | 0xFF0B | 0xFF0D => return ES,
        0x23 | 0x24 | 0x25 | 0xB0 | 0xB1 | 0x0609 | 0x060A | 0x066A | 0x2030..=0x2034 | 0x212E | 0x2213 | 0xFE5F | 0xFE69 | 0xFE6A
        | 0xFF03 | 0xFF04 | 0xFF05 => return ET,
        0x2C | 0x2E | 0x2F | 0x3A | 0xA0 | 0x060C | 0x202F | 0x2044 | 0xFE50 | 0xFE52 | 0xFE55 | 0xFF0C | 0xFF0E | 0xFF0F | 0xFF1A => {
            return CS;
        }
        _ => {}
    }
    if is_mark(character) {
        return NSM;
    }
    if character.is_control() || is_format(character) {
        return BN;
    }
    if is_currency(character) {
        return ET;
    }
    if is_arabic_letter(c) {
        return AL;
    }
    if is_right_to_left(c) {
        return R;
    }
    if character.is_alphabetic() || character.is_numeric() || (0xE000..=0xF8FF).contains(&c) {
        L
    } else {
        ON
    }
}

/// Whether `text` has a character written right to left.
pub fn has_right_to_left(text: &str) -> bool {
    text.chars().any(|ch| {
        let c = ch as u32;
        c >= 0x0590 && (is_arabic_letter(c) || is_right_to_left(c) || c == 0x200F || c == 0x061C)
    })
}

/// The paragraph's level: 1 (right to left) when its first strong letter is right to left, else 0 (rules P2, P3).
pub fn paragraph_level(text: &str) -> u8 {
    for c in text.chars() {
        match class_of(c) {
            BidiClass::L => return 0,
            BidiClass::R | BidiClass::AL => return 1,
            _ => {}
        }
    }
    0
}

/// The embedding level of each character of `text`, a line of one
/// paragraph: even is left to right, odd right to left. The paragraph's
/// level is `paragraph`, or found from the text when it is None.
pub fn levels(text: &str, paragraph: Option<u8>) -> Vec<u8> {
    use BidiClass::*;
    let para = paragraph.unwrap_or_else(|| paragraph_level(text));
    let original: Vec<BidiClass> = text.chars().map(class_of).collect();
    let n = original.len();
    let mut types = original.clone();
    let mut levels = vec![0u8; n];
    let embedding = if para % 2 == 0 { L } else { R };

    // X9: BN is left out of the rules that follow.
    let kept: Vec<usize> = (0..n).filter(|i| types[*i] != BN).collect();
    let m = kept.len();
    let t = |types: &Vec<BidiClass>, k: usize| types[kept[k]];

    // W1: a mark takes the class of what it marks.
    for k in 0..m {
        if t(&types, k) == NSM {
            types[kept[k]] = if k == 0 { embedding } else { t(&types, k - 1) };
        }
    }
    // W2: a European digit after Arabic letters is an Arabic one.
    let mut last_strong = embedding;
    for k in 0..m {
        match t(&types, k) {
            L | R | AL => last_strong = t(&types, k),
            EN if last_strong == AL => types[kept[k]] = AN,
            _ => {}
        }
    }
    // W3: Arabic letters are right to left.
    for k in 0..m {
        if t(&types, k) == AL {
            types[kept[k]] = R;
        }
    }
    // W4: one separator between two numbers of a kind joins them.
    for k in 1..m.saturating_sub(1) {
        let (before, here, after) = (t(&types, k - 1), t(&types, k), t(&types, k + 1));
        if here == ES && before == EN && after == EN {
            types[kept[k]] = EN;
        } else if here == CS && before == after && (before == EN || before == AN) {
            types[kept[k]] = before;
        }
    }
    // W5: terminators (%, $) next to a European number join it.
    let mut k = 0;
    while k < m {
        if t(&types, k) != ET {
            k += 1;
            continue;
        }
        let mut end = k;
        while end < m && t(&types, end) == ET {
            end += 1;
        }
        if (k > 0 && t(&types, k - 1) == EN) || (end < m && t(&types, end) == EN) {
            for j in k..end {
                types[kept[j]] = EN;
            }
        }
        k = end;
    }
    // W6: the separators and terminators left are neutral.
    for k in 0..m {
        if matches!(t(&types, k), ES | ET | CS) {
            types[kept[k]] = ON;
        }
    }
    // W7: a European number after a left-to-right letter is left to right.
    let mut last_strong = embedding;
    for k in 0..m {
        match t(&types, k) {
            L | R => last_strong = t(&types, k),
            EN if last_strong == L => types[kept[k]] = L,
            _ => {}
        }
    }
    // N1, N2: neutrals between letters of one direction take it; else the paragraph's.
    let neutral = |c: BidiClass| matches!(c, B | S | WS | ON);
    let strong = |c: BidiClass| if matches!(c, EN | AN) { R } else { c };
    let mut k = 0;
    while k < m {
        if !neutral(t(&types, k)) {
            k += 1;
            continue;
        }
        let mut end = k;
        while end < m && neutral(t(&types, end)) {
            end += 1;
        }
        let lead = if k == 0 { embedding } else { strong(t(&types, k - 1)) };
        let trail = if end == m { embedding } else { strong(t(&types, end)) };
        let resolved = if lead == trail { lead } else { embedding };
        for j in k..end {
            types[kept[j]] = resolved;
        }
        k = end;
    }
    // I1, I2: the levels.
    for i in 0..n {
        let c = types[i];
        levels[i] = if para % 2 == 0 {
            if c == R {
                para + 1
            } else if matches!(c, AN | EN) {
                para + 2
            } else {
                para
            }
        } else if matches!(c, L | EN | AN) {
            para + 1
        } else {
            para
        };
    }
    // What X9 left out takes the level before it.
    for i in 0..n {
        if original[i] == BN {
            levels[i] = if i == 0 { para } else { levels[i - 1] };
        }
    }
    // L1: tabs, paragraph ends and the spaces before them and at the line's end go to the paragraph's level.
    let mut trailing = true;
    for i in (0..n).rev() {
        match original[i] {
            S | B => {
                levels[i] = para;
                trailing = true;
            }
            WS | BN if trailing => levels[i] = para,
            _ => trailing = false,
        }
    }
    levels
}

/// The visual order, from the left, of runs at `levels` (rule L2): from
/// the highest level down to the lowest odd one, every sequence of runs at
/// that level or higher is turned around.
pub fn visual_order(levels: &[u8]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..levels.len()).collect();
    let Some(&highest) = levels.iter().max() else {
        return order;
    };
    let lowest_odd = levels.iter().copied().filter(|l| l % 2 == 1).min().unwrap_or(highest + 1);
    let mut level = highest;
    while level >= lowest_odd && level > 0 {
        let mut i = 0;
        while i < order.len() {
            if levels[order[i]] < level {
                i += 1;
                continue;
            }
            let mut end = i;
            while end < order.len() && levels[order[end]] >= level {
                end += 1;
            }
            order[i..end].reverse();
            i = end;
        }
        level -= 1;
    }
    order
}
