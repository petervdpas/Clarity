//! Laying out and painting a text editor: its lines, scroll bars, alignment,
//! scrolling to the caret, code colours and painting.

use super::*;

impl TextEditor {
    /// Takes its place, and scrolls so the caret shows.
    pub fn place(&mut self, area: Rect, cx: &Cx) {
        self.node.bounds = area;
        self.scroll_to_caret(cx.fonts(), cx.look());
    }

    /// Mirrors the hover, press and focus of the part that holds it.
    pub fn mirror(&mut self, holder: &Node) {
        self.node.hovered = holder.hovered;
        self.node.pressed = holder.pressed;
        self.node.focused = holder.focused;
    }

    /// Its scroll bar, when its lines do not fit: where its thumb is on the
    /// screen, and the thumb as every bar measures it.
    pub(super) fn bar(&self, fonts: &Fonts, look: &ThemeLook) -> Option<(Rect, crate::geometry::Thumb)> {
        if !self.multiline || !self.framed {
            return None;
        }
        let laid = self.layout(fonts, look);
        let inner = self.inner(look);
        let content = laid.lines.len() as f64 * laid.height;
        let max = crate::geometry::scroll_room(content, inner.height);
        if max <= 0.0 {
            return None;
        }
        let thumb = crate::geometry::Thumb { track: inner.height, content, scroll: self.scroll_y, max };
        let x = self.node.bounds.right() - self.side_gap(look) / 2.0 - 2.0;
        Some((Rect::new(x, inner.y + thumb.at(), 4.0, thumb.length()), thumb))
    }

    /// Where its scroll bar's thumb is now; none while its lines fit.
    pub fn thumb(&self) -> Option<Rect> {
        self.thumb_place.get()
    }

    pub(super) fn side_gap(&self, look: &ThemeLook) -> f64 {
        if self.framed { look.padding() * 2.0 } else { 0.0 }
    }

    /// The place inside the frame the text is drawn in.
    pub(super) fn inner(&self, look: &ThemeLook) -> Rect {
        let b = self.node.bounds;
        let top = if self.multiline && self.framed { look.padding() + look.border() } else { 0.0 };
        let side = self.side_gap(look);
        Rect::new(b.x + side, b.y + top, (b.width - 2.0 * side - self.right_inset).max(1.0), (b.height - 2.0 * top).max(1.0))
    }

    /// What is drawn for the text: the text itself, or a dot for each character of a password.
    pub(super) fn shown_chars(&self) -> Vec<char> {
        if self.password { vec![DOT; self.chars.len()] } else { self.chars.clone() }
    }

    pub(super) fn layout(&self, fonts: &Fonts, look: &ThemeLook) -> Laid {
        let width = self.inner(look).width;
        let size = look.size("text");
        let font_name = look.font(self.font_role);
        if let Some(laid) = self.laid.borrow().as_ref() {
            if (laid.width - width).abs() < 0.5 && laid.size == size && laid.font == font_name && laid.version == self.version {
                return laid.clone();
            }
        }
        let font = fonts.font(self.font_role, size, false);
        let shown = self.shown_chars();
        let text: String = shown.iter().collect();
        // The text shaped as drawn (Arabic joined, Hebrew right to left): each character's width, in the text's order.
        let whole = fonts.shape(&text, self.font_role, size, false);
        let mut from = vec![0.0; shown.len() + 1];
        for (i, c) in shown.iter().enumerate() {
            let w = if *c == '\n' || i >= whole.len() { 0.0 } else { whole.right_of(i) - whole.left_of(i) };
            from[i + 1] = from[i] + w;
        }
        let lines = if self.multiline { wrap(&shown, &from, width) } else { vec![Line { start: 0, end: shown.len() }] };
        // Each line shaped as it is drawn, so the caret and the pointer land on its letters.
        let shapes: Vec<Rc<ShapedText>> = lines
            .iter()
            .map(|l| {
                if l.start == 0 && l.end == shown.len() {
                    whole.clone()
                } else {
                    fonts.shape(&shown[l.start..l.end].iter().collect::<String>(), self.font_role, size, false)
                }
            })
            .collect();
        // Across is measured from the start of each place's own line.
        let mut across = vec![0.0; shown.len() + 1];
        for (n, line) in lines.iter().enumerate() {
            for i in line.start..=line.end {
                across[i] = shapes[n].caret_x(i - line.start);
            }
        }
        let (spacing, _) = font.metrics();
        let laid = Laid { lines, shapes, height: spacing as f64, across, width, size, font: font_name, version: self.version };
        *self.laid.borrow_mut() = Some(laid.clone());
        laid
    }

