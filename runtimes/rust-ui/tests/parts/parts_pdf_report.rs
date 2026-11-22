//! The pdfReport part (docs/ledger/reports-pdf.md): its report from the
//! program's resources, its gather handler when it shows, the pages laid out
//! with the data code sets and drawn, and its bar's options.

use clarity_ui::parts::custom::CustomPlace;
use clarity_ui::parts::pdf_report::PdfReportView;
use clarity_ui::testing::{Shot, TestHost, part_named, window_of};
use serde_json::json;
use skia_safe::Color;

const REPORT: &str = r#"<Report name="Invoice" paper="A5" margin="10mm" stylesheet="invoice.css">
  <Data name="order" type="Order" />
  <ReportHeader height="20mm"><Text at="0,0" size="100,12" key="invoiceTitle" class="title" /></ReportHeader>
  <Detail list="order.lines" height="7mm"><Field at="0,0" size="80,6" bind="line.product" /></Detail>
</Report>"#;

fn host() -> std::rc::Rc<TestHost> {
    let host = TestHost::new();
    host.resources.borrow_mut().insert("reports/Invoice.report".into(), REPORT.as_bytes().to_vec());
    host.resources.borrow_mut().insert("reports/invoice.css".into(), b".title { font-size: 28pt; color: #000000 }".to_vec());
    host
}

fn lines(n: usize) -> serde_json::Value {
    json!({ "number": "A-1", "lines": (1..=n).map(|i| json!({ "product": format!("Item {i}") })).collect::<Vec<_>>() })
}

#[test]
fn it_gathers_when_it_shows_and_draws_the_pages_of_the_data_code_set() {
    let host = host();
    let mut screen = window_of(r#"<PdfReport name="invoicePdf" at="1,1" span="12" rows="10" report="Invoice" ongather="invoiceGather" />"#, host.clone());
    Shot::of(&mut screen, 600, 700, 1.0);
    assert!(host.logged().iter().any(|l| l == "run invoicePdf gather invoiceGather"), "{:?}", host.logged());
    // The handler sets the report's data, as `invoicePdf.order = ...` does.
    host.set_state("invoicePdf", |s| s.values = Some([("order".to_string(), lines(3))].into_iter().collect()));
    let mut shot = Shot::of(&mut screen, 600, 700, 1.0);
    // The page is white, the title black ink near its top left.
    assert!(shot.any_not(40, 40, 200, 40, Color::WHITE), "the title is drawn");
    let view = view(&screen, "invoicePdf");
    assert_eq!((view.why(), view.pages()), (None, 1));
}

#[test]
fn a_report_the_program_lacks_is_said_in_its_place() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<PdfReport name="invoicePdf" at="1,1" span="12" rows="10" report="Nothing" />"#, host.clone());
    host.set_state("invoicePdf", |s| s.values = Some([("order".to_string(), lines(1))].into_iter().collect()));
    Shot::of(&mut screen, 600, 400, 1.0);
    // It says why in its place, not left blank.
    assert!(view(&screen, "invoicePdf").why().is_some_and(|w| w.contains("Nothing")), "{:?}", view(&screen, "invoicePdf").why());
}

#[test]
fn its_bar_has_only_the_options_its_line_says() {
    let host = host();
    let mut bare = window_of(r#"<PdfReport name="a" at="1,1" span="12" rows="10" report="Invoice" />"#, host.clone());
    let mut full = window_of(r#"<PdfReport name="a" at="1,1" span="12" rows="10" report="Invoice" save="true" print="true" zoom="true" pages="true" />"#, host.clone());
    host.set_state("a", |s| s.values = Some([("order".to_string(), lines(40))].into_iter().collect()));
    Shot::of(&mut bare, 600, 500, 1.0);
    Shot::of(&mut full, 600, 500, 1.0);
    let at = |s: &clarity_ui::Screen| s.element(part_named(s, "a")).unwrap().node().bounds;
    assert!(view(&bare, "a").bar(at(&bare)).is_empty());
    assert_eq!(view(&full, "a").bar(at(&full)).len(), 7, "save, print, three of zoom, two arrows");
    assert!(view(&full, "a").pages() >= 2, "40 lines over pages");
}


fn view<'s>(screen: &'s clarity_ui::Screen, name: &str) -> &'s PdfReportView {
    screen.element(part_named(screen, name)).unwrap().as_any().downcast_ref::<CustomPlace>().unwrap().inner::<PdfReportView>().unwrap()
}

