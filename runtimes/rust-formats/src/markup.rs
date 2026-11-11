//! Markup: elements with attributes, text, CDATA, comments, processing
//! instructions and a doctype, read into element nodes of the document
//! model (doc.rs) and written from them, keeping everything in its order.
//! The syntax of XML documents and of an element node written on its own
//! (a file source's `#node` column). Elements, attributes, the five named
//! entities and character references; a DOCTYPE is kept as written but its
//! internal entities are not expanded.

use crate::doc::{Content, Node};

/// Why a text is no markup.
pub type Why = String;

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }

    fn starts(&self, s: &str) -> bool {
        self.rest().starts_with(s)
    }

    /// The text up to `end`, moving past `end`.
    fn until(&mut self, end: &str) -> Result<&'a str, Why> {
        let found = self.rest().find(end).ok_or_else(|| format!("'{end}' is missing"))?;
        let taken = &self.rest()[..found];
        self.at += found + end.len();
        Ok(taken)
    }

    fn skip_space(&mut self) {
        let trimmed = self.rest().trim_start();
        self.at = self.text.len() - trimmed.len();
    }

    fn name(&mut self) -> Result<&'a str, Why> {
        let rest = self.rest();
        let end = rest.find(|c: char| c.is_whitespace() || matches!(c, '/' | '>' | '=' | '<' | '"' | '\'')).unwrap_or(rest.len());
        if end == 0 {
            return Err("a name is missing".into());
        }
        self.at += end;
        Ok(&rest[..end])
    }

    /// What is not an element or text: a comment, CDATA, a processing instruction or a doctype.
    fn other(&mut self) -> Result<Option<Content>, Why> {
        if self.starts("<!--") {
            self.at += 4;
            return Ok(Some(Content::Comment(self.until("-->")?.to_string())));
        }
        if self.starts("<![CDATA[") {
            self.at += 9;
            return Ok(Some(Content::CData(self.until("]]>")?.to_string())));
        }
        if self.starts("<?") {
            self.at += 2;
            return Ok(Some(Content::Pi(self.until("?>")?.to_string())));
        }
        if self.starts("<!DOCTYPE") {
            self.at += 9;
            // An internal subset in brackets may hold '>'.
            let rest = self.rest();
            let mut depth = 0;
            for (i, c) in rest.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    '>' if depth == 0 => {
                        self.at += i + 1;
                        return Ok(Some(Content::DocType(rest[..i].to_string())));
                    }
                    _ => {}
                }
            }
            return Err("the doctype is not closed".into());
        }
        Ok(None)
    }

    fn element(&mut self) -> Result<Node, Why> {
        self.at += 1; // '<'
        let name = self.name()?.to_string();
        let mut element = Node::element(&name);
        loop {
            self.skip_space();
            if self.starts("/>") {
                self.at += 2;
                return Ok(element);
            }
            if self.starts(">") {
                self.at += 1;
                break;
            }
            let attribute = self.name()?.to_string();
            self.skip_space();
            if !self.starts("=") {
                return Err(format!("attribute '{attribute}' of <{name}> has no value"));
            }
            self.at += 1;
            self.skip_space();
            let quote = self.rest().chars().next().filter(|c| *c == '"' || *c == '\'').ok_or_else(|| format!("the value of attribute '{attribute}' is not in quotes"))?;
            self.at += 1;
            let value = self.until(&quote.to_string())?;
            element.attributes.push((attribute, unescape(value)?));
        }
        loop {
            if self.rest().is_empty() {
                return Err(format!("<{name}> is not closed"));
            }
            if self.starts("</") {
                self.at += 2;
                let closing = self.name()?;
                if closing != name {
                    return Err(format!("<{name}> is closed by </{closing}>"));
                }
                self.skip_space();
                if !self.starts(">") {
                    return Err(format!("</{closing}> is not closed"));
                }
                self.at += 1;
                return Ok(element);
            }
            if let Some(content) = self.other()? {
                element.children.push(content);
                continue;
            }
            if self.starts("<") {
                let child = self.element()?;
                element.push(child);
                continue;
            }
            let end = self.rest().find('<').unwrap_or(self.rest().len());
            let raw = &self.rest()[..end];
            self.at += end;
            element.children.push(Content::Text(unescape(raw)?));
        }
    }
}

