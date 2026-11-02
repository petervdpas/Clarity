//! Inputs and pickers: number, spin box, radio, toggle, slider, link,
//! dropdowns, color, date, file, folder, tags, picture field.
//! Each kind is a module in the folder of this group; KINDS lists the kinds
//! the group draws with the function that makes each.

pub mod choices;
pub mod formats;
pub mod testing;

pub mod link;
pub mod number;
pub mod radio;
pub mod slider;
pub mod toggle;

pub mod choice_list;
pub mod dropdown;
pub mod tags;
pub mod typed_dropdown;

pub mod calendar;
pub mod color;
pub mod date;
pub mod file;
pub mod folder;
pub mod picker;
pub mod picture_field;

use super::MakePart;

/// The kinds this group draws, each with the module that makes it.
/// `image` is here for its variant, the picture field; a plain image is the
/// containers group's, which the picture field's maker hands it to.
pub const KINDS: &[(&str, MakePart)] = &[
    ("number", number::make),
    ("spinbox", number::make),
    ("radio", radio::make),
    ("toggle", toggle::make),
    ("slider", slider::make),
    ("link", link::make),
    ("dropdown", dropdown::make),
    ("tags", tags::make),
    ("color", color::make),
    ("date", date::make),
    ("file", file::make),
    ("folder", folder::make),
    ("image", picture_field::make),
];
