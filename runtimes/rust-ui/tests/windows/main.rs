//! Windows: the window stack, its views, frames, dialogs and their hosts.
//! One test binary for these files (each a module, its tests named
//! `<file>::<test>`), so Skia is linked once for them all.

#[allow(dead_code, unused_imports)]
#[path = "../common/core_frame_shared.rs"]
mod core_frame_shared;
#[allow(dead_code, unused_imports)]
#[path = "../common/core_windows_fake.rs"]
mod core_windows_fake;
#[allow(dead_code, unused_imports)]
#[path = "../common/dialogs_fake.rs"]
mod dialogs_fake;

mod access;
mod history;
mod core_choices;
mod core_dialogs;
mod core_frame;
mod core_frame_regions;
mod core_headless;
mod core_long_answer;
mod core_resources;
mod core_return;
mod core_settings_kept;
mod core_system_theme;
mod core_windows;
mod dialogs_ask;
mod dialogs_keys;
mod dialogs_long_call;
mod dialogs_windows;
mod signin;
mod signin_part;
mod slideouts;
mod splash;
mod frame_keyboard;
mod frame_state;
mod looks_escape_dialog;
mod looks_menu_access;
mod looks_system_theme_follow;
mod looks_system_theme_watch;
mod window_stack;
mod window_view;
mod windows_canvas_drag;
mod windows_cases;
mod windows_field_facts;
mod windows_life;
mod windows_sound;
mod windows_ticks;
