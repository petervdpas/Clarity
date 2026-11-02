//! Text shaped as the C# version shapes it (HarfBuzz and the Unicode
//! bidirectional algorithm): Arabic letters joined, Hebrew right to left,
//! mixed Latin and Hebrew each in its own direction, the caret and the
//! pointer on the shaped letters, an ellipsis at the end of a right-to-left
//! label, and Latin drawn exactly as before. The test fonts are Inter
//! (Clarity.UI's own) and Noto Sans Arabic and Noto Sans Hebrew, carried in
//! tests/fonts (SIL Open Font License), so the results are the same on every machine.

use std::rc::Rc;
use std::sync::{Arc, Once};

use clarity_ui::element::{ElementId, GridPanel};
use clarity_ui::fonts::{self, bidi, Fonts};
use clarity_ui::grid::GridItem;
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::parts::containers::markdown::MarkdownLayout;
use clarity_ui::parts::text_editor::TextEditor;
use clarity_ui::{Painter, Screen, ThemeLook};
use skia_safe::{surfaces, Color, Font, Paint};

const SALAAM: &str = "\u{0633}\u{0644}\u{0627}\u{0645}";
const SHALOM: &str = "\u{05e9}\u{05dc}\u{05d5}\u{05dd}";
const SHALOM_OLAM: &str = "\u{05e9}\u{05dc}\u{05d5}\u{05dd} \u{05e2}\u{05d5}\u{05dc}\u{05dd}";
const SIZE: f64 = 16.0;

static CARRIED: Once = Once::new();

