//! Markdown read into blocks (headings, paragraphs, list items, quotes, scene breaks, code) and
//! their spans of text, the way every renderer reads it:
//! conformance/markdown.json says what each case gives. It reads as the
//! IDE's markdownBlocks.ts does; the markdown part of Clarity.UI shows what it reads.

/// A piece of a block's text with one look.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    /// The address it opens; none for text that is no link.
    pub link: Option<String>,
}

impl Span {
    /// Plain text with no look of its own.
    pub fn plain(text: &str) -> Span {
        Span { text: text.to_string(), ..Span::default() }
    }

    /// Whether it looks the same as `other`, whatever their texts.
    fn looks_as(&self, other: &Span) -> bool {
        self.bold == other.bold && self.italic == other.italic && self.code == other.code && self.link == other.link
    }
}

/// What kind of block a block is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Heading,
    Paragraph,
    Item,
    /// A quote (`> `), with its depth.
    Quote,
    /// A scene break (`***`, `---`, `___`).
    Rule,
    Code,
}

/// A block: a heading with its level, a paragraph, a list item with its
/// depth and (numbered) its number, or code with its text as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    pub spans: Vec<Span>,
    /// A heading's level, 1 to 6.
    pub level: usize,
    /// A list item's depth, 0 to 3; a quote's, 1 to 3.
    pub depth: usize,
    /// A numbered list item's number; none for a bullet.
    pub number: Option<u64>,
    /// A code block's lines as written.
    pub text: String,
}

impl Block {
    fn new(kind: BlockKind) -> Block {
        Block { kind, spans: Vec::new(), level: 0, depth: 0, number: None, text: String::new() }
    }
}

/// The characters a backslash takes as written.
const PUNCTUATION: &str = "\\`*_[]()#+-.!{}<>";

/// The line that starts and ends a code block.
const FENCE: &str = "```";

/// The look a span gets from what holds it.
#[derive(Debug, Clone, Default)]
struct Look {
    bold: bool,
    italic: bool,
    link: Option<String>,
}

/// Addresses a link may open (a `clarity:` one is a place in the application, followed by `on link`); with any other, only the link's text shows.
fn opens(address: &str) -> bool {
    address.starts_with("http://") || address.starts_with("https://") || address.starts_with("mailto:") || address.starts_with("clarity:")
}

fn at(s: &[char], i: isize) -> char {
    if i >= 0 && (i as usize) < s.len() { s[i as usize] } else { '\0' }
}

fn word_char(s: &[char], i: isize) -> bool {
    i >= 0 && (i as usize) < s.len() && s[i as usize].is_alphanumeric()
}

/// Where `what` is found in `s` from `from` on; none when it is not.
fn index_of(s: &[char], what: &[char], from: usize) -> Option<usize> {
    if what.is_empty() || from > s.len() {
        return None;
    }
    (from..=s.len().saturating_sub(what.len())).find(|&i| s.len() >= what.len() && s[i..i + what.len()] == *what)
}

fn text_of(s: &[char]) -> String {
    s.iter().collect()
}

/// Where the single `mark` that closes an emphasis from `from` on is, skipping doubled marks.
fn closing_single(s: &[char], mark: char, from: usize) -> Option<usize> {
    let mut j = from;
    while j < s.len() {
        if s[j] != mark {
            j += 1;
            continue;
        }
        if at(s, j as isize + 1) == mark {
            j += 2;
            continue;
        }
        if mark == '_' && word_char(s, j as isize + 1) {
            j += 1;
            continue;
        }
        return Some(j);
    }
    None
}

