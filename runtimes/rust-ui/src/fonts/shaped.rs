//! A line of text as Clarity.UI draws it, measures it and places a caret in
//! it, so the three always agree. Text that needs no shaping (Latin, Greek,
//! Cyrillic, Chinese, Japanese, Korean syllables, without combining marks)
//! is drawn by Skia a run of one font at a time. Any other text is shaped by
//! HarfBuzz through Skia's shaper: split into items of one bidirectional
//! level and one font (the role's, or the fallback that has the letters),
//! each shaped in its direction and script, so Arabic letters join, marks
//! sit on their letters, Indic and Thai clusters form and the font's
//! ligatures show; the items are then laid out in visual order
//! (`bidi::visual_order`), Hebrew and Arabic from the right. Places are
//! characters (Unicode scalar values).

use std::cell::OnceCell;
use std::collections::BTreeMap;

use skia_safe::font::Edging;
use skia_safe::shaper::run_handler::{Buffer, RunHandler, RunInfo};
use skia_safe::{Canvas, Font, FontHinting, GlyphId, Paint, Point, Shaper, Typeface};

use super::bidi;
use super::unicode;

/// A piece of a line of text drawn in one typeface.
#[derive(Clone)]
pub struct TextRun {
    pub text: String,
    pub typeface: Typeface,
}

/// A piece drawn in one font: plain text Skia draws, or glyphs HarfBuzz placed.
struct Piece {
    typeface: Typeface,
    x: f64,
    /// The text of a piece that needs no shaping.
    text: Option<String>,
    /// The glyphs of a shaped piece and where each goes, from the line's start at its baseline.
    glyphs: Vec<GlyphId>,
    positions: Vec<Point>,
}

/// Each character's box across the line and how a caret treats it.
struct Places {
    left: Vec<f64>,
    right: Vec<f64>,
    backward: Vec<bool>,
    stops: Vec<bool>,
}

pub struct ShapedText {
    text: String,
    chars: Vec<char>,
    pieces: Vec<Piece>,
    size: f64,
    unhinted: bool,
    width: f64,
    right_to_left: bool,
    has_right_to_left: bool,
    shaped: bool,
    places: OnceCell<Places>,
}

/// The font a piece is drawn in: subpixel placed, and unhinted for a zoomed canvas.
pub fn font_of(typeface: &Typeface, size: f64, unhinted: bool, skew_x: f32) -> Font {
    let mut font = Font::from_typeface(typeface.clone(), size as f32);
    font.set_subpixel(true);
    font.set_edging(Edging::SubpixelAntiAlias);
    font.set_skew_x(skew_x);
    if unhinted {
        font.set_hinting(FontHinting::None);
        font.set_linear_metrics(true);
    }
    font
}

thread_local! {
    /// Skia's HarfBuzz shaper, made once per thread; None where it cannot be made.
    static SHAPER: Option<Shaper> = skia_safe::shapers::hb::shape_dont_wrap_or_reorder(None);
}

/// One shaped run as Skia's shaper hands it: its glyphs, where each goes, and the bytes each stands for.
#[derive(Default)]
struct Shaped {
    glyphs: Vec<GlyphId>,
    positions: Vec<Point>,
    offsets: Vec<Point>,
    clusters: Vec<u32>,
    advance: f64,
}

/// Takes the runs Skia's shaper makes of one item.
#[derive(Default)]
struct Collect {
    runs: Vec<Shaped>,
}

impl RunHandler for Collect {
    fn begin_line(&mut self) {}

    fn run_info(&mut self, _info: &RunInfo) {}

    fn commit_run_info(&mut self) {}

    fn run_buffer(&mut self, info: &RunInfo) -> Buffer<'_> {
        let n = info.glyph_count;
        self.runs.push(Shaped {
            glyphs: vec![0; n],
            positions: vec![Point::default(); n],
            offsets: vec![Point::default(); n],
            clusters: vec![0; n],
            advance: info.advance.x as f64,
        });
        let run = self.runs.last_mut().expect("just pushed");
        Buffer {
            glyphs: &mut run.glyphs,
            positions: &mut run.positions,
            offsets: Some(&mut run.offsets),
            clusters: Some(&mut run.clusters),
            point: Point::default(),
        }
    }

    fn commit_run_buffer(&mut self, _info: &RunInfo) {}

    fn commit_line(&mut self) {}
}

/// A part of the line shaped as one: one level, one font.
struct Item {
    start: usize,
    end: usize,
    level: u8,
    typeface: Typeface,
}

