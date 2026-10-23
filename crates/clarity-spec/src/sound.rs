//! The sound part (DESIGN 5.22): what code can tell it, and how a melody's
//! notes read, alike on every stack (conformance/melody.json).

use crate::code::check::TypeInfo;

/// What code tells a sound, with the values each takes: `rollSound.play()`.
pub const HEARS: [(&str, &[(&str, TypeInfo)]); 5] = [
    ("play", &[]),
    ("stop", &[]),
    ("tone", &[("hertz", TypeInfo::Integer), ("seconds", TypeInfo::Decimal)]),
    ("melody", &[("notes", TypeInfo::Text), ("tempo", TypeInfo::Integer)]),
    ("volume", &[("level", TypeInfo::Integer)]),
];

/// The values a sound's `hear` takes, or None when it hears no such thing.
pub fn hear(name: &str) -> Option<&'static [(&'static str, TypeInfo)]> {
    HEARS.iter().find(|(n, _)| *n == name).map(|(_, params)| *params)
}

/// A note of a melody: its frequency (None for a rest) and how long it lasts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub hertz: Option<f64>,
    pub seconds: f64,
}

/// The notes of `notes` at `tempo` beats a minute, and the words that are
/// not notes, which a running program skips.
pub fn melody(notes: &str, tempo: i64) -> (Vec<Note>, Vec<String>) {
    let beat = 60.0 / tempo.max(1) as f64;
    let mut plays = Vec::new();
    let mut skips = Vec::new();
    for word in notes.split_whitespace() {
        match note(word) {
            Some((hertz, beats)) => plays.push(Note { hertz, seconds: beats * beat }),
            None => skips.push(word.to_string()),
        }
    }
    (plays, skips)
}

/// A word's frequency (None for a rest) and its beats; None when it is not a note.
fn note(word: &str) -> Option<(Option<f64>, f64)> {
    let (pitch, beats) = match word.find(['*', '/']) {
        Some(at) => {
            let count: u32 = word[at + 1..].parse().ok().filter(|n| *n >= 1 && !word[at + 1..].starts_with('0'))?;
            (&word[..at], if word[at..].starts_with('*') { count as f64 } else { 1.0 / count as f64 })
        }
        None => (word, 1.0),
    };
    if pitch == "-" {
        return Some((None, beats));
    }
    let mut chars = pitch.chars();
    let semitone: i32 = match chars.next()? {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest = chars.as_str();
    let (shift, rest) = match rest.chars().next() {
        Some('#') => (1, &rest[1..]),
        Some('b') => (-1, &rest[1..]),
        _ => (0, rest),
    };
    let octave: i32 = match rest {
        "" => 4,
        digit if digit.len() == 1 && ('0'..='8').contains(&digit.chars().next()?) => digit.parse().ok()?,
        _ => return None,
    };
    let midi = 12 * (octave + 1) + semitone + shift;
    Some((Some(440.0 * 2f64.powf((midi - 69) as f64 / 12.0)), beats))
}
