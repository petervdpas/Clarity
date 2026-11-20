//! The neutral tree every format reads into and writes from: the JSON data
//! model, with numbers kept as the text they were written in (so a decimal
//! loses no digit) and an object's names kept in their order (so matching
//! by name goes in document order on every stack). Its own small JSON
//! reader and writer, so the library needs no serde_json features that
//! would change JSON for a whole program.

/// A value of any format, as the fit sees it.
#[derive(Debug, Clone, PartialEq)]
pub enum Tree {
    Null,
    Bool(bool),
    /// A number as written: `12.50`, `1e3`, `-7`.
    Number(String),
    Text(String),
    List(Vec<Tree>),
    /// Names and values in their order.
    Object(Vec<(String, Tree)>),
}

impl Tree {
    /// The value of `name` in an object, by its exact name.
    pub fn get(&self, name: &str) -> Option<&Tree> {
        match self {
            Tree::Object(pairs) => pairs.iter().find(|(n, _)| n == name).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Tree::Text(t) => Some(t),
            _ => None,
        }
    }

    /// Sets `name` in an object: in its place when it is there, at the end when not.
    pub fn set(&mut self, name: &str, value: Tree) {
        if let Tree::Object(pairs) = self {
            match pairs.iter_mut().find(|(n, _)| n == name) {
                Some(pair) => pair.1 = value,
                None => pairs.push((name.to_string(), value)),
            }
        }
    }
}

/// Reads JSON text into a tree; none when it is no JSON (or has more after the value).
pub fn parse(text: &str) -> Option<Tree> {
    let mut reader = Reader { bytes: text.as_bytes(), at: 0, depth: 0 };
    reader.space();
    let tree = reader.value()?;
    reader.space();
    (reader.at == reader.bytes.len()).then_some(tree)
}

/// The deepest a value nests before reading stops: a guard against a stack overflow.
const MAX_DEPTH: usize = 512;

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    depth: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn word(&mut self, word: &[u8]) -> bool {
        if self.bytes[self.at..].starts_with(word) {
            self.at += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Option<Tree> {
        match self.peek()? {
            b'n' => self.word(b"null").then_some(Tree::Null),
            b't' => self.word(b"true").then_some(Tree::Bool(true)),
            b'f' => self.word(b"false").then_some(Tree::Bool(false)),
            b'"' => self.text().map(Tree::Text),
            b'[' => self.list(),
            b'{' => self.object(),
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn deeper(&mut self) -> Option<()> {
        self.depth += 1;
        (self.depth <= MAX_DEPTH).then_some(())
    }

    fn list(&mut self) -> Option<Tree> {
        self.deeper()?;
        self.at += 1;
        let mut items = Vec::new();
        self.space();
        if self.peek() == Some(b']') {
            self.at += 1;
            self.depth -= 1;
            return Some(Tree::List(items));
        }
        loop {
            self.space();
            items.push(self.value()?);
            self.space();
            match self.peek()? {
                b',' => self.at += 1,
                b']' => {
                    self.at += 1;
                    self.depth -= 1;
                    return Some(Tree::List(items));
                }
                _ => return None,
            }
        }
    }

    fn object(&mut self) -> Option<Tree> {
        self.deeper()?;
        self.at += 1;
        let mut pairs: Vec<(String, Tree)> = Vec::new();
        self.space();
        if self.peek() == Some(b'}') {
            self.at += 1;
            self.depth -= 1;
            return Some(Tree::Object(pairs));
        }
        loop {
            self.space();
            if self.peek()? != b'"' {
                return None;
            }
            let name = self.text()?;
            self.space();
            if self.peek()? != b':' {
                return None;
            }
            self.at += 1;
            self.space();
            let value = self.value()?;
            // A name given twice: the last one counts, in the first one's place, as JSON readers do.
            match pairs.iter_mut().find(|(n, _)| *n == name) {
                Some(pair) => pair.1 = value,
                None => pairs.push((name, value)),
            }
            self.space();
            match self.peek()? {
                b',' => self.at += 1,
                b'}' => {
                    self.at += 1;
                    self.depth -= 1;
                    return Some(Tree::Object(pairs));
                }
                _ => return None,
            }
        }
    }

    fn number(&mut self) -> Option<Tree> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek()? {
            b'0' => self.at += 1,
            b'1'..=b'9' => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.at += 1;
                }
            }
            _ => return None,
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            let digits = self.at;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
            }
            if self.at == digits {
                return None;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            let digits = self.at;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
            }
            if self.at == digits {
                return None;
            }
        }
        std::str::from_utf8(&self.bytes[start..self.at]).ok().map(|n| Tree::Number(n.to_string()))
    }

    fn text(&mut self) -> Option<String> {
        self.at += 1;
        let mut out = Vec::new();
        loop {
            let b = self.peek()?;
            self.at += 1;
            match b {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = self.peek()?;
                    self.at += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let c = if (0xD800..0xDC00).contains(&first) {
                                if !self.word(b"\\u") {
                                    return None;
                                }
                                let second = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&second) {
                                    return None;
                                }
                                char::from_u32(0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00))?
                            } else {
                                char::from_u32(first)?
                            };
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return None,
                    }
                }
                0x00..=0x1f => return None,
                _ => out.push(b),
            }
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits = self.bytes.get(self.at..self.at + 4)?;
        self.at += 4;
        u32::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()
    }
}

/// The tree as compact JSON text.
pub fn write(tree: &Tree) -> String {
    let mut out = String::new();
    write_into(tree, &mut out);
    out
}

fn write_into(tree: &Tree, out: &mut String) {
    match tree {
        Tree::Null => out.push_str("null"),
        Tree::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Tree::Number(n) => out.push_str(n),
        Tree::Text(t) => quoted(t, out),
        Tree::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_into(item, out);
            }
            out.push(']');
        }
        Tree::Object(pairs) => {
            out.push('{');
            for (i, (name, value)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                quoted(name, out);
                out.push(':');
                write_into(value, out);
            }
            out.push('}');
        }
    }
}

/// A text as a JSON string: quotes, backslashes and control characters escaped, the rest as it is.
fn quoted(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_stay_as_written_and_names_in_their_order() {
        let tree = parse(r#"{"b": 1.50, "a": 12345678901234567890.123, "c": 1e3}"#).unwrap();
        assert_eq!(write(&tree), r#"{"b":1.50,"a":12345678901234567890.123,"c":1e3}"#);
    }

    #[test]
    fn texts_read_their_escapes_and_write_them_back() {
        let tree = parse(r#""a\"b\\c\ndé😀/""#).unwrap();
        assert_eq!(tree, Tree::Text("a\"b\\c\nd\u{e9}\u{1F600}/".into()));
        assert_eq!(write(&tree), "\"a\\\"b\\\\c\\nd\u{e9}\u{1F600}/\"");
    }

    #[test]
    fn what_is_no_json_is_none() {
        for bad in ["", "{", "[1,]", "{\"a\" 1}", "01", "1.", "tru", "{} x", "\"a\nb\"", "-", "1e"] {
            assert!(parse(bad).is_none(), "{bad:?}");
        }
        assert!(parse(&"[".repeat(MAX_DEPTH + 1)).is_none(), "too deep is refused, not a crash");
    }

    #[test]
    fn a_name_given_twice_keeps_the_last_value_in_the_first_place() {
        assert_eq!(write(&parse(r#"{"a":1,"b":2,"a":3}"#).unwrap()), r#"{"a":3,"b":2}"#);
    }
}
