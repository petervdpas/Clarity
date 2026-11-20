//! A report's page drawn with Skia from Clarity.Pdf's laid-out page
//! (docs/ledger/reports-pdf.md, step 5): ink where its text and lines are,
//! none elsewhere, and the same at any zoom.

use clarity_ui::pdf_painter;
use skia_safe::{Color, surfaces};

/// The page drawn at `scale` pixels per point on white.
fn drawn(laid: &clarity_pdf::Laid, scale: f32) -> (Vec<u8>, i32) {
    let (w, h) = laid.page_size(0);
    let (pw, ph) = (((w * 72.0 / 25.4) as f32 * scale).ceil() as i32, ((h * 72.0 / 25.4) as f32 * scale).ceil() as i32);
    let mut surface = surfaces::raster_n32_premul((pw, ph)).unwrap();
    surface.canvas().clear(Color::WHITE);
    pdf_painter::paint(surface.canvas(), &laid.items(0), scale, (0.0, 0.0));
    let image = surface.image_snapshot();
    let info = image.image_info();
    let mut bytes = vec![0u8; (pw * ph * 4) as usize];
    assert!(image.read_pixels(&info, &mut bytes, (pw * 4) as usize, (0, 0), skia_safe::image::CachingHint::Allow));
    (bytes, pw)
}

/// How many pixels in the box (in mm) are darker than light grey.
fn ink(bytes: &[u8], width: i32, scale: f32, (x, y, w, h): (f64, f64, f64, f64)) -> usize {
    let px = |mm: f64| (mm * 72.0 / 25.4 * scale as f64) as i32;
    let mut n = 0;
    for row in px(y)..px(y + h) {
        for col in px(x)..px(x + w) {
            let i = ((row * width + col) * 4) as usize;
            if bytes[i] < 160 && bytes[i + 1] < 160 && bytes[i + 2] < 160 {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn a_report_page_has_ink_where_its_text_is_and_none_elsewhere() {
    let report = r#"<Report name="R" paper="A5" margin="10mm"><ReportHeader height="30mm"><Text at="0,0" size="100,10" key="title" class="big" /></ReportHeader></Report>"#;
    let laid = clarity_pdf::report_layout(report, "{}", r#"{ "title": "Invoice" }"#, ".big { font-size: 24pt; color: #000000 }", "").unwrap();
    for scale in [1.0, 3.0] {
        let (bytes, width) = drawn(&laid, scale);
        let title = ink(&bytes, width, scale, (10.0, 10.0, 40.0, 10.0));
        let empty = ink(&bytes, width, scale, (10.0, 60.0, 100.0, 80.0));
        assert!(title > (40.0 * scale * scale) as usize, "the title is drawn at {scale}x: {title}");
        assert_eq!(empty, 0, "nothing drawn below it at {scale}x");
    }
}

#[test]
fn a_rule_and_a_table_line_are_drawn() {
    let laid = clarity_pdf::layout("| A | B |\n|---|---|\n| 1 | 2 |", &clarity_pdf::Stylesheet::read("hr { width: 100% }", None).unwrap()).unwrap();
    let area = laid.areas(0).into_iter().next().expect("a line under the header");
    let (bytes, width) = drawn(&laid, 4.0);
    // Light lines: count anything not white.
    let px = |mm: f64| (mm * 72.0 / 25.4 * 4.0) as i32;
    let row = px(area.y + area.height / 2.0);
    let marked = (px(area.x)..px(area.x + area.width)).filter(|col| bytes[((row * width + col) * 4) as usize] < 250).count();
    assert!(marked as f64 > (px(area.x + area.width) - px(area.x)) as f64 * 0.8, "{marked} of the line's pixels drawn: {area:?}");
}

/// With CLARITY_PAINT_SHOT set to a file, writes the sample invoice as drawn, to look at.
#[test]
fn the_sample_invoice_drawn_for_a_look() {
    let Some(shot) = std::env::var_os("CLARITY_PAINT_SHOT") else { return };
    let report = r#"<Report name="Invoice" paper="A5" margin="12mm">
  <Data name="order" type="Order" />
  <ReportHeader height="22mm"><Text at="0,0" size="80,10" key="title" class="title" /><Field at="70,2" size="54,6" bind="order.number" align="right" /></ReportHeader>
  <Detail list="order.lines" height="6mm"><Field at="0,0" size="90,5" bind="line.product" /><Field at="94,0" size="30,5" bind="line.price" align="right" /></Detail>
  <ReportFooter height="30mm"><Sum at="94,2" size="30,6" of="line.price" align="right" class="total" /><Markdown at="0,10" size="124,18" bind="order.notes" /></ReportFooter>
  <PageFooter height="6mm"><PageNumber at="57,0" size="10,5" align="center" /></PageFooter>
</Report>"#;
    let data = r#"{ "order": { "number": "A-17", "notes": "Thank you for **your** order; the logo's area is $\\pi r^2$.\n\n| Due | Amount |\n|---|--:|\n| 1 Dec | 12.50 |", "lines": [{ "product": "Notebook", "price": "4.25" }, { "product": "Pen", "price": "8.25" }] } }"#;
    let laid = clarity_pdf::report_layout(report, data, r#"{ "title": "Invoice" }"#, ".title { font-size: 20pt; font-weight: bold; color: #1f5fbf } .total { font-weight: bold }", "").unwrap();
    let scale = 2.0;
    let (w, h) = laid.page_size(0);
    let (pw, ph) = (((w * 72.0 / 25.4) as f32 * scale) as i32, ((h * 72.0 / 25.4) as f32 * scale) as i32);
    let mut surface = surfaces::raster_n32_premul((pw, ph)).unwrap();
    surface.canvas().clear(Color::WHITE);
    pdf_painter::paint(surface.canvas(), &laid.items(0), scale, (0.0, 0.0));
    let png = surface.image_snapshot().encode(None, skia_safe::EncodedImageFormat::PNG, None).unwrap();
    std::fs::write(shot, png.as_bytes()).unwrap();
}