/// A look whose text font is Inter, so the scripts it lacks come from the carried fallbacks.
fn look() -> Arc<ThemeLook> {
    CARRIED.call_once(|| {
        for name in ["NotoSansArabic-Regular.ttf", "NotoSansHebrew-Regular.ttf"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts").join(name);
            assert!(fonts::add_fallback(&std::fs::read(path).expect("the test font")));
        }
    });
    ThemeLook::over(r#"{ "fonts": { "text": "Inter" } }"#, ThemeLook::clarity(false))
}

fn new_fonts() -> Fonts {
    Fonts::new(look())
}

fn near(a: f64, b: f64, within: f64) -> bool {
    (a - b).abs() <= within
}

// -- Bidi ---------------------------------------------------------------------

#[test]
fn the_bidi_levels_follow_the_first_strong_letter() {
    assert_eq!(0, bidi::paragraph_level(&format!("abc {SHALOM}")));
    assert_eq!(1, bidi::paragraph_level(&format!("123 {SHALOM} abc")));
    assert_eq!(vec![0, 0, 0, 1, 1, 1, 1, 0], bidi::levels(&format!("ab {SHALOM} "), None));
    // A space between Hebrew words is Hebrew; the line's last space goes to the paragraph's level.
    let olam = bidi::levels(&format!("{SHALOM_OLAM} "), Some(0));
    assert_eq!(vec![1; 9], olam[..9].to_vec());
    assert_eq!(0, *olam.last().unwrap());
    // Numbers in right-to-left text run left to right (level 2).
    assert_eq!(vec![1, 1, 2, 2], bidi::levels("\u{05e9} 12", None));
}

#[test]
fn runs_of_a_higher_level_turn_around() {
    assert_eq!(vec![0, 3, 2, 1, 4], bidi::visual_order(&[0, 1, 1, 1, 0]));
    assert_eq!(vec![2, 1, 0], bidi::visual_order(&[1, 1, 1]));
    assert_eq!(vec![0, 4, 2, 3, 1], bidi::visual_order(&[0, 1, 2, 2, 1]));
}

// -- Shaping ------------------------------------------------------------------

#[test]
fn an_arabic_word_draws_joined() {
    let fonts = new_fonts();
    let word = fonts.shape(SALAAM, "text", SIZE, false);
    assert!(word.shaped());
    // Each letter alone is its isolated form; joined, the letters take their initial, middle and final forms.
    let apart: Vec<_> = SALAAM.chars().map(|c| fonts.shape(&c.to_string(), "text", SIZE, false)).collect();
    let mut isolated: Vec<u16> = apart.iter().flat_map(|s| s.glyphs()).collect();
    assert_eq!(4, isolated.len());
    let mut joined = word.glyphs();
    isolated.sort();
    joined.sort();
    assert_ne!(isolated, joined);
    let apart_width: f64 = apart.iter().map(|s| s.width()).sum();
    assert!(!near(apart_width, word.width(), 1.0), "joined {} apart {apart_width}", word.width());
}

#[test]
fn hebrew_draws_right_to_left() {
    let fonts = new_fonts();
    let word = fonts.shape(SHALOM, "text", SIZE, false);
    assert!(word.right_to_left());
    // The first letter is at the right, the last at the left.
    assert!(word.left_of(0) > word.left_of(1));
    assert!(word.left_of(1) > word.left_of(2));
    assert!(word.left_of(2) > word.left_of(3));
    assert!(near(word.width(), word.right_of(0), 0.001));
    assert!(near(0.0, word.left_of(3), 0.001));
    // The glyphs drawn from the left are the letters' own, last first.
    let mut letters: Vec<u16> = SHALOM.chars().map(|c| fonts.shape(&c.to_string(), "text", SIZE, false).glyphs()[0]).collect();
    letters.reverse();
    assert_eq!(letters, word.glyphs());
}

#[test]
fn hebrew_draws_its_letters_where_it_measures_them() {
    let fonts = new_fonts();
    let word = fonts.shape(SHALOM, "text", 40.0, false);
    let mut surface = surfaces::raster_n32_premul((200, 60)).unwrap();
    surface.canvas().clear(Color::WHITE);
    let mut paint = Paint::default();
    paint.set_color(Color::BLACK);
    paint.set_anti_alias(true);
    let drawn = fonts.draw(surface.canvas(), SHALOM, 10.0, 45.0, "text", 40.0, false, &paint);
    assert_eq!(word.width(), drawn);
    let image = surface.image_snapshot();
    let info = image.image_info();
    let mut pixels = vec![0u8; 200 * 60 * 4];
    assert!(image.read_pixels(&info, &mut pixels, 200 * 4, (0, 0), skia_safe::image::CachingHint::Allow));
    let red = |x: i32, y: i32| {
        let at = (y as usize * 200 + x as usize) * 4;
        // N32 on this system is BGRA or RGBA; black ink is dark in every channel.
        pixels[at].max(pixels[at + 2])
    };
    // Ink inside the measured width, none past it.
    let inked = |x0: i32, x1: i32| (x0..x1).any(|x| (0..60).any(|y| red(x, y) < 128));
    assert!(inked(10, (10.0 + word.width()) as i32));
    assert!(!inked((10.0 + word.width()).ceil() as i32 + 2, 200));
    // Each letter's ink is in its own box: the first letter's at the right.
    assert!(inked((10.0 + word.left_of(0)) as i32 + 2, (10.0 + word.right_of(0)) as i32 - 2));
}

#[test]
fn mixed_latin_and_hebrew_keep_their_own_directions() {
    let fonts = new_fonts();
    // A Latin paragraph: "abc" from the left, then the Hebrew word right to left.
    let latin_first = fonts.shape(&format!("abc {SHALOM}"), "text", SIZE, false);
    assert!(!latin_first.right_to_left());
    assert!(near(0.0, latin_first.left_of(0), 0.001));
    assert!(latin_first.left_of(1) > latin_first.left_of(0));
    assert!(latin_first.left_of(4) > latin_first.left_of(7));
    assert!(latin_first.left_of(7) >= latin_first.right_of(3) - 0.01);
    // A Hebrew paragraph: the Hebrew word at the right, "abc" at the left still left to right.
    let hebrew_first = fonts.shape(&format!("{SHALOM} abc"), "text", SIZE, false);
    assert!(hebrew_first.right_to_left());
    assert!(near(hebrew_first.width(), hebrew_first.right_of(0), 0.001));
    assert!(near(0.0, hebrew_first.left_of(5), 0.001));
    assert!(hebrew_first.left_of(6) > hebrew_first.left_of(5));
    assert!(hebrew_first.left_of(7) > hebrew_first.left_of(6));
}

#[test]
fn latin_text_measures_and_draws_as_before() {
    let fonts = new_fonts();
    let line = fonts.shape("Clarity draws text", "text", SIZE, false);
    assert!(!line.shaped());
    let mut font = Font::from_typeface(fonts::carried(false), SIZE as f32);
    font.set_subpixel(true);
    font.set_edging(skia_safe::font::Edging::SubpixelAntiAlias);
    let measure = |text: &str| font.measure_str(text, None).0 as f64;
    assert!(near(measure("Clarity draws text"), line.width(), 0.001));
    // The caret places add up each letter's width, as the text box measured them.
    assert!(near(measure("C"), line.caret_x(1), 0.001));
    assert!(near(measure("C") + measure("l"), line.caret_x(2), 0.001));

    // Drawn pixel for pixel as Skia draws it.
    let mut paint = Paint::default();
    paint.set_color(Color::BLACK);
    paint.set_anti_alias(true);
    let mut shaped = surfaces::raster_n32_premul((240, 30)).unwrap();
    shaped.canvas().clear(Color::WHITE);
    fonts.draw(shaped.canvas(), "Clarity draws text", 4.0, 20.0, "text", SIZE, false, &paint);
    let mut plain = surfaces::raster_n32_premul((240, 30)).unwrap();
    plain.canvas().clear(Color::WHITE);
    plain.canvas().draw_str("Clarity draws text", (4.0, 20.0), &font, &paint);
    let bytes = |surface: &mut skia_safe::Surface| {
        let image = surface.image_snapshot();
        let mut pixels = vec![0u8; 240 * 30 * 4];
        assert!(image.read_pixels(&image.image_info(), &mut pixels, 240 * 4, (0, 0), skia_safe::image::CachingHint::Allow));
        pixels
    };
    assert!(bytes(&mut plain) == bytes(&mut shaped));
}

#[test]
fn a_line_is_shaped_once_for_its_text_font_and_size() {
    let fonts = new_fonts();
    assert!(Rc::ptr_eq(&fonts.shape(SALAAM, "text", SIZE, false), &fonts.shape(SALAAM, "text", SIZE, false)));
    assert!(!Rc::ptr_eq(&fonts.shape(SALAAM, "text", SIZE, false), &fonts.shape(SALAAM, "text", SIZE + 1.0, false)));
}

#[test]
fn marks_sit_on_their_letter() {
    let fonts = new_fonts();
    // A Hebrew letter with a vowel point is one letter: no caret between them, no width of its own.
    let pointed = fonts.shape("\u{05e9}\u{05b8}", "text", SIZE, false);
    assert!(pointed.shaped());
    assert!(!pointed.is_stop(1));
    assert!(near(fonts.shape("\u{05e9}", "text", SIZE, false).width(), pointed.width(), 1.0));
}

#[test]
fn devanagari_letters_form_their_conjuncts() {
    // Clarity carries no Devanagari font: the system's fallback shapes it, where the system has one.
    let ksha = "\u{0915}\u{094d}\u{0937}";
    let fonts = new_fonts();
    let runs = fonts.runs(ksha, "text", false);
    if !runs.iter().all(|r| fonts::has_all(&r.typeface, &r.text)) {
        eprintln!("skipped: this system has no Devanagari font");
        return;
    }
    let shaped = fonts.shape(ksha, "text", SIZE, false);
    assert!(shaped.shaped());
    // Ka, virama and ssa are three characters; the font draws them as fewer glyphs (the conjunct ksha, or half ka and ssa).
    let names: Vec<String> = runs.iter().map(|r| format!("{}:{}", r.text, r.typeface.family_name())).collect();
    assert!(shaped.glyphs().len() < 3, "glyphs {:?} runs {names:?}", shaped.glyphs());
    assert!(!shaped.is_stop(1));
}

#[test]
fn a_character_the_font_lacks_comes_from_a_fallback() {
    let fonts = new_fonts();
    let runs = fonts.runs(&format!("ab {SHALOM}"), "text", false);
    assert_eq!(2, runs.len());
    assert_eq!("ab ", runs[0].text);
    assert!(fonts::has_all(&runs[1].typeface, SHALOM));
    assert!(!fonts::has(&fonts::carried(false), '\u{05e9}'));
    // Data that is no font is no fallback.
    assert!(!fonts::add_fallback(b"no font"));
}

// -- The caret ----------------------------------------------------------------

fn editor(text: &str) -> (Screen, ElementId) {
    let mut editor = TextEditor::new(false);
    editor.set_text(text);
    editor.framed = false;
    let id = clarity_ui::Element::node(&editor).id();
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(editor), GridItem::new(1, 1, 1, 1, false));
    let mut screen = Screen::new(Box::new(grid), look());
    screen.resize(300.0, 100.0, 1.0);
    screen.focus(Some(id));
    (screen, id)
}