fn inline(s: &[char], look: &Look, output: &mut Vec<Span>) {
    let mut text = String::new();
    let flush = |text: &mut String, output: &mut Vec<Span>| {
        if !text.is_empty() {
            output.push(Span { text: std::mem::take(text), bold: look.bold, italic: look.italic, code: false, link: look.link.clone() });
        }
    };
    let mut i = 0usize;
    while i < s.len() {
        let c = s[i];
        let two_marks = i + 1 < s.len() && s[i + 1] == c && (c == '*' || c == '_');
        let three_marks = two_marks && i + 2 < s.len() && s[i + 2] == c;
        let after_word = c == '_' && word_char(s, i as isize - 1);
        if c == '\\' && i + 1 < s.len() && PUNCTUATION.contains(s[i + 1]) {
            text.push(s[i + 1]);
            i += 2;
        } else if c == '`' {
            match index_of(s, &['`'], i + 1) {
                None => {
                    text.push(c);
                    i += 1;
                }
                Some(end) => {
                    flush(&mut text, output);
                    output.push(Span { text: text_of(&s[i + 1..end]), bold: look.bold, italic: look.italic, code: true, link: look.link.clone() });
                    i = end + 1;
                }
            }
        } else if three_marks && i + 3 < s.len() && s[i + 3] != ' ' && !after_word && index_of(s, &s[i..i + 3], i + 3).is_some() {
            // Bold and italic at once.
            let end = index_of(s, &s[i..i + 3], i + 3).unwrap_or(s.len());
            flush(&mut text, output);
            inline(&s[i + 3..end], &Look { bold: true, italic: true, ..look.clone() }, output);
            i = end + 3;
        } else if two_marks && i + 2 < s.len() && s[i + 2] != ' ' && !after_word {
            match index_of(s, &s[i..i + 2], i + 2) {
                None => {
                    text.push(c);
                    text.push(c);
                    i += 2;
                }
                Some(end) => {
                    flush(&mut text, output);
                    inline(&s[i + 2..end], &Look { bold: true, ..look.clone() }, output);
                    i = end + 2;
                }
            }
        } else if (c == '*' || c == '_') && i + 1 < s.len() && s[i + 1] != ' ' && !after_word {
            match closing_single(s, c, i + 1) {
                None => {
                    text.push(c);
                    i += 1;
                }
                Some(end) => {
                    flush(&mut text, output);
                    inline(&s[i + 1..end], &Look { italic: true, ..look.clone() }, output);
                    i = end + 1;
                }
            }
        } else if c == '[' {
            let close = index_of(s, &[']'], i + 1);
            let end = close.filter(|&close| at(s, close as isize + 1) == '(').and_then(|close| index_of(s, &[')'], close + 2));
            match (close, end) {
                (Some(close), Some(end)) => {
                    flush(&mut text, output);
                    let address = text_of(&s[close + 2..end]).trim().to_string();
                    let inner = if opens(&address) { Look { link: Some(address), ..look.clone() } } else { look.clone() };
                    inline(&s[i + 1..close], &inner, output);
                    i = end + 1;
                }
                _ => {
                    text.push(c);
                    i += 1;
                }
            }
        } else {
            text.push(c);
            i += 1;
        }
    }
    flush(&mut text, output);
}

/// The spans of a text: split where the look changes, merged where it does not.
pub fn spans(text: &str) -> Vec<Span> {
    let chars: Vec<char> = text.chars().collect();
    let mut raw = Vec::new();
    inline(&chars, &Look::default(), &mut raw);
    let mut merged: Vec<Span> = Vec::new();
    for span in raw {
        match merged.last_mut() {
            Some(last) if last.looks_as(&span) => last.text.push_str(&span.text),
            _ => merged.push(span),
        }
    }
    merged
}

/// A heading line (already trimmed): its level and its text; none for another line.
fn heading(trimmed: &str) -> Option<(usize, &str)> {
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some((level, rest.trim()))
}

/// A list item line: its leading spaces, its number when it is numbered, and its text; none for another line.
fn item(line: &str) -> Option<(usize, Option<u64>, &str)> {
    let rest = line.trim_start();
    let leading = line[..line.len() - rest.len()].chars().count();
    let (number, after) = if let Some(after) = rest.strip_prefix(['-', '*', '+']) {
        (None, after)
    } else {
        let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        if !(1..=9).contains(&digits) {
            return None;
        }
        let after = rest[digits..].strip_prefix(['.', ')'])?;
        (rest[..digits].parse::<u64>().ok(), after)
    };
    if !after.starts_with(char::is_whitespace) {
        return None;
    }
    Some((leading, number, after.trim()))
}

