//! YAML, syntax only (docs/FORMATS-PLAN.md, "Step 5: YAML"): YAML 1.2's
//! core schema read into the document model (doc.rs) as JSON's values are,
//! and written from it in block style.
//!
//! Reading: block mappings and sequences, flow `{}` and `[]`, plain,
//! single- and double-quoted scalars (escapes, folding), block scalars `|`
//! and `>` with chomping (`-`, `+`) and an indentation digit, comments,
//! anchors and aliases (an alias is a copy of what its anchor names), the
//! first document of a stream (`%` directives, `---`, `...`). Tags are left
//! aside, as are complex keys (`?`), which are refused. A plain scalar is a
//! number (an integer or a float; one JSON could not write, `+1`, `.5`,
//! `0x1F`, `0o17`, is kept as the JSON number it is), `true` or `false`,
//! null (`null`, `~` or nothing), else text; `.inf` and `.nan` stay text.
//! A key is the text it is written as.
//!
//! Writing: block style, two spaces, keys in order; a text is quoted
//! (double quotes, with escapes) where it would read back as something
//! else, a text of several lines is a `|` block. A document that was read
//! keeps its text (`Kept::text`), and is written back as it was outside
//! what changed: an unchanged entry or item as written (its comments, its
//! quoting, its spacing), a changed one rewritten where it changed, its
//! unchanged entries kept as written; what is around the document (the
//! comments before and after, `---`, `...`, the documents after it) kept.

use crate::doc::{Bom, Document, Kept, Node};
use crate::format::Format;
use crate::tree::Tree;
use crate::{FormatInfo, Nodes, OptionInfo, OptionKind, Problem, ProblemKind, Tables};

pub struct Yaml;

impl Format for Yaml {
    fn info(&self) -> FormatInfo {
        FormatInfo {
            name: "yaml",
            title: "YAML",
            text: true,
            nests: true,
            nodes: Nodes::Values,
            attributes: false,
            tables: Tables::One,
            path_separator: Some('/'),
            schema_kinds: &[],
            options: vec![OptionInfo { name: "at", kind: OptionKind::Text, label: "formatOptionAt", reads: true, writes: true, required_writing: None }],
        }
    }

    fn parse(&self, input: &[u8], _options: &[(String, Tree)], _names: &[String]) -> Result<Document, Problem> {
        let (bom, bytes) = match input.strip_prefix(b"\xEF\xBB\xBF") {
            Some(rest) => (Bom::Utf8, rest),
            None => (Bom::None, input),
        };
        let text = std::str::from_utf8(bytes).map_err(|_| no_yaml(None))?;
        let parsed = parse(text).map_err(|at| no_yaml(Some(line_of(text, at))))?;
        let kept = Kept { bom, final_break: text.ends_with('\n'), text: Some(text.to_string()), ..Kept::default() };
        Ok(Document { top: Node::from_tree("", &parsed.root.tree), columns: Vec::new(), kept: Some(kept) })
    }

    fn write(&self, doc: &Document, _options: &[(String, Tree)]) -> Result<Vec<u8>, Problem> {
        let tree = doc.top.to_tree();
        let kept = doc.kept.as_ref();
        let text = match kept.and_then(|k| k.text.as_deref()).and_then(|t| parse(t).ok().map(|p| (t, p))) {
            Some((source, parsed)) => patched(source, &parsed, &tree),
            None => fresh(&tree),
        };
        let mut bytes = Vec::with_capacity(text.len() + 3);
        if kept.is_some_and(|k| k.bom == Bom::Utf8) {
            bytes.extend_from_slice(b"\xEF\xBB\xBF");
        }
        bytes.extend_from_slice(text.as_bytes());
        Ok(bytes)
    }
}

fn no_yaml(line: Option<usize>) -> Problem {
    let text = match line {
        Some(line) => format!("the text is no YAML at line {line}"),
        None => "the text is no YAML".to_string(),
    };
    Problem::new(ProblemKind::NotFitting, text)
}

fn line_of(text: &str, at: usize) -> usize {
    text.as_bytes()[..at.min(text.len())].iter().filter(|&&b| b == b'\n').count() + 1
}

// ---------------------------------------------------------------- reading

/// A node as read, with where it was written, so writing back can keep it.
#[derive(Debug, Clone)]
struct Item {
    tree: Tree,
    shape: Shape,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
enum Shape {
    /// A block mapping, its keys at `col`.
    Map { col: usize, entries: Vec<Entry> },
    /// A block sequence, its dashes at `col`.
    Seq { col: usize, items: Vec<Entry> },
    /// A scalar, a flow collection or an alias.
    Other,
}

/// An entry of a block mapping (with its key) or an item of a block sequence.
#[derive(Debug, Clone)]
struct Entry {
    key: String,
    /// Where the blank and comment lines before it start (it owns them).
    lead: usize,
    /// Where its key, or its dash, starts.
    start: usize,
    /// Just after its key's colon, or its dash.
    head: usize,
    /// Just after its last line.
    end: usize,
    value: Item,
}

struct Parsed {
    root: Item,
}

/// The deepest a value nests before reading stops: a guard against a stack overflow.
const MAX_DEPTH: usize = 512;

/// Reads the first document of `text`; the place it stops being YAML, else.
fn parse(text: &str) -> Result<Parsed, usize> {
    let mut p = Parser { s: text, b: text.as_bytes(), pos: 0, end: text.len(), anchors: Vec::new(), depth: 0 };
    let marker = p.document_start()?;
    p.end = p.document_end(marker);
    let mut root = p.node(-1, false)?;
    // What is before the first entry is the document's, not the entry's.
    if let Shape::Map { entries, .. } | Shape::Seq { items: entries, .. } = &mut root.shape {
        if let Some(first) = entries.first_mut() {
            first.lead = first.start;
        }
    }
    p.skip_blank();
    if p.pos < p.end {
        return Err(p.pos);
    }
    Ok(Parsed { root })
}

struct Parser<'a> {
    s: &'a str,
    b: &'a [u8],
    pos: usize,
    /// Where the first document ends.
    end: usize,
    anchors: Vec<(String, Tree)>,
    depth: usize,
}

fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

fn is_flow_indicator(c: u8) -> bool {
    matches!(c, b',' | b'[' | b']' | b'{' | b'}')
}

impl<'a> Parser<'a> {
    fn at(&self, i: usize) -> Option<u8> {
        (i < self.end).then(|| self.b[i])
    }

    fn peek(&self) -> Option<u8> {
        self.at(self.pos)
    }

    /// Whether the place holds a space, a line break or nothing.
    fn blank_at(&self, i: usize) -> bool {
        self.at(i).is_none_or(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'))
    }

    fn line_start(&self, i: usize) -> usize {
        self.b[..i].iter().rposition(|&c| c == b'\n').map_or(0, |p| p + 1)
    }

    fn col(&self, i: usize) -> usize {
        i - self.line_start(i)
    }

    fn at_line_start(&self) -> bool {
        self.pos == 0 || self.b[self.pos - 1] == b'\n'
    }

    /// Whether the line at `i` is a document marker (`---` or `...`).
    fn marker_at(&self, i: usize) -> bool {
        let rest = &self.b[i..];
        (rest.starts_with(b"---") || rest.starts_with(b"...")) && rest.get(3).is_none_or(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'))
    }

    /// Skips what is before the first document: blank and comment lines,
    /// directives, its `---`. Whether there was a `---`.
    fn document_start(&mut self) -> Result<bool, usize> {
        loop {
            let line = self.pos;
            while self.at(self.pos).is_some_and(is_space) {
                self.pos += 1;
            }
            match self.peek() {
                None => return Ok(false),
                Some(b'\n' | b'\r' | b'#') => self.next_line(),
                Some(b'%') if self.pos == line => self.next_line(),
                Some(b'-') if self.pos == line && self.marker_at(line) => {
                    self.pos += 3;
                    return Ok(true);
                }
                _ => {
                    self.pos = line;
                    return Ok(false);
                }
            }
        }
    }

    /// Where the first document ends: the next marker line, else the end.
    fn document_end(&self, marker: bool) -> usize {
        let mut i = if marker { self.next_line_from(self.pos) } else { self.line_start(self.pos) };
        while i < self.b.len() {
            if self.marker_at(i) {
                return i;
            }
            i = self.next_line_from(i);
        }
        self.b.len()
    }

    fn next_line_from(&self, i: usize) -> usize {
        self.b[i..].iter().position(|&c| c == b'\n').map_or(self.b.len(), |p| i + p + 1)
    }

    fn next_line(&mut self) {
        self.pos = self.next_line_from(self.pos).min(self.end);
    }

    fn skip_inline(&mut self) {
        while self.peek().is_some_and(is_space) {
            self.pos += 1;
        }
    }

    /// Skips spaces, line breaks and comments.
    fn skip_blank(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                b'#' => self.next_line(),
                _ => break,
            }
        }
    }

    /// Ends the line: spaces, a comment, the line break; else not YAML.
    fn finish_line(&mut self) -> Result<(), usize> {
        if self.at_line_start() && self.pos > 0 {
            return Ok(());
        }
        self.skip_inline();
        if self.peek() == Some(b'#') {
            self.next_line();
            return Ok(());
        }
        if self.peek() == Some(b'\r') {
            self.pos += 1;
        }
        match self.peek() {
            None => Ok(()),
            Some(b'\n') => {
                self.pos += 1;
                Ok(())
            }
            _ => Err(self.pos),
        }
    }

    /// Whether the line ends here (spaces, then a comment or the line break).
    fn line_ends(&self) -> bool {
        let mut i = self.pos;
        while self.at(i).is_some_and(is_space) {
            i += 1;
        }
        matches!(self.at(i), None | Some(b'#' | b'\n' | b'\r'))
    }

