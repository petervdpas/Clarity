//! The pdfReport part (docs/ledger/reports-pdf.md): shows a report of the
//! application (`reports/<Name>.report`, carried among its resources) laid
//! out by Clarity.Pdf with the data code sets (`invoicePdf.order = ...`),
//! page under page, drawn with Skia by `pdf_painter`. When it first shows it
//! runs its `gather` handler, which sets that data. Its options, off unless
//! its line says them: `save` (Save as PDF), `print`, `zoom` and `pages`
//! (page arrows and which page of how many). A program built without PDF
//! reports (Clarity.UI's `pdf` feature) says so in its place.

use std::rc::Rc;

use crate::locale;
use crate::markup::PartSpec;
use crate::parts::{Part, PartHost};

/// Makes a pdfReport part.
pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    #[cfg(feature = "pdf")]
    {
        let view = shown::PdfReportView::new(&spec);
        super::custom::holding(spec, host, Some(Box::new(view)), None)
    }
    #[cfg(not(feature = "pdf"))]
    {
        let why = locale::fill(&locale::translate("reportNotShown", "The report cannot be shown: {why}"), "why", "this program carries no PDF reports");
        super::custom::holding(spec, host, None, Some(why))
    }
}

#[cfg(feature = "pdf")]
pub use shown::{PdfReportView, laid, pdf_of};

/// A PDF of a report of the running program; a program built without PDF reports makes none.
#[cfg(not(feature = "pdf"))]
pub fn pdf_of(_report_name: &str, _data_json: &str) -> Result<Vec<u8>, String> {
    Err("this program carries no PDF reports".to_string())
}

#[cfg(feature = "pdf")]
mod shown {
    use std::cell::Cell;

    use serde_json::{Map, Value};

    use super::locale;
    use crate::geometry::Rect;
    use crate::input::{Modifiers, PointerInput};
    use crate::markup::PartSpec;
    use crate::painter::{Painter, TextAlign};
    use crate::parts::custom::{CustomCx, CustomPart};

    /// The height of its bar of buttons, when it has one.
    const BAR: f64 = 30.0;
    /// The room around and between pages.
    const GAP: f64 = 12.0;

