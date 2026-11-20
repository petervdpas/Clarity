//! Clarity.Pdf: Markdown laid out into pages by Typst and written as a PDF
//! (docs/ledger/reports-pdf.md). The Markdown is read into blocks as every
//! Clarity renderer reads it (`clarity_runtime::markdown`); its look comes
//! from a stylesheet, a subset of CSS over Clarity's own sheet, which may
//! take the app's theme's fonts and colors. Each block becomes Typst code in
//! which every text is a Typst string, so nothing in the text is read as
//! Typst. The fonts are carried in the crate: making a PDF needs no internet
//! and gives the same file every time.

mod css;
mod fonts;
mod paint;
mod report;
mod style;
mod theme;
mod typst_out;

use std::collections::BTreeSet;
use std::sync::OnceLock;

use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::layout::{Frame, FrameItem, Point};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook, FontStyle};
use typst::utils::LazyHash;
use typst::visualize::{Color, Paint};
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

pub use paint::{Item, PathOp, Rgba};
pub use style::Stylesheet;
pub use theme::Theme;

/// What is wrong in a stylesheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssErrorKind {
    /// A selector or at-rule a stylesheet does not have.
    UnknownSelector,
    /// A property the selector does not take.
    UnknownProperty,
    /// A value the property does not take.
    BadValue,
    /// A block never closed with `}`.
    NotClosed,
    /// A `font-family` list with no font that is carried.
    NoCarriedFont,
    /// A `var()` the theme does not give, with no fallback.
    UnknownVariable,
}

/// An error in a stylesheet: where, what kind and what it is about (a
/// selector, a property, `property: value`, a font list or a variable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssError {
    pub line: usize,
    pub kind: CssErrorKind,
    pub subject: String,
}

impl std::fmt::Display for CssError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line = self.line;
        let subject = &self.subject;
        match self.kind {
            CssErrorKind::UnknownSelector => write!(f, "line {line}: a stylesheet has no '{subject}'"),
            CssErrorKind::UnknownProperty => write!(f, "line {line}: '{subject}' is not taken there"),
            CssErrorKind::BadValue => write!(f, "line {line}: '{subject}' is no value it takes"),
            CssErrorKind::NotClosed => write!(f, "line {line}: '{subject}' is not closed with }}"),
            CssErrorKind::NoCarriedFont => write!(f, "line {line}: no font of '{subject}' is carried"),
            CssErrorKind::UnknownVariable => write!(f, "line {line}: the theme gives no '{subject}'"),
        }
    }
}

impl std::error::Error for CssError {}

/// Why no PDF was made: Typst's messages.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfError(pub Vec<String>);

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.join("; "))
    }
}

impl std::error::Error for PdfError {}

fn error(diagnostics: impl IntoIterator<Item = SourceDiagnostic>) -> PdfError {
    PdfError(diagnostics.into_iter().map(|d| d.message.to_string()).collect())
}

/// Why `made` gave no PDF.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// The stylesheet's errors, each with its line.
    Stylesheet(Vec<CssError>),
    /// The theme is no theme file.
    Theme(String),
    /// Typst laid out no pages.
    Layout(PdfError),
    /// The report, its data or its texts are not right.
    Report(String),
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::Stylesheet(errors) => {
                let said: Vec<String> = errors.iter().map(ToString::to_string).collect();
                write!(f, "the stylesheet is not right: {}", said.join("; "))
            }
            Problem::Theme(why) => write!(f, "the theme is not right: {why}"),
            Problem::Layout(why) => write!(f, "the PDF could not be made: {why}"),
            Problem::Report(why) => write!(f, "the report is not right: {why}"),
        }
    }
}

impl std::error::Error for Problem {}