fn ed(screen: &Screen, id: ElementId) -> &TextEditor {
    screen.element(id).and_then(|e| e.as_any().downcast_ref::<TextEditor>()).expect("the editor")
}

fn press(screen: &mut Screen, key: Key, modifiers: Modifiers) {
    screen.key_pressed(KeyInput::new(key, modifiers));
}

#[test]
fn the_caret_in_right_to_left_text_moves_the_way_the_arrow_points() {
    let (mut screen, id) = editor(SHALOM);
    press(&mut screen, Key::Home, Modifiers::NONE);
    assert_eq!(0, ed(&screen, id).caret());
    // The start of Hebrew is at the right: Left goes on into the word.
    press(&mut screen, Key::Left, Modifiers::NONE);
    assert_eq!(1, ed(&screen, id).caret());
    press(&mut screen, Key::Left, Modifiers::NONE);
    assert_eq!(2, ed(&screen, id).caret());
    press(&mut screen, Key::Right, Modifiers::NONE);
    assert_eq!(1, ed(&screen, id).caret());
    press(&mut screen, Key::End, Modifiers::NONE);
    press(&mut screen, Key::Right, Modifiers::NONE);
    assert_eq!(3, ed(&screen, id).caret());
    // Shift with Left selects the letter the caret passes.
    press(&mut screen, Key::Home, Modifiers::NONE);
    press(&mut screen, Key::Left, Modifiers::SHIFT);
    assert_eq!("\u{05e9}", ed(&screen, id).selected_text());
}

