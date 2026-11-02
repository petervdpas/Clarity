//! The parts: buttons, containers, text, sound, timer and custom parts.
//! One test binary for these files (each a module, its tests named
//! `<file>::<test>`), so Skia is linked once for them all.

#[allow(dead_code, unused_imports)]
#[path = "../common/core_windows_fake.rs"]
mod core_windows_fake;

mod check_box;
mod core_custom_line;
mod core_help;
mod core_parts;
mod core_popover_route;
mod core_radio_label;
mod icon;
mod parts_accordion;
mod parts_button;
mod parts_custom;
mod parts_display;
mod parts_display_look;
mod parts_frames;
mod parts_handler_fails;
mod parts_label;
mod parts_markdown;
mod parts_markdown_link_cursor;
mod parts_panel;
mod parts_popover;
mod parts_read_only_rule;
mod parts_scroll_view;
mod parts_small_display;
mod parts_small_text_parts;
mod parts_splitter;
mod parts_steps;
mod parts_tabs;
mod press_counter;
mod text_box;
mod text_editor;
