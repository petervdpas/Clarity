//! clarity-gen's tests, one test program: each file a module, run one with
//! `cargo test -p clarity-gen --test gen <module>::`. The helpers several
//! share are modules here too, or in clarity-gen-testing when the stack
//! crates' tests share them.

mod ide_build;
mod clarity_live;

mod accordions;
mod agent_tools;
mod app_settings;
mod asks;
mod audio;
mod blob_fields;
mod built;
mod built_ins;
mod button_kinds;
mod canvas_drag;
mod carried;
mod cascade;
mod charts;
mod checks_cover;
mod checks_generate;
mod clarity_engines;
mod clipboard;
mod colors;
mod commands;
mod consts;
mod counting;
mod csharp_web;
mod date_defaults;
mod decimal_division;
mod diagram_tools;
mod dropdown_options;
mod engines;
mod environment;
mod everything;
mod everything_build;
mod everything_live;
mod examples_build;
mod file_checks;
mod file_sources;
mod files;
mod folders;
mod folding;
mod form_labels;
mod held_lists;
mod http;
mod http_build;
mod icons;
mod installed;
mod libraries;
mod library_choices;
mod list_fields;
mod list_items;
mod list_looks;
mod live_canvas;
mod live_charts;
mod loop_items_build;
mod mapping_check;
mod modules;
mod native_parts;
mod no_datasource;
mod out_layout;
mod outline;
mod overflow;
mod overlays;
mod paging;
mod part_properties;
mod pdf;
mod picture_functions_build;
mod pictures;
mod query_id;
mod ranges;
mod readme;
mod record_fields;
mod ref_copy;
mod reference_settings;
mod required_fields;
mod resources;
mod ribbons;
mod screen_engines;
mod search;
mod secrets;
mod servers;
mod settings;
mod shortcut_keys;
mod sidebars;
mod slideouts;
mod small_parts;
mod songs_build;
mod sound;
mod spin_range;
mod splash;
mod table_pages;
mod text_align;
mod texts_build;
mod themes;
mod timers_and_sounds;
mod toolchain;
mod tools;
mod tree_levels;
mod trees;
mod typing_build;
mod value_renames;
mod videos_build;
mod views;
mod web_windows;
mod whole_division;
mod windows_resources;
mod windows_runtime;
mod wizards;

/// Writes a guard's file only when its content differs, so writing it again
/// changes nothing on disk (a running IDE sees no change and does not restart).
pub(crate) fn write_changed(path: impl AsRef<std::path::Path>, bytes: impl AsRef<[u8]>) {
    let (path, bytes) = (path.as_ref(), bytes.as_ref());
    if std::fs::read(path).ok().as_deref() != Some(bytes) {
        std::fs::write(path, bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}
