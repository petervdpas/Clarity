//! The colour dialog of a painting canvas (`does adjust`, docs/IMAGE-PLAN.md):
//! hue, saturation, brightness and contrast, each made less and more, shown
//! on the picture as they change; OK keeps it as one edit, Cancel puts the
//! picture back. On the selection when there is one, else the whole picture.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use clarity_image::RgbaImage;
use skia_safe::Color;

use super::CanvasPart;
use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Cursor, PointerInput};
use crate::locale;
use crate::overlay::Overlay;
use crate::painter::{Painter, TextAlign};

const WIDTH: f64 = 300.0;
const ROW: f64 = 30.0;
const PAD: f64 = 12.0;
const HEADER: f64 = 30.0;

/// The four values, each with its range and step: hue in degrees, the others from -100 to 100.
const VALUES: [(&str, &str, f64, f64); 4] = [
    ("adjustHue", "Hue", 180.0, 10.0),
    ("adjustSaturation", "Saturation", 100.0, 10.0),
    ("adjustBrightness", "Brightness", 100.0, 5.0),
    ("adjustContrast", "Contrast", 100.0, 5.0),
];

/// What the dialog and its canvas share: the values as they are, and how it ended.
#[derive(Default)]
pub struct Adjusting {
    pub values: RefCell<[f64; 4]>,
    /// Bumped at each change, so the canvas shows it once.
    pub changes: Cell<u64>,
    /// OK (true) or Cancel (false); none while it is open.
    pub ended: Cell<Option<bool>>,
}

/// The canvas while its colour dialog is open: the picture as it was, what is adjusted, and the dialog.
pub(super) struct Open {
    pub shared: Rc<Adjusting>,
    pub original: Vec<u8>,
    pub pixels: RgbaImage,
    pub seen: u64,
    pub dialog: ElementId,
}

impl CanvasPart {
    /// `does adjust`: the colour dialog opens over the window, in the middle.
    pub(super) fn open_adjust(&mut self, cx: &mut Cx) {
        if self.raster_ref().adjusting.is_some() {
            return;
        }
        let pixels = self.painted_pixels();
        let original = clarity_image::encode(&pixels);
        let shared = Rc::new(Adjusting::default());
        let whole = cx.whole();
        let height = HEADER + PAD + ROW * VALUES.len() as f64 + PAD + 32.0 + PAD;
        let place = Rect::new(whole.x + (whole.width - WIDTH) / 2.0, whole.y + (whole.height - height) / 3.0, WIDTH, height);
        let dialog = AdjustDialog { node: Node::new(), shared: shared.clone(), place, buttons: Vec::new() };
        let id = cx.open(Overlay::new(Box::new(dialog), place));
        self.raster().adjusting = Some(Open { shared, original, pixels, seen: 0, dialog: id });
    }

    /// Laid out while the dialog is open: a change shows on the picture; OK keeps it, Cancel puts the picture back.
    pub(super) fn follow_adjust(&mut self, cx: &mut Cx) {
        let Some(open) = self.raster().adjusting.as_mut() else { return };
        let ended = open.shared.ended.get();
        let changed = open.shared.changes.get() != open.seen;
        open.seen = open.shared.changes.get();
        let values = *open.shared.values.borrow();
        let (pixels, original, dialog) = (open.pixels.clone(), open.original.clone(), open.dialog);
        match ended {
            Some(keep) => {
                self.raster().adjusting = None;
                cx.close(dialog);
                if keep && values.iter().any(|v| *v != 0.0) {
                    let made = self.adjusted(&pixels, values);
                    self.show_and_send(clarity_image::encode(&made), cx);
                } else {
                    self.show_only(&original);
                    cx.invalidate();
                }
            }
            None if changed => {
                let made = self.adjusted(&pixels, values);
                self.show_only(&clarity_image::encode(&made));
                cx.invalidate();
            }
            None => {}
        }
    }

    /// The picture with the colours adjusted: the selected pixels only when there is a selection.
    fn adjusted(&self, pixels: &RgbaImage, v: [f64; 4]) -> RgbaImage {
        let mut all = pixels.clone();
        clarity_image::ops::adjust(&mut all, v[0], v[1], v[2], v[3]);
        let Some(s) = self.selection() else { return all };
        let mut out = pixels.clone();
        let (ax, ay, w, h) = s.area;
        for dy in 0..h {
            for dx in 0..w {
                let (x, y) = (ax + i64::from(dx), ay + i64::from(dy));
                if s.mask[(dy * w + dx) as usize] > 0 && x >= 0 && y >= 0 && x < i64::from(out.width()) && y < i64::from(out.height()) {
                    out.put_pixel(x as u32, y as u32, *all.get_pixel(x as u32, y as u32));
                }
            }
        }
        out
    }
}

