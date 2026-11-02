//! How a sound reads a melody (DESIGN 5.22), alike on every stack
//! (conformance/melody.json), and how tones are made into WAV data, which
//! every sound output plays as it plays a file.

/// Samples a second of the WAV data tones are made into.
pub const SAMPLE_RATE: u32 = 44100;

/// A note of a melody: its frequency (none for a rest) and how long it lasts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub hertz: Option<f64>,
    pub seconds: f64,
}

/// The notes of `notes` at `tempo` beats a minute, and the words that are not notes.
pub fn read(notes: &str, tempo: i64) -> (Vec<Note>, Vec<String>) {
    let beat = 60.0 / tempo.max(1) as f64;
    let mut plays = Vec::new();
    let mut skips = Vec::new();
    for word in notes.split_whitespace() {
        match parse(word) {
            Some((hertz, beats)) => plays.push(Note { hertz, seconds: beats * beat }),
            None => skips.push(word.to_string()),
        }
    }
    (plays, skips)
}

/// A word as a note or a rest and its beats; none when it is neither.
fn parse(word: &str) -> Option<(Option<f64>, f64)> {
    let mut pitch = word;
    let mut beats = 1.0;
    if let Some(at) = word.find(['*', '/']) {
        let count = &word[at + 1..];
        if count.is_empty() || count.starts_with('0') || !count.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: u32 = count.parse().ok()?;
        beats = if word.as_bytes()[at] == b'*' { n as f64 } else { 1.0 / n as f64 };
        pitch = &word[..at];
    }
    if pitch == "-" {
        return Some((None, beats));
    }
    let mut chars = pitch.chars();
    let semitone = match chars.next()? {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let mut rest = chars.as_str();
    let mut shift = 0;
    if let Some(after) = rest.strip_prefix('#') {
        shift = 1;
        rest = after;
    } else if let Some(after) = rest.strip_prefix('b') {
        shift = -1;
        rest = after;
    }
    let octave = match rest.as_bytes() {
        [] => 4,
        [d @ b'0'..=b'8'] => (d - b'0') as i32,
        _ => return None,
    };
    let midi = 12 * (octave + 1) + semitone + shift;
    Some((Some(440.0 * 2f64.powf((midi - 69) as f64 / 12.0)), beats))
}

/// The notes as 16-bit mono WAV data, each tone swelling and fading so it starts and ends without a click.
pub fn wav(notes: &[Note]) -> Vec<u8> {
    let rate = SAMPLE_RATE as f64;
    let mut samples: Vec<i16> = Vec::new();
    for note in notes {
        let count = (note.seconds * rate) as usize;
        let edge = ((0.01 * rate) as usize).min(count / 4);
        for i in 0..count {
            let Some(hertz) = note.hertz else {
                samples.push(0);
                continue;
            };
            let level = (i.min(count - 1 - i) as f64 / edge.max(1) as f64).min(1.0);
            samples.push(((2.0 * std::f64::consts::PI * hertz * i as f64 / rate).sin() * level * 0.8 * i16::MAX as f64) as i16);
        }
    }
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}
