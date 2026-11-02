//! Small parts that show and take nothing: help under an input, a badge in
//! a pill, a progress bar as full as its value is of its max
//! (conformance/small-parts.json `progress`), and a busy indicator turning
//! beside what it is busy with. `make` picks the part by the spec's kind.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use serde_json::Value;
use skia_safe::{Paint, PaintCap, PaintStyle};

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let base = PartBase::new(spec, host);
    match base.spec.kind.as_str() {
        "badge" => Box::new(BadgePart { base }),
        "progress" => Box::new(ProgressPart { base }),
        "busy" => Box::new(BusyPart { base, clock: Cell::new(None), started: Instant::now() }),
        _ => Box::new(HelpPart { base }),
    }
}

/// A value as the person reads it: yes or no in their language, an offered
/// value by its label, a list as its items, anything else as written.
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
        Some(value) => match host.choices(path).into_iter().find(|c| &c.value == value) {
            Some(choice) => choice.label,
            None => plain(value),
        },
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
