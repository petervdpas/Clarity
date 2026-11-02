//! The markdown part: Markdown read into blocks the same way on every stack
//! (`blocks`, conformance/markdown.json), shown as headings, paragraphs,
//! list items and code, wrapped to its width and scrolling when longer than
//! its place. Its text is a Markdown file of the application's resources
//! (its `source`, by name, as the host gives the file), the text a key gives
//! in the language shown (its `caption`), or the text it binds. A link opens
//! through the host, in the person's browser or mail program.
//!
//! It keeps its own scroll offset, turned by the wheel, with a thin bar at
//! its right while it is longer than its place.

pub mod blocks;

use std::cell::RefCell;
use std::rc::Rc;

use skia_safe::{Paint, PaintStyle};

use crate::element::{Cx, Element};
use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::input::{Modifiers, PointerInput};
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::ThemeLook;

use blocks::{BlockKind, Span};

/// The room at the right kept for the scroll bar, as C#'s ScrollView keeps it.
pub const BAR_ROOM: f64 = 8.0;

/// Space between blocks.
const GAP: f64 = 6.0;

/// The padding of a code block across, then down.
const CODE_ACROSS: f64 = 8.0;
const CODE_DOWN: f64 = 6.0;

/// A piece of text as a markdown page places it: a word with the space
/// after it, or a list item's marker. Its place is in the page, from its top left.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownRun {
    pub text: String,
    pub place: Rect,
    pub size: f64,
    /// Its font role: `text` or `mono`.
    pub font: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub link: Option<String>,
}

/// A page of Markdown laid out at one width: its runs, the places of its code blocks, and how high it is.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarkdownLayout {
    pub runs: Vec<MarkdownRun>,
    pub code_boxes: Vec<Rect>,
    pub height: f64,
}

fn line_height(size: f64) -> f64 {
    (size * 1.45).ceil()
}

fn heading_scale(level: usize) -> f64 {
    match level {
        1 => 1.4,
        2 => 1.2,
        3 => 1.05,
        _ => 1.0,
    }
}

/// Words with the space after them; a piece of a word ends with no space.
fn pieces(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_space = false;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            in_space = true;
        } else if in_space {
            out.push(&text[start..i]);
            start = i;
            in_space = false;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// A piece of a word as it is placed: its text, the span it is of, its font role, whether bold, and its width.
struct Piece<'a> {
    text: String,
    span: &'a Span,
    font: &'static str,
    bold: bool,
    width: f64,
}

/// Where text flows: the fonts, the size and the line from `left` to `right`.
struct Flow<'f> {
    fonts: &'f Fonts,
    size: f64,
    left: f64,
    right: f64,
    code_block: bool,
}

impl Flow<'_> {
    fn measure(&self, text: &str, font: &str, bold: bool) -> f64 {
        self.fonts.measure(text, font, self.size, bold)
    }

    fn trailing_space(&self, text: &str, font: &str, bold: bool) -> f64 {
        let trimmed = text.trim_end();
        if trimmed.len() == text.len() { 0.0 } else { self.measure(&text[trimmed.len()..], font, bold) }
    }

    /// How many characters of `text` fit in `width`.
    fn fits(&self, text: &str, width: f64, font: &str, bold: bool) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let mut count = 0;
        while count < chars.len() && self.measure(&chars[..count + 1].iter().collect::<String>(), font, bold) <= width {
            count += 1;
        }
        count
    }

    fn run(&self, text: &str, place: Rect, piece: &Piece) -> MarkdownRun {
        MarkdownRun {
            text: text.to_string(),
            place,
            size: self.size,
            font: piece.font.to_string(),
            bold: piece.bold,
            italic: piece.span.italic,
            code: piece.span.code || self.code_block,
            link: piece.span.link.clone(),
        }
    }

    /// Places a word at `x`, `y`: on the next line when it does not fit,
    /// and a piece wider than a line cut where the line ends.
    fn place(&self, runs: &mut Vec<MarkdownRun>, word: &mut Vec<Piece<'_>>, x: &mut f64, y: &mut f64) {
        let Some(last) = word.last() else { return };
        let height = line_height(self.size);
        let shown = word.iter().map(|p| p.width).sum::<f64>() - self.trailing_space(&last.text, last.font, last.bold);
        if *x > self.left && *x + shown > self.right {
            *x = self.left;
            *y += height;
        }
        for piece in word.iter() {
            let mut text = piece.text.clone();
            let mut width = piece.width;
            // A piece wider than what is left of the line is cut where it ends, character by character.
            while *x + width - self.trailing_space(&text, piece.font, piece.bold) > self.right && text.chars().count() > 1 {
                let fits = self.fits(&text, self.right - *x, piece.font, piece.bold);
                if fits == 0 && *x > self.left {
                    *x = self.left;
                    *y += height;
                    continue;
                }
                let fits = fits.max(1);
                let cut: String = text.chars().take(fits).collect();
                let cut_width = self.measure(&cut, piece.font, piece.bold);
                runs.push(self.run(&cut, Rect::new(*x, *y, cut_width, height), piece));
                text = text.chars().skip(fits).collect();
                width = self.measure(&text, piece.font, piece.bold);
                *x = self.left;
                *y += height;
            }
            runs.push(self.run(&text, Rect::new(*x, *y, width, height), piece));
            *x += width;
        }
        word.clear();
    }

    /// Places the spans from `left` to `right`, from `y` down, word by word.
    /// Gives where the next block starts.
    fn spans(&self, runs: &mut Vec<MarkdownRun>, spans: &[Span], y: f64, font: &'static str, bold: bool) -> f64 {
        let mut x = self.left;
        let mut y = y;
        // A word may be made of pieces of spans that look different ("then" and "."): it moves to a new line as a whole.
        let mut word: Vec<Piece> = Vec::new();
        for span in spans {
            let span_font = if span.code { "mono" } else { font };
            let span_bold = bold || span.bold;
            for text in pieces(&span.text) {
                let width = self.measure(text, span_font, span_bold);
                word.push(Piece { text: text.to_string(), span, font: span_font, bold: span_bold, width });
                if text.ends_with(char::is_whitespace) {
                    self.place(runs, &mut word, &mut x, &mut y);
                }
            }
        }
        self.place(runs, &mut word, &mut x, &mut y);
        y + line_height(self.size)
    }
}

