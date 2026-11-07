//! The sound part (DESIGN 5.22): what code can tell it, and how a melody's
//! notes read, alike on every stack (conformance/melody.json).

use crate::code::check::TypeInfo;

/// What code tells a sound, with the values each takes: `rollSound.play()`,
/// or `music.play(path)` for a song from disk (docs/AUDIO-PLAN.md).
pub const HEARS: [(&str, &[(&str, TypeInfo)]); 8] = [
    ("play", &[("path", TypeInfo::Text)]),
    ("pause", &[]),
    ("resume", &[]),
    ("seek", &[("seconds", TypeInfo::Decimal)]),
    ("stop", &[]),
    ("tone", &[("hertz", TypeInfo::Integer), ("seconds", TypeInfo::Decimal)]),
    ("melody", &[("notes", TypeInfo::Text), ("tempo", TypeInfo::Integer)]),
    ("volume", &[("level", TypeInfo::Integer)]),
];

/// The values a sound's `hear` takes, or None when it hears no such thing.
pub fn hear(name: &str) -> Option<&'static [(&'static str, TypeInfo)]> {
    HEARS.iter().find(|(n, _)| *n == name).map(|(_, params)| *params)
}

/// How many of its values a hear needs: `play()` plays its source, `play(path)` a file.
pub fn least(name: &str) -> usize {
    if name == "play" { 0 } else { hear(name).map_or(0, <[_]>::len) }
}

/// What code reads of a sound, as it plays (docs/AUDIO-PLAN.md): kept by
/// Clarity.UI in the part's values, read as a custom part's are.
pub const READS: [(&str, TypeInfo); 10] = [
    ("playing", TypeInfo::Bool),
    ("paused", TypeInfo::Bool),
    // Whole seconds played, and the song's length in whole seconds (0 when its
    // file does not say): what a slider and a time shown take.
    ("position", TypeInfo::Integer),
    ("length", TypeInfo::Integer),
    // The path of the file that plays; empty for a resource or nothing.
    ("file", TypeInfo::Text),
    // Why the last song did not play, for `on fail`; empty when it did.
    ("problem", TypeInfo::Text),
    // What the playing song's file says of it (ID3, Vorbis comments); empty
    // when it does not say, and 0 for a track it does not number.
    ("title", TypeInfo::Text),
    ("artist", TypeInfo::Text),
    ("album", TypeInfo::Text),
    ("track", TypeInfo::Integer),
];

/// The type of what code reads of a sound, or None when a sound says no such thing.
pub fn read(name: &str) -> Option<TypeInfo> {
    READS.iter().find(|(n, _)| *n == name).map(|(_, ty)| ty.clone())
}

/// What a sound says of `name` before it played anything, as JSON: false, 0 or empty.
pub fn read_fallback(name: &str) -> &'static str {
    match read(name) {
        Some(TypeInfo::Bool) => "false",
        Some(TypeInfo::Integer) => "0",
        _ => "\"\"",
    }
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