impl ShapedText {
    /// The line `text`, already split into runs of one font (`Fonts::runs`), shaped at `size`.
    pub fn of(text: &str, runs: &[TextRun], size: f64, unhinted: bool) -> ShapedText {
        if !unicode::needs_shaping(text) {
            return Self::plain(text, runs, size, unhinted);
        }
        // Fail safe: where Skia's shaper cannot be made, the text is drawn unshaped, as before.
        Self::shape(text, runs, size, unhinted).unwrap_or_else(|| Self::plain(text, runs, size, unhinted))
    }

    fn new(text: &str, pieces: Vec<Piece>, width: f64, size: f64, unhinted: bool) -> ShapedText {
        ShapedText {
            text: text.to_string(),
            chars: text.chars().collect(),
            pieces,
            size,
            unhinted,
            width,
            right_to_left: false,
            has_right_to_left: false,
            shaped: false,
            places: OnceCell::new(),
        }
    }

    /// Text drawn as Skia draws it, a run of one font at a time.
    fn plain(text: &str, runs: &[TextRun], size: f64, unhinted: bool) -> ShapedText {
        let mut pieces = Vec::with_capacity(runs.len());
        let mut at = 0.0;
        for run in runs {
            let font = font_of(&run.typeface, size, unhinted, 0.0);
            let width = if run.text.is_empty() { 0.0 } else { font.measure_str(&run.text, None).0 as f64 };
            pieces.push(Piece { typeface: run.typeface.clone(), x: at, text: Some(run.text.clone()), glyphs: Vec::new(), positions: Vec::new() });
            at += width;
        }
        Self::new(text, pieces, at, size, unhinted)
    }

    /// The line split where its level or its font changes; Skia's shaper splits each by script.
    fn items(chars: &[char], runs: &[TextRun], levels: &[u8]) -> Vec<Item> {
        let mut fonts: Vec<Typeface> = Vec::with_capacity(chars.len());
        for run in runs {
            for _ in run.text.chars() {
                if fonts.len() < chars.len() {
                    fonts.push(run.typeface.clone());
                }
            }
        }
        while fonts.len() < chars.len() {
            fonts.push(runs.last().map(|r| r.typeface.clone()).unwrap_or_else(|| super::carried(false)));
        }
        let mut items = Vec::new();
        let mut start = 0;
        for i in 1..=chars.len() {
            if i < chars.len() && levels[i] == levels[start] && super::same_face(&fonts[i], &fonts[start]) {
                continue;
            }
            items.push(Item { start, end: i, level: levels[start], typeface: fonts[start].clone() });
            start = i;
        }
        items
    }

    fn shape(text: &str, runs: &[TextRun], size: f64, unhinted: bool) -> Option<ShapedText> {
        let chars: Vec<char> = text.chars().collect();
        let paragraph = bidi::paragraph_level(text);
        let levels = bidi::levels(text, Some(paragraph));
        let items = Self::items(&chars, runs, &levels);
        let order = bidi::visual_order(&items.iter().map(|i| i.level).collect::<Vec<_>>());

        let n = chars.len();
        let mut left = vec![0.0; n];
        let mut right = vec![0.0; n];
        let starts = unicode::starts(&chars);
        let mut pieces = Vec::with_capacity(items.len());
        let mut pen = 0.0;
        for index in order {
            let item = &items[index];
            let piece_text: String = chars[item.start..item.end].iter().collect();
            // The byte each character of the item starts at, to read the shaper's clusters back as characters.
            let mut char_at_byte = BTreeMap::new();
            for (c, (byte, _)) in piece_text.char_indices().enumerate() {
                char_at_byte.insert(byte as u32, item.start + c);
            }
            let font = font_of(&item.typeface, size, unhinted, 0.0);
            let mut collect = Collect::default();
            let len = piece_text.len();
            let shaped = SHAPER.with(|shaper| {
                let shaper = shaper.as_ref()?;
                let mut fonts = Shaper::new_trivial_font_run_iterator(&font, len);
                let mut levels = skia_safe::shapers::primitive::trivial_bidi_run_iterator(item.level, len);
                let mut scripts = skia_safe::shapers::hb::script_run_iterator(&piece_text, None);
                let mut languages = Shaper::new_trivial_language_run_iterator("und", len);
                shaper.shape_with_iterators(&piece_text, &mut fonts, &mut levels, &mut scripts, &mut languages, f32::MAX, &mut collect);
                Some(())
            });
            shaped?;
            // Runs of one script come in the text's order; right to left, the last is drawn first.
            if item.level % 2 == 1 {
                collect.runs.reverse();
            }
            let mut glyphs = Vec::new();
            let mut positions = Vec::new();
            // Each cluster (the characters a glyph or glyphs stand for) from its left to its right.
            let mut from: BTreeMap<usize, (f64, f64)> = BTreeMap::new();
            let mut x = pen;
            for run in &collect.runs {
                let count = run.glyphs.len();
                let base = run.positions.first().map(|p| p.x as f64).unwrap_or(0.0);
                for g in 0..count {
                    let at = run.positions[g].x as f64 - base;
                    let next = if g + 1 < count { run.positions[g + 1].x as f64 - base } else { run.advance };
                    let advance = next - at;
                    let gx = x + at;
                    glyphs.push(run.glyphs[g]);
                    positions.push(Point::new((gx + run.offsets[g].x as f64) as f32, run.offsets[g].y));
                    let cluster = char_at_byte.range(..=run.clusters[g]).next_back().map(|(_, c)| *c).unwrap_or(item.start);
                    let entry = from.entry(cluster).or_insert((gx, gx + advance));
                    entry.0 = entry.0.min(gx);
                    entry.1 = entry.1.max(gx + advance);
                }
                x += run.advance;
            }
            place(item, &from, &starts, &mut left, &mut right);
            pieces.push(Piece { typeface: item.typeface.clone(), x: pen, text: None, glyphs, positions });
            pen = x;
        }
        let mut stops = starts.clone();
        stops.push(true);
        let mut shaped = Self::new(text, pieces, pen, size, unhinted);
        shaped.shaped = true;
        shaped.right_to_left = paragraph == 1;
        shaped.has_right_to_left = levels.iter().any(|l| l % 2 == 1);
        let _ = shaped.places.set(Places { left, right, backward: levels.iter().map(|l| l % 2 == 1).collect(), stops });
        Some(shaped)
    }