impl MarkdownLayout {
    /// The blocks of `markdown` laid out to `width`, in the look's text size and fonts.
    pub fn of(markdown: &str, width: f64, fonts: &Fonts, look: &ThemeLook) -> MarkdownLayout {
        let mut runs = Vec::new();
        let mut code_boxes = Vec::new();
        let size = look.size("text");
        let mut y = 0.0;
        let flow = |size: f64, left: f64, right: f64, code_block: bool| Flow { fonts, size, left, right, code_block };
        for (index, block) in blocks::read(markdown).iter().enumerate() {
            if index > 0 {
                y += GAP;
            }
            match block.kind {
                BlockKind::Heading => {
                    let heading = size * heading_scale(block.level);
                    y = flow(heading, 0.0, width, false).spans(&mut runs, &block.spans, y, "text", true);
                }
                BlockKind::Item => {
                    let indent = block.depth as f64 * 1.25 * size;
                    let marker = match block.number {
                        Some(n) => format!("{n}."),
                        None => "\u{2022}".to_string(),
                    };
                    let shown = fonts.measure(&marker, "text", size, false);
                    let room = size.max(shown);
                    runs.push(MarkdownRun {
                        text: marker,
                        place: Rect::new(indent + room - shown, y, shown, line_height(size)),
                        size,
                        font: "text".to_string(),
                        bold: false,
                        italic: false,
                        code: false,
                        link: None,
                    });
                    y = flow(size, indent + room + size * 0.5, width, false).spans(&mut runs, &block.spans, y, "text", false);
                }
                BlockKind::Code => {
                    let top = y;
                    y += CODE_DOWN;
                    for line in block.text.split('\n') {
                        let span = Span { code: true, ..Span::plain(line) };
                        y = flow(size, CODE_ACROSS, width - CODE_ACROSS, true).spans(&mut runs, std::slice::from_ref(&span), y, "mono", false);
                    }
                    y += CODE_DOWN;
                    code_boxes.push(Rect::new(0.0, top, width, y - top));
                }
                BlockKind::Paragraph => {
                    y = flow(size, 0.0, width, false).spans(&mut runs, &block.spans, y, "text", false);
                }
            }
        }
        MarkdownLayout { runs, code_boxes, height: y.ceil() }
    }

    /// The address of the link at a point of the page, from its top left; none off a link.
    pub fn link_at(&self, x: f64, y: f64) -> Option<&str> {
        self.runs.iter().find(|r| r.link.is_some() && r.place.contains(x, y)).and_then(|r| r.link.as_deref())
    }
}

/// What a layout was made for: its text, width and look.
struct LaidFor {
    text: String,
    width: f64,
    look: *const ThemeLook,
}

pub struct MarkdownPart {
    base: PartBase,
    /// The resource file read last: its name and its text.
    resource: RefCell<Option<(String, String)>>,
    laid: MarkdownLayout,
    laid_for: Option<LaidFor>,
    scroll_y: f64,
    max_scroll_y: f64,
    pressed_link: Option<String>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(MarkdownPart {
        base: PartBase::new(spec, host),
        resource: RefCell::new(None),
        laid: MarkdownLayout::default(),
        laid_for: None,
        scroll_y: 0.0,
        max_scroll_y: 0.0,
        pressed_link: None,
    })
}

impl MarkdownPart {
    /// The Markdown it shows: its resource file, its key's text, or what it binds.
    pub fn markdown(&self) -> String {
        if let Some(source) = self.base.spec.text("source").filter(|s| !s.is_empty()) {
            return self.resource_text(source);
        }
        if let Some(caption) = self.base.spec.text("caption").filter(|c| !c.is_empty()) {
            return self.base.host.text(Some(caption));
        }
        match self.base.bound().and_then(|path| self.base.host.read(path)) {
            Some(serde_json::Value::String(text)) => text,
            _ => String::new(),
        }
    }

