//! A pixel read past the edge of a shot fails its test instead of reading
//! memory that is not the picture's, and no test reads a pixel any other way.

use clarity_ui::element::GridPanel;
use clarity_ui::testing::Shot;
use clarity_ui::{Screen, ThemeLook};

#[test]
fn a_pixel_past_the_edge_fails_the_test_instead_of_crashing_the_run() {
    let mut screen = Screen::new(Box::new(GridPanel::new(1)), ThemeLook::clarity(false));
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(shot.pixel(799, 399), ThemeLook::clarity(false).color("background"));
    // A test once read row 405 of a 400 row shot.
    for (x, y) in [(30, 405), (800, 0), (-1, 0), (0, -1)] {
        let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| shot.pixel(x, y)));
        assert!(read.is_err(), "{x},{y} was read");
    }
}

#[test]
fn tests_read_pixels_only_through_pixel_at() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    // Written in two pieces, so this file does not find itself.
    let needle = format!(".{}(", "get_color");
    let mut unchecked = Vec::new();
    let mut folders = vec![folder.clone()];
    while let Some(dir) = folders.pop() {
        for entry in std::fs::read_dir(&dir).expect("the tests folder reads").flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for (i, line) in text.lines().enumerate() {
                if line.contains(&needle) {
                    unchecked.push(format!("{}:{}", path.strip_prefix(&folder).unwrap_or(&path).display(), i + 1));
                }
            }
        }
    }
    assert!(unchecked.is_empty(), "read pixels with testing::pixel_at or Shot::pixel, not get_color: {}", unchecked.join(", "));
}