/// The five named entities and character references replaced.
pub fn unescape(raw: &str) -> Result<String, Why> {
    if !raw.contains('&') {
        return Ok(raw.to_string());
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let end = after.find(';').ok_or("an '&' without its ';'")?;
        let name = &after[..end];
        let c = match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ if name.starts_with("#x") => u32::from_str_radix(&name[2..], 16).ok().and_then(char::from_u32).ok_or_else(|| format!("'&{name};' is no character"))?,
            _ if name.starts_with('#') => name[1..].parse::<u32>().ok().and_then(char::from_u32).ok_or_else(|| format!("'&{name};' is no character"))?,
            _ => return Err(format!("the entity '&{name};' is not known")),
        };
        out.push(c);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Text as markup text: `&`, `<` and `>` escaped.
pub fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Text as an attribute value in double quotes.
pub fn escape_attribute(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

/// A document: what comes before the root element, the root, what follows
/// it. An XML declaration is read and dropped (its encoding was honoured
/// when the bytes became text).
pub fn parse(text: &str) -> Result<(Vec<Content>, Node, Vec<Content>), Why> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut reader = Reader { text, at: 0 };
    let mut before = Vec::new();
    reader.skip_space();
    if reader.starts("<?xml") && reader.rest()[5..].starts_with(|c: char| c.is_whitespace() || c == '?') {
        reader.until("?>")?;
    }
    loop {
        reader.skip_space();
        if let Some(content) = reader.other()? {
            before.push(content);
            continue;
        }
        if reader.starts("<") {
            break;
        }
        return Err(if reader.rest().is_empty() { "there is no root element".into() } else { "text before the root element".into() });
    }
    let root = reader.element()?;
    let mut after = Vec::new();
    loop {
        reader.skip_space();
        if reader.rest().is_empty() {
            break;
        }
        match reader.other()? {
            Some(content) => after.push(content),
            None => return Err("more after the root element".into()),
        }
    }
    Ok((before, root, after))
}

/// One element written on its own, read back.
pub fn parse_element(text: &str) -> Result<Node, Why> {
    parse(text).map(|(_, root, _)| root)
}

/// What a node holds, written.
pub fn write_content(content: &Content, out: &mut String, indent: Option<usize>) {
    match content {
        Content::Node(e) => write_element(e, out, indent),
        Content::Text(t) => out.push_str(&escape_text(t)),
        Content::CData(t) => {
            out.push_str("<![CDATA[");
            out.push_str(t);
            out.push_str("]]>");
        }
        Content::Comment(t) => {
            out.push_str("<!--");
            out.push_str(t);
            out.push_str("-->");
        }
        Content::Pi(t) => {
            out.push_str("<?");
            out.push_str(t);
            out.push_str("?>");
        }
        Content::DocType(t) => {
            out.push_str("<!DOCTYPE");
            out.push_str(t);
            out.push('>');
        }
    }
}

/// Writes an element; with `indent`, at that depth, one element per line,
/// where it holds only elements (text it holds is written as it is).
pub fn write_element(e: &Node, out: &mut String, indent: Option<usize>) {
    out.push('<');
    out.push_str(&e.name);
    for (name, value) in &e.attributes {
        out.push(' ');
        out.push_str(name);
        out.push_str("=\"");
        out.push_str(&escape_attribute(value));
        out.push('"');
    }
    if e.children.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    let only_elements = e.children.iter().all(|c| matches!(c, Content::Node(_) | Content::Comment(_)));
    match indent {
        Some(depth) if only_elements => {
            for child in &e.children {
                out.push('\n');
                out.push_str(&"  ".repeat(depth + 1));
                write_content(child, out, Some(depth + 1));
            }
            out.push('\n');
            out.push_str(&"  ".repeat(depth));
        }
        _ => {
            for child in &e.children {
                write_content(child, out, indent.map(|d| d + 1));
            }
        }
    }
    out.push_str("</");
    out.push_str(&e.name);
    out.push('>');
}

/// An element as its text.
pub fn element_text(e: &Node) -> String {
    let mut out = String::new();
    write_element(e, &mut out, None);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_reads_and_writes_again_whole() {
        let text = "<?xml version=\"1.0\"?>\n<!-- prices -->\n<shop xmlns:o=\"urn:o\"><o:order number=\"A&amp;1\">Ada &lt;3<![CDATA[<raw>]]><!-- x --><line/></o:order><?keep me?></shop>";
        let (before, root, _) = parse(text).unwrap();
        assert_eq!(before, vec![Content::Comment(" prices ".into())]);
        let order = root.nodes().next().unwrap();
        assert_eq!(crate::doc::local(&order.name), "order");
        assert_eq!(order.attribute("number"), Some("A&1"));
        assert_eq!(order.text(), "Ada <3<raw>");
        assert_eq!(element_text(&root), "<shop xmlns:o=\"urn:o\"><o:order number=\"A&amp;1\">Ada &lt;3<![CDATA[<raw>]]><!-- x --><line/></o:order><?keep me?></shop>");
    }

    #[test]
    fn what_is_no_markup_says_why() {
        assert_eq!(parse("<a><b></a>").unwrap_err(), "<b> is closed by </a>");
        assert_eq!(parse("<a x=1/>").unwrap_err(), "the value of attribute 'x' is not in quotes");
        assert_eq!(parse("<a>&nbsp;</a>").unwrap_err(), "the entity '&nbsp;' is not known");
        assert_eq!(parse("").unwrap_err(), "there is no root element");
        assert_eq!(unescape("&#65;&#x42;").unwrap(), "AB");
    }

    #[test]
    fn indent_writes_an_element_a_line() {
        let root = parse_element("<a><b>1</b><c><d/></c></a>").unwrap();
        let mut out = String::new();
        write_element(&root, &mut out, Some(0));
        assert_eq!(out, "<a>\n  <b>1</b>\n  <c>\n    <d/>\n  </c>\n</a>");
    }
}
