//! The screen: pointer, keys, focus, layout, drawing, looks and text shaping.
//! One test binary for these files (each a module, its tests named
//! `<file>::<test>`), so Skia is linked once for them all.

mod check_fix_screen;
mod core_context_menu;
mod core_pixels;
mod core_tooltip;
mod drawing;
mod g8c_live_buttons;
mod g8c_live_popover;
mod g8c_live_text;
mod grid_around;
mod grid_layout;
mod looks_contrast;
mod markup;
mod pixels;
mod screen;
mod screen_hooks;
mod screen_parity;
mod screen_tree;
mod shaping;
mod shortcuts;
mod theme_look;