    /// A node more indented than `min`, or null when there is none.
    /// `inline`: it follows a key on its line, so no block collection starts there.
    fn node(&mut self, min: isize, inline: bool) -> Result<Item, usize> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.pos);
        }
        let got = self.node_inner(min, inline);
        self.depth -= 1;
        got
    }

    fn node_inner(&mut self, min: isize, inline: bool) -> Result<Item, usize> {
        let lead = self.pos;
        self.skip_blank();
        if self.peek().is_none() || (self.col(self.pos) as isize) <= min {
            self.pos = lead;
            return Ok(Item { tree: Tree::Null, shape: Shape::Other, start: lead, end: lead });
        }
        let start = self.pos;
        // Properties: an anchor names the node, a tag is left aside.
        let mut anchor = None;
        loop {
            match self.peek() {
                Some(b'&') => {
                    self.pos += 1;
                    anchor = Some(self.name());
                    self.skip_inline();
                }
                Some(b'!') => {
                    while !self.blank_at(self.pos) {
                        self.pos += 1;
                    }
                    self.skip_inline();
                }
                _ => break,
            }
        }
        let item = if self.pos > start && self.line_ends() {
            // The node of the properties starts on a line below.
            self.finish_line()?;
            let mut item = self.node(min, false)?;
            item.start = start;
            item
        } else {
            self.value(min, inline, lead, start)?
        };
        if let Some(name) = anchor {
            self.anchors.push((name, item.tree.clone()));
        }
        Ok(item)
    }

    fn value(&mut self, min: isize, inline: bool, lead: usize, start: usize) -> Result<Item, usize> {
        let here = self.pos;
        let col = self.col(here);
        let other = |tree, end| Ok(Item { tree, shape: Shape::Other, start, end });
        match self.peek() {
            Some(b'*') => {
                self.pos += 1;
                let name = self.name();
                let tree = self.anchors.iter().rev().find(|(n, _)| *n == name).map(|(_, t)| t.clone()).ok_or(here)?;
                other(tree, self.pos)
            }
            Some(b'-') if self.blank_at(here + 1) => {
                if inline {
                    return Err(here);
                }
                self.seq(col, lead)
            }
            Some(b'[' | b'{') => {
                let tree = self.flow()?;
                other(tree, self.pos)
            }
            Some(b'|' | b'>') => {
                let text = self.block_scalar(min)?;
                other(Tree::Text(text), self.pos)
            }
            Some(b'?') if self.blank_at(here + 1) => Err(here),
            _ if !inline && self.key_at(here).is_some() => self.map(col, lead),
            Some(b'"' | b'\'') => {
                let text = self.quoted()?;
                other(Tree::Text(text), self.pos)
            }
            _ => {
                let text = self.plain(min, false)?;
                other(resolve(&text), self.pos)
            }
        }
    }

    /// An anchor's or alias's name.
    fn name(&mut self) -> String {
        let from = self.pos;
        while !self.blank_at(self.pos) && !is_flow_indicator(self.b[self.pos]) {
            self.pos += 1;
        }
        self.s[from..self.pos].to_string()
    }

    /// The key that starts at `i` on its line, followed by `:`, and where it ends.
    fn key_at(&mut self, i: usize) -> Option<(String, usize)> {
        let saved = self.pos;
        self.pos = i;
        let line_end = self.next_line_from(i);
        let key = match self.peek() {
            Some(b'"' | b'\'') => self.quoted().ok().filter(|_| self.pos < line_end),
            _ => self.plain_line(false).filter(|k| !k.is_empty()),
        };
        let found = key.and_then(|key| {
            self.skip_inline();
            (self.peek() == Some(b':') && self.blank_at(self.pos + 1)).then(|| (key, self.pos + 1))
        });
        self.pos = saved;
        found
    }

    /// A block mapping, its keys at `col`.
    fn map(&mut self, col: usize, first_lead: usize) -> Result<Item, usize> {
        let mut entries: Vec<Entry> = Vec::new();
        let mut tree = Vec::new();
        loop {
            let lead = if entries.is_empty() { first_lead } else { self.pos };
            if !entries.is_empty() {
                self.skip_blank();
            }
            if self.peek().is_none() || self.col(self.pos) < col {
                self.pos = lead;
                break;
            }
            if self.col(self.pos) > col {
                return Err(self.pos);
            }
            let start = self.pos;
            let (key, head) = self.key_at(start).ok_or(start)?;
            self.pos = head;
            let value = if self.line_ends() {
                self.finish_line()?;
                // A sequence may stand at the key's own column.
                let next = self.pos;
                self.skip_blank();
                let same = self.peek() == Some(b'-') && self.col(self.pos) == col && self.blank_at(self.pos + 1);
                self.pos = next;
                if same {
                    self.skip_blank();
                    self.seq(col, next)?
                } else {
                    self.node(col as isize, false)?
                }
            } else {
                self.skip_inline();
                self.node(col as isize, true)?
            };
            if !self.at_line_start() {
                self.finish_line()?;
            }
            tree.push((key.clone(), value.tree.clone()));
            entries.push(Entry { key, lead, start, head, end: self.pos, value });
        }
        let start = entries.first().map_or(self.pos, |e| e.start);
        Ok(Item { tree: Tree::Object(tree), shape: Shape::Map { col, entries }, start, end: self.pos })
    }

    /// A block sequence, its dashes at `col`.
    fn seq(&mut self, col: usize, first_lead: usize) -> Result<Item, usize> {
        let mut items: Vec<Entry> = Vec::new();
        let mut tree = Vec::new();
        loop {
            let lead = if items.is_empty() { first_lead } else { self.pos };
            if !items.is_empty() {
                self.skip_blank();
            }
            let is_item = self.peek() == Some(b'-') && self.blank_at(self.pos + 1);
            if self.peek().is_none() || self.col(self.pos) < col || (self.col(self.pos) == col && !is_item) {
                self.pos = lead;
                break;
            }
            if self.col(self.pos) > col || !is_item {
                return Err(self.pos);
            }
            let start = self.pos;
            self.pos += 1;
            let head = self.pos;
            let value = if self.line_ends() {
                self.finish_line()?;
                self.node(col as isize, false)?
            } else {
                self.skip_inline();
                self.node(col as isize, false)?
            };
            if !self.at_line_start() {
                self.finish_line()?;
            }
            tree.push(value.tree.clone());
            items.push(Entry { key: String::new(), lead, start, head, end: self.pos, value });
        }
        let start = items.first().map_or(self.pos, |e| e.start);
        Ok(Item { tree: Tree::List(tree), shape: Shape::Seq { col, items }, start, end: self.pos })
    }

    /// A flow collection, `[...]` or `{...}`, over as many lines as it takes.
    fn flow(&mut self) -> Result<Tree, usize> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.pos);
        }
        let list = self.peek() == Some(b'[');
        let close = if list { b']' } else { b'}' };
        self.pos += 1;
        let mut items = Vec::new();
        let mut pairs = Vec::new();
        loop {
            self.skip_blank();
            match self.peek() {
                None => return Err(self.pos),
                Some(c) if c == close => {
                    self.pos += 1;
                    break;
                }
                _ => {}
            }
            let (first, raw) = self.flow_node()?;
            self.skip_blank();
            let pair = if self.peek() == Some(b':') {
                self.pos += 1;
                self.skip_blank();
                let value = match self.peek() {
                    Some(b',') => Tree::Null,
                    Some(c) if c == close => Tree::Null,
                    _ => self.flow_node()?.0,
                };
                Some((raw.clone().ok_or(self.pos)?, value))
            } else {
                None
            };
            match (list, pair) {
                (true, Some(pair)) => items.push(Tree::Object(vec![pair])),
                (true, None) => items.push(first),
                (false, Some(pair)) => pairs.push(pair),
                (false, None) => pairs.push((raw.ok_or(self.pos)?, Tree::Null)),
            }
            self.skip_blank();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(c) if c == close => {}
                _ => return Err(self.pos),
            }
        }
        self.depth -= 1;
        Ok(if list { Tree::List(items) } else { Tree::Object(pairs) })
    }

    /// A node inside a flow collection, and its text as a key when it is a scalar.
    fn flow_node(&mut self) -> Result<(Tree, Option<String>), usize> {
        loop {
            match self.peek() {
                Some(b'&') => {
                    self.pos += 1;
                    let _ = self.name();
                    self.skip_blank();
                }
                Some(b'!') => {
                    while !self.blank_at(self.pos) && !is_flow_indicator(self.b[self.pos]) {
                        self.pos += 1;
                    }
                    self.skip_blank();
                }
                _ => break,
            }
        }
        let here = self.pos;
        match self.peek() {
            Some(b'[' | b'{') => Ok((self.flow()?, None)),
            Some(b'*') => {
                self.pos += 1;
                let name = self.name();
                let tree = self.anchors.iter().rev().find(|(n, _)| *n == name).map(|(_, t)| t.clone()).ok_or(here)?;
                Ok((tree, None))
            }
            Some(b'"' | b'\'') => {
                let text = self.quoted()?;
                Ok((Tree::Text(text.clone()), Some(text)))
            }
            _ => {
                let text = self.plain(-1, true)?;
                Ok((resolve(&text), Some(text)))
            }
        }
    }

    /// One line of a plain scalar from here, trimmed; none when none may start here.
    fn plain_line(&mut self, flow: bool) -> Option<String> {
        let first = self.peek()?;
        let next_blank = self.blank_at(self.pos + 1);
        let forbidden = matches!(first, b',' | b'[' | b']' | b'{' | b'}' | b'#' | b'&' | b'*' | b'!' | b'|' | b'>' | b'\'' | b'"' | b'%' | b'@' | b'`');
        if forbidden || (matches!(first, b'-' | b'?' | b':') && (next_blank || (flow && self.at(self.pos + 1).is_some_and(is_flow_indicator)))) {
            return None;
        }
        Some(self.plain_rest(flow))
    }

    /// The rest of a plain scalar's line, trimmed.
    fn plain_rest(&mut self, flow: bool) -> String {
        let from = self.pos;
        let mut last = from;
        while let Some(c) = self.peek() {
            match c {
                b'\n' | b'\r' => break,
                b':' if self.blank_at(self.pos + 1) || (flow && self.at(self.pos + 1).is_some_and(is_flow_indicator)) => break,
                b'#' if self.pos > from && is_space(self.b[self.pos - 1]) => break,
                c if flow && is_flow_indicator(c) => break,
                c => {
                    self.pos += 1;
                    if !is_space(c) {
                        last = self.pos;
                    }
                }
            }
        }
        self.pos = last;
        self.s[from..last].to_string()
    }

    /// A plain scalar, over the lines below that are more indented than `min`, folded.
    fn plain(&mut self, min: isize, flow: bool) -> Result<String, usize> {
        let mut text = self.plain_line(flow).ok_or(self.pos)?;
        loop {
            let here = self.pos;
            let mut i = here;
            while self.at(i).is_some_and(is_space) {
                i += 1;
            }
            if self.at(i) == Some(b'\r') {
                i += 1;
            }
            if self.at(i) != Some(b'\n') {
                break;
            }
            // The lines below: empty ones, then one that goes on.
            let mut breaks = 0;
            let mut next = i + 1;
            let found = loop {
                let mut j = next;
                while self.at(j).is_some_and(is_space) {
                    j += 1;
                }
                match self.at(j) {
                    None => break None,
                    Some(b'\r') if self.at(j + 1) == Some(b'\n') => {
                        breaks += 1;
                        next = j + 2;
                    }
                    Some(b'\n') => {
                        breaks += 1;
                        next = j + 1;
                    }
                    Some(b'#') => break None,
                    Some(_) if (self.col(j) as isize) <= min || (self.col(j) == 0 && self.marker_at(j)) => break None,
                    Some(c) if flow && is_flow_indicator(c) => break None,
                    Some(_) => break Some(j),
                }
            };
            let Some(j) = found else { break };
            self.pos = j;
            let line = self.plain_rest(flow);
            if line.is_empty() {
                self.pos = here;
                break;
            }
            text.push_str(&if breaks == 0 { " ".to_string() } else { "\n".repeat(breaks) });
            text.push_str(&line);
        }
        Ok(text)
    }

    /// A single- or double-quoted scalar, folded over lines, its escapes read.
    fn quoted(&mut self) -> Result<String, usize> {
        let double = self.peek() == Some(b'"');
        let open = self.pos;
        self.pos += 1;
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else { return Err(open) };
            match c {
                b'"' if double => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\'' if !double => {
                    if self.at(self.pos + 1) == Some(b'\'') {
                        out.push('\'');
                        self.pos += 2;
                    } else {
                        self.pos += 1;
                        return Ok(out);
                    }
                }
                b'\\' if double => {
                    self.pos += 1;
                    let Some(e) = self.peek() else { return Err(open) };
                    self.pos += 1;
                    let simple = match e {
                        b'0' => Some('\0'),
                        b'a' => Some('\u{7}'),
                        b'b' => Some('\u{8}'),
                        b't' | b'\t' => Some('\t'),
                        b'n' => Some('\n'),
                        b'v' => Some('\u{b}'),
                        b'f' => Some('\u{c}'),
                        b'r' => Some('\r'),
                        b'e' => Some('\u{1b}'),
                        b' ' => Some(' '),
                        b'"' => Some('"'),
                        b'/' => Some('/'),
                        b'\\' => Some('\\'),
                        b'N' => Some('\u{85}'),
                        b'_' => Some('\u{a0}'),
                        b'L' => Some('\u{2028}'),
                        b'P' => Some('\u{2029}'),
                        _ => None,
                    };
                    if let Some(ch) = simple {
                        out.push(ch);
                        continue;
                    }
                    let digits = match e {
                        b'x' => 2,
                        b'u' => 4,
                        b'U' => 8,
                        b'\r' | b'\n' => {
                            // An escaped line break joins the lines without a space.
                            if e == b'\r' && self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                            while self.peek().is_some_and(is_space) {
                                self.pos += 1;
                            }
                            continue;
                        }
                        _ => return Err(self.pos - 1),
                    };
                    let hex = self.s.get(self.pos..self.pos + digits).filter(|_| self.pos + digits <= self.end).ok_or(self.pos)?;
                    let ch = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32).ok_or(self.pos)?;
                    out.push(ch);
                    self.pos += digits;
                }
                b'\r' | b'\n' => {
                    // A line break folds: one is a space, more are line breaks less one.
                    while out.ends_with([' ', '\t']) {
                        out.pop();
                    }
                    let mut breaks = 0;
                    loop {
                        match self.peek() {
                            Some(b'\r') => self.pos += 1,
                            Some(b'\n') => {
                                breaks += 1;
                                self.pos += 1;
                            }
                            Some(c) if is_space(c) => self.pos += 1,
                            _ => break,
                        }
                    }
                    if self.col(self.pos) == 0 && self.pos < self.end && self.marker_at(self.pos) {
                        return Err(open);
                    }
                    if breaks == 1 {
                        out.push(' ');
                    } else {
                        out.push_str(&"\n".repeat(breaks - 1));
                    }
                }
                _ => {
                    let ch = self.s[self.pos..].chars().next().ok_or(self.pos)?;
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    /// A block scalar, `|` or `>`, its lines more indented than `min`.
    fn block_scalar(&mut self, min: isize) -> Result<String, usize> {
        let literal = self.peek() == Some(b'|');
        self.pos += 1;
        let mut chomp = b' ';
        let mut indent: Option<usize> = None;
        for _ in 0..2 {
            match self.peek() {
                Some(c @ (b'+' | b'-')) if chomp == b' ' => {
                    chomp = c;
                    self.pos += 1;
                }
                Some(c @ b'1'..=b'9') if indent.is_none() => {
                    indent = Some((min + (c - b'0') as isize).max(0) as usize);
                    self.pos += 1;
                }
                _ => break,
            }
        }
        if !self.blank_at(self.pos) && self.peek() != Some(b'#') {
            return Err(self.pos);
        }
        self.finish_line()?;
        let header_end = self.pos;
        // The lines: their indentation is the first line's with text.
        let mut i = self.pos;
        let indent = match indent {
            Some(n) => n,
            None => loop {
                if i >= self.end {
                    break (min + 1).max(0) as usize;
                }
                let mut j = i;
                while self.at(j) == Some(b' ') {
                    j += 1;
                }
                match self.at(j) {
                    None => break (min + 1).max(0) as usize,
                    Some(b'\n') => i = j + 1,
                    Some(b'\r') if self.at(j + 1) == Some(b'\n') => i = j + 2,
                    _ => break j - i,
                }
            },
        };
        if (indent as isize) <= min {
            self.pos = header_end;
            return Ok(String::new());
        }
        let mut lines: Vec<&str> = Vec::new();
        let mut i = header_end;
        let mut content_end = header_end;
        let mut all_end = header_end;
        while i < self.end {
            let next = self.next_line_from(i).min(self.end);
            let line = self.s[i..next].trim_end_matches('\n').trim_end_matches('\r');
            let spaces = line.bytes().take_while(|&c| c == b' ').count();
            if line.bytes().all(|c| c == b' ') {
                let rest = line.get(indent..).unwrap_or("");
                if !rest.is_empty() {
                    content_end = next;
                }
                lines.push(rest);
                all_end = next;
            } else if spaces >= indent {
                lines.push(&line[indent..]);
                content_end = next;
                all_end = next;
            } else {
                break;
            }
            i = next;
        }
        let body_len = lines.iter().rposition(|l| !l.is_empty()).map_or(0, |p| p + 1);
        let trailing = lines.len() - body_len;
        let body = &lines[..body_len];
        let mut text = if literal { body.join("\n") } else { folded(body) };
        match chomp {
            b'-' => {}
            b'+' => {
                text.push_str(&"\n".repeat(if body.is_empty() { trailing } else { trailing + 1 }));
            }
            _ => {
                if !body.is_empty() {
                    text.push('\n');
                }
            }
        }
        self.pos = if chomp == b'+' { all_end } else { content_end.max(header_end) };
        Ok(text)
    }
}

/// The lines of a `>` block, folded: a break between two lines of text is
/// a space, empty lines are breaks, a more indented line keeps its breaks.
fn folded(lines: &[&str]) -> String {
    let more = |l: &str| l.starts_with([' ', '\t']);
    let mut out = String::new();
    let mut prev: Option<&str> = None;
    let mut empties = 0;
    for &line in lines {
        if line.is_empty() {
            empties += 1;
            continue;
        }
        match prev {
            None => out.push_str(&"\n".repeat(empties)),
            Some(p) if !more(p) && !more(line) => out.push_str(&if empties == 0 { " ".to_string() } else { "\n".repeat(empties) }),
            Some(_) => out.push_str(&"\n".repeat(empties + 1)),
        }
        out.push_str(line);
        prev = Some(line);
        empties = 0;
    }
    out
}

/// A plain scalar's value by YAML 1.2's core schema.
fn resolve(text: &str) -> Tree {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return Tree::Null,
        "true" | "True" | "TRUE" => return Tree::Bool(true),
        "false" | "False" | "FALSE" => return Tree::Bool(false),
        _ => {}
    }
    number(text).map_or_else(|| Tree::Text(text.to_string()), Tree::Number)
}

