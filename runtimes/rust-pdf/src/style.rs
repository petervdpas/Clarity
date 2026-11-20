//! A stylesheet read and cascaded: Clarity's own sheet first, the given one
//! over it, a later declaration winning per property. What each element and
//! the page are given is kept as it was written (an `em` stays an `em`), and
//! Typst carries the inheritance: a block's text settings are set inside the
//! body's.

use std::collections::BTreeMap;

use crate::css::{self, Block, Decl};
use crate::fonts;
use crate::theme::Theme;
use crate::{CssError, CssErrorKind};

/// Clarity's own stylesheet.
pub(crate) const CLARITY: &str = include_str!("clarity.css");

/// What a stylesheet can style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Element {
    Body,
    Heading(u8),
    P,
    Ul,
    Ol,
    Li,
    Blockquote,
    Pre,
    Code,
    A,
    Hr,
    Strong,
    Em,
    Table,
    Th,
    Td,
}

impl Element {
    fn named(name: &str) -> Option<Element> {
        Some(match name {
            "body" => Element::Body,
            "h1" => Element::Heading(1),
            "h2" => Element::Heading(2),
            "h3" => Element::Heading(3),
            "h4" => Element::Heading(4),
            "h5" => Element::Heading(5),
            "h6" => Element::Heading(6),
            "p" => Element::P,
            "ul" => Element::Ul,
            "ol" => Element::Ol,
            "li" => Element::Li,
            "blockquote" => Element::Blockquote,
            "pre" => Element::Pre,
            "code" => Element::Code,
            "a" => Element::A,
            "hr" => Element::Hr,
            "strong" => Element::Strong,
            "em" => Element::Em,
            "table" => Element::Table,
            "th" => Element::Th,
            "td" => Element::Td,
            _ => return None,
        })
    }
}

/// A length as written.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Len {
    Pt(f64),
    Em(f64),
    Percent(f64),
}

impl Len {
    /// As Typst writes it.
    pub fn typst(self) -> String {
        match self {
            Len::Pt(v) => format!("{}pt", round(v)),
            Len::Em(v) => format!("{}em", round(v)),
            Len::Percent(v) => format!("{}%", round(v)),
        }
    }
}

fn round(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

/// A color, with its opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn typst(self) -> String {
        if self.a == 255 {
            format!("rgb(\"#{:02x}{:02x}{:02x}\")", self.r, self.g, self.b)
        } else {
            format!("rgb(\"#{:02x}{:02x}{:02x}{:02x}\")", self.r, self.g, self.b, self.a)
        }
    }
}

/// A border line: its thickness, how it is drawn and its color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Border {
    pub width: Len,
    pub dash: Option<&'static str>,
    pub color: Color,
}

