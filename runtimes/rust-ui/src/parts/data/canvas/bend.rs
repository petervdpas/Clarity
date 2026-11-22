//! A canvas with `on bend`: the link under the pointer, its points to drag,
//! adding a point and taking one away.

use super::*;

impl CanvasPart {
    /// The link under a unit, the topmost first: its bend point there, else a new one on the segment pressed.
    pub(super) fn link_at(&self, p: (f64, f64)) -> Option<(String, Vec<(f64, f64)>, usize)> {
        let near = 8.0 / self.viewport().scale;
        for shape in self.kept().shown.iter().rev() {
            // A box drawn over the press takes it: the link under it is not pressed.
            if let Shape::Rect { x, y, width, height, .. } = shape {
                if p.0 >= *x && p.0 <= x + width && p.1 >= *y && p.1 <= y + height {
                    return None;
                }
            }
            let Shape::Link { name, x1, y1, x2, y2, via, .. } = shape else { continue };
            let all = link_points(*x1, *y1, *x2, *y2, via);
            let mut bends: Vec<(f64, f64)> = all[1..all.len() - 1].to_vec();
            if let Some(on) = bends.iter().position(|v| distance(*v, p) <= near) {
                return Some((name.clone(), bends, on));
            }
            for i in 0..all.len() - 1 {
                if distance_to_segment(p, all[i], all[i + 1]) <= near {
                    bends.insert(i, p);
                    return Some((name.clone(), bends, i));
                }
            }
        }
        None
    }

    /// The bend points of the link in hand.
    pub(super) fn bend_handles(&self) -> Vec<(f64, f64)> {
        let Some(in_hand) = self.in_hand.as_ref().filter(|_| self.handles("bend")) else { return Vec::new() };
        self.shown()
            .into_iter()
            .find_map(|s| match s {
                Shape::Link { name, x1, y1, x2, y2, via, .. } if &name == in_hand => {
                    let all = link_points(x1, y1, x2, y2, &via);
                    Some(all[1..all.len() - 1].to_vec())
                }
                _ => None,
            })
            .unwrap_or_default()
    }

    pub(super) fn bent(&mut self, name: String, via: Vec<(f64, f64)>, cx: &mut Cx) {
        self.seen = Some(self.drawing_sent());
        let mut at = pointer_at((0, 0));
        at.insert("bent".into(), Value::from(name.clone()));
        at.insert("via".into(), Value::Array(via.iter().flat_map(|p| [Value::from(p.0.round() as i64), Value::from(p.1.round() as i64)]).collect()));
        self.bending = Some((name, via));
        cx.invalidate();
        self.point("bend", at);
    }

    /// A double click on a bend point of the link in hand takes it away.
    pub(super) fn unbend(&mut self, x: f64, y: f64, cx: &mut Cx) -> bool {
        let Some(name) = self.in_hand.clone().filter(|_| self.handles("bend")) else { return false };
        let p = self.unit_at(x, y);
        let near = 8.0 / self.viewport().scale;
        let mut handles = self.bend_handles();
        let Some(point) = handles.iter().position(|v| distance(*v, p) <= near) else { return false };
        handles.remove(point);
        self.bent(name, handles, cx);
        true
    }
}