    /// The boxes of plain text's characters, measured on first need: each character as wide as Skia measures it.
    fn places(&self) -> &Places {
        self.places.get_or_init(|| {
            let n = self.chars.len();
            let mut left = Vec::with_capacity(n);
            let mut right = Vec::with_capacity(n);
            for piece in &self.pieces {
                let text = piece.text.as_deref().unwrap_or("");
                let font = font_of(&piece.typeface, self.size, self.unhinted, 0.0);
                let glyphs = font.str_to_glyphs_vec(text);
                let mut widths = vec![0.0f32; glyphs.len()];
                font.get_widths(&glyphs, &mut widths);
                let mut x = piece.x;
                for (i, c) in text.chars().enumerate() {
                    // One glyph per character; a line break draws nothing.
                    let w = if c == '\n' { 0.0 } else { widths.get(i).copied().unwrap_or(0.0) as f64 };
                    left.push(x);
                    x += w;
                    right.push(x);
                }
            }
            left.resize(n, right.last().copied().unwrap_or(0.0));
            right.resize(n, right.last().copied().unwrap_or(0.0));
            let mut stops = unicode::starts(&self.chars);
            stops.push(true);
            Places { left, right, backward: vec![false; n], stops }
        })
    }

    /// The text it shows.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// How many characters it has.
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// How wide it is.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Whether its paragraph runs right to left (its first strong letter is Hebrew or Arabic).
    pub fn right_to_left(&self) -> bool {
        self.right_to_left
    }

    /// Whether any of it runs right to left.
    pub fn has_right_to_left(&self) -> bool {
        self.has_right_to_left
    }

    /// Whether HarfBuzz shaped it; false for text drawn as before, a run of one font at a time.
    pub fn shaped(&self) -> bool {
        self.shaped
    }

    /// The glyphs drawn, from the left: what a test reads to see Arabic letters joined.
    pub fn glyphs(&self) -> Vec<GlyphId> {
        let mut all = Vec::new();
        for piece in &self.pieces {
            match &piece.text {
                Some(text) => all.extend(font_of(&piece.typeface, self.size, self.unhinted, 0.0).str_to_glyphs_vec(text)),
                None => all.extend_from_slice(&piece.glyphs),
            }
        }
        all
    }

    /// The left edge of the character at `index`, from the line's start.
    pub fn left_of(&self, index: usize) -> f64 {
        self.places().left[index]
    }

    /// The right edge of the character at `index`, from the line's start.
    pub fn right_of(&self, index: usize) -> f64 {
        self.places().right[index]
    }

    /// Whether the character at `index` runs right to left.
    pub fn backward(&self, index: usize) -> bool {
        self.places().backward[index]
    }

    /// Whether a caret may stand at `place`: not inside a letter made of several characters.
    pub fn is_stop(&self, place: usize) -> bool {
        place <= self.chars.len() && self.places().stops[place]
    }

    /// Where the caret stands at `place` (0 is before the first character),
    /// from the line's start: at the leading edge of the character after it
    /// (its left in Latin, its right in Hebrew), at the end at the trailing
    /// edge of the last.
    pub fn caret_x(&self, place: usize) -> f64 {
        let places = self.places();
        let n = self.chars.len();
        if n == 0 {
            return 0.0;
        }
        if place < n {
            return if places.backward[place] { places.right[place] } else { places.left[place] };
        }
        let last = n - 1;
        if places.backward[last] { places.left[last] } else { places.right[last] }
    }

