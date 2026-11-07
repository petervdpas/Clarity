//! Text functions in code (conformance/texts.json): characters counted once
//! however many bytes they take, positions from 1 as lists count.

/// `words(text)`: the words of a text, apart by white space.
pub fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

/// `split(text, separator)`: the pieces between each separator, empty ones
/// kept; an empty separator gives the characters.
pub fn split(text: &str, separator: &str) -> Vec<String> {
    if separator.is_empty() {
        return text.chars().map(String::from).collect();
    }
    text.split(separator).map(str::to_string).collect()
}

/// `trim(text)`: the text without white space at its ends.
pub fn trim(text: &str) -> String {
    text.trim().to_string()
}

/// `lower(text)`: the text in lower case, each letter to one letter.
pub fn lower(text: &str) -> String {
    text.chars().map(|c| one(c.to_lowercase(), c)).collect()
}

/// `upper(text)`: the text in upper case, each letter to one letter (ß stays ß, as in every runtime).
pub fn upper(text: &str) -> String {
    text.chars().map(|c| one(c.to_uppercase(), c)).collect()
}

/// The one letter a case change gives, else the letter as it was.
fn one(mut changed: impl Iterator<Item = char>, letter: char) -> char {
    match (changed.next(), changed.next()) {
        (Some(single), None) => single,
        _ => letter,
    }
}

/// `replace(text, find, with)`: every `find` replaced; an empty `find` changes nothing.
pub fn replace(text: &str, find: &str, with: &str) -> String {
    if find.is_empty() { text.to_string() } else { text.replace(find, with) }
}

/// `startsWith(text, prefix)`.
pub fn starts_with(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

/// `endsWith(text, suffix)`.
pub fn ends_with(text: &str, suffix: &str) -> bool {
    text.ends_with(suffix)
}

/// `slice(text, from, count)`: `count` characters from position `from`
/// (from 1), clamped to the text.
pub fn slice(text: &str, from: i64, count: i64) -> String {
    let length = text.chars().count() as i64;
    let start = from.max(1);
    let end = from.saturating_add(count.max(0)).min(length + 1);
    if end <= start {
        return String::new();
    }
    text.chars().skip((start - 1) as usize).take((end - start) as usize).collect()
}

/// `indexOf(text, find)`: the position of `find`'s first character (from 1), 0 when it is not there.
pub fn index_of(text: &str, find: &str) -> i64 {
    text.find(find).map_or(0, |at| text[..at].chars().count() as i64 + 1)
}