/// What a press on the dialog does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Press {
    Less(usize),
    More(usize),
    Ok,
    Cancel,
}

/// The colour dialog itself.
pub struct AdjustDialog {
    node: Node,
    shared: Rc<Adjusting>,
    place: Rect,
    buttons: Vec<(Press, Rect)>,
}

impl AdjustDialog {
    /// Where its buttons lie: less and more for each value, then Cancel and OK.
    fn lay(&mut self) {
        let b = self.place;
        let mut buttons = Vec::new();
        for i in 0..VALUES.len() {
            let y = b.y + HEADER + PAD + i as f64 * ROW + 4.0;
            buttons.push((Press::Less(i), Rect::new(b.right() - PAD - 22.0 * 2.0 - 52.0, y, 22.0, 22.0)));
            buttons.push((Press::More(i), Rect::new(b.right() - PAD - 22.0, y, 22.0, 22.0)));
        }
        let y = b.bottom() - PAD - 30.0;
        buttons.push((Press::Cancel, Rect::new(b.right() - PAD - 90.0 * 2.0 - 8.0, y, 90.0, 30.0)));
        buttons.push((Press::Ok, Rect::new(b.right() - PAD - 90.0, y, 90.0, 30.0)));
        self.buttons = buttons;
    }

    /// Where the OK and Cancel buttons and each value's buttons lie, for tests.
    pub fn place_of(&self, what: &str) -> Option<Rect> {
        let want = match what {
            "ok" => Press::Ok,
            "cancel" => Press::Cancel,
            "moreBrightness" => Press::More(2),
            "lessBrightness" => Press::Less(2),
            _ => return None,
        };
        self.buttons.iter().find(|(p, _)| *p == want).map(|(_, r)| *r)
    }
}

impl Element for AdjustDialog {
    crate::element_node!();

    fn arrange(&mut self, _area: Rect, _cx: &mut Cx) {
        self.node.bounds = self.place;
        self.lay();
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        self.buttons.iter().any(|(_, r)| r.contains(x, y)).then_some(Cursor::Hand)
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(press) = self.buttons.iter().find(|(_, r)| r.contains(at.screen_x, at.screen_y)).map(|(p, _)| *p) else { return };
        match press {
            Press::Less(i) | Press::More(i) => {
                let (_, _, reach, step) = VALUES[i];
                let mut values = self.shared.values.borrow_mut();
                let by = if matches!(press, Press::More(_)) { step } else { -step };
                values[i] = (values[i] + by).clamp(-reach, reach);
                self.shared.changes.set(self.shared.changes.get() + 1);
            }
            Press::Ok => self.shared.ended.set(Some(true)),
            Press::Cancel => self.shared.ended.set(Some(false)),
        }
        cx.invalidate();
    }

    fn dialog_key(&mut self, key: crate::input::KeyInput, cx: &mut Cx) -> bool {
        match key.key {
            crate::input::Key::Escape => self.shared.ended.set(Some(false)),
            crate::input::Key::Enter => self.shared.ended.set(Some(true)),
            _ => return false,
        }
        cx.invalidate();
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        p.fill(Rect::new(b.x + 3.0, b.y + 3.0, b.width, b.height), Color::from_argb(30, 0, 0, 0), 8.0);
        p.fill(b, look.color("panel"), 8.0);
        p.stroke(b, look.color("strong_border"), look.border(), 8.0);
        let size = look.size("text");
        p.text(&locale::translate("adjustColors", "Colours"), Rect::new(b.x + PAD, b.y, b.width - PAD * 2.0, HEADER), look.color("text"), size, "text", true, TextAlign::Left);
        let values = *self.shared.values.borrow();
        for (i, (key, english, _, _)) in VALUES.iter().enumerate() {
            let y = b.y + HEADER + PAD + i as f64 * ROW;
            p.text(&locale::translate(key, english), Rect::new(b.x + PAD, y, 120.0, ROW), look.color("muted"), size, "text", false, TextAlign::Left);
            p.text(&format!("{}", values[i].round()), Rect::new(b.right() - PAD - 22.0 - 52.0, y, 52.0, ROW), look.color("text"), size, "text", false, TextAlign::Center);
        }
        for (press, place) in &self.buttons {
            let (word, strong) = match press {
                Press::Less(_) => ("-".to_string(), false),
                Press::More(_) => ("+".to_string(), false),
                Press::Ok => (locale::translate("ok", "OK"), true),
                Press::Cancel => (locale::translate("cancel", "Cancel"), false),
            };
            p.fill(*place, look.color(if strong { "accent_soft" } else { "raised" }), 4.0);
            p.stroke(*place, look.color(if strong { "accent" } else { "border" }), 1.0, 4.0);
            p.text(&word, *place, look.color("text"), size, "text", strong, TextAlign::Center);
        }
    }
}
