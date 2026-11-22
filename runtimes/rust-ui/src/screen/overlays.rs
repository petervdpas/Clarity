//! A screen's overlays: opening and closing them, and the modal one on top.

use super::*;

impl Screen {
    pub(super) fn open_now(&mut self, mut overlay: Overlay, cx: &mut Cx) {
        self.hide_tip();
        if overlay.covers {
            overlay.place = self.whole();
        }
        element::arrange_safely(overlay.content.as_mut(), overlay.place, cx);
        element::attach_all(overlay.content.as_mut(), cx);
        self.overlays.push(overlay);
        self.redraw = true;
    }

    pub(super) fn close_now(&mut self, id: ElementId, cx: &mut Cx) {
        let Some(index) = self.overlays.iter().position(|o| o.id() == id) else { return };
        let focused_inside = self.focused.is_some_and(|f| element::find(self.overlays[index].content.as_ref(), f).is_some());
        if focused_inside {
            self.focus_now(None, cx);
        }
        let mut overlay = self.overlays.remove(index);
        element::detach_all(overlay.content.as_mut(), cx);
        for gone in [&mut self.hovered, &mut self.pressed] {
            if gone.is_some_and(|g| element::find(overlay.content.as_ref(), g).is_some()) {
                *gone = None;
            }
        }
        self.redraw = true;
        if let Some(closed) = overlay.closed.take() {
            closed();
        }
        if let Some(e) = overlay.opener.and_then(|o| self.element_mut(o)) {
            e.on_overlay_closed(id, cx);
        }
    }

    /// Opens an overlay over the window.
    pub fn open(&mut self, overlay: Overlay) -> ElementId {
        let mut cx = self.cx();
        let id = cx.open(overlay);
        self.settle(cx);
        id
    }

    /// Closes the overlay of that id.
    pub fn close(&mut self, id: ElementId) {
        let mut cx = self.cx();
        cx.close(id);
        self.settle(cx);
    }

    pub(super) fn modal_on_top(&self) -> bool {
        self.overlays.last().is_some_and(|o| o.modal)
    }
}