    /// The place nearest `x`, from the line's start.
    pub fn place_at(&self, x: f64) -> usize {
        let places = self.places();
        let mut best = 0;
        let mut distance = f64::MAX;
        for p in 0..=self.chars.len() {
            if !places.stops[p] {
                continue;
            }
            let d = (self.caret_x(p) - x).abs();
            if d < distance - 1e-6 {
                best = p;
                distance = d;
            }
        }
        best
    }

    /// The place one step to the right (or left) of `place` as the line
    /// shows: the arrow keys in text that runs both ways. The place itself
    /// at the line's edge.
    pub fn step(&self, place: usize, to_right: bool) -> usize {
        let places = self.places();
        let x = self.caret_x(place);
        let mut best = place;
        let mut best_x = if to_right { f64::MAX } else { f64::MIN };
        for p in 0..=self.chars.len() {
            if !places.stops[p] || p == place {
                continue;
            }
            let px = self.caret_x(p);
            let beyond = if to_right { px > x + 0.01 } else { px < x - 0.01 };
            if !beyond {
                continue;
            }
            let nearer = if to_right { px < best_x - 0.01 } else { px > best_x + 0.01 };
            let as_near = (px - best_x).abs() <= 0.01 && p.abs_diff(place) < best.abs_diff(place);
            if nearer || as_near {
                best = p;
                best_x = px;
            }
        }
        best
    }

    /// The stretches across the line the characters from `start` to `end`
    /// cover, from the left: one in text of one direction, more where
    /// directions mix.
    pub fn spans(&self, start: usize, end: usize) -> Vec<(f64, f64)> {
        let places = self.places();
        let n = self.chars.len();
        let start = start.min(n);
        let end = end.clamp(start, n);
        if start == end {
            return Vec::new();
        }
        if !self.has_right_to_left {
            return vec![(places.left[start], places.right[end - 1])];
        }
        let mut boxes: Vec<(f64, f64)> =
            (start..end).filter(|i| places.right[*i] > places.left[*i]).map(|i| (places.left[i], places.right[i])).collect();
        boxes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut spans: Vec<(f64, f64)> = Vec::new();
        for b in boxes {
            match spans.last_mut() {
                Some(last) if b.0 <= last.1 + 0.5 => last.1 = last.1.max(b.1),
                _ => spans.push(b),
            }
        }
        spans
    }

    /// Draws the line from `x` at `baseline`; a slant (italic) when `skew_x` is not 0.
    pub fn draw(&self, canvas: &Canvas, x: f64, baseline: f64, paint: &Paint, skew_x: f32) {
        for piece in &self.pieces {
            let font = font_of(&piece.typeface, self.size, self.unhinted, skew_x);
            match &piece.text {
                Some(text) => {
                    if !text.is_empty() {
                        canvas.draw_str(text, ((x + piece.x) as f32, baseline as f32), &font, paint);
                    }
                }
                None => {
                    if !piece.glyphs.is_empty() {
                        canvas.draw_glyphs_at(&piece.glyphs, piece.positions.as_slice(), (x as f32, baseline as f32), &font, paint);
                    }
                }
            }
        }
    }
}

/// Where each character of `item` is: a cluster's width shared by the
/// characters that start a letter in it, in the item's direction.
fn place(item: &Item, from: &BTreeMap<usize, (f64, f64)>, starts: &[bool], left: &mut [f64], right: &mut [f64]) {
    let clusters: Vec<usize> = from.keys().copied().collect();
    let rtl = item.level % 2 == 1;
    for (c, first) in clusters.iter().enumerate() {
        let first = *first;
        let end = clusters.get(c + 1).copied().unwrap_or(item.end);
        let (l, r) = from[&first];
        let letters = (first..end).filter(|u| starts[*u]).count();
        let each = (r - l) / letters.max(1) as f64;
        let mut cursor = if rtl { r } else { l };
        for u in first..end {
            let wide = if starts[u] || (letters == 0 && u == first) { each } else { 0.0 };
            if rtl {
                right[u] = cursor;
                cursor -= wide;
                left[u] = cursor;
            } else {
                left[u] = cursor;
                cursor += wide;
                right[u] = cursor;
            }
        }
    }
    // Characters before the first cluster (none, when HarfBuzz numbered them all) sit at the item's start.
    if let Some(first) = clusters.first() {
        let edge = if rtl { from[first].1 } else { from[first].0 };
        for u in item.start..*first {
            left[u] = edge;
            right[u] = edge;
        }
    }
}

