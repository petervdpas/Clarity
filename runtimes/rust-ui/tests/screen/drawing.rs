//! What a screen draws: the theme's background, a plate at its place in the
//! grid, a line of text inside its box only, and the same at twice the scale.

use clarity_ui::element::{GridPanel, Plate, TextLine};
use clarity_ui::fonts::{self, Fonts};
use clarity_ui::grid::GridItem;
use clarity_ui::testing::Shot;
use clarity_ui::{Screen, ThemeLook};

fn window_with(element: Box<dyn clarity_ui::Element>, columns: i64, place: GridItem) -> Screen {
    let mut grid = GridPanel::new(columns);
    grid.add(element, place);
    Screen::new(Box::new(grid), ThemeLook::clarity(false))
}

#[test]
fn a_window_is_the_themes_background_with_its_parts_at_their_places() {
    let look = ThemeLook::clarity(false);
    let mut screen = window_with(Box::new(Plate::new("accent", None)), 2, GridItem::new(2, 2, 1, 1, false));
    let mut pixels = Shot::of(&mut screen, 400, 100, 1.0);
    // Column 2 starts at 204; row 2 at 44 and is 36 high.
    assert_eq!(pixels.pixel(300, 60), look.color("accent"));
    assert_eq!(pixels.pixel(100, 60), look.color("background"));
    assert_eq!(pixels.pixel(300, 20), look.color("background"));
    assert_eq!(pixels.pixel(300, 90), look.color("background"));
}

#[test]
fn twice_the_scale_draws_the_same_window_in_twice_the_pixels() {
    let look = ThemeLook::clarity(false);
    let mut screen = window_with(Box::new(Plate::new("accent", None)), 2, GridItem::new(2, 2, 1, 1, false));
    let mut pixels = Shot::of(&mut screen, 800, 200, 2.0);
    assert_eq!(pixels.pixel(600, 120), look.color("accent"));
    assert_eq!(pixels.pixel(600, 40), look.color("background"));
}

#[test]
fn text_is_drawn_in_its_box_and_nowhere_else() {
    let look = ThemeLook::clarity(false);
    let mut screen = window_with(Box::new(TextLine::new("Hello, Clarity")), 1, GridItem::new(1, 1, 1, 1, false));
    let mut pixels = Shot::of(&mut screen, 300, 120, 1.0);
    let background = look.color("background");
    let mut inked = 0;
    for y in 0..120 {
        for x in 0..300 {
            if pixels.pixel(x, y) != background {
                assert!(y < 36, "ink at {x},{y}, below the text's row");
                inked += 1;
            }
        }
    }
    assert!(inked > 50, "the text drew nothing");
}

#[test]
fn text_is_measured_with_the_themes_font_size() {
    let fonts = Fonts::new(ThemeLook::clarity(false));
    let small = fonts.measure("Clarity", "text", 12.0, false);
    let large = fonts.measure("Clarity", "text", 24.0, false);
    assert!(small > 10.0);
    assert!(large > small * 1.8);
}

#[test]
fn inter_is_carried_for_where_there_are_no_fonts_of_the_system() {
    assert_eq!(fonts::carried(false).family_name(), "Inter");
    assert_eq!(fonts::carried(true).family_name(), "Inter");
    assert!(*fonts::carried(true).font_style().weight() > *fonts::carried(false).font_style().weight());
}