impl Border {
    pub fn typst(self) -> String {
        match self.dash {
            Some(dash) => format!("(thickness: {}, paint: {}, dash: \"{dash}\")", self.width.typst(), self.color.typst()),
            None => format!("{} + {}", self.width.typst(), self.color.typst()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    Left,
    Center,
    Right,
    Justify,
}

/// A list's markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Marker {
    Disc,
    Circle,
    Square,
    None,
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

impl Marker {
    /// The marker of an item, with its number for a numbered one.
    pub fn text(self, number: u64) -> String {
        match self {
            Marker::Disc => "\u{2022}".into(),
            Marker::Circle => "\u{25e6}".into(),
            Marker::Square => "\u{25aa}".into(),
            Marker::None => String::new(),
            Marker::Decimal => format!("{number}."),
            Marker::LowerAlpha => format!("{}.", alpha(number)),
            Marker::UpperAlpha => format!("{}.", alpha(number).to_uppercase()),
            Marker::LowerRoman => format!("{}.", roman(number)),
            Marker::UpperRoman => format!("{}.", roman(number).to_uppercase()),
        }
    }
}

fn alpha(mut n: u64) -> String {
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    out.iter().rev().collect()
}

fn roman(mut n: u64) -> String {
    const NUMERALS: [(u64, &str); 13] =
        [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")];
    let mut out = String::new();
    for (value, numeral) in NUMERALS {
        while n >= value {
            out.push_str(numeral);
            n -= value;
        }
    }
    out
}

/// What a stylesheet gives an element; what it leaves out it inherits or goes without.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Style {
    pub font: Option<String>,
    pub size: Option<Len>,
    pub weight: Option<u16>,
    pub italic: Option<bool>,
    pub color: Option<Color>,
    pub background: Option<Color>,
    pub line_height: Option<f64>,
    pub align: Option<Align>,
    pub underline: Option<bool>,
    pub margin_top: Option<Len>,
    pub margin_bottom: Option<Len>,
    /// Top, right, bottom, left.
    pub padding: [Option<Len>; 4],
    pub border_left: Option<Option<Border>>,
    pub border_top: Option<Option<Border>>,
    pub border_bottom: Option<Option<Border>>,
    pub radius: Option<Len>,
    pub width: Option<Len>,
    pub marker: Option<Marker>,
}

/// What a margin box of the page shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Piece {
    Text(String),
    Page,
    Pages,
}

/// A margin box of the page.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MarginBox {
    pub content: Vec<Piece>,
    pub style: Style,
}

/// Where a margin box sits: at the top or bottom, then left, centre or right.
pub(crate) const BOXES: [&str; 6] = ["@top-left", "@top-center", "@top-right", "@bottom-left", "@bottom-center", "@bottom-right"];

/// The page: its size and margins in millimetres, and its margin boxes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Page {
    pub width: f64,
    pub height: f64,
    /// Top, right, bottom, left.
    pub margin: [f64; 4],
    pub boxes: BTreeMap<&'static str, MarginBox>,
}

/// A stylesheet over Clarity's own, ready to lay out with.
#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub(crate) page: Page,
    pub(crate) styles: BTreeMap<Element, Style>,
    /// The classes a report's parts name (`.title`), by name.
    pub(crate) classes: BTreeMap<String, Style>,
}

impl Default for Stylesheet {
    /// Clarity's own stylesheet with no theme.
    fn default() -> Self {
        Stylesheet::read("", None).expect("Clarity's own stylesheet reads")
    }
}

impl Stylesheet {
    /// Reads a stylesheet over Clarity's own; the theme, when given, fills its `var(--role)`s.
    pub fn read(css: &str, theme: Option<&Theme>) -> Result<Stylesheet, Vec<CssError>> {
        let empty = Theme::default();
        let mut cascade = Cascade {
            theme: theme.unwrap_or(&empty),
            sheet: Stylesheet { page: Page { width: 210.0, height: 297.0, margin: [25.0; 4], boxes: BTreeMap::new() }, styles: BTreeMap::new(), classes: BTreeMap::new() },
            errors: Vec::new(),
        };
        cascade.apply(CLARITY);
        assert!(cascade.errors.is_empty(), "Clarity's own stylesheet: {:?}", cascade.errors);
        cascade.apply(css);
        if cascade.errors.is_empty() { Ok(cascade.sheet) } else { Err(cascade.errors) }
    }

    /// The page's width and height in millimetres.
    pub fn page_size(&self) -> (f64, f64) {
        (self.page.width, self.page.height)
    }

    pub(crate) fn style(&self, element: Element) -> Style {
        self.styles.get(&element).cloned().unwrap_or_default()
    }

    /// The style of the class `name` (`.title`), when the stylesheet has it.
    pub(crate) fn class(&self, name: &str) -> Option<&Style> {
        self.classes.get(name)
    }
}

struct Cascade<'a> {
    theme: &'a Theme,
    sheet: Stylesheet,
    errors: Vec<CssError>,
}

fn err(line: usize, kind: CssErrorKind, subject: impl Into<String>) -> CssError {
    CssError { line, kind, subject: subject.into() }
}