/// In the browser a page is drawn from Typst's SVG through resvg, not with
/// Skia's font engine (which jumps there on Typst's fonts): the same page
/// read and drawn that way shows its text.
#[test]
fn a_page_drawn_from_its_svg_shows_its_text() {
    let data = json!({ "order": lines(3) }).to_string();
    let laid = clarity_pdf::report_layout(REPORT, &data, r#"{"invoiceTitle":"Invoice"}"#, ".title { font-size: 28pt }", "").unwrap();
    let sheet = clarity_ui::picture::SvgPicture::read(laid.svg(0).as_bytes()).expect("Typst's SVG reads");
    let image = sheet.image_at(300, 420).expect("it draws");
    let pixels = image.peek_pixels().expect("a raster image");
    let bytes = pixels.bytes().expect("its bytes");
    let inked = bytes.chunks(4).filter(|p| p[3] > 0 && (p[0] < 128 || p[1] < 128 || p[2] < 128)).count();
    assert!(inked > 200, "the title and the lines show as dark pixels, found {inked}");
}

/// In the IDE's preview, which serves a report's sample data beside it, the
/// part shows the report filled with it while no code gave it data.
#[test]
fn without_data_it_shows_the_sample_data_the_preview_serves() {
    let host = host();
    let sample = json!({ "order": lines(4) }).to_string();
    host.resources.borrow_mut().insert("reports/Invoice.sample.json".into(), sample.into_bytes());
    let mut screen = window_of(r#"<PdfReport name="invoicePdf" at="1,1" span="12" rows="10" report="Invoice" />"#, host.clone());
    let mut shot = Shot::of(&mut screen, 600, 700, 1.0);
    assert!(shot.any_not(40, 40, 200, 40, Color::WHITE), "the title is drawn");
    let view = view(&screen, "invoicePdf");
    assert_eq!((view.why(), view.pages()), (None, 1), "laid out from the sample");
}

#[test]
fn a_report_s_data_is_written_as_clarity_data_writes_every_value() {
    // What each Data is, as the generator writes it beside the report (reports/<Name>.data.json).
    let kinds = r#"{"order":{"kind":"record","entity":"Order","fields":[{"name":"number","kind":"text"},{"name":"kind","kind":"enum","values":["Person","Business"]},{"name":"total","kind":"decimal"},{"name":"placed","kind":"datetime"},{"name":"lines","kind":"records","entity":"Line","fields":[{"name":"product","kind":"text"},{"name":"count","kind":"integer"}],"base":{}}],"base":{}}}"#;
    // As a stack's own JSON code may write it: fields out of order, a decimal as a
    // number with zeros at the end, an enum in another case, a space in a date and time.
    let data = json!({ "order": { "total": 1250.50, "placed": "2026-01-02 09:30:00", "kind": "business", "number": "A-1", "lines": [{ "count": 2.0, "product": "Pen" }] } });
    let written = clarity_ui::parts::pdf_report::normal_data(Some(kinds), data.as_object().unwrap());
    assert_eq!(
        serde_json::Value::Object(written),
        json!({ "order": { "number": "A-1", "kind": "Business", "total": "1250.5", "placed": "2026-01-02T09:30:00", "lines": [{ "product": "Pen", "count": 2 }] } })
    );
    // Without the file (a program generated before it), the data stays as it came.
    assert_eq!(clarity_ui::parts::pdf_report::normal_data(None, data.as_object().unwrap()), *data.as_object().unwrap());
}
