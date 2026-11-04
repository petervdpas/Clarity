//! The grid against the cases every version of Clarity.UI shares
//! (conformance/grid-layout.json), and the room a part needs around its rows.

use clarity_ui::grid::{place, place_rows, GridItem, ROW_HEIGHT};
use serde_json::Value;

#[test]
fn a_grid_places_its_parts_as_every_version_does() {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/grid-layout.json")).unwrap()).unwrap();
    assert_eq!(ROW_HEIGHT, file["row_height"].as_f64().unwrap());
    let cases = file["cases"].as_array().unwrap();
    assert!(cases.iter().any(|c| c["row_height"].as_f64().is_some_and(|r| r != ROW_HEIGHT)), "cases at another row height");
    for case in cases {
        let what = case["what"].as_str().unwrap();
        let parts: Vec<GridItem> = case["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| GridItem::new(p["col"].as_i64().unwrap(), p["row"].as_i64().unwrap(), p["width"].as_i64().unwrap(), p["height"].as_i64().unwrap(), p["grows"].as_bool().unwrap()).at_bottom(p["bottom"].as_bool().unwrap_or(false)))
            .collect();
        if what.contains("bottom") {
            assert!(parts.iter().any(|p| p.bottom), "{what}: a part marked bottom");
        }
        let row = case["row_height"].as_f64().unwrap_or(ROW_HEIGHT);
        let placed = place_rows(&parts, case["columns"].as_i64().unwrap(), case["width"].as_f64().unwrap(), case["height"].as_f64().unwrap(), case["gap"].as_f64().unwrap(), row);
        // The same as the windows lay out once the project's row height is set.
        clarity_ui::grid::set_row_height(row);
        assert_eq!(placed, place(&parts, case["columns"].as_i64().unwrap(), case["width"].as_f64().unwrap(), case["height"].as_f64().unwrap(), case["gap"].as_f64().unwrap()), "{what}");
        clarity_ui::grid::set_row_height(ROW_HEIGHT);
        let gives = &case["gives"];
        assert!((gives["content"].as_f64().unwrap() - placed.content).abs() < 0.001, "{what}: content {}", placed.content);
        let boxes = gives["boxes"].as_array().unwrap();
        assert_eq!(boxes.len(), placed.boxes.len(), "{what}");
        for (i, want) in boxes.iter().enumerate() {
            let want: Vec<f64> = want.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            let got = placed.boxes[i];
            let have = [got.x, got.y, got.width, got.height];
            for k in 0..4 {
                assert!((want[k] - have[k]).abs() < 0.001, "{what}: part {} is at {got}, not {want:?}", i + 1);
            }
        }
    }
}

/// The `made` cases: each part made from its markup says itself whether it
/// grows (a custom part by its `grows`, as its declaration says).
#[test]
fn parts_made_from_their_markup_grow_as_every_version_does() {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/grid-layout.json")).unwrap()).unwrap();
    let cases = file["made"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let what = case["what"].as_str().unwrap();
        let host = clarity_ui::testing::TestHost::new();
        let parts: Vec<GridItem> = case["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let window = clarity_ui::markup::read(&format!("<Window name=\"Test\">{}</Window>", p.as_str().unwrap())).unwrap();
                let part = clarity_ui::parts::make(window.parts.into_iter().next().unwrap(), host.clone());
                clarity_ui::parts::place_of(part.as_ref())
            })
            .collect();
        let placed = place(&parts, case["columns"].as_i64().unwrap(), case["width"].as_f64().unwrap(), case["height"].as_f64().unwrap(), case["gap"].as_f64().unwrap());
        let gives = &case["gives"];
        assert!((gives["content"].as_f64().unwrap() - placed.content).abs() < 0.001, "{what}: content {}", placed.content);
        for (i, want) in gives["boxes"].as_array().unwrap().iter().enumerate() {
            let want: Vec<f64> = want.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            let got = placed.boxes[i];
            let have = [got.x, got.y, got.width, got.height];
            for k in 0..4 {
                assert!((want[k] - have[k]).abs() < 0.001, "{what}: part {} is at {got}, not {want:?}", i + 1);
            }
        }
    }
}

#[test]
fn a_part_that_needs_room_around_its_rows_makes_them_higher() {
    let parts = [GridItem { around: 30.0, ..GridItem::new(1, 1, 1, 2, false) }, GridItem::new(2, 3, 1, 1, false)];
    let placed = place(&parts, 2, 400.0, 600.0, 8.0);
    // Two rows of 36 with the gap, and the room around: the next row starts below it.
    assert_eq!(36.0 * 2.0 + 8.0 + 30.0, placed.boxes[0].height);
    assert_eq!(placed.boxes[0].bottom() + 8.0, placed.boxes[1].y);
}

#[test]
fn an_empty_grid_is_one_row_high() {
    let placed = place(&[], 12, 800.0, 600.0, 8.0);
    assert!(placed.boxes.is_empty());
    assert_eq!(ROW_HEIGHT, placed.content);
}

/// The designer draws its rows by their heights: the height a part marked
/// `bottom` moves down by shows as the row above it.
#[test]
fn the_rows_above_a_part_at_the_bottom_take_the_spare_height_for_the_designer() {
    let parts = [GridItem::new(1, 1, 1, 1, false), GridItem::new(1, 3, 1, 1, false).at_bottom(true)];
    let placed = place_rows(&parts, 1, 200.0, 300.0, 8.0, 36.0);
    assert_eq!(placed.rows, vec![36.0, 36.0 + 176.0, 36.0]);
    assert_eq!(placed.boxes[1].y, 264.0);
    assert_eq!(placed.content, 300.0);
}
