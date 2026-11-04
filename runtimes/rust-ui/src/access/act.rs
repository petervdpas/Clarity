//! What a screen reader asks a node to do, done as the person's input does
//! it: a click at the node (or the key a part takes when nothing else is
//! there to click), the focus as Tab gives it, text typed over what the
//! input holds and taken as when it is left, Up and Down for a number. So
//! the same handlers, checks and undo apply as for the person's own input.

use crate::element::{self, Element, ElementId};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers};
use crate::screen::Screen;

use super::{owner_of, tree, AccessAction, VIRTUAL};

/// Does `action` on the node `id` of the screen's tree, with `value` for
/// SetValue; false when there is no such node or it takes no such action now.
pub fn act(screen: &mut Screen, id: u64, action: AccessAction, value: Option<&str>) -> bool {
    let Some(node) = tree(screen).node(id).cloned() else { return false };
    let Some(owner) = owner_of(id) else { return false };
    let own = id & VIRTUAL == 0;
    if node.states.disabled && !matches!(action, AccessAction::Focus | AccessAction::ScrollIntoView) {
        return false;
    }
    match action {
        AccessAction::ScrollIntoView => {
            scroll_into_view(screen, id);
            true
        }
        AccessAction::Focus => {
            let takes = screen.element(owner).is_some_and(|e| e.focusable());
            if takes {
                scroll_into_view(screen, id);
                screen.focus(Some(owner));
            }
            takes
        }
        AccessAction::Click => {
            let place = scroll_into_view(screen, id).unwrap_or(node.bounds);
            if own {
                click_element(screen, owner, place);
            } else {
                click_at(screen, place);
            }
            true
        }
        AccessAction::Expand | AccessAction::Collapse => {
            let wanted = action == AccessAction::Expand;
            if node.states.expanded == Some(wanted) {
                return true;
            }
            scroll_into_view(screen, id);
            let now = tree(screen).node(id).cloned().unwrap_or(node);
            match now.toggle {
                Some(toggle) => click_at(screen, toggle),
                None if own => click_element(screen, owner, now.bounds),
                None => click_at(screen, now.bounds),
            }
            true
        }
        AccessAction::SetValue => {
            if !node.actions.contains(&AccessAction::SetValue) {
                return false;
            }
            scroll_into_view(screen, id);
            screen.focus(Some(owner));
            screen.key_pressed(KeyInput::new(Key::A, Modifiers::CONTROL));
            match value.unwrap_or_default() {
                "" => {
                    screen.key_pressed(KeyInput::plain(Key::Delete));
                }
                text => screen.text_typed(text),
            }
            screen.take_typed();
            true
        }
        AccessAction::Increment | AccessAction::Decrement => {
            if !node.actions.contains(&action) {
                return false;
            }
            scroll_into_view(screen, id);
            screen.focus(Some(owner));
            screen.key_pressed(KeyInput::plain(if action == AccessAction::Increment { Key::Up } else { Key::Down }));
            true
        }
    }
}

/// A press and release in the middle of `place`.
fn click_at(screen: &mut Screen, place: Rect) {
    screen.click(place.x + place.width / 2.0, place.y + place.height / 2.0);
}

/// Clicks an element in its middle when that is where it shows; else (it
/// lies under something, or is clipped) it gets the focus and is pressed as
/// a key presses it.
fn click_element(screen: &mut Screen, id: ElementId, place: Rect) {
    let (x, y) = (place.x + place.width / 2.0, place.y + place.height / 2.0);
    let there = screen.at(x, y).is_some_and(|at| screen.element(id).is_some_and(|e| element::find(e, at).is_some()));
    if there {
        screen.click(x, y);
        return;
    }
    if screen.element(id).is_some_and(|e| e.focusable()) {
        screen.focus(Some(id));
    }
    screen.act(id, |e, cx| {
        element::safely(e, cx, (), |e, cx| match e.as_part_mut() {
            Some(part) => part.press(cx),
            None => e.on_click(cx),
        })
    });
}

/// Scrolls the window that shows node `id` with the wheel, as the person
/// scrolls to it, until the node is inside the window's body; where it is then.
pub fn scroll_into_view(screen: &mut Screen, id: u64) -> Option<Rect> {
    let owner = owner_of(id)?;
    for _ in 0..20 {
        let b = tree(screen).node(id)?.bounds;
        let Some(body) = body_of(screen, owner) else { return Some(b) };
        let by = if b.y < body.y {
            b.y - body.y - 10.0
        } else if b.bottom() > body.bottom() {
            (b.bottom() - body.bottom() + 10.0).min(b.y - body.y)
        } else {
            return Some(b);
        };
        screen.wheel(body.x + crate::window::view::CONTENT_PADDING / 2.0, body.y + body.height / 2.0, 0.0, by, Modifiers::NONE);
        if tree(screen).node(id).map(|n| n.bounds) == Some(b) {
            break;
        }
    }
    tree(screen).node(id).map(|n| n.bounds)
}

/// The body of the window view that holds an element: where its parts show.
fn body_of(screen: &Screen, id: ElementId) -> Option<Rect> {
    fn view_in(e: &dyn Element, id: ElementId) -> Option<Rect> {
        for child in e.node().children() {
            if let Some(found) = view_in(child.as_ref(), id) {
                return Some(found);
            }
        }
        let view = e.as_any().downcast_ref::<crate::window::view::WindowView>()?;
        element::find(e, id).map(|_| view.body_place())
    }
    let trees = std::iter::once(screen.root()).chain(screen.overlays().iter().map(|o| o.content.as_ref()));
    trees.filter_map(|t| view_in(t, id)).last()
}