/// A PDF of a Markdown text, as `pdf of text stylesheet css theme true` makes
/// it on every stack: the stylesheet's CSS (empty for Clarity's own sheet
/// alone) and the theme file's JSON (empty for none).
pub fn made(markdown: &str, stylesheet: &str, theme: &str) -> Result<Vec<u8>, Problem> {
    let theme = if theme.trim().is_empty() { None } else { Some(Theme::read(theme).map_err(Problem::Theme)?) };
    let sheet = Stylesheet::read(stylesheet, theme.as_ref()).map_err(Problem::Stylesheet)?;
    pdf(markdown, &sheet).map_err(Problem::Layout)
}

/// A report (`.report`) laid out with its data (JSON, by the names of its
/// Data) and its texts (JSON, by key), in its stylesheet (empty for
/// Clarity's own alone; its classes give the parts their look) reading the
/// theme file's JSON (empty for none).
pub fn report_layout(report: &str, data: &str, texts: &str, stylesheet: &str, theme: &str) -> Result<Laid, Problem> {
    report_layout_with(report, data, texts, stylesheet, theme, &ReportExtras::default())
}

/// What a report is laid out with besides its data and texts: the program's
/// pictures its Image parts name, and how its values show (the language and
/// currency, and each bind's field type as Clarity writes formats: `money`,
/// `decimal 2`, `date`), which the program carries beside the report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReportExtras {
    pub images: std::collections::BTreeMap<String, Vec<u8>>,
    pub language: Option<String>,
    pub currency: Option<String>,
    pub types: std::collections::BTreeMap<String, String>,
}

/// Where a report's pictures lie for Typst: one file each, by number.
pub(crate) const PICTURES: &str = "/pictures/";

/// `report_layout` with the program's pictures and how its values show.
pub fn report_layout_with(report: &str, data: &str, texts: &str, stylesheet: &str, theme: &str, extras: &ReportExtras) -> Result<Laid, Problem> {
    let read = report::Report::read(report).map_err(Problem::Report)?;
    let json = |text: &str, what: &str| -> Result<serde_json::Value, Problem> {
        if text.trim().is_empty() { Ok(serde_json::Value::Object(Default::default())) } else { serde_json::from_str(text).map_err(|e| Problem::Report(format!("the {what} are no JSON: {e}"))) }
    };
    let (data, texts) = (json(data, "data")?, json(texts, "texts")?);
    let theme = if theme.trim().is_empty() { None } else { Some(Theme::read(theme).map_err(Problem::Theme)?) };
    let sheet = Stylesheet::read(stylesheet, theme.as_ref()).map_err(Problem::Stylesheet)?;
    let source = |broken: &BTreeSet<String>| {
        let files = std::cell::RefCell::new(Vec::new());
        let text = read.source(&data, &texts, &sheet, broken, extras, &files).map_err(Problem::Report)?;
        Ok::<_, Problem>((text, files.into_inner()))
    };
    let compiled = |(text, files): (String, Vec<Vec<u8>>)| compile_with(text, files);
    match compiled(source(&BTreeSet::new())?) {
        Ok(document) => Ok(Laid { document }),
        Err(e) => {
            let broken: BTreeSet<String> = typst_out::formulas(&read.markdown_texts(&data))
                .into_iter()
                .filter(|tex| typst_out::formula_alone(tex).is_none_or(|alone| compile(alone).is_err()))
                .collect();
            if broken.is_empty() {
                return Err(Problem::Layout(e));
            }
            compiled(source(&broken)?).map(|document| Laid { document }).map_err(Problem::Layout)
        }
    }
}

/// A report with its pictures as a PDF file; see `report_layout_with`.
pub fn report_with(report: &str, data: &str, texts: &str, stylesheet: &str, theme: &str, extras: &ReportExtras) -> Result<Vec<u8>, Problem> {
    report_layout_with(report, data, texts, stylesheet, theme, extras)?.pdf().map_err(Problem::Layout)
}

/// A report as a PDF file; see `report_layout`.
pub fn report(report: &str, data: &str, texts: &str, stylesheet: &str, theme: &str) -> Result<Vec<u8>, Problem> {
    report_layout(report, data, texts, stylesheet, theme)?.pdf().map_err(Problem::Layout)
}