/// A scene break: three or more of one of `*`, `-` and `_`, alone on the line (spaces between allowed).
fn rule(trimmed: &str) -> bool {
    let marks: Vec<char> = trimmed.chars().filter(|c| *c != ' ').collect();
    marks.len() >= 3 && matches!(marks[0], '*' | '-' | '_') && marks.iter().all(|c| *c == marks[0])
}

/// A quote line: its depth (`>` marks, spaces between allowed, at most 3) and its text after one space; none for another line.
fn quote(trimmed: &str) -> Option<(usize, &str)> {
    if !trimmed.starts_with('>') {
        return None;
    }
    let mut depth = 0;
    let mut rest = trimmed;
    while let Some(after) = rest.strip_prefix('>') {
        depth += 1;
        rest = after.strip_prefix(' ').filter(|a| a.starts_with('>')).unwrap_or(after);
    }
    Some((depth.min(3), rest.strip_prefix(' ').unwrap_or(rest).trim_end()))
}

/// Whether a line ends in a hard line break: two spaces, or a backslash.
fn breaks(line: &str) -> bool {
    line.ends_with("  ") || line.trim_end().ends_with('\\')
}

/// The blocks of a text in Markdown.
pub fn read(markdown: &str) -> Vec<Block> {
    let text = markdown.replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    let mut blocks: Vec<Block> = Vec::new();
    // The paragraph, item or quote that following lines join: where it is, and its text so far.
    let mut open: Option<usize> = None;
    let mut open_text = String::new();
    // Whether the last line joined ended in a hard line break.
    let mut broken = false;
    let close = |blocks: &mut Vec<Block>, open: &mut Option<usize>, open_text: &mut String| {
        if let Some(at) = open.take() {
            blocks[at].spans = spans(open_text);
        }
        open_text.clear();
    };
    // Joins `more` to the open block: with a line break after a line that asked for one, else a space.
    let join = |open_text: &mut String, more: &str, broken: bool| {
        if broken {
            if open_text.ends_with('\\') {
                open_text.pop();
            }
            open_text.push('\n');
        } else {
            open_text.push(' ');
        }
        open_text.push_str(more);
    };
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.starts_with(FENCE) {
            close(&mut blocks, &mut open, &mut open_text);
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with(FENCE) {
                code.push(lines[i]);
                i += 1;
            }
            blocks.push(Block { text: code.join("\n"), ..Block::new(BlockKind::Code) });
            i += 1;
            continue;
        }
        i += 1;
        let was_broken = std::mem::replace(&mut broken, breaks(line));
        if trimmed.is_empty() {
            close(&mut blocks, &mut open, &mut open_text);
            continue;
        }
        if let Some((level, text)) = heading(trimmed) {
            close(&mut blocks, &mut open, &mut open_text);
            blocks.push(Block { spans: spans(text), level, ..Block::new(BlockKind::Heading) });
            continue;
        }
        if rule(trimmed) {
            close(&mut blocks, &mut open, &mut open_text);
            blocks.push(Block::new(BlockKind::Rule));
            continue;
        }
        if let Some((depth, text)) = quote(trimmed) {
            let same = open.is_some_and(|at| blocks[at].kind == BlockKind::Quote && blocks[at].depth == depth);
            if text.is_empty() {
                // A `>` alone ends the quote's paragraph.
                close(&mut blocks, &mut open, &mut open_text);
            } else if same {
                join(&mut open_text, text, was_broken);
            } else {
                close(&mut blocks, &mut open, &mut open_text);
                blocks.push(Block { depth, ..Block::new(BlockKind::Quote) });
                open = Some(blocks.len() - 1);
                open_text.push_str(text);
            }
            continue;
        }
        if let Some((leading, number, text)) = item(line) {
            close(&mut blocks, &mut open, &mut open_text);
            blocks.push(Block { depth: (leading / 2).min(3), number, ..Block::new(BlockKind::Item) });
            open = Some(blocks.len() - 1);
            open_text.push_str(text);
            continue;
        }
        if open.is_some() {
            join(&mut open_text, trimmed, was_broken);
        } else {
            blocks.push(Block::new(BlockKind::Paragraph));
            open = Some(blocks.len() - 1);
            open_text.push_str(trimmed);
        }
    }
    close(&mut blocks, &mut open, &mut open_text);
    blocks
}
