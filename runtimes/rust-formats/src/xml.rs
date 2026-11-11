//! XML (docs/FORMATS-PLAN.md, "XML"), syntax only: bytes become a document
//! of element nodes (markup.rs) and a document becomes bytes again. Bytes
//! become text by a byte order mark (UTF-8, UTF-16), else the declaration's
//! encoding (utf-8, windows-1252, iso-8859-1), else UTF-8. A document read
//! is written back in its own encoding and layout; a new one starts with
//! the declaration naming `encoding` (UTF-8 without it), laid out by
//! `indent`, with `namespace` as the root's default namespace. What the
//! records are and where fields are is the engine's (shape.rs, build.rs,
//! table.rs).

use crate::doc::{Bom, Content, Document, Form, Kept, Node};
use crate::encoding::{self, Encoding};
use crate::format::Format;
use crate::markup;
use crate::tree::Tree;
use crate::{FormatInfo, Nodes, OptionInfo, OptionKind, Problem, ProblemKind, Tables};

pub struct Xml;

fn not(text: impl Into<String>) -> Problem {
    Problem::new(ProblemKind::NotFitting, text)
}

fn option_text<'a>(options: &'a [(String, Tree)], name: &str) -> Option<&'a str> {
    options.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.as_text())
}

/// The encoding an XML declaration names, as written; none when it names
/// none. Only the declaration is read, up to its `?>`, so bytes of another
/// encoding after it do not hide it.
pub fn declared_encoding(bytes: &[u8]) -> Option<String> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let end = bytes.windows(2).take(200).position(|w| w == b"?>")?;
    let decl = std::str::from_utf8(&bytes[..end]).ok()?.strip_prefix("<?xml")?;
    let at = decl.find("encoding")?;
    let after = decl[at + 8..].trim_start().strip_prefix('=')?.trim_start();
    let quote = after.chars().next()?;
    Some(after[1..after[1..].find(quote)? + 1].to_string())
}

/// The declared encoding of text already decoded (UTF-16 read from its bytes).
fn declared_in(text: &str) -> Option<String> {
    let decl = text.strip_prefix('\u{feff}').unwrap_or(text).strip_prefix("<?xml")?;
    let decl = &decl[..decl.find("?>")?];
    let after = decl[decl.find("encoding")? + 8..].trim_start().strip_prefix('=')?.trim_start();
    let quote = after.chars().next()?;
    Some(after[1..after[1..].find(quote)? + 1].to_string())
}

/// The bytes as text, and the byte order mark they had.
pub fn decode(bytes: &[u8]) -> Result<(String, Bom), Problem> {
    let no_xml = || not("the text is no XML");
    let utf16 = |rest: &[u8], big: bool| {
        if rest.len() % 2 != 0 {
            return None;
        }
        let units: Vec<u16> = rest.chunks(2).map(|c| if big { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) }).collect();
        String::from_utf16(&units).ok()
    };
    if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE") {
        return utf16(rest, false).map(|t| (t, Bom::Utf16 { big: false })).ok_or_else(no_xml);
    }
    if let Some(rest) = bytes.strip_prefix(b"\xFE\xFF") {
        return utf16(rest, true).map(|t| (t, Bom::Utf16 { big: true })).ok_or_else(no_xml);
    }
    let bom = if bytes.starts_with(b"\xEF\xBB\xBF") { Bom::Utf8 } else { Bom::None };
    let named = declared_encoding(bytes).map(|e| e.to_ascii_lowercase());
    let encoding = match named.as_deref() {
        // Without a byte order mark, UTF-16 named in readable bytes was converted and not renamed.
        None | Some("us-ascii" | "ascii" | "utf-16" | "utf16") => Encoding::Utf8,
        Some(name) => Encoding::named(name).ok_or_else(|| not(format!("xml reads utf-8, utf-16, windows-1252 or iso-8859-1, not \"{name}\"")))?,
    };
    encoding::decode(bytes, encoding).map(|t| (t, bom)).ok_or_else(no_xml)
}

/// Whether the nodes around the root stood on their own lines: after the
/// first one, when there is one, else whether the text ended its line.
fn breaks(text: &str) -> bool {
    let start = text.trim_start_matches('\u{feff}').trim_start();
    let first_end = if start.starts_with("<!--") { start.find("-->").map(|i| i + 3) } else if start.starts_with("<?") || start.starts_with("<!") { start.find('>').map(|i| i + 1) } else { None };
    match first_end {
        Some(end) => start[end..].starts_with(['\r', '\n']),
        None => text.ends_with('\n'),
    }
}

/// Whether a name can be an element's: not empty, no space and none of the characters XML marks up with.
fn is_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|c| c.is_whitespace() || matches!(c, '<' | '>' | '&' | '"' | '\'' | '=' | '/' | '?' | '!'))
}

fn bad_names(node: &Node) -> Option<&str> {
    if node.form == Form::Element && !node.name.is_empty() && !is_name(&node.name) {
        return Some(&node.name);
    }
    node.nodes().find_map(bad_names)
}