/// A run of text on a laid-out page, in one look.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub text: String,
    /// Where it starts from the page's left, in millimetres.
    pub x: f64,
    /// Where its baseline lies from the page's top, in millimetres.
    pub y: f64,
    /// How wide it is, in millimetres.
    pub width: f64,
    /// Its size in points.
    pub size: f64,
    /// Its font's family.
    pub font: String,
    pub bold: bool,
    pub italic: bool,
    /// Its color as `#rrggbb` (`#rrggbbaa` when not opaque).
    pub color: String,
}

/// A filled or stroked area on a laid-out page (a background, a bar, a
/// line), by its bounds in millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: String,
}

/// Pages laid out, to read or write as a PDF.
pub struct Laid {
    document: PagedDocument,
}

impl Laid {
    /// How many pages there are.
    pub fn pages(&self) -> usize {
        self.document.pages().len()
    }

    /// A page's width and height in millimetres.
    pub fn page_size(&self, page: usize) -> (f64, f64) {
        let size = self.document.pages()[page].frame.size();
        (size.x.to_mm(), size.y.to_mm())
    }

    /// The runs of text of a page, in the order they are laid out.
    pub fn runs(&self, page: usize) -> Vec<Run> {
        let mut found = Found::default();
        collect(&self.document.pages()[page].frame, Point::zero(), &mut found);
        found.runs
    }

    /// The areas of a page: backgrounds, bars and lines.
    pub fn areas(&self, page: usize) -> Vec<Area> {
        let mut found = Found::default();
        collect(&self.document.pages()[page].frame, Point::zero(), &mut found);
        found.areas
    }

    /// What to draw of a page, for Clarity.UI's painter: glyph runs, shapes and clips, in points.
    pub fn items(&self, page: usize) -> Vec<Item> {
        let mut out = Vec::new();
        paint::items(&self.document.pages()[page].frame, typst::layout::Transform::identity(), &shared().fonts, &mut out);
        out
    }

    /// A page as SVG, for a preview drawn by a browser (the IDE's).
    pub fn svg(&self, page: usize) -> String {
        typst_svg::svg(&self.document.pages()[page], &Default::default())
    }

    /// The pages as a PDF file.
    pub fn pdf(&self) -> Result<Vec<u8>, PdfError> {
        typst_pdf::pdf(&self.document, &typst_pdf::PdfOptions::default()).map_err(error)
    }
}

#[derive(Default)]
struct Found {
    runs: Vec<Run>,
    areas: Vec<Area>,
}

fn hex(paint: &Paint) -> String {
    match paint {
        Paint::Solid(color) => color.to_hex().to_string(),
        _ => Color::BLACK.to_hex().to_string(),
    }
}

fn collect(frame: &Frame, at: Point, found: &mut Found) {
    for (place, item) in frame.items() {
        let place = at + *place;
        match item {
            FrameItem::Group(group) => collect(&group.frame, place + Point::new(group.transform.tx, group.transform.ty), found),
            FrameItem::Text(text) => {
                let info = text.font.font().info();
                found.runs.push(Run {
                    text: text.text.to_string(),
                    x: place.x.to_mm(),
                    y: place.y.to_mm(),
                    width: text.width().to_mm(),
                    size: text.size.to_pt(),
                    font: info.family.clone(),
                    bold: info.variant.weight.to_number() >= 600,
                    italic: info.variant.style != FontStyle::Normal,
                    color: hex(&text.fill),
                });
            }
            FrameItem::Shape(shape, _) => {
                let paint = shape.fill.as_ref().or(shape.stroke.as_ref().map(|s| &s.paint));
                let Some(paint) = paint else { continue };
                let bounds = shape.bbox(true);
                found.areas.push(Area {
                    x: (place.x + bounds.min.x).to_mm(),
                    y: (place.y + bounds.min.y).to_mm(),
                    width: (bounds.max.x - bounds.min.x).to_mm(),
                    height: (bounds.max.y - bounds.min.y).to_mm(),
                    color: hex(paint),
                });
            }
            _ => {}
        }
    }
}

