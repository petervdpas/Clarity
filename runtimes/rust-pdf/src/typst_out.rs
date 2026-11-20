//! Markdown's blocks written as Typst code in a stylesheet's look. Every
//! text goes in as a Typst string, so nothing in it is read as Typst; every
//! look comes from the stylesheet, so all spacing between blocks is theirs
//! (Typst's own is set to nothing).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use clarity_runtime::markdown::{self, Block, BlockKind, ColumnAlign, Span};

use crate::style::{Align, Element, Len, Marker, Piece, Style, Stylesheet};

/// Where MiTeX's definitions lie for Typst (`mitex/`, carried in the crate).
pub(crate) const MITEX: [(&str, &str); 4] = [
    ("/mitex/mod.typ", include_str!("../mitex/mod.typ")),
    ("/mitex/prelude.typ", include_str!("../mitex/prelude.typ")),
    ("/mitex/latex/standard.typ", include_str!("../mitex/latex/standard.typ")),
    ("/mitex/xarrow.typ", include_str!("../mitex/xarrow.typ")),
];

/// The formulas of a Markdown text, as their TeX: math blocks and inline math, in tables too.
pub(crate) fn formulas(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut spans_of = |spans: &[Span]| out.extend(spans.iter().filter(|s| s.math).map(|s| s.text.clone()));
    let blocks = markdown::read(text);
    for block in &blocks {
        spans_of(&block.spans);
        for cell in block.header.iter().chain(block.rows.iter().flatten()) {
            spans_of(cell);
        }
    }
    out.extend(blocks.iter().filter(|b| b.kind == BlockKind::Math).map(|b| b.text.clone()));
    out
}

/// A Typst source that sets one formula, to see whether it can be set.
pub(crate) fn formula_alone(tex: &str) -> Option<String> {
    let code = mitex::convert_math(tex, None).ok()?;
    Some(format!("{IMPORT}#math.equation(block: true, eval({}, mode: \"math\", scope: mitex-scope))\n", string(&code)))
}

const IMPORT: &str = "#import \"/mitex/mod.typ\": mitex-scope\n";

/// The Typst source of a Markdown text in a stylesheet's look; the formulas
/// in `broken` (which Typst cannot set) show their TeX as code does.
pub(crate) fn source(text: &str, sheet: &Stylesheet, broken: &BTreeSet<String>) -> String {
    let mut out = String::new();
    if !formulas(text).is_empty() {
        out.push_str(IMPORT);
    }
    out.push_str("#show math.equation: set text(font: \"New Computer Modern Math\")\n");
    page(&mut out, sheet);
    looks(&mut out, sheet);
    out.push_str(&blocks(text, sheet, broken));
    out
}

/// The import of MiTeX's definitions, for a text with formulas, and the font of math.
pub(crate) fn math_start(any: bool) -> String {
    let import = if any { IMPORT } else { "" };
    format!("{import}#show math.equation: set text(font: \"New Computer Modern Math\")\n")
}

/// The stylesheet's looks for everything after the page: no spacing of
/// Typst's own, the body's text, the headings' sizes.
pub(crate) fn looks(out: &mut String, sheet: &Stylesheet) {
    out.push_str("#set block(spacing: 0pt)\n#set par(spacing: 0pt)\n");
    out.push_str("#set text(top-edge: \"ascender\", bottom-edge: \"descender\")\n");
    let body = sheet.style(Element::Body);
    out.push_str(&settings(&body));
    let body_size = match body.size {
        Some(Len::Pt(pt)) => pt,
        Some(Len::Em(em)) => em * 11.0,
        Some(Len::Percent(p)) => p / 100.0 * 11.0,
        None => 11.0,
    };
    // A heading is laid out as its text, in its level's look; it stays a
    // heading for the PDF's outline.
    out.push_str("#show heading: it => it.body\n");
    for level in 1..=6u8 {
        let mut style = sheet.style(Element::Heading(level));
        style.size = Some(Len::Pt(match style.size {
            Some(Len::Pt(pt)) => pt,
            Some(Len::Em(em)) => em * body_size,
            Some(Len::Percent(p)) => p / 100.0 * body_size,
            None => body_size,
        }));
        style.weight = style.weight.or(body.weight).or(Some(400));
        let args = text_args(&style);
        let _ = writeln!(out, "#show heading.where(level: {level}): set text({args})");
    }
    out.push('\n');
}

/// A Markdown text's blocks as Typst code in a stylesheet's look.
pub(crate) fn blocks(text: &str, sheet: &Stylesheet, broken: &BTreeSet<String>) -> String {
    let mut out = String::new();
    let writer = Writer { sheet, broken };
    for block in markdown::read(text) {
        writer.block(&mut out, &block);
        out.push('\n');
    }
    out
}

