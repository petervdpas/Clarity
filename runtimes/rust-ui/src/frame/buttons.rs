//! How a toolbar's and a ribbon's buttons are measured and drawn: in the
//! button look, an icon beside the caption or, large, above it; a line for
//! a separator.

use crate::element::Cx;
use crate::geometry::Rect;
use crate::markup::CommandSpec;
use crate::painter::{Painter, TextAlign};
use crate::parts::containers::icon::draw_icon;
use crate::parts::PartHost;

use super::rules;

pub const SEPARATOR_WIDTH: f64 = 9.0;
pub const LARGE_HEIGHT: f64 = 58.0;
pub const LARGE_ICON: f64 = 24.0;
pub const SMALL_ICON: f64 = 16.0;

fn is_large(item: &CommandSpec, large: bool) -> bool {
    large && item.flag("large")
}

/// How wide a button is: its icon and caption with room around them; `large` where large buttons are drawn large (a ribbon).
pub fn width(host: &dyn PartHost, item: &CommandSpec, large: bool, cx: &Cx) -> f64 {
    if rules::is_separator(item) {
        return SEPARATOR_WIDTH;
    }
    let caption = host.text(item.text("caption"));
    let text = if caption.is_empty() { 0.0 } else { cx.fonts().measure(&caption, "text", cx.look().size("small"), false) };
    let icon = item.text("icon").is_some_and(|i| !i.is_empty());
    if is_large(item, large) {
        return (text.max(LARGE_ICON) + 20.0).ceil();
    }
    (if icon { SMALL_ICON } else { 0.0 } + if icon && text > 0.0 { 6.0 } else { 0.0 } + text + 20.0).ceil()
}

/// Draws a button at `area`, under the pointer (`hover`) or held down (`held`).
pub fn draw(p: &Painter, host: &dyn PartHost, item: &CommandSpec, area: Rect, large: bool, hover: bool, held: bool) {
    let look = p.look;
    if rules::is_separator(item) {
        p.fill(Rect::new(area.x + area.width / 2.0, area.y + 2.0, look.border(), area.height - 4.0), look.color("strong_border"), 0.0);
        return;
    }
    let enabled = rules::takes(host, item.text("name"));
    let _dimmed = p.dimmed(enabled);
    let state = if enabled && held { Some("pressed") } else if enabled && hover { Some("hover") } else { None };
    let values = look.part("button", state);
    let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
    p.fill(area, values.background.unwrap_or_else(|| look.color("panel")), radius);
    p.stroke(area, values.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
    let color = values.text.unwrap_or_else(|| look.color("text"));
    let caption = host.text(item.text("caption"));
    let size = look.size("small");
    let path = item.text("icon").filter(|i| !i.is_empty()).and_then(|i| host.icon_path(i));
    if is_large(item, large) {
        if let Some(path) = &path {
            draw_icon(p, Some(path), Rect::new(area.x + (area.width - LARGE_ICON) / 2.0, area.y + 6.0, LARGE_ICON, LARGE_ICON), color);
        }
        p.text(&caption, Rect::new(area.x, area.bottom() - 24.0, area.width, 20.0), color, size, "text", false, TextAlign::Center);
        return;
    }
    let mut x = area.x + 10.0;
    if let Some(path) = &path {
        draw_icon(p, Some(path), Rect::new(x, area.y + (area.height - SMALL_ICON) / 2.0, SMALL_ICON, SMALL_ICON), color);
        x += SMALL_ICON + 6.0;
    }
    p.text(&caption, Rect::new(x, area.y, area.right() - x, area.height), color, size, "text", false, TextAlign::Left);
}