    /// How far a line moves right for its alignment, when it fits.
    pub(super) fn align_shift(&self, laid: &Laid, line: usize, inner: Rect) -> f64 {
        if self.align == TextAlign::Left || self.multiline {
            return 0.0;
        }
        let spare = inner.width - laid.shapes[line].width();
        if spare <= 0.0 {
            0.0
        } else if self.align == TextAlign::Right {
            spare
        } else {
            spare / 2.0
        }
    }

    /// Scrolls so the caret shows, after it moved or the text changed.
    pub(super) fn scroll_to_caret(&mut self, fonts: &Fonts, look: &ThemeLook) {
        if self.node.bounds.width <= 0.0 {
            return;
        }
        let laid = self.layout(fonts, look);
        let inner = self.inner(look);
        if self.multiline {
            let most = crate::geometry::scroll_room(laid.lines.len() as f64 * laid.height, inner.height);
            if self.follow {
                let top = line_of(&laid, self.caret) as f64 * laid.height;
                if top < self.scroll_y {
                    self.scroll_y = top;
                } else if top + laid.height > self.scroll_y + inner.height {
                    self.scroll_y = top + laid.height - inner.height;
                }
            }
            self.scroll_y = self.scroll_y.clamp(0.0, most);
            self.scroll_x = 0.0;
        } else {
            let full = laid.shapes[0].width();
            let most = (full - inner.width + 1.0).max(0.0);
            if self.follow {
                let x = laid.across[self.caret];
                if x < self.scroll_x {
                    self.scroll_x = x;
                } else if x > self.scroll_x + inner.width - 1.0 {
                    self.scroll_x = x - inner.width + 1.0;
                }
            }
            self.scroll_x = self.scroll_x.clamp(0.0, most);
            self.scroll_y = 0.0;
        }
        self.follow = false;
    }

