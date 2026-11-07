//! The video part (docs/VIDEO-PLAN.md): what code can tell it and read of
//! it, as a sound's songs (crate::sound), with no tones or melodies.

use crate::code::check::TypeInfo;

/// What code tells a video, with the values each takes: `film.play()` plays
/// its source, `film.play(path)` a file from disk.
pub const HEARS: [(&str, &[(&str, TypeInfo)]); 6] = [
    ("play", &[("path", TypeInfo::Text)]),
    ("pause", &[]),
    ("resume", &[]),
    ("seek", &[("seconds", TypeInfo::Decimal)]),
    ("stop", &[]),
    ("volume", &[("level", TypeInfo::Integer)]),
];

/// The values a video's `hear` takes, or None when it hears no such thing.
pub fn hear(name: &str) -> Option<&'static [(&'static str, TypeInfo)]> {
    HEARS.iter().find(|(n, _)| *n == name).map(|(_, params)| *params)
}

/// How many of its values a hear needs: `play()` plays its source, `play(path)` a file.
pub fn least(name: &str) -> usize {
    if name == "play" { 0 } else { hear(name).map_or(0, <[_]>::len) }
}

/// What code reads of a video as it plays, kept by Clarity.UI in the part's
/// values and read as a custom part's are.
pub const READS: [(&str, TypeInfo); 6] = [
    ("playing", TypeInfo::Bool),
    ("paused", TypeInfo::Bool),
    // Whole seconds played, and the video's length in whole seconds.
    ("position", TypeInfo::Integer),
    ("length", TypeInfo::Integer),
    // The path of the file that plays; empty for a resource or nothing.
    ("file", TypeInfo::Text),
    // Why the last video did not play, for `on fail`; empty when it did.
    ("problem", TypeInfo::Text),
];

/// The type of what code reads of a video, or None when a video says no such thing.
pub fn read(name: &str) -> Option<TypeInfo> {
    READS.iter().find(|(n, _)| *n == name).map(|(_, ty)| ty.clone())
}

/// What a video says of `name` before it played anything, as JSON: false, 0 or empty.
pub fn read_fallback(name: &str) -> &'static str {
    match read(name) {
        Some(TypeInfo::Bool) => "false",
        Some(TypeInfo::Integer) => "0",
        _ => "\"\"",
    }
}