impl Cascade<'_> {
    fn apply(&mut self, css: &str) {
        let (blocks, errors) = css::parse(css);
        self.errors.extend(errors);
        for block in blocks {
            if block.prelude == "@page" {
                self.page(&block);
                continue;
            }
            let mut elements = Vec::new();
            let mut classes = Vec::new();
            for name in block.prelude.split(',').map(str::trim) {
                match (Element::named(name), name.strip_prefix('.')) {
                    (Some(element), _) => elements.push(element),
                    // A class a report's part names: letters, digits, `-` and `_`.
                    (None, Some(class)) if !class.is_empty() && class.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') => classes.push(class.to_string()),
                    _ => self.errors.push(err(block.line, CssErrorKind::UnknownSelector, name)),
                }
            }
            if let Some(nested) = block.nested.first() {
                self.errors.push(err(nested.line, CssErrorKind::UnknownSelector, nested.prelude.clone()));
            }
            for decl in &block.decls {
                let Some(value) = self.value(decl) else { continue };
                for &element in &elements {
                    let mut style = self.sheet.styles.remove(&element).unwrap_or_default();
                    if let Err(e) = declare(&mut style, element_allows(element), decl, &value) {
                        self.errors.push(e);
                    }
                    self.sheet.styles.insert(element, style);
                }
                for class in &classes {
                    let mut style = self.sheet.classes.remove(class).unwrap_or_default();
                    if let Err(e) = declare(&mut style, Allows::Element { lists: false, width: false }, decl, &value) {
                        self.errors.push(e);
                    }
                    self.sheet.classes.insert(class.clone(), style);
                }
                if elements.is_empty() && classes.is_empty() {
                    // The declarations of an unknown selector are still checked.
                    let _ = declare(&mut Style::default(), Allows::Element { lists: true, width: true }, decl, &value).map_err(|e| self.errors.push(e));
                }
            }
        }
    }

    /// A declaration's value with its `var()`s filled in.
    fn value(&mut self, decl: &Decl) -> Option<String> {
        let mut value = decl.value.clone();
        for _ in 0..16 {
            let Some(start) = value.find("var(") else { return Some(value) };
            let inner_start = start + 4;
            let mut depth = 1;
            let mut end = None;
            for (i, c) in value[inner_start..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(inner_start + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else {
                self.errors.push(err(decl.line, CssErrorKind::BadValue, format!("{}: {}", decl.name, decl.value)));
                return None;
            };
            let inner = &value[inner_start..end];
            let (name, fallback) = match inner.split_once(',') {
                Some((n, f)) => (n.trim(), Some(f.trim())),
                None => (inner.trim(), None),
            };
            let replacement = match (name.strip_prefix("--").and_then(|n| self.theme.value(n)), fallback) {
                (Some(v), _) => v.to_string(),
                (None, Some(f)) => f.to_string(),
                (None, None) => {
                    self.errors.push(err(decl.line, CssErrorKind::UnknownVariable, name));
                    return None;
                }
            };
            value = format!("{}{}{}", &value[..start], replacement, &value[end + 1..]);
        }
        Some(value)
    }

    fn page(&mut self, block: &Block) {
        for decl in &block.decls {
            let Some(value) = self.value(decl) else { continue };
            let bad = || err(decl.line, CssErrorKind::BadValue, format!("{}: {}", decl.name, decl.value));
            match decl.name.as_str() {
                "size" => match paper(&value) {
                    Some((w, h)) => (self.sheet.page.width, self.sheet.page.height) = (w, h),
                    None => self.errors.push(bad()),
                },
                "margin" => match sides(&value).and_then(|s| s.map(mm).into_iter().collect::<Option<Vec<_>>>()) {
                    Some(m) => self.sheet.page.margin = [m[0], m[1], m[2], m[3]],
                    None => self.errors.push(bad()),
                },
                name => self.errors.push(err(decl.line, CssErrorKind::UnknownProperty, name)),
            }
        }
        for nested in &block.nested {
            let Some(&place) = BOXES.iter().find(|b| **b == nested.prelude) else {
                self.errors.push(err(nested.line, CssErrorKind::UnknownSelector, nested.prelude.clone()));
                continue;
            };
            let mut margin_box = self.sheet.page.boxes.remove(place).unwrap_or_default();
            for decl in &nested.decls {
                let Some(value) = self.value(decl) else { continue };
                if decl.name == "content" {
                    match content(&value) {
                        Some(c) => margin_box.content = c,
                        None => self.errors.push(err(decl.line, CssErrorKind::BadValue, format!("content: {}", decl.value))),
                    }
                } else if let Err(e) = declare(&mut margin_box.style, Allows::MarginBox, decl, &value) {
                    self.errors.push(e);
                }
            }
            self.sheet.page.boxes.insert(place, margin_box);
        }
    }
}

/// Which properties a selector takes beyond the text ones.
#[derive(Clone, Copy)]
enum Allows {
    Element { lists: bool, width: bool },
    MarginBox,
}

fn element_allows(element: Element) -> Allows {
    Allows::Element { lists: matches!(element, Element::Ul | Element::Ol), width: matches!(element, Element::Hr | Element::Table) }
}

/// Sets one declaration on a style.
fn declare(style: &mut Style, allows: Allows, decl: &Decl, value: &str) -> Result<(), CssError> {
    let bad = || err(decl.line, CssErrorKind::BadValue, format!("{}: {}", decl.name, decl.value));
    let value = value.trim();
    match decl.name.as_str() {
        "font-family" => {
            style.font = Some(fonts::family(value).ok_or_else(|| err(decl.line, CssErrorKind::NoCarriedFont, value))?);
        }
        "font-size" => style.size = Some(length(value).ok_or_else(bad)?),
        "font-weight" => {
            style.weight = Some(match value {
                "normal" => 400,
                "bold" => 700,
                n => n.parse::<u16>().ok().filter(|n| (100..=900).contains(n) && n % 100 == 0).ok_or_else(bad)?,
            })
        }
        "font-style" => {
            style.italic = Some(match value {
                "normal" => false,
                "italic" | "oblique" => true,
                _ => return Err(bad()),
            })
        }
        "color" => style.color = Some(color(value).ok_or_else(bad)?),
        "text-align" => {
            style.align = Some(match value {
                "left" | "start" => Align::Left,
                "center" => Align::Center,
                "right" | "end" => Align::Right,
                "justify" => Align::Justify,
                _ => return Err(bad()),
            })
        }
        name if matches!(allows, Allows::MarginBox) => return Err(err(decl.line, CssErrorKind::UnknownProperty, name)),
        "background-color" | "background" => style.background = Some(color(value).ok_or_else(bad)?),
        "line-height" => {
            style.line_height = Some(match value {
                "normal" => 1.2,
                n => n.parse::<f64>().ok().filter(|n| *n > 0.0).ok_or_else(bad)?,
            })
        }
        "text-decoration" => {
            style.underline = Some(match value {
                "none" => false,
                "underline" => true,
                _ => return Err(bad()),
            })
        }
        "margin-top" => style.margin_top = Some(length(value).ok_or_else(bad)?),
        "margin-bottom" => style.margin_bottom = Some(length(value).ok_or_else(bad)?),
        "margin" => {
            let s = sides(value).ok_or_else(bad)?;
            (style.margin_top, style.margin_bottom) = (Some(s[0]), Some(s[2]));
        }
        "padding" => style.padding = sides(value).ok_or_else(bad)?.map(Some),
        "padding-left" => style.padding[3] = Some(length(value).ok_or_else(bad)?),
        "border-left" => style.border_left = Some(border(value).ok_or_else(bad)?),
        "border-top" => style.border_top = Some(border(value).ok_or_else(bad)?),
        "border-bottom" => style.border_bottom = Some(border(value).ok_or_else(bad)?),
        "border-radius" => style.radius = Some(length(value).ok_or_else(bad)?),
        "width" if matches!(allows, Allows::Element { width: true, .. }) => style.width = Some(length(value).ok_or_else(bad)?),
        "list-style-type" if matches!(allows, Allows::Element { lists: true, .. }) => {
            style.marker = Some(match value {
                "disc" => Marker::Disc,
                "circle" => Marker::Circle,
                "square" => Marker::Square,
                "none" => Marker::None,
                "decimal" => Marker::Decimal,
                "lower-alpha" | "lower-latin" => Marker::LowerAlpha,
                "upper-alpha" | "upper-latin" => Marker::UpperAlpha,
                "lower-roman" => Marker::LowerRoman,
                "upper-roman" => Marker::UpperRoman,
                _ => return Err(bad()),
            })
        }
        name => return Err(err(decl.line, CssErrorKind::UnknownProperty, name)),
    }
    Ok(())
}

/// A length: a number with pt, px, mm, cm, in, em or %, or 0.
pub(crate) fn length(text: &str) -> Option<Len> {
    let text = text.trim();
    if text == "0" {
        return Some(Len::Pt(0.0));
    }
    let split = text.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))?;
    let (number, unit) = text.split_at(split);
    let n: f64 = number.parse().ok()?;
    Some(match unit {
        "pt" => Len::Pt(n),
        "px" => Len::Pt(n * 0.75),
        "mm" => Len::Pt(n * 72.0 / 25.4),
        "cm" => Len::Pt(n * 720.0 / 25.4),
        "in" => Len::Pt(n * 72.0),
        "em" => Len::Em(n),
        "%" => Len::Percent(n),
        _ => return None,
    })
}