    /// The colored stretches of its code, found once per version of the text; none for plain text or a password.
    pub(super) fn inks(&self) -> Option<std::cell::Ref<'_, Vec<(usize, usize, crate::parts::code_colors::Ink)>>> {
        let language = self.language.filter(|_| !self.password)?;
        let fresh = self.inked.borrow().as_ref().is_some_and(|(v, _)| *v == self.version);
        if !fresh {
            *self.inked.borrow_mut() = Some((self.version, crate::parts::code_colors::spans(language, &self.chars)));
        }
        Some(std::cell::Ref::map(self.inked.borrow(), |i| &i.as_ref().expect("found above").1))
    }

    /// Draws the frame, the text, the selection and the caret.
    pub fn paint(&self, p: &Painter) {
        let look = p.look;
        let node = &self.node;
        let state = if !self.enabled {
            None
        } else if node.focused {
            Some("focus")
        } else if node.hovered {
            Some("hover")
        } else {
            None
        };
        // Code to read, not to type in, looks as the theme's code part, beside its input look.
        let reading = self.read_only && self.language.is_some();
        let input = if reading { look.part("code", state).over(&look.part("input", state)) } else { look.part("input", state) };
        let radius = input.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("input"));
        let background = input.background.unwrap_or_else(|| look.color("input"));
        let border = if self.marked { look.color("error") } else { input.border.unwrap_or_else(|| look.color("strong_border")) };
        let faded = !self.enabled;
        let bounds = node.bounds;
        if self.framed {
            p.fill(bounds, fade(background, faded), radius);
            p.stroke(bounds, fade(border, faded), look.border(), radius);
            if node.focused {
                p.focus_ring(bounds, radius);
            }
        }

        // Its scroll bar in its right margin, when its lines do not fit.
        let bar = self.bar(p.fonts, look);
        self.thumb_place.set(bar.map(|(place, _)| place));
        if let Some((place, _)) = bar {
            let color = if self.grab.is_some() || node.hovered { "muted" } else { "strong_border" };
            p.fill(place, fade(look.color(color), faded), 2.0);
        }

        let laid = self.layout(p.fonts, look);
        let inner = self.inner(look);
        let size = look.size("text");
        let text_color = fade(if self.framed { input.text.unwrap_or_else(|| look.color("text")) } else { look.color("text") }, faded);
        let font = p.fonts.font(self.font_role, size, false);
        let mut paint = Paint::default();
        paint.set_color(text_color);
        paint.set_anti_alias(true);
        let _clip = p.clip(Rect { x: inner.x - 1.0, width: inner.width + 2.0, ..inner });
        let (_, metrics) = font.metrics();
        let (ascent, descent) = (metrics.ascent as f64, metrics.descent as f64);
        let top_of = |line: usize| {
            if self.multiline { inner.y + line as f64 * laid.height - self.scroll_y } else { inner.y + (inner.height - laid.height) / 2.0 }
        };
        let baseline = |line: usize| top_of(line) + (laid.height - (descent - ascent)) / 2.0 - ascent;

        if self.chars.is_empty() {
            if let Some(placeholder) = self.placeholder.as_deref().filter(|p| !p.is_empty()) {
                let mut muted = Paint::default();
                muted.set_color(fade(look.color("muted"), faded));
                muted.set_anti_alias(true);
                let width = p.fonts.measure(placeholder, self.font_role, size, false);
                let x = if self.align == TextAlign::Left || self.multiline {
                    inner.x
                } else if self.align == TextAlign::Right {
                    inner.right() - width
                } else {
                    inner.x + (inner.width - width) / 2.0
                };
                p.fonts.draw(p.canvas, placeholder, x, baseline(0), self.font_role, size, false, &muted);
            }
        }

        let (start, end) = self.selection();
        for (i, line) in laid.lines.iter().enumerate() {
            let top = top_of(i);
            if top + laid.height < inner.y || top > inner.bottom() {
                continue;
            }
            let left = inner.x - self.scroll_x + self.align_shift(&laid, i, inner);
            let shape = &laid.shapes[i];
            // The selection behind the text, while it has the focus: one stretch, or more where directions mix.
            if node.focused && start != end && start <= line.end && end >= line.start {
                let from = start.max(line.start) - line.start;
                let to = end.min(line.end) - line.start;
                let mut spans = shape.spans(from, to);
                if spans.is_empty() {
                    spans.push((shape.caret_x(from), shape.caret_x(from)));
                }
                // A selected line break shows as a little space, at the line's end.
                if end > line.end && line.end < self.chars.len() {
                    if shape.right_to_left() {
                        spans[0].0 -= size / 3.0;
                    } else if let Some(last) = spans.last_mut() {
                        last.1 += size / 3.0;
                    }
                }
                for (a, b) in spans {
                    p.fill(Rect::new(left + a, top, (b - a).max(1.0), laid.height), look.color("selection"), 0.0);
                }
            }
            if line.end > line.start {
                match self.inks() {
                    // Code: each stretch in its color, where the shaped line puts it.
                    Some(inks) => {
                        let mut at = line.start;
                        let draw = |from: usize, to: usize, ink: Option<crate::parts::code_colors::Ink>| {
                            let piece: String = self.chars[from..to].iter().filter(|c| **c != '\n').collect();
                            if piece.is_empty() {
                                return;
                            }
                            let mut colored = paint.clone();
                            if let Some(ink) = ink {
                                colored.set_color(fade(look.try_color(ink.role()).unwrap_or_else(|| look.color(ink.fallback())), faded));
                            }
                            p.fonts.draw(p.canvas, &piece, left + shape.caret_x(from - line.start), baseline(i), self.font_role, size, false, &colored);
                        };
                        for &(from, to, ink) in inks.iter().filter(|(f, t, _)| *t > line.start && *f < line.end) {
                            let (from, to) = (from.max(line.start), to.min(line.end));
                            if from > at {
                                draw(at, from, None);
                            }
                            draw(from, to, Some(ink));
                            at = to;
                        }
                        if at < line.end {
                            draw(at, line.end, None);
                        }
                    }
                    None => shape.draw(p.canvas, left, baseline(i), &paint, 0.0),
                }
            }
        }

        if node.focused && self.enabled {
            let line = line_of(&laid, self.caret);
            let mut x = inner.x - self.scroll_x + self.align_shift(&laid, line, inner) + laid.across[self.caret];
            let top = top_of(line);
            // What an input method composes shows at the caret on a plate of its own, underlined, the marked part stronger.
            if let Some(composing) = &self.composing {
                let width = p.fonts.measure(&composing.text, "text", size, false);
                let place = Rect::new(x, top, width, laid.height);
                p.fill(place, look.color("input"), 0.0);
                p.fonts.draw(p.canvas, &composing.text, x, baseline(line), "text", size, false, &paint);
                p.fill(Rect::new(x, top + laid.height - 2.0, width, 1.0), text_color, 0.0);
                if let Some((from, to)) = composing.marked.filter(|(f, t)| f < t && *t <= composing.text.len()) {
                    let at = p.fonts.measure(&composing.text[..from], "text", size, false);
                    let wide = p.fonts.measure(&composing.text[from..to], "text", size, false);
                    p.fill(Rect::new(x + at, top + laid.height - 2.0, wide, 2.0), text_color, 0.0);
                }
                x += width;
            }
            p.fill(Rect::new(x.round(), top, 1.0, laid.height), text_color, 0.0);
            self.caret_drawn.set(Some(Rect::new(x.round(), top, 1.0, laid.height)));
        }
    }
}
