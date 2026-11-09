//! A small XML reader for `.cxaml` (DESIGN 7.8): elements, attributes and
//! text, with where each element and attribute is in the file, so a part read
//! from markup has its span as a part read from Clarity has. Comments, the
//! XML declaration and whitespace between elements are passed over.

use crate::diagnostic::Diagnostic;
use crate::span::Span;

/// An attribute: its name, its value with the entities read, and where its value is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
    pub span: Span,
}

/// An element: its name, its attributes in the order written, its child
/// elements, and where it is, from its `<` to the end of its closing tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Element>,
    pub span: Span,
}

impl Element {
    /// The value of the attribute `name`, if it has one.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|a| a.name == name).map(|a| a.value.as_str())
    }

    /// The attribute `name`, if it has one.
    pub fn attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name == name)
    }
}

/// The root element of `text`, or what is wrong with it, where.
pub fn read(text: &str) -> Result<Element, Diagnostic> {
    let mut reader = Reader { text, at: 0 };
    reader.skip_misc()?;
    if reader.at >= text.len() {
        return Err(reader.error("the file holds no element", reader.at));
    }
    let root = reader.element()?;
    reader.skip_misc()?;
    if reader.at < text.len() {
        return Err(reader.error("only one root element is allowed; this is after it", reader.at));
    }
    Ok(root)
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn rest(&self) -> &str {
        &self.text[self.at..]
    }

    fn span(&self, start: usize, end: usize) -> Span {
        let before = &self.text[..start];
        let line = before.matches('\n').count() as u32 + 1;
        let col = (start - before.rfind('\n').map_or(0, |n| n + 1)) as u32 + 1;
        Span { start: start as u32, end: end as u32, line, col }
    }

    fn error(&self, message: &str, at: usize) -> Diagnostic {
        Diagnostic::error(message.to_string(), self.span(at, (at + 1).min(self.text.len())))
    }

    fn skip_space(&mut self) {
        let trimmed = self.rest().trim_start();
        self.at = self.text.len() - trimmed.len();
    }

    /// Whitespace, comments and the declaration between elements.
    fn skip_misc(&mut self) -> Result<(), Diagnostic> {
        loop {
            self.skip_space();
            if self.rest().starts_with("<!--") {
                let start = self.at;
                let end = self.rest().find("-->").ok_or_else(|| self.error("a comment is not closed with -->", start))?;
                self.at += end + 3;
            } else if self.rest().starts_with("<?") {
                let start = self.at;
                let end = self.rest().find("?>").ok_or_else(|| self.error("the declaration is not closed with ?>", start))?;
                self.at += end + 2;
            } else {
                return Ok(());
            }
        }
    }

    fn name(&mut self) -> Result<String, Diagnostic> {
        let len = self.rest().find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))).unwrap_or(self.rest().len());
        if len == 0 {
            return Err(self.error("a name is expected here", self.at));
        }
        let name = self.rest()[..len].to_string();
        self.at += len;
        Ok(name)
    }

    fn element(&mut self) -> Result<Element, Diagnostic> {
        let start = self.at;
        if !self.rest().starts_with('<') {
            return Err(self.error("an element is expected here, starting with <", start));
        }
        self.at += 1;
        let name = self.name()?;
        let mut attributes: Vec<Attribute> = Vec::new();
        loop {
            self.skip_space();
            if self.rest().starts_with("/>") {
                self.at += 2;
                return Ok(Element { name, attributes, children: Vec::new(), span: self.span(start, self.at) });
            }
            if self.rest().starts_with('>') {
                self.at += 1;
                break;
            }
            let at = self.at;
            let attribute = self.name()?;
            self.skip_space();
            if !self.rest().starts_with('=') {
                return Err(self.error(&format!("'{attribute}' needs = and a value in quotes"), self.at));
            }
            self.at += 1;
            self.skip_space();
            let quote = self.rest().chars().next().filter(|c| *c == '"' || *c == '\'').ok_or_else(|| self.error("a value in quotes is expected here", self.at))?;
            self.at += 1;
            let value_start = self.at;
            let end = self.rest().find(quote).ok_or_else(|| self.error("the value's quote is not closed", value_start))?;
            let raw = &self.text[value_start..value_start + end];
            self.at += end + 1;
            if attributes.iter().any(|a| a.name == attribute) {
                return Err(self.error(&format!("'{attribute}' is written twice on <{name}>"), at));
            }
            let value = unescaped(raw).map_err(|m| self.error(&m, value_start))?;
            attributes.push(Attribute { name: attribute, value, span: self.span(value_start, value_start + end) });
        }
        let mut children = Vec::new();
        loop {
            self.skip_misc()?;
            if self.rest().starts_with("</") {
                self.at += 2;
                let close = self.at;
                let closing = self.name()?;
                if closing != name {
                    return Err(self.error(&format!("<{name}> is closed by </{closing}>"), close));
                }
                self.skip_space();
                if !self.rest().starts_with('>') {
                    return Err(self.error("the closing tag needs >", self.at));
                }
                self.at += 1;
                return Ok(Element { name, attributes, children, span: self.span(start, self.at) });
            }
            if self.at >= self.text.len() {
                return Err(self.error(&format!("<{name}> is not closed"), start));
            }
            if !self.rest().starts_with('<') {
                return Err(self.error(&format!("text is not allowed in <{name}>; values go in attributes"), self.at));
            }
            children.push(self.element()?);
        }
    }
}

/// An attribute's text with its entities read: `&amp;`, `&lt;`, `&gt;`,
/// `&quot;`, `&apos;` and numbered ones such as `&#10;`.
fn unescaped(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        let semi = after.find(';').ok_or_else(|| "an & starts an entity that is not closed with ;".to_string())?;
        let entity = &after[..semi];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let number = entity.strip_prefix("#x").map(|h| u32::from_str_radix(h, 16)).or_else(|| entity.strip_prefix('#').map(str::parse::<u32>));
                number.and_then(Result::ok).and_then(char::from_u32).ok_or_else(|| format!("&{entity}; is not an entity XML knows"))?
            }
        };
        out.push(c);
        rest = &after[semi + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Text as an XML attribute holds it: the characters XML gives a meaning, and
/// line ends and tabs, which an attribute would otherwise read as spaces.
pub fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_attributes_and_where_they_are() {
        let text = "<?xml version=\"1.0\"?>\n<!-- a note -->\n<Window name=\"Main\">\n  <Button caption=\"a &amp; b&#10;c\" />\n</Window>\n";
        let root = read(text).unwrap();
        assert_eq!(root.name, "Window");
        assert_eq!(root.get("name"), Some("Main"));
        let button = &root.children[0];
        assert_eq!(button.get("caption"), Some("a & b\nc"));
        assert_eq!((button.span.line, button.span.col), (4, 3));
        assert_eq!(&text[button.span.start as usize..button.span.end as usize], "<Button caption=\"a &amp; b&#10;c\" />");
        assert_eq!(escaped("a & b\nc"), "a &amp; b&#10;c");
    }

    #[test]
    fn what_is_wrong_is_said_where() {
        assert!(read("<A><B></A>").unwrap_err().message.contains("<B> is closed by </A>"));
        assert!(read("<A x=\"1\" x=\"2\" />").unwrap_err().message.contains("'x' is written twice"));
        assert!(read("<A>text</A>").unwrap_err().message.contains("text is not allowed"));
        assert!(read("<A /><B />").unwrap_err().message.contains("only one root element"));
    }
}
