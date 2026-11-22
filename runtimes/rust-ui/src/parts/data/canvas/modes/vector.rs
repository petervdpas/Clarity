//! The vector mode: the bound SVG's shapes, the shape being sketched, and the
//! text tool typing a shape's text until Enter places it.

use super::*;

impl CanvasPart {
    /// The shapes of the bound drawing, as the runtime reads its SVG.
    pub fn vector_shapes(&self) -> Vec<Map<String, Value>> {
        let svg = self.base.bound().and_then(|path| self.base.host.read(path)).and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
        if let Some((kept, shapes)) = self.modes.vector.borrow().as_ref() {
            if *kept == svg {
                return shapes.clone();
            }
        }
        let (uw, uh) = self.units();
        let (_, shapes) = super::super::drawing::drawing_shapes(&svg, [uw as i64, uh as i64]);
        *self.modes.vector.borrow_mut() = Some((svg, shapes.clone()));
        shapes
    }

    /// The topmost shape whose box holds a unit, a few pixels around it too.
    pub(super) fn vector_shape_at(&self, p: (f64, f64)) -> Option<usize> {
        let near = 4.0 / self.viewport().scale.max(1e-6);
        self.vector_shapes().iter().enumerate().rev().find_map(|(i, shape)| {
            let b = box_of(shape)?;
            (p.0 >= b.x - near && p.0 <= b.right() + near && p.1 >= b.y - near && p.1 <= b.bottom() + near).then_some(i)
        })
    }

    /// The shape `tool` draws from one unit to another, in the canvas's ink and brush; none when it has no size.
    pub fn sketched(&self, tool: &str, from: (f64, f64), to: (f64, f64), points: &[(f64, f64)]) -> Option<Value> {
        let (left, top) = (from.0.min(to.0), from.1.min(to.1));
        let (w, h) = ((to.0 - from.0).abs(), (to.1 - from.1).abs());
        if tool != "path" && w == 0.0 && h == 0.0 {
            return None;
        }
        let (ink, brush) = (self.ink(), as_value(self.brush()));
        let v = as_value;
        Some(match tool {
            "line" => json!({"op": "line", "x1": v(from.0), "y1": v(from.1), "x2": v(to.0), "y2": v(to.1), "color": ink, "width": brush}),
            "rect" => json!({"op": "rect", "x": v(left), "y": v(top), "width": v(w), "height": v(h), "color": ink}),
            "ellipse" => json!({"op": "ellipse", "x": v(left + w / 2.0), "y": v(top + h / 2.0), "rx": v(w / 2.0), "ry": v(h / 2.0), "color": ink}),
            "path" if points.len() >= 2 => {
                let all: Vec<Value> = points.iter().flat_map(|p| [v(p.0), v(p.1)]).collect();
                json!({"op": "path", "points": all, "color": ink, "width": brush})
            }
            _ => return None,
        })
    }

    pub(super) fn start_typing(&mut self, unit: (f64, f64), cx: &mut Cx) {
        self.end_typing(true, cx);
        let mut editor = TextEditor::new(false);
        editor.placeholder = Some(locale::translate("canvasText", "The text"));
        // The canvas keeps the keyboard and hands the input its keys and text.
        editor.focus_moved(true, cx);
        self.modes.typing = Some((unit, editor));
        self.place_typing(cx);
        cx.invalidate();
    }

    /// The box the text being typed takes on the screen, where it goes.
    pub(super) fn typing_box(&self, cx: &Cx) -> Option<Rect> {
        let ((x, y), editor) = self.modes.typing.as_ref()?;
        let point = self.at(self.viewport(), *x, *y);
        let size = cx.look().size("text");
        let width = (cx.fonts().measure(&editor.text(), "text", size, false) + 24.0).max(160.0);
        let height = (size * 2.0).max(28.0);
        Some(Rect::new(point.x as f64, point.y as f64 - height / 2.0, width, height))
    }

    /// Places the text input where its text goes, as wide as the text needs.
    pub(in crate::parts::data::canvas) fn place_typing(&mut self, cx: &Cx) {
        if let Some(place) = self.typing_box(cx) {
            if let Some((_, editor)) = self.modes.typing.as_mut() {
                editor.place(place, cx);
            }
        }
    }

    /// A press in the text input moves its caret; true when it was in it.
    pub(in crate::parts::data::canvas) fn typing_pressed(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        let Some((_, editor)) = self.modes.typing.as_mut() else { return false };
        let place = editor.node().bounds;
        if !place.contains(at.screen_x, at.screen_y) {
            return false;
        }
        let b = self.base.node.bounds;
        let inside = PointerInput { x: at.x - (place.x - b.x), y: at.y - (place.y - b.y), ..at };
        editor.node_mut().pressed = true;
        editor.pressed(inside, cx);
        true
    }

    /// An input method composes in the text being typed.
    pub(in crate::parts::data::canvas) fn typing_compose(&mut self, composing: Option<crate::element::Composition>, cx: &mut Cx) {
        if let Some((_, editor)) = self.modes.typing.as_mut() {
            editor.compose(composing, cx);
        }
    }

    /// A press that went to the text input selects as it moves and ends as it is let go; true when it was one.
    pub(super) fn typing_dragged(&mut self, at: PointerInput, released: bool, cx: &mut Cx) -> bool {
        let b = self.base.node.bounds;
        let Some((_, editor)) = self.modes.typing.as_mut().filter(|(_, e)| e.node().pressed) else { return false };
        let place = editor.node().bounds;
        let inside = PointerInput { x: at.x - (place.x - b.x), y: at.y - (place.y - b.y), ..at };
        if released {
            editor.released();
            editor.node_mut().pressed = false;
        } else {
            editor.dragged(inside, cx);
        }
        true
    }

    /// Places the text typed (`place`) or drops it.
    pub fn end_typing(&mut self, place: bool, cx: &mut Cx) {
        let Some((at, mut editor)) = self.modes.typing.take() else { return };
        editor.focus_moved(false, cx);
        let text = editor.text();
        cx.invalidate();
        if place && !text.trim().is_empty() {
            let size = as_value((self.brush() * 4.0).max(10.0));
            self.act(json!({"add": {"op": "text", "x": as_value(at.0), "y": as_value(at.1), "text": text, "color": self.ink(), "size": size}}));
        }
    }
}
