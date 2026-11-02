//! A small reader of the XML Clarity writes for windows and apps (`.cxaml`):
//! elements with their attributes, in a tree. Text between elements, the
//! declaration and comments are left out; a file that is no well-formed
//! XML is refused with why.

use std::fmt;

/// An element as it is written: its name, its attributes in order, and the elements in it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct XmlElement {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<XmlElement>,
}

impl XmlElement {
    /// The value of an attribute; none when it is not written.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

/// Why a markup could not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlError(pub String);

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for XmlError {}

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }

    fn fail<T>(&self, why: &str) -> Result<T, XmlError> {
        let line = self.text[..self.at].matches('\n').count() + 1;
        Err(XmlError(format!("{why} (line {line})")))
    }

    fn skip_space(&mut self) {
        let rest = self.rest();
        self.at += rest.len() - rest.trim_start().len();
    }

    /// Skips text, the declaration, comments and doctype up to the next element's `<`.
    fn skip_between(&mut self) -> Result<(), XmlError> {
        loop {
            match self.rest().find('<') {
                None => {
                    self.at = self.text.len();
                    return Ok(());
                }
                Some(i) => self.at += i,
            }
            let rest = self.rest();
            if rest.starts_with("<?") {
                match rest.find("?>") {
                    Some(end) => self.at += end + 2,
                    None => return self.fail("a declaration that does not end"),
                }
            } else if rest.starts_with("<!--") {
                match rest.find("-->") {
                    Some(end) => self.at += end + 3,
                    None => return self.fail("a comment that does not end"),
                }
            } else if rest.starts_with("<!") {
                match rest.find('>') {
                    Some(end) => self.at += end + 1,
                    None => return self.fail("a declaration that does not end"),
                }
            } else {
                return Ok(());
            }
        }
    }

    fn name(&mut self) -> Result<String, XmlError> {
        let rest = self.rest();
        let end = rest.find(|c: char| c.is_whitespace() || c == '>' || c == '/' || c == '=').unwrap_or(rest.len());
        if end == 0 {
            return self.fail("a name is missing");
        }
        self.at += end;
        Ok(rest[..end].to_string())
    }

    fn element(&mut self) -> Result<XmlElement, XmlError> {
        // At `<`.
        self.at += 1;
        let name = self.name()?;
        let mut element = XmlElement { name, ..XmlElement::default() };
        loop {
            self.skip_space();
            let rest = self.rest();
            if rest.starts_with("/>") {
                self.at += 2;
                return Ok(element);
            }
            if rest.starts_with('>') {
                self.at += 1;
                break;
            }
            if rest.is_empty() {
                return self.fail("an element that does not end");
            }
            let key = self.name()?;
            self.skip_space();
            if !self.rest().starts_with('=') {
                return self.fail("an attribute without a value");
            }
            self.at += 1;
            self.skip_space();
            let quote = match self.rest().chars().next() {
                Some(q @ ('"' | '\'')) => q,
                _ => return self.fail("an attribute value without quotes"),
            };
            self.at += 1;
            let Some(end) = self.rest().find(quote) else {
                return self.fail("an attribute value that does not end");
            };
            let raw = &self.rest()[..end];
            self.at += end + 1;
            let value = unescape(raw).map_err(|why| XmlError(format!("{why} in {key}")))?;
            element.attributes.push((key, value));
        }
        loop {
            self.skip_between()?;
            let rest = self.rest();
            if rest.is_empty() {
                return self.fail(&format!("<{}> does not end", element.name));
            }
            if rest.starts_with("</") {
                self.at += 2;
                let closing = self.name()?;
                self.skip_space();
                if !self.rest().starts_with('>') || closing != element.name {
                    return self.fail(&format!("<{}> ends with </{closing}>", element.name));
                }
                self.at += 1;
                return Ok(element);
            }
            element.children.push(self.element()?);
        }
    }
}

/// The text of an attribute with its entities read: `&amp;`, `&quot;`, `&#10;`.
fn unescape(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';') else {
            return Err("an entity that does not end".to_string());
        };
        let entity = &rest[1..end];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok()
                } else if let Some(dec) = entity.strip_prefix('#') {
                    dec.parse().ok()
                } else {
                    None
                };
                code.and_then(char::from_u32).ok_or_else(|| format!("an unknown entity &{entity};"))?
            }
        };
        out.push(c);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The root element of a markup.
pub fn parse(text: &str) -> Result<XmlElement, XmlError> {
    let mut reader = Reader { text, at: 0 };
    reader.skip_between()?;
    if reader.rest().is_empty() {
        return Err(XmlError("the markup is empty".to_string()));
    }
    let root = reader.element()?;
    reader.skip_between()?;
    if !reader.rest().is_empty() {
        return reader.fail("more than one root element");
    }
    Ok(root)
}