/// The text in the encoding named; a character it cannot hold as a character reference.
fn encode(text: &str, name: &str, bom: Bom) -> Result<Vec<u8>, Problem> {
    let mut out = Vec::new();
    if let Bom::Utf16 { big } = bom {
        out.extend_from_slice(if big { &[0xFE, 0xFF] } else { &[0xFF, 0xFE] });
        for unit in text.encode_utf16() {
            out.extend_from_slice(&if big { unit.to_be_bytes() } else { unit.to_le_bytes() });
        }
        return Ok(out);
    }
    if bom == Bom::Utf8 {
        out.extend_from_slice(b"\xEF\xBB\xBF");
    }
    let lower = name.to_ascii_lowercase();
    // UTF-16 named without a byte order mark was read as UTF-8, and is written so.
    let encoding = match lower.as_str() {
        "us-ascii" | "ascii" | "utf-16" | "utf16" => Encoding::Utf8,
        other => Encoding::named(other).ok_or_else(|| Problem::new(ProblemKind::BadOptions, format!("xml writes utf-8, utf-16, windows-1252 or iso-8859-1, not \"{lower}\"")))?,
    };
    out.extend(encoding::encode_or(text, encoding, |c| format!("&#{};", c as u32)));
    Ok(out)
}

impl Format for Xml {
    fn info(&self) -> FormatInfo {
        FormatInfo {
            name: "xml",
            title: "XML",
            text: true,
            nests: true,
            nodes: Nodes::Elements,
            attributes: true,
            tables: Tables::One,
            path_separator: Some('/'),
            schema_kinds: &["xsd"],
            options: vec![
                OptionInfo { name: "at", kind: OptionKind::Text, label: "formatOptionAt", reads: true, writes: true, required_writing: Some("the elements around the records and the record's own, like at \"orders/order\"") },
                OptionInfo { name: "namespace", kind: OptionKind::Text, label: "formatOptionNamespace", reads: false, writes: true, required_writing: None },
                OptionInfo { name: "indent", kind: OptionKind::Bool, label: "formatOptionIndent", reads: false, writes: true, required_writing: None },
                OptionInfo { name: "encoding", kind: OptionKind::Text, label: "formatOptionEncoding", reads: false, writes: true, required_writing: None },
            ],
        }
    }

    fn parse(&self, input: &[u8], _options: &[(String, Tree)], _names: &[String]) -> Result<Document, Problem> {
        let (text, bom) = decode(input)?;
        let (before, root, after) = markup::parse(&text).map_err(|_| not("the text is no XML"))?;
        let mut top = Node::element("");
        top.children = before.into_iter().chain(std::iter::once(Content::Node(root))).chain(after).collect();
        let kept = Kept { encoding: declared_in(&text), bom, breaks: breaks(&text), final_break: text.ends_with('\n'), text: None };
        Ok(Document { top, columns: Vec::new(), kept: Some(kept) })
    }

    fn write(&self, doc: &Document, options: &[(String, Tree)]) -> Result<Vec<u8>, Problem> {
        if let Some(name) = bad_names(&doc.top) {
            return Err(Problem::new(ProblemKind::BadOptions, format!("'{name}' is no XML name")));
        }
        let (encoding, bom, sep, indent, final_break) = match &doc.kept {
            // A document read: its own encoding and layout.
            Some(kept) => (kept.encoding.clone().unwrap_or_else(|| "UTF-8".into()), kept.bom, if kept.breaks { "\n" } else { "" }, false, kept.final_break),
            None => {
                let encoding = match option_text(options, "encoding").map(str::to_ascii_lowercase) {
                    None => "UTF-8",
                    Some(name) => match Encoding::named(&name) {
                        Some(Encoding::Utf8) => "UTF-8",
                        Some(Encoding::Windows1252) => "windows-1252",
                        Some(Encoding::Latin1) => "ISO-8859-1",
                        None => return Err(Problem::new(ProblemKind::BadOptions, format!("xml writes utf-8, windows-1252 or iso-8859-1, not \"{name}\""))),
                    },
                };
                let indent = options.iter().any(|(n, v)| n == "indent" && *v == Tree::Bool(true));
                (encoding.to_string(), Bom::None, if indent { "\n" } else { "" }, indent, indent)
            }
        };
        let mut top = doc.top.clone();
        if doc.kept.is_none() {
            if let (Some(namespace), Some(root)) = (option_text(options, "namespace"), top.nodes_mut().next()) {
                root.attributes.insert(0, ("xmlns".into(), namespace.to_string()));
            }
        }
        let mut out = format!("<?xml version=\"1.0\" encoding=\"{encoding}\"?>");
        for content in &top.children {
            if matches!(content, Content::Text(_)) {
                continue;
            }
            out.push_str(sep);
            let depth = (indent && matches!(content, Content::Node(_))).then_some(0);
            markup::write_content(content, &mut out, depth);
        }
        if final_break {
            out.push('\n');
        }
        encode(&out, &encoding, bom)
    }

    fn schema(&self, text: &str) -> Result<Vec<crate::describe::Described>, Problem> {
        crate::xsd::schema(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_encoding_is_read_from_the_declaration() {
        assert_eq!(declared_encoding(b"<?xml version='1.0' encoding='Windows-1252'?><a/>").as_deref(), Some("Windows-1252"));
        assert_eq!(declared_encoding(b"<a/>"), None);
        assert_eq!(declared_encoding(b"<?xml version='1.0' encoding='iso-8859-1'?><a>\xE9</a>").as_deref(), Some("iso-8859-1"), "bytes after the declaration do not hide it");
        assert_eq!(decode(b"<?xml version='1.0' encoding='windows-1252'?><a>\x80</a>").unwrap().0, "<?xml version='1.0' encoding='windows-1252'?><a>\u{20AC}</a>");
        assert_eq!(decode(b"<?xml version='1.0' encoding='ebcdic'?><a/>").unwrap_err().text, "xml reads utf-8, utf-16, windows-1252 or iso-8859-1, not \"ebcdic\"");
    }
}
