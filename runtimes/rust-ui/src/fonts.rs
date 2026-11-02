//! The fonts text is drawn in: a theme's font role names its families, most
//! wanted first (`system-ui, Segoe UI, sans-serif`); the first the system
//! has is taken. Where the system has none, Clarity.UI's own Inter is (it
//! carries Regular and Bold, under the SIL Open Font License).
//!
//! A character the role's font lacks (Japanese, Chinese, Korean, Arabic in
//! Inter) is drawn in a fallback font, a run of text at a time: first the
//! fonts the application carries (`resources/fonts/*.ttf` or `*.otf`, or
//! given with `add_fallback`), then the system's font for that character.
//!
//! A line of text is shaped once per text, font role, size and weight
//! (`ShapedText`, kept in a cache), and measured, drawn and hit by the caret
//! from that one shape, so they agree.

pub mod bidi;
pub mod shaped;
pub mod unicode;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use skia_safe::{Canvas, Data, Font, FontMgr, FontStyle, Paint, Typeface};

pub use shaped::{ShapedText, TextRun};

use crate::theme::ThemeLook;

const INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");

/// How many shapes a `Fonts` keeps before it starts again.
const MOST_SHAPES: usize = 4096;

/// The fallback fonts added, as data, in the order they were added: every thread reads them.
static FALLBACKS: Mutex<Vec<Arc<[u8]>>> = Mutex::new(Vec::new());

/// Counts the fallbacks added: a shape or a fallback found before one was added may be another now.
static FALLBACKS_ADDED: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static CARRIED: RefCell<Option<(Typeface, Typeface)>> = const { RefCell::new(None) };
    /// This thread's typefaces of the added fallbacks, and how many were added when they were made.
    static CARRIED_FALLBACKS: RefCell<(usize, Vec<Typeface>)> = const { RefCell::new((usize::MAX, Vec::new())) };
    /// The fallback found for a character and weight; None where nothing has it.
    static FALLBACK_OF: RefCell<(usize, HashMap<(char, bool), Option<Typeface>>)> = RefCell::new((usize::MAX, HashMap::new()));
}

/// Clarity.UI's own font, Inter, regular or bold.
pub fn carried(bold: bool) -> Typeface {
    CARRIED.with(|cell| {
        let mut cell = cell.borrow_mut();
        let (regular, heavy) = cell.get_or_insert_with(|| {
            let manager = FontMgr::new();
            let load = |bytes: &'static [u8]| {
                manager.new_from_data(Data::new_copy(bytes), None).expect("Clarity.UI carries Inter, which Skia reads")
            };
            (load(INTER_REGULAR), load(INTER_BOLD))
        });
        if bold { heavy.clone() } else { regular.clone() }
    })
}

/// Adds a font (TrueType or OpenType data) for the characters the theme's
/// fonts lack; fallbacks added first are asked first. False when the data
/// is no font.
pub fn add_fallback(font: &[u8]) -> bool {
    if FontMgr::new().new_from_data(Data::new_copy(font), None).is_none() {
        return false;
    }
    FALLBACKS.lock().unwrap_or_else(|e| e.into_inner()).push(Arc::from(font));
    FALLBACKS_ADDED.fetch_add(1, Ordering::SeqCst);
    true
}

/// Adds the fallback fonts an application carries (`AppFiles::resources`):
/// its resources under `fonts/` ending in `.ttf` or `.otf`, in the order of
/// their names. A file that is no font is said and left out. How many it added.
pub fn carry_fallbacks(resources: &[(&str, &[u8])]) -> usize {
    let mut fonts: Vec<&(&str, &[u8])> = resources
        .iter()
        .filter(|(name, _)| {
            let name = name.to_ascii_lowercase().replace('\\', "/");
            (name.starts_with("fonts/") || name.starts_with("resources/fonts/")) && (name.ends_with(".ttf") || name.ends_with(".otf"))
        })
        .collect();
    fonts.sort_by(|a, b| a.0.cmp(b.0));
    let mut added = 0;
    for (name, bytes) in fonts {
        if add_fallback(bytes) {
            added += 1;
        } else {
            eprintln!("the fallback font {name} does not load");
        }
    }
    added
}

