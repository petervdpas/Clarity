//! CSV (RFC 4180), syntax only: fields separated by a comma (or the
//! `separator` given), quoted with `"` (or the `quote` given) where they
//! hold a separator, a quote or a line break, a quote inside doubled;
//! lines ended by CRLF or LF; a UTF-8 byte order mark dropped. Text comes
//! in UTF-8 unless `encoding` names `windows-1252` or `iso-8859-1`. The
//! first row names the columns unless `noHeader`; then the names given
//! (the entity's fields in order), else `column1` and on. As a document: a
//! list of rows, each row an object of its cells named by its column (an
//! empty field is null), the columns kept as the document's `columns`.

use crate::doc::{Document, Form, Node};
use crate::encoding::{self, Encoding};
use crate::format::Format;
use crate::tree::{self, Tree};
use crate::{FormatInfo, Nodes, OptionInfo, OptionKind, Problem, ProblemKind, Tables};

/// The options of a CSV reading or writing, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub separator: char,
    pub quote: char,
    pub header: bool,
    pub encoding: Encoding,
}

impl Options {
    /// The options from the names and values given; why not, when one does not fit.
    pub fn of(given: &[(String, Tree)]) -> Result<Options, String> {
        let text = |name: &str| given.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.as_text());
        let one = |name: &str, default: char| -> Result<char, String> {
            match text(name) {
                None => Ok(default),
                Some("\\t") => Ok('\t'),
                Some(t) => {
                    let mut chars = t.chars();
                    match (chars.next(), chars.next()) {
                        (Some(c), None) if c != '\r' && c != '\n' => Ok(c),
                        _ => Err(format!("the option '{name}' of csv is one character, not \"{t}\"")),
                    }
                }
            }
        };
        let separator = one("separator", ',')?;
        let quote = one("quote", '"')?;
        if separator == quote {
            return Err("the separator and the quote of csv are two different characters".into());
        }
        let header = !given.iter().any(|(n, v)| n == "noHeader" && *v == Tree::Bool(true));
        let encoding = match text("encoding") {
            None => Encoding::Utf8,
            Some(name) => Encoding::named(name).ok_or_else(|| format!("csv reads and writes utf-8, windows-1252 or iso-8859-1, not \"{}\"", name.to_ascii_lowercase()))?,
        };
        Ok(Options { separator, quote, header, encoding })
    }
}