/// A plain scalar that is a core-schema number, as the JSON number it is.
fn number(text: &str) -> Option<String> {
    let radix = |digits: &str, radix: u32| (!digits.is_empty()).then(|| u128::from_str_radix(digits, radix).ok().map(|n| n.to_string())).flatten();
    if let Some(hex) = text.strip_prefix("0x") {
        return radix(hex, 16);
    }
    if let Some(octal) = text.strip_prefix("0o") {
        return radix(octal, 8);
    }
    let (sign, rest) = match text.as_bytes().first() {
        Some(b'-') => ("-", &text[1..]),
        Some(b'+') => ("", &text[1..]),
        _ => ("", text),
    };
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], Some(&rest[at + 1..])),
        None => (rest, None),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (mantissa, None),
    };
    let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
    if !digits(whole) || !fraction.is_none_or(digits) || (whole.is_empty() && fraction.is_none_or(str::is_empty)) {
        return None;
    }
    let exponent = match exponent {
        None => String::new(),
        Some(e) => {
            let unsigned = e.strip_prefix(['+', '-']).unwrap_or(e);
            if unsigned.is_empty() || !digits(unsigned) {
                return None;
            }
            format!("e{e}")
        }
    };
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let fraction = match fraction {
        Some(f) if !f.is_empty() => format!(".{f}"),
        _ => String::new(),
    };
    Some(format!("{sign}{whole}{fraction}{exponent}"))
}