/// Lays out a Markdown text into pages in a stylesheet's look. A formula
/// Typst cannot set shows its TeX as code does, and the rest lays out.
pub fn layout(markdown: &str, sheet: &Stylesheet) -> Result<Laid, PdfError> {
    match compile(typst_out::source(markdown, sheet, &BTreeSet::new())) {
        Ok(document) => Ok(Laid { document }),
        Err(e) => {
            let broken: BTreeSet<String> =
                typst_out::formulas(markdown).into_iter().filter(|tex| typst_out::formula_alone(tex).is_none_or(|alone| compile(alone).is_err())).collect();
            if broken.is_empty() {
                return Err(e);
            }
            compile(typst_out::source(markdown, sheet, &broken)).map(|document| Laid { document })
        }
    }
}

fn compile(source: String) -> Result<PagedDocument, PdfError> {
    compile_with(source, Vec::new())
}

/// Compiles with pictures as files of their own (`/pictures/0`, `/pictures/1`).
fn compile_with(source: String, pictures: Vec<Vec<u8>>) -> Result<PagedDocument, PdfError> {
    let mut world = OneFile::new(source);
    world.pictures = pictures.into_iter().map(Bytes::new).collect();
    typst::compile::<PagedDocument>(&world).output.map_err(error)
}

/// A Markdown text as a PDF file in a stylesheet's look.
pub fn pdf(markdown: &str, sheet: &Stylesheet) -> Result<Vec<u8>, PdfError> {
    layout(markdown, sheet)?.pdf()
}

/// The Typst source a Markdown text is laid out from.
pub fn typst_source(markdown: &str, sheet: &Stylesheet) -> String {
    typst_out::source(markdown, sheet, &BTreeSet::new())
}

/// A carried font's file and its index in it, by the number an `Item::Glyphs` names.
pub fn font_data(font: usize) -> Option<(&'static [u8], u32)> {
    shared().fonts.get(font).map(|f| (f.data().as_slice(), f.index()))
}

/// The carried fonts and the library, made once.
struct Shared {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| {
        let fonts = fonts::all();
        Shared { library: LazyHash::new(Library::builder().build()), book: LazyHash::new(FontBook::from_fonts(&fonts)), fonts }
    })
}

/// A Typst world of one source, MiTeX's carried definitions and the carried
/// fonts: no other files, no packages.
struct OneFile {
    main: FileId,
    source: Source,
    /// A report's pictures, read as `/pictures/<number>`.
    pictures: Vec<Bytes>,
}

impl OneFile {
    fn new(text: String) -> OneFile {
        let path = VirtualPath::new("main.typ").expect("a plain file name");
        let main = FileId::unique(RootedPath::new(VirtualRoot::Project, path));
        OneFile { main, source: Source::new(main, text), pictures: Vec::new() }
    }
}

/// MiTeX's definitions as Typst sources, made once.
fn mitex() -> &'static [Source] {
    static SOURCES: OnceLock<Vec<Source>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        typst_out::MITEX
            .iter()
            .map(|(path, text)| {
                let id = FileId::new(RootedPath::new(VirtualRoot::Project, VirtualPath::new(path).expect("a plain path")));
                Source::new(id, text.to_string())
            })
            .collect()
    })
}

impl World for OneFile {
    fn library(&self) -> &LazyHash<Library> {
        &shared().library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &shared().book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            return Ok(self.source.clone());
        }
        mitex().iter().find(|s| s.id() == id).cloned().ok_or(FileError::AccessDenied)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().get_with_slash().to_string();
        if let Some(number) = path.strip_prefix(PICTURES).and_then(|n| n.parse::<usize>().ok()) {
            return self.pictures.get(number).cloned().ok_or(FileError::AccessDenied);
        }
        self.source(id).map(|s| Bytes::from_string(s.text().to_string()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        shared().fonts.get(index).cloned()
    }

    /// No date: a PDF is the same whenever it is made.
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}
