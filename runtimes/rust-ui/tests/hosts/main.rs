//! The desktop host: its window, keys, file dialogs, sound and growing.
//! One test binary for these files (each a module, its tests named
//! `<file>::<test>`), so Skia is linked once for them all.

#[allow(dead_code, unused_imports)]
#[path = "../common/core_frame_shared.rs"]
mod core_frame_shared;
#[allow(dead_code, unused_imports)]
#[path = "../common/core_windows_fake.rs"]
mod core_windows_fake;

mod desktop_check_fix_hosts;
mod desktop_grow;
mod desktop_host;
mod desktop_keys;
mod desktop_platform;
mod desktop_split;
mod looks_pointer_place;