    /// The text of a resource file, read once per name; empty when the application carries none.
    fn resource_text(&self, name: &str) -> String {
        if let Some((read, text)) = self.resource.borrow().as_ref() {
            if read == name {
                return text.clone();
            }
        }
        let text = self.base.host.resource(name).map(|bytes| String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string()).unwrap_or_default();
        *self.resource.borrow_mut() = Some((name.to_string(), text.clone()));
        text
    }

    /// The runs it shows, as last laid out.
    pub fn runs(&self) -> Vec<MarkdownRun> {
        self.laid.runs.clone()
    }

    /// The places of its code blocks in its page, as last laid out.
    pub fn code_boxes(&self) -> Vec<Rect> {
        self.laid.code_boxes.clone()
    }

    /// Where its page lies on the screen now: its place, moved up as far as it scrolled.
    pub fn page(&self) -> Rect {
        let b = self.base.node.bounds;
        Rect::new(b.x, b.y - self.scroll_y, (b.width - BAR_ROOM).max(0.0), self.laid.height)
    }

    /// How far it is scrolled down.
    pub fn scroll_y(&self) -> f64 {
        self.scroll_y
    }

    /// How far it can scroll down: how much longer its page is than its place.
    pub fn max_scroll_y(&self) -> f64 {
        self.max_scroll_y
    }

    /// Its blocks laid out to `width`, again only when its text, width or look changed.
    fn lay_out(&mut self, width: f64, cx: &Cx) {
        let text = self.markdown();
        let look: *const ThemeLook = cx.look();
        let same = self.laid_for.as_ref().is_some_and(|was| was.text == text && was.width == width && std::ptr::eq(was.look, look));
        if !same {
            self.laid = MarkdownLayout::of(&text, width, cx.fonts(), cx.look());
            self.laid_for = Some(LaidFor { text, width, look });
        }
    }

    fn draw_runs(&self, p: &Painter, page: Rect) {
        let look = p.look;
        let shade = look.color("muted").with_a(31);
        let radius = look.radius("input");
        for code in &self.laid.code_boxes {
            p.fill(code.offset(page.x, page.y), shade, radius);
        }
        for run in &self.laid.runs {
            let place = run.place.offset(page.x, page.y);
            let mut font = p.fonts.font(&run.font, run.size, run.bold);
            if run.italic {
                font.set_skew_x(-0.2);
            }
            let color = if run.link.is_some() { look.color("accent") } else { look.color("text") };
            let mut paint = Paint::default();
            paint.set_color(color);
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            let (_, metrics) = font.metrics();
            let baseline = place.y + (place.height - (metrics.descent - metrics.ascent) as f64) / 2.0 - metrics.ascent as f64;
            let text = run.text.trim_end();
            if text.is_empty() {
                continue;
            }
            let width = font.measure_str(text, None).0 as f64;
            let in_block = self.laid.code_boxes.iter().any(|c| c.contains(run.place.x, run.place.y + 1.0));
            if run.code && !in_block {
                p.fill(Rect::new(place.x - 1.0, place.y + 1.0, width + 2.0, place.height - 2.0), shade, 3.0);
            }
            p.canvas.draw_str(text, (place.x as f32, baseline as f32), &font, &paint);
            if run.link.is_some() {
                p.fill(Rect::new(place.x, (baseline + 2.0).round(), width, 1.0), color, 0.0);
            }
        }
    }
}

impl Element for MarkdownPart {
    part_element!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.lay_out((area.width - BAR_ROOM).max(0.0), cx);
        self.max_scroll_y = (self.laid.height - area.height).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y);
    }

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let _dim = p.dimmed(self.base.enabled());
        {
            let _clip = p.clip(bounds);
            self.draw_runs(p, self.page());
        }
        if self.max_scroll_y > 0.0 && self.laid.height > 0.0 {
            // A thin bar at the right says how much of the page shows, and where.
            let shown = bounds.height / self.laid.height;
            let bar = Rect::new(bounds.right() - BAR_ROOM / 2.0 - 2.0, bounds.y + bounds.height * (self.scroll_y / self.laid.height), 4.0, (bounds.height * shown).max(16.0));
            p.fill(bar, p.look.color("strong_border"), 2.0);
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        let to = (self.scroll_y + dy).clamp(0.0, self.max_scroll_y);
        if to == self.scroll_y {
            return false;
        }
        self.scroll_y = to;
        cx.invalidate();
        true
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.pressed_link = self.laid.link_at(at.x, at.y + self.scroll_y).map(str::to_string);
    }

    fn on_pointer_released(&mut self, at: PointerInput, _cx: &mut Cx) {
        let Some(link) = self.pressed_link.take() else { return };
        if self.base.enabled() && self.laid.link_at(at.x, at.y + self.scroll_y) == Some(link.as_str()) {
            self.base.host.open_address(&link);
        }
    }
}

impl Part for MarkdownPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}
