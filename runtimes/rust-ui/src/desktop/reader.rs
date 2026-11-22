//! The screen reader (AccessKit): what each window tells it, and what it asks.

use super::*;

impl Host {
    /// The screen reader, while one reads the window, is told what changed in its tree.
    pub(super) fn tell_reader(&mut self, i: usize) {
        let win = &mut self.wins[i];
        let (Some(reader), Some(shown)) = (win.reader.as_mut(), win.shown.as_ref()) else { return };
        if !reader.active {
            return;
        }
        let now = crate::access::tree(&win.screen);
        if reader.told.as_ref() == Some(&now) {
            return;
        }
        let update = crate::access::kit::update(reader.told.as_ref(), &now, shown.per_unit());
        reader.adapter.update_if_active(|| update);
        reader.told = Some(now);
    }

    /// The screen reader asked for the whole tree, for an action (done as the person's input), or stopped.
    pub(super) fn on_reader(&mut self, event: accesskit_winit::Event) {
        let Some(i) = self.wins.iter().position(|w| w.shown.as_ref().is_some_and(|s| s.window.id() == event.window_id)) else { return };
        match event.window_event {
            accesskit_winit::WindowEvent::InitialTreeRequested => {
                if let Some(reader) = self.wins[i].reader.as_mut() {
                    reader.active = true;
                    reader.told = None;
                }
                self.tell_reader(i);
            }
            accesskit_winit::WindowEvent::ActionRequested(request) => {
                if self.blocked(i) {
                    return;
                }
                if let Some((node, action, value)) = crate::access::kit::request(&request) {
                    crate::access::act::act(&mut self.wins[i].screen, node, action, value.as_deref());
                }
                if let Some(shown) = &self.wins[i].shown {
                    shown.window.request_redraw();
                }
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                if let Some(reader) = self.wins[i].reader.as_mut() {
                    reader.active = false;
                    reader.told = None;
                }
            }
        }
    }
}