/// A length in millimetres; ems and percentages have none on paper.
pub(crate) fn mm(len: Len) -> Option<f64> {
    match len {
        Len::Pt(pt) => Some(pt * 25.4 / 72.0),
        _ => None,
    }
}

/// One to four lengths as top, right, bottom, left.
fn sides(text: &str) -> Option<[Len; 4]> {
    let l: Vec<Len> = text.split_whitespace().map(length).collect::<Option<_>>()?;
    Some(match l.as_slice() {
        [a] => [*a; 4],
        [v, h] => [*v, *h, *v, *h],
        [t, h, b] => [*t, *h, *b, *h],
        [t, r, b, l] => [*t, *r, *b, *l],
        _ => return None,
    })
}

/// `@page`'s size: a paper's name or two lengths, then `portrait` or `landscape`.
pub(crate) fn paper(text: &str) -> Option<(f64, f64)> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let (size, turn) = match words.as_slice() {
        [rest @ .., "landscape"] => (rest, true),
        [rest @ .., "portrait"] => (rest, false),
        rest => (rest, false),
    };
    let (w, h) = match size {
        [name] => match name.to_ascii_lowercase().as_str() {
            "a3" => (297.0, 420.0),
            "a4" => (210.0, 297.0),
            "a5" => (148.0, 210.0),
            "letter" => (215.9, 279.4),
            "legal" => (215.9, 355.6),
            _ => return None,
        },
        [w, h] => (mm(length(w)?)?, mm(length(h)?)?),
        _ => return None,
    };
    Some(if turn { (w.max(h), w.min(h)) } else { (w, h) })
}