fn page(out: &mut String, sheet: &Stylesheet) {
    let p = &sheet.page;
    let [top, right, bottom, left] = p.margin;
    let _ = write!(out, "#set page(width: {}mm, height: {}mm, margin: (top: {top}mm, right: {right}mm, bottom: {bottom}mm, left: {left}mm)", p.width, p.height);
    for (name, places) in [("header", ["@top-left", "@top-center", "@top-right"]), ("footer", ["@bottom-left", "@bottom-center", "@bottom-right"])] {
        if places.iter().all(|place| p.boxes.get(place).is_none_or(|b| b.content.is_empty())) {
            continue;
        }
        let cells: Vec<String> = places
            .iter()
            .zip(["left", "center", "right"])
            .map(|(place, side)| {
                let Some(margin_box) = p.boxes.get(place) else { return format!("align({side})[]") };
                let mut cell = settings(&margin_box.style);
                for piece in &margin_box.content {
                    match piece {
                        Piece::Text(t) => {
                            let _ = write!(cell, "#{};", string(t));
                        }
                        Piece::Page => cell.push_str("#counter(page).display();"),
                        Piece::Pages => cell.push_str("#str(counter(page).final().first());"),
                    }
                }
                format!("align({side})[{cell}]")
            })
            .collect();
        let _ = write!(out, ", {name}: context grid(columns: (1fr, 1fr, 1fr), {})", cells.join(", "));
    }
    out.push_str(")\n");
}

/// The Typst `text` arguments of what a style sets.
pub(crate) fn text_args(style: &Style) -> String {
    let mut args = Vec::new();
    if let Some(font) = &style.font {
        args.push(format!("font: {}", string(font)));
    }
    if let Some(size) = style.size {
        args.push(format!("size: {}", size.typst()));
    }
    if let Some(weight) = style.weight {
        args.push(format!("weight: {weight}"));
    }
    if let Some(italic) = style.italic {
        args.push(format!("style: \"{}\"", if italic { "italic" } else { "normal" }));
    }
    if let Some(color) = style.color {
        args.push(format!("fill: {}", color.typst()));
    }
    args.join(", ")
}

/// The set rules of a style's text, its lines and its alignment.
pub(crate) fn settings(style: &Style) -> String {
    let mut out = String::new();
    let args = text_args(style);
    if !args.is_empty() {
        let _ = writeln!(out, "#set text({args})");
    }
    if let Some(height) = style.line_height {
        // The gap between lines, from the font's ascender and descender (about 1.2em in all).
        let _ = writeln!(out, "#set par(leading: {}em)", ((height - 1.2).max(0.0) * 1000.0).round() / 1000.0);
    }
    match style.align {
        Some(Align::Justify) => out.push_str("#set par(justify: true)\n#set align(left)\n"),
        Some(Align::Left) => out.push_str("#set par(justify: false)\n#set align(left)\n"),
        Some(Align::Center) => out.push_str("#set par(justify: false)\n#set align(center)\n"),
        Some(Align::Right) => out.push_str("#set par(justify: false)\n#set align(right)\n"),
        None => {}
    }
    out
}

/// The arguments of the block a style puts around its element.
fn block_args(style: &Style) -> String {
    let mut args = vec![
        format!("above: {}", style.margin_top.unwrap_or(Len::Pt(0.0)).typst()),
        format!("below: {}", style.margin_bottom.unwrap_or(Len::Pt(0.0)).typst()),
    ];
    let border_left = style.border_left.flatten();
    let border_top = style.border_top.flatten();
    if let Some(width) = style.width {
        args.push(format!("width: {}", width.typst()));
    } else if style.background.is_some() || border_left.is_some() || border_top.is_some() {
        args.push("width: 100%".into());
    }
    let pads: Vec<String> = ["top", "right", "bottom", "left"].iter().zip(style.padding).filter_map(|(side, len)| len.map(|l| format!("{side}: {}", l.typst()))).collect();
    if !pads.is_empty() {
        args.push(format!("inset: ({})", pads.join(", ")));
    }
    if let Some(fill) = style.background {
        args.push(format!("fill: {}", fill.typst()));
    }
    if let Some(radius) = style.radius {
        args.push(format!("radius: {}", radius.typst()));
    }
    let strokes: Vec<String> = [("left", border_left), ("top", border_top)].iter().filter_map(|(side, b)| b.map(|b| format!("{side}: {}", b.typst()))).collect();
    if !strokes.is_empty() {
        args.push(format!("stroke: ({})", strokes.join(", ")));
    }
    args.join(", ")
}