/// The typefaces of the added fallbacks, made on this thread.
fn carried_fallbacks() -> Vec<Typeface> {
    let added = FALLBACKS_ADDED.load(Ordering::SeqCst);
    CARRIED_FALLBACKS.with(|cell| {
        let mut cell = cell.borrow_mut();
        if cell.0 != added {
            let manager = FontMgr::new();
            let data: Vec<Arc<[u8]>> = FALLBACKS.lock().unwrap_or_else(|e| e.into_inner()).clone();
            cell.1 = data.iter().filter_map(|bytes| manager.new_from_data(Data::new_copy(bytes), None)).collect();
            cell.0 = added;
        }
        cell.1.clone()
    })
}

/// Whether `typeface` has a glyph for `character`.
pub fn has(typeface: &Typeface, character: char) -> bool {
    typeface.unichar_to_glyph(character as i32) != 0
}

/// Whether `typeface` has a glyph for every character of `text`.
pub fn has_all(typeface: &Typeface, text: &str) -> bool {
    text.chars().all(|c| has(typeface, c))
}

/// The font for a character the theme's fonts lack: a carried one that has
/// it, else the system's; None when nothing has it.
fn fallback(character: char, bold: bool) -> Option<Typeface> {
    let added = FALLBACKS_ADDED.load(Ordering::SeqCst);
    FALLBACK_OF.with(|cell| {
        let mut cell = cell.borrow_mut();
        if cell.0 != added {
            cell.1.clear();
            cell.0 = added;
        }
        cell.1
            .entry((character, bold))
            .or_insert_with(|| {
                if let Some(found) = carried_fallbacks().into_iter().find(|t| has(t, character)) {
                    return Some(found);
                }
                let style = if bold { FontStyle::bold() } else { FontStyle::normal() };
                FontMgr::new().match_family_style_character("", style, &[], character as i32).filter(|t| has(t, character))
            })
            .clone()
    })
}

/// Whether two typefaces are one font: the system hands out a new typeface
/// for the same font for each character it is asked about.
pub fn same_face(a: &Typeface, b: &Typeface) -> bool {
    a.unique_id() == b.unique_id() || (a.family_name() == b.family_name() && a.font_style() == b.font_style())
}

/// The key a shape is kept under: text, role, bold, size and unhinted.
type ShapeKey = (String, String, bool, u64, bool);

/// The fonts of a theme: a typeface per font role and weight, found once.
pub struct Fonts {
    look: Arc<ThemeLook>,
    manager: FontMgr,
    typefaces: RefCell<HashMap<(String, bool), Typeface>>,
    shapes: RefCell<HashMap<ShapeKey, Rc<ShapedText>>>,
    shapes_for: Cell<usize>,
}

impl Fonts {
    pub fn new(look: Arc<ThemeLook>) -> Fonts {
        Fonts {
            look,
            manager: FontMgr::new(),
            typefaces: RefCell::new(HashMap::new()),
            shapes: RefCell::new(HashMap::new()),
            shapes_for: Cell::new(usize::MAX),
        }
    }

    /// The typeface of a font role of the theme.
    pub fn typeface(&self, role: &str, bold: bool) -> Typeface {
        if let Some(found) = self.typefaces.borrow().get(&(role.to_string(), bold)) {
            return found.clone();
        }
        let typeface = self.of_families(&self.look.font(role), bold);
        self.typefaces.borrow_mut().insert((role.to_string(), bold), typeface.clone());
        typeface
    }

    fn of_families(&self, families: &str, bold: bool) -> Typeface {
        let style = if bold { FontStyle::bold() } else { FontStyle::normal() };
        for written in families.split(',') {
            let family = match written.trim().trim_matches(|c| c == '"' || c == '\'') {
                "system-ui" | "-apple-system" | "sans-serif" => "sans-serif",
                "ui-monospace" | "monospace" => "monospace",
                "serif" | "ui-serif" => "serif",
                named => named,
            };
            if family == "Inter" {
                return carried(bold);
            }
            if !family.is_empty() {
                if let Some(typeface) = self.manager.match_family_style(family, style) {
                    return typeface;
                }
            }
        }
        carried(bold)
    }