    /// What a button of its bar does.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Action {
        Save,
        Print,
        ZoomOut,
        ZoomWhole,
        ZoomIn,
        Previous,
        Next,
    }

    /// A report shown in its place.
    pub struct PdfReportView {
        report: String,
        save: bool,
        print: bool,
        zoom_on: bool,
        pages_on: bool,
        /// The data code set, by the names of the report's Data.
        data: Map<String, Value>,
        /// How much larger than fitting the width it shows the pages.
        zoom: f64,
        /// How far down it is scrolled, in pixels.
        scroll: f64,
        laid: Option<clarity_pdf::Laid>,
        items: Vec<Vec<clarity_pdf::Item>>,
        /// In the browser, each page as Typst's SVG (its glyphs as paths), drawn by resvg:
        /// there Skia's font engine jumps (longjmp) on Typst's fonts and stops Clarity.UI.
        #[cfg(target_os = "emscripten")]
        sheets: Vec<crate::picture::SvgPicture>,
        /// Why it shows no pages, when it cannot.
        why: Option<String>,
        /// Where it last was, for its pointer and wheel.
        bounds: Cell<Rect>,
    }

    impl PdfReportView {
        pub fn new(spec: &PartSpec) -> PdfReportView {
            PdfReportView {
                report: spec.text("report").unwrap_or_default().to_string(),
                save: spec.flag("save"),
                print: spec.flag("print"),
                zoom_on: spec.flag("zoom"),
                pages_on: spec.flag("pages"),
                data: Map::new(),
                zoom: 1.0,
                scroll: 0.0,
                laid: None,
                items: Vec::new(),
                #[cfg(target_os = "emscripten")]
                sheets: Vec::new(),
                why: None,
                bounds: Cell::new(Rect::new(0.0, 0.0, 0.0, 0.0)),
            }
        }

        /// Why it shows no pages, when it cannot.
        pub fn why(&self) -> Option<&str> {
            self.why.as_deref()
        }

        /// How many pages it shows.
        pub fn pages(&self) -> usize {
            self.laid.as_ref().map_or(0, clarity_pdf::Laid::pages)
        }

        /// Its bar's buttons, as they show at `bounds`.
        pub fn bar(&self, bounds: Rect) -> Vec<Action> {
            self.buttons(bounds).into_iter().map(|(a, _, _)| a).collect()
        }

        fn has_bar(&self) -> bool {
            self.save || self.print || self.zoom_on || self.pages_on
        }

        /// Lays the report out again with the data it holds.
        fn lay_out(&mut self, cx: &CustomCx) {
            let formats = cx.formats();
            // While code gave it no data, the report's sample data, which only the IDE's preview and designer serve.
            let sample: Option<Map<String, Value>> =
                if self.data.is_empty() { cx.resource(&format!("reports/{}.sample.json", self.report)).and_then(|b| serde_json::from_slice(&b).ok()) } else { None };
            let data = sample.as_ref().unwrap_or(&self.data);
            match laid(&|name| cx.resource(name), &self.report, data, formats.language, formats.currency) {
                Ok(laid) => {
                    self.items = (0..laid.pages()).map(|p| laid.items(p)).collect();
                    #[cfg(target_os = "emscripten")]
                    {
                        self.sheets = (0..laid.pages()).filter_map(|p| crate::picture::SvgPicture::read(laid.svg(p).as_bytes())).collect();
                    }
                    self.laid = Some(laid);
                    self.why = None;
                }
                // Before its gather handler set the data, it shows empty pages, not a problem.
                Err(_) if self.data.is_empty() => {
                    self.laid = None;
                    self.why = None;
                }
                Err(problem) => {
                    self.laid = None;
                    self.why = Some(problem.to_string());
                }
            }
        }

        /// Pixels per point at its width.
        fn scale(&self, area: Rect) -> f64 {
            let Some(laid) = &self.laid else { return 1.0 };
            let (width, _) = laid.page_size(0);
            let points = width * 72.0 / 25.4;
            ((area.width - 2.0 * GAP).max(10.0) / points) * self.zoom
        }

        /// Where its pages lie: below its bar.
        fn pages_area(&self, bounds: Rect) -> Rect {
            let bar = if self.has_bar() { BAR } else { 0.0 };
            Rect::new(bounds.x, bounds.y + bar, bounds.width, (bounds.height - bar).max(0.0))
        }

        /// Each page's top and height in pixels from the top of its pages, unscrolled.
        fn page_spans(&self, area: Rect) -> Vec<(f64, f64)> {
            let Some(laid) = &self.laid else { return Vec::new() };
            let scale = self.scale(area);
            let mut top = GAP;
            (0..laid.pages())
                .map(|p| {
                    let (_, h) = laid.page_size(p);
                    let height = h * 72.0 / 25.4 * scale;
                    let span = (top, height);
                    top += height + GAP;
                    span
                })
                .collect()
        }

        fn longest_scroll(&self, area: Rect) -> f64 {
            let total = self.page_spans(area).last().map_or(0.0, |(t, h)| t + h + GAP);
            (total - area.height).max(0.0)
        }

        /// The page at the top of what shows, counted from 0.
        fn current_page(&self, area: Rect) -> usize {
            self.page_spans(area).iter().rposition(|(top, _)| *top <= self.scroll + GAP + 1.0).unwrap_or(0)
        }

        /// Its bar's buttons and their places.
        fn buttons(&self, bounds: Rect) -> Vec<(Action, Rect, String)> {
            let mut out = Vec::new();
            let mut x = bounds.x + 6.0;
            let y = bounds.y + 4.0;
            let mut add = |action: Action, label: String, width: f64| {
                out.push((action, Rect::new(x, y, width, BAR - 8.0), label));
                x += width + 4.0;
            };
            if self.save {
                add(Action::Save, locale::translate("savePdf", "Save as PDF"), 96.0);
            }
            if self.print {
                add(Action::Print, locale::translate("printPdf", "Print"), 56.0);
            }
            if self.zoom_on {
                add(Action::ZoomOut, "-".into(), 26.0);
                add(Action::ZoomWhole, format!("{}%", (self.zoom * 100.0).round()), 52.0);
                add(Action::ZoomIn, "+".into(), 26.0);
            }
            if self.pages_on {
                add(Action::Previous, "<".into(), 26.0);
                add(Action::Next, ">".into(), 26.0);
            }
            out
        }

        fn act(&mut self, action: Action) {
            let area = self.pages_area(self.bounds.get());
            match action {
                Action::ZoomOut => self.zoom = (self.zoom / 1.25).max(0.25),
                Action::ZoomIn => self.zoom = (self.zoom * 1.25).min(8.0),
                Action::ZoomWhole => self.zoom = 1.0,
                Action::Previous | Action::Next => {
                    let spans = self.page_spans(area);
                    let now = self.current_page(area);
                    let to = if action == Action::Next { (now + 1).min(spans.len().saturating_sub(1)) } else { now.saturating_sub(1) };
                    if let Some((top, _)) = spans.get(to) {
                        self.scroll = top - GAP;
                    }
                }
                Action::Save | Action::Print => {
                    let Some(bytes) = self.laid.as_ref().and_then(|l| l.pdf().ok()) else { return };
                    let name = format!("{}.pdf", self.report);
                    let picker = crate::files::system();
                    if action == Action::Print {
                        // The system's program for PDFs shows it, and prints it.
                        picker.show(&name, &bytes);
                    } else {
                        let title = locale::translate("savePdf", "Save as PDF");
                        let options = crate::files::PickerOptions { suggested_name: Some(name), ..crate::files::PickerOptions::titled(&title) };
                        std::thread::spawn(move || {
                            if let Some(place) = picker.save_file(&options) {
                                let _ = picker.write(&place, &bytes);
                            }
                        });
                    }
                }
            }
            self.scroll = self.scroll.clamp(0.0, self.longest_scroll(area));
        }
    }

    /// A report of the program laid out with its data, as both the part and
    /// code's `pdf of Invoice with order = current` lay it out: the report,
    /// its stylesheet, its pictures and its formats from the program's
    /// resources (`reports/`), its texts in the language shown.
    pub fn laid(resource: &dyn Fn(&str) -> Option<Vec<u8>>, report_name: &str, data: &Map<String, Value>, language: Option<String>, currency: Option<String>) -> Result<clarity_pdf::Laid, String> {
        let text = |name: &str| resource(name).map(|bytes| String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string());
        let report = text(&format!("reports/{report_name}.report")).ok_or_else(|| format!("the program carries no report {report_name}"))?;
        let stylesheet = attribute(&report, "stylesheet").and_then(|css| text(&format!("reports/{css}"))).unwrap_or_default();
        let texts: Map<String, Value> = keys(&report)
            .into_iter()
            .map(|k| {
                let shown = clarity_runtime::i18n::value(&k).unwrap_or_else(|| k.clone());
                (k, Value::from(shown))
            })
            .collect();
        let images: std::collections::BTreeMap<String, Vec<u8>> = sources(&report).into_iter().filter_map(|s| resource(&s).map(|b| (s, b))).collect();
        let types: std::collections::BTreeMap<String, String> = text(&format!("reports/{report_name}.formats.json")).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let extras = clarity_pdf::ReportExtras { images, language, currency, types };
        let data = Value::Object(data.clone()).to_string();
        let texts = Value::Object(texts).to_string();
        // A layout that panics is said in the part's place; the window goes on.
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| clarity_pdf::report_layout_with(&report, &data, &texts, &stylesheet, "", &extras)))
            .map_err(|panic| format!("the report stopped while it was laid out: {}", panic.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| panic.downcast_ref::<String>().cloned()).unwrap_or_default()))?
            .map_err(|p| p.to_string())
    }

    /// A PDF of a report of the running program (`pdf of Invoice with order = current`),
    /// its data as JSON by the report's Data names, in the language shown.
    pub fn pdf_of(report_name: &str, data_json: &str) -> Result<Vec<u8>, String> {
        let files = crate::app_files::running().ok_or_else(|| "no program of Clarity.UI is running to make the report with".to_string())?;
        let data: Map<String, Value> = serde_json::from_str(data_json).map_err(|e| format!("the report's data is no JSON: {e}"))?;
        let currency = serde_json::from_str::<Value>(files.model).ok().and_then(|m| m.pointer("/project/locale/currency").and_then(Value::as_str).map(str::to_string));
        let laid = laid(&|name| files.resource(name).map(<[u8]>::to_vec), report_name, &data, clarity_runtime::i18n::language(), currency)?;
        laid.pdf().map_err(|p| p.to_string())
    }

    /// An attribute of the report's root element: `stylesheet="invoice.css"`.
    fn attribute(report: &str, name: &str) -> Option<String> {
        let root = &report[report.find("<Report")?..];
        let root = &root[..root.find('>')?];
        let start = root.find(&format!("{name}=\""))? + name.len() + 2;
        root[start..].find('"').map(|end| root[start..start + end].to_string())
    }

    /// The pictures its Image parts name: `source="logo.png"`.
    fn sources(report: &str) -> Vec<String> {
        report.match_indices("source=\"").filter_map(|(at, _)| report[at + 8..].find('"').map(|end| report[at + 8..at + 8 + end].to_string())).collect()
    }

    /// The keys its Text parts name.
    fn keys(report: &str) -> Vec<String> {
        report.match_indices("key=\"").filter_map(|(at, _)| report[at + 5..].find('"').map(|end| report[at + 5..at + 5 + end].to_string())).collect()
    }

    impl CustomPart for PdfReportView {
        fn set(&mut self, name: &str, value: &Value, cx: &mut CustomCx) {
            if matches!(name, "enabled" | "value") {
                return;
            }
            self.data.insert(name.to_string(), value.clone());
            self.lay_out(cx);
            cx.invalidate();
        }

        fn hear(&mut self, _hear: &str, _values: &[Value], _cx: &mut CustomCx) {}

        fn start(&mut self, cx: &mut CustomCx) {
            self.bounds.set(cx.bounds());
            self.lay_out(cx);
            // It shows: its handler gathers the report's data.
            cx.fire("gather");
        }

        fn draw(&self, p: &Painter, bounds: Rect) {
            self.bounds.set(bounds);
            p.fill(bounds, p.look.color("background"), 0.0);
            if self.has_bar() {
                let bar = Rect::new(bounds.x, bounds.y, bounds.width, BAR);
                p.fill(bar, p.look.color("panel"), 0.0);
                for (action, at, label) in self.buttons(bounds) {
                    p.fill(at, p.look.color("input"), p.look.radius("input"));
                    p.stroke(at, p.look.color("border"), 1.0, p.look.radius("input"));
                    let _ = action;
                    p.text(&label, at, p.look.color("text"), p.look.size("small"), "text", false, TextAlign::Center);
                }
                if self.pages_on && let Some(laid) = &self.laid {
                    let area = self.pages_area(bounds);
                    let said = locale::fill(&locale::fill(&locale::translate("pageOfPages", "Page {page} of {pages}"), "page", &(self.current_page(area) + 1).to_string()), "pages", &laid.pages().to_string());
                    let right = Rect::new(bounds.x + bounds.width - 140.0, bounds.y + 4.0, 132.0, BAR - 8.0);
                    p.text(&said, right, p.look.color("text"), p.look.size("small"), "text", false, TextAlign::Right);
                }
            }
            let area = self.pages_area(bounds);
            let _clip = p.clip(area);
            if let Some(why) = &self.why {
                p.text(why, area.inset(8.0), p.look.color("error"), p.look.size("small"), "text", false, TextAlign::Left);
                return;
            }
            let Some(laid) = &self.laid else { return };
            let scale = self.scale(area);
            for (page, (top, height)) in self.page_spans(area).into_iter().enumerate() {
                let y = area.y + top - self.scroll;
                if y + height < area.y || y > area.y + area.height {
                    continue;
                }
                let (w, _) = laid.page_size(page);
                let width = w * 72.0 / 25.4 * scale;
                let x = area.x + ((area.width - width) / 2.0).max(GAP);
                let sheet = Rect::new(x, y, width, height);
                p.fill(sheet, skia_safe::Color::WHITE, 0.0);
                p.stroke(sheet, p.look.color("border"), 1.0, 0.0);
                #[cfg(target_os = "emscripten")]
                if let Some(sheet) = self.sheets.get(page) {
                    // At the screen's own pixels, so the page is sharp on a dense screen.
                    let dense = p.canvas.local_to_device_as_3x3().scale_x().abs().max(1.0) as f64;
                    if let Some(image) = sheet.image_at((width * dense).round() as i32, (height * dense).round() as i32) {
                        p.canvas.draw_image_rect(&image, None, skia_safe::Rect::from_xywh(x as f32, y as f32, width as f32, height as f32), &skia_safe::Paint::default());
                    }
                    continue;
                }
                crate::pdf_painter::paint(p.canvas, &self.items[page], scale as f32, (x as f32, y as f32));
            }
        }

        fn click(&mut self, cx: &mut CustomCx) {
            let _ = cx;
        }

        fn pointer_pressed(&mut self, at: PointerInput, cx: &mut CustomCx) {
            let bounds = cx.bounds();
            self.bounds.set(bounds);
            let (x, y) = (bounds.x + at.x, bounds.y + at.y);
            if let Some((action, _, _)) = self.buttons(bounds).into_iter().find(|(_, r, _)| r.contains(x, y)) {
                self.act(action);
                cx.invalidate();
            }
        }

        fn wheel(&mut self, _dx: f64, dy: f64, held: Modifiers, cx: &mut CustomCx) -> bool {
            let area = self.pages_area(cx.bounds());
            if self.zoom_on && (held.has(Modifiers::CONTROL) || held.has(Modifiers::META)) {
                self.act(if dy > 0.0 { Action::ZoomIn } else { Action::ZoomOut });
            } else {
                self.scroll = (self.scroll - dy).clamp(0.0, self.longest_scroll(area));
            }
            cx.invalidate();
            true
        }
    }
}
