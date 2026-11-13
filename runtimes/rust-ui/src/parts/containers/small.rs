//! Small parts that show and take nothing: help under an input, a badge in
//! a pill, a progress bar as full as its value is of its max
//! (conformance/small-parts.json `progress`), and a busy indicator turning
//! beside what it is busy with, and a status item: a status bar's panel.
//! `make` picks the part by the spec's kind.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use serde_json::Value;
use skia_safe::{Paint, PaintCap, PaintJoin, PaintStyle, Path, PathBuilder};

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::formats::{format_of, formats_of};
use crate::parts::{Part, PartBase, PartHost};

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let base = PartBase::new(spec, host);
    match base.spec.kind.as_str() {
        "badge" => Box::new(BadgePart { base }),
        "progress" => Box::new(ProgressPart { base }),
        "busy" => Box::new(BusyPart { base, clock: Cell::new(None), started: Instant::now() }),
        "statusItem" => Box::new(StatusItemPart { base }),
        _ => Box::new(HelpPart { base }),
    }
}

/// A value as the person reads it: yes or no in their language, a list as
/// each of its items shown, a number or date in the person's format when
/// its field or type has one, an offered value by its label, anything else
/// as written.
pub fn shown_value(host: &dyn PartHost, path: &str, value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(yes)) => {
            if *yes {
                locale::translate("yes", "Yes")
            } else {
                locale::translate("no", "No")
            }
        }
        // A list field shows each item as one is shown, joined.
        Some(Value::Array(items)) => items.iter().map(|item| shown_value(host, path, Some(item))).collect::<Vec<_>>().join(", "),
        Some(value) => {
            if let Some(format) = format_of(host, path).filter(|_| value.as_str() != Some("")) {
                return formats_of(host).show(&format, value);
            }
            match host.choices(path).into_iter().find(|c| &c.value == value) {
                Some(choice) => choice.label,
                // A number whose type the host does not say is still shown in the person's format.
                None if value.is_number() => crate::parts::data::rows::shown_formatted(value, None, &formats_of(host)),
                None => plain(value),
            }
        }
    }
}

/// A value as written: a text as it is, a number in digits, a list as its items.
fn plain(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(yes) => if *yes { locale::translate("yes", "Yes") } else { locale::translate("no", "No") },
        Value::Array(items) => items.iter().map(plain).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

// -- Help ------------------------------------------------------------------------

/// Help under an input: the error of the input its `for` names, in the error color; else its caption, else the help of that input's field.
pub struct HelpPart {
    base: PartBase,
}

impl HelpPart {
    /// The error of the input it is for; none when it has none.
    fn error(&self) -> Option<String> {
        let target = self.base.spec.text("for").filter(|t| !t.is_empty())?;
        let error = self.base.host.state(Some(target))?.error?;
        let shown = error.show();
        if shown.is_empty() { None } else { Some(shown) }
    }

    /// Whether it shows its input's error.
    pub fn shows_error(&self) -> bool {
        self.error().is_some()
    }

    /// The help the field of the input it is for declares, as shown in the person's language; none when it has none.
    fn field_help(&self) -> Option<String> {
        let target = self.base.spec.text("for").filter(|t| !t.is_empty())?;
        let bound = self.base.host.bound_of(target)?;
        let help = self.base.host.field(&bound)?.help?;
        Some(crate::facts::shown(&help, ""))
    }

    /// What it says: its input's error, else its caption, else the help its input's field declares.
    pub fn text(&self) -> String {
        self.error().or_else(|| self.base.caption()).or_else(|| self.field_help()).unwrap_or_default()
    }
}

impl Element for HelpPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let size = p.look.size("small");
        let shown = p.fit(&self.text(), bounds.width, size, "text", false);
        let color = p.look.color(if self.shows_error() { "error" } else { "muted" });
        p.text(&shown, bounds, color, size, "text", false, TextAlign::Left);
    }
}

impl Part for HelpPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

// -- Badge -----------------------------------------------------------------------

/// A badge: what it binds, else its caption, in a pill of the accent color or of its style.
pub struct BadgePart {
    base: PartBase,
}