    /// The font of a role at a size.
    pub fn font(&self, role: &str, size: f64, bold: bool) -> Font {
        shaped::font_of(&self.typeface(role, bold), size, false, 0.0)
    }

    /// The text in runs, each in one typeface: the role's where it has the
    /// characters, a fallback's where it lacks them. Spaces and marks that
    /// join a character stay in the run before them.
    pub fn runs(&self, text: &str, role: &str, bold: bool) -> Vec<TextRun> {
        let main = self.typeface(role, bold);
        if text.is_empty() || has_all(&main, text) {
            return vec![TextRun { text: text.to_string(), typeface: main }];
        }
        let mut runs = Vec::new();
        let mut run = String::new();
        let mut current: Option<Typeface> = None;
        for c in text.chars() {
            let joins = c.is_whitespace() || c.is_control() || unicode::is_mark(c) || unicode::is_format(c);
            let typeface = match &current {
                Some(now) if joins => now.clone(),
                _ if has(&main, c) => main.clone(),
                _ => fallback(c, bold).unwrap_or_else(|| main.clone()),
            };
            if let Some(now) = &current {
                if !same_face(now, &typeface) {
                    runs.push(TextRun { text: std::mem::take(&mut run), typeface: now.clone() });
                }
            }
            current = Some(typeface);
            run.push(c);
        }
        if let Some(now) = current {
            runs.push(TextRun { text: run, typeface: now });
        }
        runs
    }

    /// `text`, one line, as it is drawn in a font role at `size`: shaped, in
    /// visual order, with its caret places. `unhinted` measures as a zoomed
    /// canvas draws.
    pub fn shape_unhinted(&self, text: &str, role: &str, size: f64, bold: bool, unhinted: bool) -> Rc<ShapedText> {
        let added = FALLBACKS_ADDED.load(Ordering::SeqCst);
        if self.shapes_for.get() != added {
            self.shapes.borrow_mut().clear();
            self.shapes_for.set(added);
        }
        let key = (text.to_string(), role.to_string(), bold, size.to_bits(), unhinted);
        if let Some(known) = self.shapes.borrow().get(&key) {
            return known.clone();
        }
        let shaped = Rc::new(ShapedText::of(text, &self.runs(text, role, bold), size, unhinted));
        let mut shapes = self.shapes.borrow_mut();
        if shapes.len() >= MOST_SHAPES {
            shapes.clear();
        }
        shapes.insert(key, shaped.clone());
        shaped
    }

    /// `text`, one line, shaped as it is drawn in a font role at `size`.
    pub fn shape(&self, text: &str, role: &str, size: f64, bold: bool) -> Rc<ShapedText> {
        self.shape_unhinted(text, role, size, bold, false)
    }

    /// How wide `text` is in a font role, at `size`, as it is drawn.
    pub fn measure(&self, text: &str, role: &str, size: f64, bold: bool) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        self.shape(text, role, size, bold).width()
    }

    /// Draws a line of `text` from `x` at `baseline`, shaped, each run in its
    /// own typeface; how wide it was.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&self, canvas: &Canvas, text: &str, x: f64, baseline: f64, role: &str, size: f64, bold: bool, paint: &Paint) -> f64 {
        self.draw_slanted(canvas, text, x, baseline, role, size, bold, paint, 0.0)
    }

    /// `draw`, slanted by `skew_x` for italics.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_slanted(&self, canvas: &Canvas, text: &str, x: f64, baseline: f64, role: &str, size: f64, bold: bool, paint: &Paint, skew_x: f32) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        let shaped = self.shape(text, role, size, bold);
        shaped.draw(canvas, x, baseline, paint, skew_x);
        shaped.width()
    }
}