// ---------------------------------------------------------------- writing

/// A new document's text.
fn fresh(tree: &Tree) -> String {
    match tree {
        Tree::Object(pairs) if !pairs.is_empty() => join(&map_pieces(pairs, None, 0, ""), 0),
        Tree::List(items) if !items.is_empty() => join(&seq_pieces(items, None, 0, ""), 0),
        Tree::Text(t) if t.contains('\n') => format!("{}\n", quote(t)),
        other => format!("{}\n", scalar(other)),
    }
}

/// A document read, written back with only what changed rewritten.
fn patched(src: &str, parsed: &Parsed, tree: &Tree) -> String {
    let root = &parsed.root;
    if root.tree == *tree {
        return src.to_string();
    }
    let line_start = |i: usize| src.as_bytes()[..i].iter().rposition(|&c| c == b'\n').map_or(0, |p| p + 1);
    let (body, from, to) = match (&root.shape, tree) {
        (Shape::Map { col, entries }, Tree::Object(pairs)) if !pairs.is_empty() => (join(&map_pieces(pairs, Some(entries), *col, src), *col), line_start(entries[0].start), root.end),
        (Shape::Seq { col, items }, Tree::List(list)) if !list.is_empty() => (join(&seq_pieces(list, Some(items), *col, src), *col), line_start(items[0].start), root.end),
        _ => {
            let from = line_start(root.start);
            let mut body = String::new();
            if src[from..root.start].trim_start().starts_with("---") {
                body.push_str("---\n");
            }
            body.push_str(&fresh(tree));
            let mut to = root.end;
            if to > 0 && src.as_bytes()[to - 1] != b'\n' {
                while to < src.len() && src.as_bytes()[to] != b'\n' {
                    to += 1;
                }
                to = (to + 1).min(src.len());
            }
            (body, from, to)
        }
    };
    let mut out = String::with_capacity(src.len() + body.len());
    out.push_str(&src[..from]);
    out.push_str(&body);
    out.push_str(&src[to..]);
    out
}