/// A border: `none`, or a width, `solid`, `dashed` or `dotted`, and a color.
fn border(text: &str) -> Option<Option<Border>> {
    if text.trim() == "none" {
        return Some(None);
    }
    let (mut width, mut dash, mut paint) = (None, None, None);
    let mut style_given = false;
    for word in words_outside_brackets(text) {
        match word.as_str() {
            "solid" => style_given = true,
            "dashed" => (style_given, dash) = (true, Some("dashed")),
            "dotted" => (style_given, dash) = (true, Some("dotted")),
            w => {
                if let Some(l) = length(w) {
                    width = Some(l);
                } else {
                    paint = Some(color(w)?);
                }
            }
        }
    }
    if !style_given {
        return None;
    }
    Some(Some(Border { width: width.unwrap_or(Len::Pt(0.75)), dash, color: paint.unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 }) }))
}

fn words_outside_brackets(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            c if c.is_whitespace() && depth == 0 => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                continue;
            }
            _ => {}
        }
        word.push(c);
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// A color: hex, `rgb()`, `rgba()`, a basic name or `transparent`.
pub(crate) fn color(text: &str) -> Option<Color> {
    let text = text.trim().to_ascii_lowercase();
    if let Some(hex) = text.strip_prefix('#') {
        let digits: Vec<u8> = hex.chars().map(|c| c.to_digit(16).map(|d| d as u8)).collect::<Option<_>>()?;
        let pair = |i: usize| digits[i] * 16 + digits[i + 1];
        return Some(match digits.len() {
            3 => Color { r: digits[0] * 17, g: digits[1] * 17, b: digits[2] * 17, a: 255 },
            4 => Color { r: digits[0] * 17, g: digits[1] * 17, b: digits[2] * 17, a: digits[3] * 17 },
            6 => Color { r: pair(0), g: pair(2), b: pair(4), a: 255 },
            8 => Color { r: pair(0), g: pair(2), b: pair(4), a: pair(6) },
            _ => return None,
        });
    }
    if let Some(inner) = text.strip_prefix("rgba(").or_else(|| text.strip_prefix("rgb(")).and_then(|t| t.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        let channel = |p: &str| -> Option<u8> {
            match p.strip_suffix('%') {
                Some(pct) => pct.parse::<f64>().ok().filter(|v| (0.0..=100.0).contains(v)).map(|v| (v * 2.55).round() as u8),
                None => p.parse::<u8>().ok(),
            }
        };
        let alpha = |p: &str| -> Option<u8> {
            let v = match p.strip_suffix('%') {
                Some(pct) => pct.parse::<f64>().ok()? / 100.0,
                None => p.parse::<f64>().ok()?,
            };
            (0.0..=1.0).contains(&v).then(|| (v * 255.0).round() as u8)
        };
        return match parts.as_slice() {
            [r, g, b] => Some(Color { r: channel(r)?, g: channel(g)?, b: channel(b)?, a: 255 }),
            [r, g, b, a] => Some(Color { r: channel(r)?, g: channel(g)?, b: channel(b)?, a: alpha(a)? }),
            _ => None,
        };
    }
    let (r, g, b, a) = match text.as_str() {
        "transparent" => (0, 0, 0, 0),
        "black" => (0, 0, 0, 255),
        "silver" => (192, 192, 192, 255),
        "gray" | "grey" => (128, 128, 128, 255),
        "white" => (255, 255, 255, 255),
        "maroon" => (128, 0, 0, 255),
        "red" => (255, 0, 0, 255),
        "purple" => (128, 0, 128, 255),
        "fuchsia" => (255, 0, 255, 255),
        "green" => (0, 128, 0, 255),
        "lime" => (0, 255, 0, 255),
        "olive" => (128, 128, 0, 255),
        "yellow" => (255, 255, 0, 255),
        "navy" => (0, 0, 128, 255),
        "blue" => (0, 0, 255, 255),
        "teal" => (0, 128, 128, 255),
        "aqua" => (0, 255, 255, 255),
        _ => return None,
    };
    Some(Color { r, g, b, a })
}

/// A margin box's `content`: texts in quotes, `counter(page)` and `counter(pages)`.
fn content(text: &str) -> Option<Vec<Piece>> {
    let mut pieces = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') {
            let end = rest[1..].find(q)? + 1;
            pieces.push(Piece::Text(rest[1..end].to_string()));
            rest = rest[end + 1..].trim_start();
        } else if let Some(r) = rest.strip_prefix("counter(page)") {
            pieces.push(Piece::Page);
            rest = r.trim_start();
        } else if let Some(r) = rest.strip_prefix("counter(pages)") {
            pieces.push(Piece::Pages);
            rest = r.trim_start();
        } else {
            rest = rest.strip_prefix("none")?.trim_start();
        }
    }
    Some(pieces)
}
