//! The inputs: choices, dates, numbers, files, folders, colors and tags.
//! One test binary for these files (each a module, its tests named
//! `<file>::<test>`), so Skia is linked once for them all.

#[allow(dead_code, unused_imports)]
#[path = "../common/data_rows_host.rs"]
mod data_rows_host;
#[allow(dead_code, unused_imports)]
#[path = "../common/inputs_program.rs"]
mod inputs_program;
#[allow(dead_code, unused_imports)]
#[path = "../common/inputs_test_picker.rs"]
mod inputs_test_picker;

mod inputs_choice;
mod inputs_color;
mod inputs_date;
mod inputs_date_calendar_view;
mod inputs_dropdown;
mod inputs_file;
mod inputs_folder;
mod inputs_form_case;
mod inputs_formats;
mod inputs_input_parity;
mod inputs_input_undo;
mod inputs_record_keys;
mod inputs_number;
mod inputs_picture_field;
mod inputs_radio;
mod inputs_slider;
mod inputs_tags;
mod inputs_tags_keyboard;
mod inputs_toggle_link;
mod inputs_typed_dropdown;
mod inputs_typed_then_key;
mod looks_calendar_arrows;