impl BadgePart {
    /// What it says.
    pub fn text(&self) -> String {
        match self.base.bound() {
            Some(path) => shown_value(self.base.host.as_ref(), path, self.base.host.read(path).as_ref()),
            None => self.base.caption().unwrap_or_default(),
        }
    }

    /// Where the pill is drawn: as wide as its text, in the middle of its place's height.
    pub fn pill(&self, p: &Painter) -> Rect {
        let bounds = self.base.node.bounds;
        let size = p.look.size("small");
        let height = bounds.height.min((size * 1.5).ceil() + 2.0);
        let width = bounds.width.min(p.fonts.measure(&self.text(), "text", size, false) + 16.0);
        Rect::new(bounds.x, (bounds.y + (bounds.height - height) / 2.0).round(), width, height)
    }
}

impl Element for BadgePart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let style = self.base.spec.text("style").map(|name| look.style(name)).unwrap_or_default();
        let pill = self.pill(p);
        let size = look.size("small");
        let _dim = p.dimmed(self.base.enabled());
        p.fill(pill, style.background.unwrap_or_else(|| look.color("accent")), pill.height / 2.0);
        if let Some(border) = style.border {
            p.stroke(pill, border, look.border(), pill.height / 2.0);
        }
        let inside = Rect { x: pill.x + 8.0, width: (pill.width - 16.0).max(0.0), ..pill };
        let shown = p.fit(&self.text(), inside.width, size, "text", false);
        p.text(&shown, inside, style.text.unwrap_or_else(|| look.color("on_accent")), size, "text", false, TextAlign::Left);
    }
}

impl Part for BadgePart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn draws_own_style(&self) -> bool {
        true
    }
}

// -- Progress --------------------------------------------------------------------

/// A bar filled as far as its value is of its max, 100 without one.
pub struct ProgressPart {
    base: PartBase,
}

impl ProgressPart {
    /// How full a bar is, from 0 to 1, for a value and its max (100 without
    /// one): below 0 is empty, above max is full, and no value is empty.
    pub fn full(value: Option<&Value>, max: Option<&str>) -> f64 {
        let n = match value {
            None | Some(Value::Null) => f64::NAN,
            Some(Value::String(text)) if text.trim().is_empty() => f64::NAN,
            Some(Value::String(text)) => text.trim().parse::<f64>().unwrap_or(f64::NAN),
            Some(Value::Number(number)) => number.as_f64().unwrap_or(f64::NAN),
            Some(Value::Bool(yes)) => if *yes { 1.0 } else { 0.0 },
            Some(_) => f64::NAN,
        };
        let top = match max {
            None => 100.0,
            Some(written) => written.trim().parse::<f64>().unwrap_or(f64::NAN),
        };
        if !n.is_finite() || top.is_nan() || top <= 0.0 {
            return 0.0;
        }
        (n / top).clamp(0.0, 1.0)
    }

    /// How full it is now.
    pub fn shown(&self) -> f64 {
        let value = self.base.bound().and_then(|path| self.base.host.read(path));
        ProgressPart::full(value.as_ref(), self.base.spec.text("max"))
    }
}

impl Element for ProgressPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let look = p.look;
        let bar = Rect::new(bounds.x, bounds.y + (bounds.height - 8.0) / 2.0, bounds.width, 8.0);
        let _dim = p.dimmed(self.base.enabled());
        p.fill(bar, look.color("raised"), 4.0);
        let full = self.shown();
        if full > 0.0 {
            let _clip = p.clip(Rect { width: bar.width * full, ..bar });
            p.fill(bar, look.color("accent"), 4.0);
        }
        p.stroke(bar, look.color("border"), look.border(), 4.0);
    }
}

impl Part for ProgressPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

// -- Busy ------------------------------------------------------------------------

/// How long a turn of the busy indicator takes, in milliseconds.
const TURN: f64 = 900.0;