/// An element's block: its text settings around a block of its look holding `inner`.
fn styled_block(out: &mut String, styles: &[&Style], block_style: &Style, inner: &str) {
    out.push_str("#[\n");
    for style in styles {
        out.push_str(&settings(style));
    }
    let inner = if block_style.underline == Some(true) { format!("#underline[{inner}]") } else { inner.to_string() };
    let _ = writeln!(out, "#block({})[{inner}]", block_args(block_style));
    out.push_str("]\n");
}

/// Writes blocks in a stylesheet's look, with the formulas Typst cannot set as code.
struct Writer<'a> {
    sheet: &'a Stylesheet,
    broken: &'a BTreeSet<String>,
}

impl Writer<'_> {
    fn block(&self, out: &mut String, block: &Block) {
        let sheet = self.sheet;
        let spans = |list: &[Span]| self.spans(list);
        match block.kind {
            BlockKind::Heading => {
                let level = block.level.clamp(1, 6) as u8;
                let style = sheet.style(Element::Heading(level));
                let mut aligned = Style { align: style.align, ..Style::default() };
                aligned.line_height = style.line_height;
                let inner = format!("#heading(level: {level})[{}]", spans(&block.spans));
                styled_block(out, &[&aligned], &style, &inner);
            }
            BlockKind::Paragraph => {
                let style = sheet.style(Element::P);
                styled_block(out, &[&style], &style, &spans(&block.spans));
            }
            BlockKind::Item => {
                let list = sheet.style(if block.number.is_some() { Element::Ol } else { Element::Ul });
                let item = sheet.style(Element::Li);
                let marker = list.marker.unwrap_or(if block.number.is_some() { Marker::Decimal } else { Marker::Disc });
                let indent = list.padding[3].unwrap_or(Len::Em(1.25)).typst();
                let inner = format!(
                    "#pad(left: {} * {indent})[#grid(columns: ({indent}, 1fr), [#{};], [{}])]",
                    block.depth,
                    string(&marker.text(block.number.unwrap_or(1))),
                    spans(&block.spans)
                );
                let text_only = |s: &Style| Style { font: s.font.clone(), size: s.size, weight: s.weight, italic: s.italic, color: s.color, line_height: s.line_height, align: s.align, ..Style::default() };
                styled_block(out, &[&text_only(&list), &text_only(&item)], &item, &inner);
            }
            BlockKind::Quote => {
                let style = sheet.style(Element::Blockquote);
                let indent = style.padding[3].unwrap_or(Len::Em(0.9)).typst();
                let inner = spans(&block.spans);
                let mut wrapped = String::new();
                styled_block(&mut wrapped, &[&style], &style, &inner);
                if block.depth > 1 {
                    let _ = writeln!(out, "#pad(left: {} * {indent})[{wrapped}]", block.depth - 1);
                } else {
                    out.push_str(&wrapped);
                }
            }
            BlockKind::Rule => {
                let style = sheet.style(Element::Hr);
                let line = match style.border_top.flatten() {
                    Some(border) => format!("#block(width: 100%)[#align(center)[#line(length: {}, stroke: {})]]", style.width.unwrap_or(Len::Percent(100.0)).typst(), border.typst()),
                    None => String::new(),
                };
                let space = Style { margin_top: style.margin_top, margin_bottom: style.margin_bottom, ..Style::default() };
                styled_block(out, &[], &space, &line);
            }
            BlockKind::Code => {
                let style = sheet.style(Element::Pre);
                let lines: Vec<String> = block.text.split('\n').map(|line| format!("#{};", string(&kept_spaces(line)))).collect();
                let mut text = style.clone();
                text.align = Some(text.align.unwrap_or(Align::Left));
                styled_block(out, &[&text], &style, &lines.join("#linebreak();"));
            }
            BlockKind::Math => match self.formula(&block.text, true) {
                Some(formula) => {
                    let p = sheet.style(Element::P);
                    let space = Style { margin_top: p.margin_top, margin_bottom: p.margin_bottom, width: Some(Len::Percent(100.0)), ..Style::default() };
                    styled_block(out, &[], &space, &formula);
                }
                None => self.block(out, &Block { kind: BlockKind::Code, ..block.clone() }),
            },
            BlockKind::Table => self.table(out, block),
        }
    }

    /// A formula set by Typst from its TeX; none when it cannot be set.
    fn formula(&self, tex: &str, block: bool) -> Option<String> {
        if self.broken.contains(tex) {
            return None;
        }
        let code = mitex::convert_math(tex, None).ok()?;
        Some(format!("#math.equation(block: {block}, eval({}, mode: \"math\", scope: mitex-scope));", string(&code)))
    }

    /// A table: a header repeated on every page it runs over, each column aligned
    /// as its delimiter says (else as `th` and `td` do), cells in their look.
    fn table(&self, out: &mut String, block: &Block) {
        let sheet = self.sheet;
        let (table, th, td) = (sheet.style(Element::Table), sheet.style(Element::Th), sheet.style(Element::Td));
        let columns = block.align.len().max(1);
        let align_of = |style: &Style, column: ColumnAlign| -> &'static str {
            match column {
                ColumnAlign::Left => "left",
                ColumnAlign::Center => "center",
                ColumnAlign::Right => "right",
                ColumnAlign::None => match style.align {
                    Some(Align::Center) => "center",
                    Some(Align::Right) => "right",
                    _ => "left",
                },
            }
        };
        let aligns = |style: &Style| -> String { format!("({},)", block.align.iter().map(|a| align_of(style, *a)).collect::<Vec<_>>().join(", ")) };
        let stroke = |style: &Style| style.border_bottom.flatten().map(|b| format!("(bottom: {})", b.typst())).unwrap_or_else(|| "none".into());
        let fill = |style: &Style| style.background.map(|c| c.typst()).unwrap_or_else(|| "none".into());
        let inset = |style: &Style| {
            let sides: Vec<String> = ["top", "right", "bottom", "left"].iter().zip(style.padding).map(|(side, len)| format!("{side}: {}", len.unwrap_or(Len::Pt(0.0)).typst())).collect();
            format!("({})", sides.join(", "))
        };
        let track = if table.width.is_some() { "1fr" } else { "auto" };
        let cell = |style: &Style, spans: &[Span]| wrap_text(style, self.spans(spans));
        let header: Vec<String> = block.header.iter().map(|c| format!("[{}]", cell(&th, c))).collect();
        let body: Vec<String> = block.rows.iter().flatten().map(|c| format!("[{}]", cell(&td, c))).collect();
        let mut inner = format!(
            "#table(columns: ({}), align: (x, y) => (if y == 0 {{ {} }} else {{ {} }}).at(x), stroke: (x, y) => if y == 0 {{ {} }} else {{ {} }}, fill: (x, y) => if y == 0 {{ {} }} else {{ {} }}, inset: (x, y) => if y == 0 {{ {} }} else {{ {} }}, table.header({})",
            vec![track; columns].join(", ") + if columns == 1 { "," } else { "" },
            aligns(&th),
            aligns(&td),
            stroke(&th),
            stroke(&td),
            fill(&th),
            fill(&td),
            inset(&th),
            inset(&td),
            header.join(", "),
        );
        for c in body {
            inner.push_str(", ");
            inner.push_str(&c);
        }
        inner.push(')');
        styled_block(out, &[&table], &table, &inner);
    }


    /// A block's spans as Typst markup, each in its look.
    fn spans(&self, spans: &[Span]) -> String {
        let sheet = self.sheet;
        let mut out = String::new();
        for span in spans {
            let formula = if span.math { self.formula(&span.text, false) } else { None };
            let mut m = formula.clone().unwrap_or_else(|| format!("#{};", string(&span.text)));
            if span.code || (span.math && formula.is_none()) {
                let code = sheet.style(Element::Code);
                m = wrap_text(&code, m);
                let [top, right, bottom, left] = code.padding;
                let mut args = Vec::new();
                if let Some(fill) = code.background {
                    args.push(format!("fill: {}", fill.typst()));
                }
                if let Some(radius) = code.radius {
                    args.push(format!("radius: {}", radius.typst()));
                }
                let len = |l: Option<Len>| l.unwrap_or(Len::Pt(0.0)).typst();
                args.push(format!("inset: (left: {}, right: {})", len(left), len(right)));
                args.push(format!("outset: (top: {}, bottom: {})", len(top), len(bottom)));
                m = format!("#box({})[{m}];", args.join(", "));
            }
            if span.italic {
                m = wrap_text(&sheet.style(Element::Em), m);
            }
            if span.bold {
                m = wrap_text(&sheet.style(Element::Strong), m);
            }
            if let Some(link) = &span.link {
                let a = sheet.style(Element::A);
                if a.underline == Some(true) {
                    m = format!("#underline[{m}];");
                }
                m = wrap_text(&a, m);
                m = format!("#link({})[{m}];", string(link));
            }
            out.push_str(&m);
        }
        out
    }
}

/// A line of code with its leading spaces kept: they are no-break spaces, which a line does not drop.
fn kept_spaces(line: &str) -> String {
    let trimmed = line.trim_start_matches(' ');
    format!("{}{trimmed}", "\u{a0}".repeat(line.len() - trimmed.len()))
}

fn wrap_text(style: &Style, markup: String) -> String {
    let args = text_args(style);
    if args.is_empty() { markup } else { format!("#text({args})[{markup}];") }
}

/// A text as a Typst string, with its quotes.
pub(crate) fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
