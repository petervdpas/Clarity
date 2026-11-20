//! A laid-out page as a list of what to draw (docs/ledger/reports-pdf.md, step 5),
//! which Clarity.UI's painter draws with Skia: glyph runs by carried font,
//! shapes, and clips, each with the transform it is drawn in.

use clarity_pdf::{Item, PathOp, Stylesheet};

fn place(t: &[f32; 6], x: f32, y: f32) -> (f32, f32) {
    (t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5])
}

#[test]
fn a_page_lists_its_glyphs_where_its_runs_lie() {
    let laid = clarity_pdf::layout("# Invoice\n\nThank you.\n\n---", &Stylesheet::default()).unwrap();
    let items = laid.items(0);
    let runs = laid.runs(0);
    let glyphs: Vec<&Item> = items.iter().filter(|i| matches!(i, Item::Glyphs { .. })).collect();
    assert_eq!(glyphs.len(), runs.len(), "one glyph run per run of text");
    let Item::Glyphs { font, size, color, transform, glyphs: first } = glyphs[0] else { unreachable!() };
    let (data, _) = clarity_pdf::font_data(*font).expect("a carried font");
    assert!(data.len() > 10_000, "the font's file");
    assert!((*size as f64 - runs[0].size).abs() < 0.01);
    assert_eq!(color[3], 255);
    assert_eq!(first.len(), "Invoice".chars().count());
    // The first glyph lies where the run starts, in points (1 mm is 72 / 25.4 pt).
    let (x, y) = place(transform, first[0].1, first[0].2);
    assert!((x as f64 * 25.4 / 72.0 - runs[0].x).abs() < 0.1 && (y as f64 * 25.4 / 72.0 - runs[0].y).abs() < 0.1, "{x},{y} against {:?}", runs[0]);
    // The rule is a shape with a stroke.
    assert!(items.iter().any(|i| matches!(i, Item::Shape { stroke: Some(_), path, .. } if matches!(path.first(), Some(PathOp::Move(..))))), "{items:#?}");
}

#[test]
fn a_report_part_is_clipped_to_its_box() {
    let report = r#"<Report name="R" paper="A5"><ReportHeader height="20mm"><Text at="0,0" size="10,5" key="long" /></ReportHeader></Report>"#;
    let laid = clarity_pdf::report_layout(report, "{}", r#"{ "long": "A text far longer than its box" }"#, "", "").unwrap();
    let items = laid.items(0);
    let opened = items.iter().position(|i| matches!(i, Item::Push { clip: Some(_), .. })).expect("a clip");
    let closed = items.iter().rposition(|i| matches!(i, Item::Pop)).unwrap();
    let text = items.iter().position(|i| matches!(i, Item::Glyphs { .. })).unwrap();
    assert!(opened < text && text < closed, "the text inside its clip");
}