/// One entry or item to write: the lines before it as written, and its text
/// from its key or dash on (its later lines indented in full), with its line break.
struct Piece {
    lead: String,
    body: String,
}

fn join(pieces: &[Piece], col: usize) -> String {
    let mut out = String::new();
    for piece in pieces {
        out.push_str(&piece.lead);
        out.push_str(&" ".repeat(col));
        out.push_str(&piece.body);
    }
    out
}

fn lead_of(src: &str, entry: &Entry) -> String {
    let line = src.as_bytes()[..entry.start].iter().rposition(|&c| c == b'\n').map_or(0, |p| p + 1);
    if entry.lead < line { src[entry.lead..line].to_string() } else { String::new() }
}

/// The entries of a mapping at `col`: an unchanged one as written, a changed one with its key as written.
fn map_pieces(pairs: &[(String, Tree)], old: Option<&[Entry]>, col: usize, src: &str) -> Vec<Piece> {
    let old = old.unwrap_or(&[]);
    let mut used = vec![false; old.len()];
    let mut pieces = Vec::with_capacity(pairs.len());
    for (key, value) in pairs {
        let found = old.iter().enumerate().position(|(i, e)| !used[i] && e.key == *key);
        let piece = match found {
            Some(i) => {
                used[i] = true;
                let e = &old[i];
                if e.value.tree == *value {
                    Piece { lead: lead_of(src, e), body: src[e.start..e.end].to_string() }
                } else {
                    let (_, rest) = after(value, Some(&e.value), col, false, src);
                    Piece { lead: lead_of(src, e), body: format!("{}{rest}", &src[e.start..e.head]) }
                }
            }
            None => {
                let (_, rest) = after(value, None, col, false, src);
                Piece { lead: String::new(), body: format!("{}:{rest}", quote_if(key)) }
            }
        };
        pieces.push(piece);
    }
    pieces
}