/// A turning indicator with what it is busy with: what it binds, else its caption.
pub struct BusyPart {
    base: PartBase,
    /// The time it turns by, in milliseconds; none for the clock's. A test sets its own.
    pub clock: Cell<Option<f64>>,
    started: Instant,
}

impl BusyPart {
    /// What it says.
    pub fn text(&self) -> String {
        match self.base.bound() {
            Some(path) => plain(&self.base.host.read(path).unwrap_or(Value::Null)),
            None => self.base.caption().unwrap_or_default(),
        }
    }

    fn now(&self) -> f64 {
        self.clock.get().unwrap_or_else(|| self.started.elapsed().as_secs_f64() * 1000.0)
    }
}

impl Element for BusyPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let look = p.look;
        let spinner = Rect::new(bounds.x, bounds.y + (bounds.height - 16.0) / 2.0, 16.0, 16.0);
        let mut ring = Paint::default();
        ring.set_color(look.color("border"));
        ring.set_anti_alias(true);
        ring.set_style(PaintStyle::Stroke);
        ring.set_stroke_width(2.0);
        p.canvas.draw_oval(spinner.inset(1.0).sk(), &ring);
        let angle = (self.now() % TURN / TURN * 360.0) as f32;
        let mut arc = ring.clone();
        arc.set_color(look.color("accent"));
        arc.set_stroke_cap(PaintCap::Round);
        p.canvas.draw_arc(spinner.inset(1.0).sk(), angle - 90.0, 90.0, false, &arc);
        let words = Rect::new(spinner.right() + 8.0, bounds.y, (bounds.width - 24.0).max(0.0), bounds.height);
        p.text(&self.text(), words, look.color("text"), look.size("text"), "text", false, TextAlign::Left);
    }

    // It keeps turning while it is on the screen: the screen draws every frame while it does.
    fn on_attached(&mut self, cx: &mut Cx) {
        cx.animate(self.base.node.id(), true);
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        cx.animate(self.base.node.id(), false);
    }
}

impl Part for BusyPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

// -- Status item -----------------------------------------------------------------

/// How wide a status item's gauge is, in pixels.
const GAUGE: f64 = 40.0;

/// How big its icon and its mark are, in pixels.
const MARK: f64 = 14.0;

/// A status bar's panel: its caption, its icon, a gauge of the number it
/// binds at its right end, a check mark while code sets it checked, and a
/// warning while code sets an error on it, which is then its tooltip.
pub struct StatusItemPart {
    base: PartBase,
}

/// Where a status item draws its pieces, left to right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusPlaces {
    /// Its icon, when it has one.
    pub icon: Option<Rect>,
    /// The check mark or the warning.
    pub mark: Rect,
    /// Its caption.
    pub text: Rect,
    /// The gauge, when it binds a number.
    pub gauge: Option<Rect>,
}

impl StatusItemPart {
    /// What it says: its caption as code set it, else as written.
    pub fn says(&self) -> String {
        self.base.caption().unwrap_or_default()
    }

    /// How full its gauge is, from 0 to 1, as a progress bar's; none when it binds nothing.
    pub fn full(&self) -> Option<f64> {
        let path = self.base.bound()?;
        Some(ProgressPart::full(self.base.host.read(path).as_ref(), self.base.spec.text("max")))
    }

    /// Whether code set it checked.
    pub fn checked(&self) -> bool {
        self.base.state().and_then(|s| s.checked) == Some(true)
    }

    /// The error code set on it, which it warns of; none for none or an empty one.
    pub fn warning(&self) -> Option<String> {
        let error = self.base.state().and_then(|s| s.error.clone())?;
        Some(error.show()).filter(|t| !t.is_empty())
    }

    fn icon_name(&self) -> Option<&str> {
        self.base.spec.text("icon").filter(|i| !i.is_empty())
    }