#[test]
fn the_caret_in_mixed_text_moves_right_across_the_line() {
    let text = format!("ab {SHALOM}");
    let (mut screen, id) = editor(&text);
    let shape = new_fonts().shape(&text, "text", look().size("text"), false);
    // Rightward from the left edge every step lands further right, through both directions.
    press(&mut screen, Key::Home, Modifiers::NONE);
    let mut seen = vec![shape.caret_x(ed(&screen, id).caret())];
    for _ in 0..text.chars().count() {
        press(&mut screen, Key::Right, Modifiers::NONE);
        seen.push(shape.caret_x(ed(&screen, id).caret()));
    }
    let mut ordered = seen.clone();
    ordered.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(ordered, seen);
    ordered.dedup();
    assert_eq!(text.chars().count() + 1, ordered.len());
}

#[test]
fn a_pointer_in_right_to_left_text_lands_between_the_right_letters() {
    let (screen, id) = editor(SHALOM);
    let look = look();
    let fonts = new_fonts();
    let shape = fonts.shape(SHALOM, "text", look.size("text"), false);
    let editor = ed(&screen, id);
    // From the editor's left; it has no frame, so its text starts there.
    let x0 = 0.0;
    // At the right edge is the start, at the left the end.
    assert_eq!(0, editor.place_at(x0 + shape.width() - 0.5, 5.0, &fonts, &look));
    assert_eq!(4, editor.place_at(x0 + 0.5, 5.0, &fonts, &look));
    // Just right of the line between the first two letters is the place between them.
    assert_eq!(1, editor.place_at(x0 + shape.left_of(0) + 1.0, 5.0, &fonts, &look));
    assert_eq!(2, editor.place_at(x0 + shape.left_of(1) + 1.0, 5.0, &fonts, &look));
}

// -- Labels and pages -----------------------------------------------------------

#[test]
fn an_ellipsis_cuts_a_right_to_left_label_at_its_end_on_the_left() {
    let look = look();
    let fonts = Fonts::new(look.clone());
    let mut surface = surfaces::raster_n32_premul((100, 30)).unwrap();
    let painter = Painter::new(surface.canvas(), &look, &fonts);
    let whole = fonts.measure(SHALOM_OLAM, "text", SIZE, false);
    let cut = painter.fit(SHALOM_OLAM, whole * 0.7, SIZE, "text", false);
    assert!(cut.ends_with('\u{2026}'));
    let count = cut.chars().count();
    assert!(count < SHALOM_OLAM.chars().count());
    let shape = fonts.shape(&cut, "text", SIZE, false);
    // The dots are the leftmost thing drawn; the first letter the rightmost.
    assert!(near(0.0, shape.left_of(count - 1), 0.001));
    assert!(near(shape.width(), shape.right_of(0), 0.001));
}

#[test]
fn a_right_to_left_markdown_paragraph_reads_from_the_right() {
    let look = look();
    let laid = MarkdownLayout::of(SHALOM_OLAM, 400.0, &Fonts::new(look.clone()), &look);
    let first: Vec<_> = laid.runs.iter().filter(|r| r.text.starts_with('\u{05e9}')).collect();
    let second: Vec<_> = laid.runs.iter().filter(|r| r.text.starts_with('\u{05e2}')).collect();
    assert_eq!(1, first.len());
    assert_eq!(1, second.len());
    assert!(first[0].place.x > second[0].place.x);
    assert!(first[0].place.right() > 380.0);
}