/// The items of a sequence at `col`: one equal to an item read as written,
/// else one in the place of an item read rewritten where it changed, else new.
fn seq_pieces(list: &[Tree], old: Option<&[Entry]>, col: usize, src: &str) -> Vec<Piece> {
    let old = old.unwrap_or(&[]);
    let mut used = vec![false; old.len()];
    let mut pieces = Vec::with_capacity(list.len());
    for (i, value) in list.iter().enumerate() {
        if let Some(j) = old.iter().enumerate().position(|(j, e)| !used[j] && e.value.tree == *value) {
            used[j] = true;
            pieces.push(Piece { lead: lead_of(src, &old[j]), body: src[old[j].start..old[j].end].to_string() });
            continue;
        }
        let place = (i < old.len() && !used[i]).then(|| &old[i]);
        if place.is_some() {
            used[i] = true;
        }
        let (lead, rest) = after(value, place.map(|e| &e.value), col, true, src);
        let mut piece_lead = place.map(|e| lead_of(src, e)).unwrap_or_default();
        piece_lead.push_str(&lead);
        let head = place.map_or("-", |e| &src[e.start..e.head]);
        pieces.push(Piece { lead: piece_lead, body: format!("{head}{rest}") });
    }
    pieces
}

/// A value after its key's colon or its dash at `col`, to its last line
/// break; and, after a dash, the lines that go before the dash's line.
fn after(value: &Tree, old: Option<&Item>, col: usize, dash: bool, src: &str) -> (String, String) {
    let nested = |pieces: Vec<Piece>, inner: usize| -> (String, String) {
        let mut pieces = pieces.into_iter();
        if dash {
            let first = pieces.next().expect("not empty");
            let mut rest = " ".repeat(inner - col - 1);
            rest.push_str(&first.body);
            rest.push_str(&join(&pieces.collect::<Vec<_>>(), inner));
            (first.lead, rest)
        } else {
            (String::new(), format!("\n{}", join(&pieces.collect::<Vec<_>>(), inner)))
        }
    };
    match value {
        Tree::Object(pairs) if !pairs.is_empty() => {
            let (inner, entries) = match old.map(|o| &o.shape) {
                Some(Shape::Map { col: c, entries }) if *c > col => (*c, Some(entries.as_slice())),
                _ => (col + 2, None),
            };
            nested(map_pieces(pairs, entries, inner, src), inner)
        }
        Tree::List(list) if !list.is_empty() => {
            let (inner, items) = match old.map(|o| &o.shape) {
                Some(Shape::Seq { col: c, items }) if *c > col || (!dash && *c == col) => (*c, Some(items.as_slice())),
                _ => (col + 2, None),
            };
            nested(seq_pieces(list, items, inner, src), inner)
        }
        Tree::Text(t) if block_fits(t) => {
            let body = t.trim_end_matches('\n');
            let trailing = t.len() - body.len();
            let chomp = match trailing {
                0 => "-",
                1 => "",
                _ => "+",
            };
            let mut out = format!(" |{chomp}\n");
            for line in body.split('\n') {
                if !line.is_empty() {
                    out.push_str(&" ".repeat(col + 2));
                    out.push_str(line);
                }
                out.push('\n');
            }
            out.push_str(&"\n".repeat(trailing.saturating_sub(1)));
            (String::new(), out)
        }
        other => (String::new(), format!(" {}\n", scalar(other))),
    }
}

/// Whether a text is written as a `|` block: several lines, nothing a block cannot hold.
fn block_fits(t: &str) -> bool {
    let first = t.split('\n').find(|l| !l.is_empty());
    t.contains('\n') && first.is_some_and(|l| !l.starts_with([' ', '\t'])) && !t.chars().any(|c| c != '\n' && c != '\t' && needs_escape(c))
}

/// One value on its line.
fn scalar(value: &Tree) -> String {
    match value {
        Tree::Null => "null".into(),
        Tree::Bool(b) => b.to_string(),
        Tree::Number(n) => n.clone(),
        Tree::Text(t) => quote_if(t),
        Tree::Object(_) => "{}".into(),
        Tree::List(_) => "[]".into(),
    }
}

/// A text, quoted where it would read back as something else.
fn quote_if(t: &str) -> String {
    if needs_quotes(t) { quote(t) } else { t.to_string() }
}

