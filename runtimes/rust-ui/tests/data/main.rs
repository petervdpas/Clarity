//! The data parts: tables, lists, trees, charts, trends and canvases.
//! One test binary for these files (each a module, its tests named
//! `<file>::<test>`), so Skia is linked once for them all.

#[allow(dead_code, unused_imports)]
#[path = "../common/canvas_host.rs"]
mod canvas_host;
#[allow(dead_code, unused_imports)]
#[path = "../common/data_rows_host.rs"]
mod data_rows_host;

mod canvas_paint;
mod canvas_shape;
mod check_fix_data;
mod core_svg;
mod data_canvas;
mod data_canvas_engine;
mod data_chart;
mod data_list;
mod data_table;
mod data_tree;
mod data_trend;
mod image_svg;
mod table_edge;
mod table_key;
mod table_row;
mod table_sideways;
mod table_tip;
mod table_width;
