//! Containers and small displays. The containers hold grids of parts: a
//! panel and a form, a button group, a scroll box, tabs, steps, an
//! accordion, a splitter, a popover and a slide-out; a separator lines them up. The small
//! displays show something and take no input: help, badge, progress, busy,
//! display, icon, image, markdown and text. Each kind is a module in the folder of
//! this group; KINDS lists the kinds the group draws with the function that
//! makes each.

pub mod accordion;
pub mod buttons;
pub mod container;
pub mod display;
pub mod icon;
pub mod image;
pub mod markdown;
pub mod panel;
pub mod popover;
pub mod scroll;
pub mod slideout;
pub mod scroll_view;
pub mod separator;
pub mod small;
pub mod splitter;
pub mod steps;
pub mod tabs;
pub mod text;

use super::MakePart;

/// The kinds this group draws, each with the module that makes it.
pub const KINDS: &[(&str, MakePart)] = &[
    ("panel", panel::make),
    ("form", panel::make),
    ("buttons", buttons::make),
    ("separator", separator::make),
    ("scroll", scroll::make),
    ("tabs", tabs::make),
    ("steps", steps::make),
    ("accordion", accordion::make),
    ("splitter", splitter::make),
    ("popover", popover::make),
    ("slideout", slideout::make),
    ("help", small::make),
    ("badge", small::make),
    ("progress", small::make),
    ("busy", small::make),
    ("display", display::make),
    ("icon", icon::make),
    ("image", image::make),
    ("markdown", markdown::make),
    ("text", text::make),
];