fn needs_quotes(t: &str) -> bool {
    let Some(first) = t.chars().next() else { return true };
    resolve(t) != Tree::Text(t.to_string())
        || matches!(t.to_ascii_lowercase().trim_start_matches(['+', '-']), ".inf" | ".nan")
        || "-?:,[]{}#&*!|>'\"%@`".contains(first)
        || t.starts_with("...")
        || t.contains(": ")
        || t.contains(" #")
        || t.ends_with(':')
        || t.starts_with([' ', '\t'])
        || t.ends_with([' ', '\t'])
        || t.chars().any(|c| c == '\t' || needs_escape(c))
}

fn needs_escape(c: char) -> bool {
    (c as u32) < 0x20 || matches!(c, '\u{7f}' | '\u{85}' | '\u{2028}' | '\u{2029}' | '\u{feff}')
}

/// A text in double quotes, with escapes.
fn quote(t: &str) -> String {
    let mut out = String::with_capacity(t.len() + 2);
    out.push('"');
    for c in t.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '\u{85}' => out.push_str("\\N"),
            '\u{2028}' => out.push_str("\\L"),
            '\u{2029}' => out.push_str("\\P"),
            '\u{feff}' => out.push_str("\\uFEFF"),
            c if needs_escape(c) => out.push_str(&format!("\\x{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree;

    fn read(text: &str) -> Tree {
        parse(text).unwrap_or_else(|at| panic!("no YAML at {at}: {text}")).root.tree
    }

    fn json(text: &str) -> Tree {
        tree::parse(text).unwrap()
    }

    #[test]
    fn plain_scalars_take_the_core_schema() {
        assert_eq!(read("[1, -2, +3, 007, 1.50, .5, 1e3, 0x1F, 0o17, true, False, ~, null, .inf, abc, 1.2.3]"), json(r#"[1,-2,3,7,1.50,0.5,1e3,31,15,true,false,null,null,".inf","abc","1.2.3"]"#));
    }

    #[test]
    fn block_and_flow_collections_nest() {
        let text = "# head\nshop:\n  name: Corner # a comment\n  tags: [a, 'b c', {k: v}]\n  orders:\n  - number: A-1\n    lines:\n      - name: Tea\n        quantity: 2\n  - number: A-2\n    empty:\n";
        assert_eq!(read(text), json(r#"{"shop":{"name":"Corner","tags":["a","b c",{"k":"v"}],"orders":[{"number":"A-1","lines":[{"name":"Tea","quantity":2}]},{"number":"A-2","empty":null}]}}"#));
    }

    #[test]
    fn quoted_and_block_scalars_read_their_escapes_and_lines() {
        assert_eq!(read(r#"a: "x\ty\u00e9\n" "#), json(r#"{"a":"x\ty\u00e9\n"}"#));
        assert_eq!(read("a: 'it''s'\nb: \"one\n  two\n\n  three\""), json(r#"{"a":"it's","b":"one two\nthree"}"#));
        assert_eq!(read("a: |\n  one\n   two\n\nb: >-\n  folded\n  line\n\n  next\nc: |+\n  kept\n\n"), json(r#"{"a":"one\n two\n","b":"folded line\nnext","c":"kept\n\n"}"#));
        assert_eq!(read("- plain\n  over lines\n- x"), json(r#"["plain over lines","x"]"#));
    }

    #[test]
    fn anchors_aliases_and_documents() {
        assert_eq!(read("%YAML 1.2\n---\nbase: &b {a: 1}\ncopy: *b\n...\n--- second"), json(r#"{"base":{"a":1},"copy":{"a":1}}"#));
        assert_eq!(read(""), Tree::Null);
        assert_eq!(read("--- !tagged\n- !!str x"), json(r#"["x"]"#));
    }

    #[test]
    fn what_is_no_yaml_says_where() {
        for bad in ["a: b: c", "a: 1\n  b: 2", "[1, 2", "? complex\n: key", "a: \"open", "- a\nb: 1", "x: *missing"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        let got = Yaml.parse(b"a: 1\nb: [\n", &[], &[]).unwrap_err();
        assert_eq!(got.text, "the text is no YAML at line 3");
    }

    #[test]
    fn written_text_reads_back_as_it_was() {
        let value = json(r#"{"a":"","b":"12","c":"true","d":"- x","e":"k: v","f":"two\nlines\n","g":"x #y","h":" pad","i":"tab\tin","j":[],"k":{},"l":[{"m":1,"n":[1,2]},[3]],"o":"end\n\n","p":"no end\nx","q":"null","r":".inf"}"#);
        let written = fresh(&value);
        assert_eq!(read(&written), value, "{written}");
        assert_eq!(fresh(&json(r#"{"a":[{"b":1,"c":"x"}],"d":{"e":"two\nlines"}}"#)), "a:\n  - b: 1\n    c: x\nd:\n  e: |-\n    two\n    lines\n");
    }

    #[test]
    fn a_changed_document_keeps_what_did_not_change_as_written() {
        let src = "# orders\nlist:\n  # first\n  - number: 'A-1'   # keep\n    note: hi\n  - number: A-2\n    total: 3\nnext: ~ # end\n";
        let parsed = parse(src).unwrap();
        let mut tree = parsed.root.tree.clone();
        let Some(Tree::List(items)) = tree.get("list").cloned() else { panic!() };
        let mut second = items[1].clone();
        second.set("total", Tree::Number("4".into()));
        tree.set("list", Tree::List(vec![items[0].clone(), second, json(r#"{"number":"A-3"}"#)]));
        assert_eq!(patched(src, &parsed, &tree), "# orders\nlist:\n  # first\n  - number: 'A-1'   # keep\n    note: hi\n  - number: A-2\n    total: 4\n  - number: A-3\nnext: ~ # end\n");
    }
}