    /// Where its pieces go in its place: from the left its icon, its mark
    /// and its caption, and the gauge at the right end.
    pub fn places(&self) -> StatusPlaces {
        let bounds = self.base.node.bounds;
        let middle = |height: f64| bounds.y + ((bounds.height - height) / 2.0).round();
        let mut x = bounds.x + 4.0;
        let icon = self.icon_name().map(|_| {
            let place = Rect::new(x, middle(MARK + 2.0), MARK + 2.0, MARK + 2.0);
            x += MARK + 6.0;
            place
        });
        let mark = Rect::new(x, middle(MARK), MARK, MARK);
        x += MARK + 6.0;
        let gauge = self.base.bound().map(|_| Rect::new((bounds.right() - 4.0 - GAUGE).max(x), middle(6.0), GAUGE, 6.0));
        let end = gauge.map(|g| g.x - 8.0).unwrap_or(bounds.right() - 4.0);
        let text = Rect::new(x, bounds.y, (end - x).max(0.0), bounds.height);
        StatusPlaces { icon, mark, text, gauge }
    }
}

/// Draws a check mark in `place`.
fn check_mark(p: &Painter, place: Rect, color: skia_safe::Color) {
    let mut path = PathBuilder::new();
    let (x, y, w, h) = (place.x as f32, place.y as f32, place.width as f32, place.height as f32);
    path.move_to((x + w * 0.15, y + h * 0.55));
    path.line_to((x + w * 0.4, y + h * 0.8));
    path.line_to((x + w * 0.88, y + h * 0.22));
    stroked(p, &path.detach(), color);
}

/// Draws a warning in `place`: a triangle with a mark in it.
fn warning_sign(p: &Painter, place: Rect, color: skia_safe::Color) {
    let (x, y, w, h) = (place.x as f32, place.y as f32, place.width as f32, place.height as f32);
    let mut path = PathBuilder::new();
    path.move_to((x + w * 0.5, y + h * 0.06));
    path.line_to((x + w * 0.96, y + h * 0.92));
    path.line_to((x + w * 0.04, y + h * 0.92));
    path.close();
    path.move_to((x + w * 0.5, y + h * 0.38));
    path.line_to((x + w * 0.5, y + h * 0.6));
    path.move_to((x + w * 0.5, y + h * 0.76));
    path.line_to((x + w * 0.5, y + h * 0.77));
    stroked(p, &path.detach(), color);
}

fn stroked(p: &Painter, path: &Path, color: skia_safe::Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(2.0);
    paint.set_stroke_cap(PaintCap::Round);
    paint.set_stroke_join(PaintJoin::Round);
    p.canvas.draw_path(path, &paint);
}

impl Element for StatusItemPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let style = self.base.spec.text("style").map(|name| look.style(name)).unwrap_or_default();
        let places = self.places();
        let bounds = self.base.node.bounds;
        let _dim = p.dimmed(self.base.enabled());
        if let Some(background) = style.background {
            p.fill(bounds, background, look.radius("button"));
        }
        let warning = self.warning();
        let text_color = style.text.unwrap_or_else(|| look.color("text"));
        if let (Some(place), Some(name)) = (places.icon, self.icon_name()) {
            let path = self.base.host.icon_path(name);
            let color = if warning.is_some() { look.color("warning") } else { look.color("muted") };
            crate::parts::containers::icon::draw_icon(p, path.as_deref(), place, color);
        }
        if warning.is_some() {
            warning_sign(p, places.mark, look.color("warning"));
        } else if self.checked() {
            check_mark(p, places.mark, look.color("success"));
        }
        let size = look.size("small");
        let shown = p.fit(&self.says(), places.text.width, size, "text", false);
        p.text(&shown, places.text, text_color, size, "text", false, TextAlign::Left);
        if let (Some(gauge), Some(full)) = (places.gauge, self.full()) {
            p.fill(gauge, look.color("raised"), 3.0);
            if full > 0.0 {
                let _clip = p.clip(Rect { width: gauge.width * full, ..gauge });
                p.fill(gauge, look.color("accent"), 3.0);
            }
            p.stroke(gauge, look.color("border"), look.border(), 3.0);
        }
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if self.base.enabled() {
            self.base.fire("click");
            cx.invalidate();
        }
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let _ = (x, y);
        self.warning()
    }
}

impl Part for StatusItemPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn draws_own_style(&self) -> bool {
        true
    }
}