/// The rows of a CSV text, each a list of fields; none when a quote is not closed.
pub fn rows(text: &str, separator: char, quote: char) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    // Whether the line has anything at all, so an empty last line is no row.
    let mut started = false;
    while let Some(c) = chars.next() {
        if quoted {
            if c == quote {
                if chars.peek() == Some(&quote) {
                    field.push(quote);
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            c if c == quote && field.is_empty() => {
                quoted = true;
                started = true;
            }
            c if c == separator => {
                row.push(std::mem::take(&mut field));
                started = true;
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                if started || !field.is_empty() {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                started = false;
            }
            c => {
                field.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return None;
    }
    if started || !field.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Some(rows)
}

/// A value as a CSV field: text as it is, a list as its JSON.
pub(crate) fn cell(value: &Tree) -> String {
    match value {
        Tree::Null => String::new(),
        Tree::Text(t) => t.clone(),
        Tree::Number(n) => n.clone(),
        Tree::Bool(b) => b.to_string(),
        other => tree::write(other),
    }
}

/// A field written: quoted when it holds the separator, the quote, a line
/// break, or space at either end, a quote inside doubled.
fn quoted(field: &str, options: &Options) -> String {
    let needs = field.contains(options.separator) || field.contains(options.quote) || field.contains(['\r', '\n']) || field.starts_with(' ') || field.ends_with(' ');
    if !needs {
        return field.to_string();
    }
    let q = options.quote.to_string();
    format!("{q}{}{q}", field.replace(options.quote, &format!("{q}{q}")))
}

/// CSV in the catalog.
pub struct Csv;

fn bad(text: String) -> Problem {
    Problem::new(ProblemKind::BadOptions, text)
}

impl Format for Csv {
    fn info(&self) -> FormatInfo {
        FormatInfo {
            name: "csv",
            title: "CSV",
            text: true,
            nests: false,
            nodes: Nodes::Rows,
            attributes: false,
            tables: Tables::One,
            path_separator: None,
            schema_kinds: &[],
            options: vec![
                OptionInfo { name: "separator", kind: OptionKind::Text, label: "formatOptionSeparator", reads: true, writes: true, required_writing: None },
                OptionInfo { name: "noHeader", kind: OptionKind::Bool, label: "formatOptionNoHeader", reads: true, writes: true, required_writing: None },
                OptionInfo { name: "quote", kind: OptionKind::Text, label: "formatOptionQuote", reads: true, writes: true, required_writing: None },
                OptionInfo { name: "encoding", kind: OptionKind::Text, label: "formatOptionEncoding", reads: true, writes: true, required_writing: None },
            ],
        }
    }

    fn parse(&self, input: &[u8], options: &[(String, Tree)], names: &[String]) -> Result<Document, Problem> {
        let csv = Options::of(options).map_err(bad)?;
        let no_csv = || Problem::new(ProblemKind::NotFitting, "the text is no CSV");
        let text = encoding::decode(input, csv.encoding).ok_or_else(no_csv)?;
        let mut lines = rows(&text, csv.separator, csv.quote).ok_or_else(no_csv)?;
        let columns: Vec<String> = if csv.header {
            if lines.is_empty() { Vec::new() } else { lines.remove(0).into_iter().map(|c| c.trim().to_string()).collect() }
        } else if !names.is_empty() {
            names.to_vec()
        } else {
            let width = lines.iter().map(Vec::len).max().unwrap_or(0);
            (1..=width).map(|i| format!("column{i}")).collect()
        };
        let mut top = Node::new("", Form::List);
        for line in lines {
            let mut row = Node::new("", Form::Object);
            let mut cells = line.into_iter();
            for column in &columns {
                let value = match cells.next() {
                    Some(v) if !v.is_empty() => Tree::Text(v),
                    _ => Tree::Null,
                };
                row.push(Node::value(column, value));
            }
            top.push(row);
        }
        Ok(Document { top, columns, kept: None })
    }

    /// A header row of the document's columns unless `noHeader`, then a row
    /// per row, its cells in the columns' order, lines ended by CRLF.
    fn write(&self, doc: &Document, options: &[(String, Tree)]) -> Result<Vec<u8>, Problem> {
        let csv = Options::of(options).map_err(bad)?;
        let sep = csv.separator.to_string();
        let mut out = String::new();
        if csv.header {
            out.push_str(&doc.columns.iter().map(|c| quoted(c, &csv)).collect::<Vec<_>>().join(&sep));
            out.push_str("\r\n");
        }
        for row in doc.top.nodes() {
            let cells: Vec<String> = row.nodes().map(|c| quoted(&cell(&c.to_tree()), &csv)).collect();
            out.push_str(&cells.join(&sep));
            out.push_str("\r\n");
        }
        encoding::encode(&out, csv.encoding).map_err(|text| Problem::new(ProblemKind::Failed, text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_read_quotes_doubled_quotes_and_line_breaks_inside_them() {
        let text = "a,b,c\r\n\"x, y\",\"say \"\"hi\"\"\",\"two\nlines\"\n1,,3\n";
        assert_eq!(rows(text, ',', '"').unwrap(), vec![vec!["a", "b", "c"], vec!["x, y", "say \"hi\"", "two\nlines"], vec!["1", "", "3"]]);
        assert_eq!(rows("a,b\n1,\n", ',', '"').unwrap(), vec![vec!["a", "b"], vec!["1", ""]], "an empty last field is a field");
        assert!(rows("a,\"open\n", ',', '"').is_none(), "a quote not closed");
        assert_eq!(rows("a;b\n\n1;2", ';', '"').unwrap(), vec![vec!["a", "b"], vec!["1", "2"]], "an empty line is no row, a last line without its break is one");
    }
}
