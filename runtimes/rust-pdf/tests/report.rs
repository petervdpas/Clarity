//! A `.report` laid out with its data and texts (docs/ledger/reports-pdf.md, step 6, R1):
//! bands at their heights, parts at their places in mm, the detail once per
//! item, the page header and footer on every page.

use clarity_pdf::{Laid, Run};

const INVOICE: &str = r#"<Report name="Invoice" paper="A4" margin="20mm">
  <Data name="order" type="Order" />
  <ReportHeader height="30mm">
    <Text at="0,0" size="80,10" key="invoiceTitle" />
    <Field at="120,0" size="60,6" bind="order.number" />
  </ReportHeader>
  <PageHeader height="10mm">
    <Text at="0,0" size="80,6" key="shopName" />
  </PageHeader>
  <Detail list="order.lines" height="7mm">
    <Field at="0,0" size="100,6" bind="line.product" />
    <Field at="150,0" size="30,6" bind="line.price" align="right" />
  </Detail>
  <ReportFooter height="20mm">
    <Sum at="150,0" size="30,6" of="line.price" align="right" />
    <Markdown at="0,8" size="170,10" bind="order.notes" />
  </ReportFooter>
  <PageFooter height="8mm">
    <PageNumber at="80,0" size="20,5" />
  </PageFooter>
</Report>"#;

const TEXTS: &str = r#"{ "invoiceTitle": "Invoice", "shopName": "Paper Shop" }"#;

fn data(lines: usize) -> String {
    let lines: Vec<String> = (1..=lines).map(|i| format!(r#"{{ "product": "Item {i}", "price": "{i}.25" }}"#)).collect();
    format!(r#"{{ "order": {{ "number": "A-17", "notes": "Thank you for **your** order.", "lines": [{}] }} }}"#, lines.join(", "))
}

fn laid(lines: usize) -> Laid {
    clarity_pdf::report_layout(INVOICE, &data(lines), TEXTS, "", "").unwrap_or_else(|e| panic!("{e}"))
}

fn run<'a>(runs: &'a [Run], text: &str) -> &'a Run {
    runs.iter().find(|r| r.text.contains(text)).unwrap_or_else(|| panic!("no {text:?} in {runs:#?}"))
}

#[test]
fn parts_lie_at_their_places_in_their_bands() {
    let laid = laid(2);
    let (width, _) = laid.page_size(0);
    assert!((width - 210.0).abs() < 0.1);
    let runs = laid.runs(0);
    let shop = run(&runs, "Paper Shop");
    let title = run(&runs, "Invoice");
    assert!((shop.x - 20.0).abs() < 0.5 && (title.x - 20.0).abs() < 0.5, "at the margin: {shop:?} {title:?}");
    // On page 1 the report header comes first, at the top margin; the page header follows its 30mm.
    assert!(title.y > 19.0 && title.y < 25.0 && shop.y > 49.0 && shop.y < 56.0, "{title:?} {shop:?}");
    let number = run(&runs, "A-17");
    assert!((number.x - 140.0).abs() < 0.5, "120mm in: {number:?}");
}

/// `headerFirst="false"`: the page header at the top of page 1 too, the
/// report header below it; by default (`true`) the report header comes first
/// on page 1 and the page header tops every later page.
#[test]
fn header_first_says_which_header_tops_the_first_page() {
    let after = clarity_pdf::report_layout(&INVOICE.replace(r#"margin="20mm">"#, r#"margin="20mm" headerFirst="false">"#), &data(2), TEXTS, "", "").unwrap();
    let runs = after.runs(0);
    let (shop, title) = (run(&runs, "Paper Shop"), run(&runs, "Invoice"));
    assert!(shop.y < title.y && title.y > 30.0 && title.y < 40.0, "the page header first: {shop:?} {title:?}");
    let first = laid(80);
    assert!(first.pages() >= 2);
    let later = first.runs(1);
    let shop = run(&later, "Paper Shop");
    assert!(shop.y > 19.0 && shop.y < 25.0, "a later page starts with its page header: {shop:?}");
    let rows: Vec<&Run> = later.iter().filter(|r| r.text.starts_with("Item ")).collect();
    assert!(rows.iter().all(|r| r.y > shop.y), "the lines below it");
    let wrong = clarity_pdf::report_layout(&INVOICE.replace(r#"margin="20mm">"#, r#"margin="20mm" headerFirst="maybe">"#), &data(2), TEXTS, "", "").err().map(|p| p.to_string()).unwrap_or_default();
    assert!(wrong.contains("headerFirst"), "{wrong}");
}

#[test]
fn the_detail_is_once_per_item_below_the_report_header() {
    let laid = laid(3);
    let runs = laid.runs(0);
    let rows: Vec<&Run> = (1..=3).map(|i| run(&runs, &format!("Item {i}"))).collect();
    let title = run(&runs, "Invoice");
    assert!(rows[0].y > title.y + 25.0, "after the 30mm report header: {:?}", rows[0]);
    for pair in rows.windows(2) {
        assert!((pair[1].y - pair[0].y - 7.0).abs() < 0.1, "7mm apart: {pair:?}");
    }
    // Right aligned: the price ends at the part's right edge, 20 + 150 + 30.
    let price = run(&runs, "3.25");
    assert!((price.x + price.width - 200.0).abs() < 0.5, "{price:?}");
}

#[test]
fn a_long_list_runs_over_pages_with_the_page_bands_on_each() {
    let laid = laid(80);
    assert!(laid.pages() >= 2, "{} pages", laid.pages());
    for page in 0..laid.pages() {
        let runs = laid.runs(page);
        assert!(runs.iter().any(|r| r.text.contains("Paper Shop")), "page {page} has its header");
        assert!(runs.iter().any(|r| r.text == (page + 1).to_string()), "page {page} has its number: {runs:#?}");
    }
    // The report header once, on the first page.
    assert!(!laid.runs(1).iter().any(|r| r.text.contains("Invoice")));
    assert!(laid.runs(laid.pages() - 1).iter().any(|r| r.text.contains("Item 80")));
}

#[test]
fn a_sum_adds_the_list_exactly_and_markdown_keeps_its_look() {
    let laid = laid(3);
    let last = laid.pages() - 1;
    let runs = laid.runs(last);
    // 1.25 + 2.25 + 3.25, exactly.
    let sum = run(&runs, "6.75");
    assert!((sum.x + sum.width - 200.0).abs() < 0.5, "{sum:?}");
    let bold = run(&runs, "your");
    assert!(bold.bold, "{bold:?}");
}

#[test]
fn the_same_report_and_data_give_the_same_file() {
    let made = || clarity_pdf::report(INVOICE, &data(5), TEXTS, "", "").unwrap();
    assert!(made().starts_with(b"%PDF-"));
    assert_eq!(made(), made());
}

#[test]
fn what_is_wrong_is_said() {
    let wrong = |report: &str, data: &str| clarity_pdf::report(report, data, TEXTS, "", "").unwrap_err().to_string();
    assert!(wrong("<Report", "{}").contains("report"), "not XML");
    let said = wrong(&INVOICE.replace("<PageHeader", "<Pagehead").replace("</PageHeader>", "</Pagehead>"), &data(1));
    assert!(said.contains("Pagehead"), "{said}");
    let said = wrong(INVOICE, "{}");
    assert!(said.contains("order"), "the data the report takes is missing: {said}");
    let said = wrong(&INVOICE.replace(r#"at="120,0""#, r#"at="120""#), &data(1));
    assert!(said.contains("at"), "{said}");
    // Text never runs as Typst.
    assert!(clarity_pdf::report(INVOICE, &data(1).replace("A-17", "#panic(\\\"no\\\")"), TEXTS, "", "").is_ok());
}

#[test]
fn a_part_takes_the_look_of_its_class_in_the_report_s_stylesheet() {
    let report = INVOICE.replace(r#"<Text at="0,0" size="80,10" key="invoiceTitle" />"#, r#"<Text at="0,0" size="80,10" key="invoiceTitle" class="title" />"#);
    let sheet = ".title { font-size: 18pt; font-weight: bold; color: #1f5fbf } body { font-size: 9pt }";
    let laid = clarity_pdf::report_layout(&report, &data(1), TEXTS, sheet, "").unwrap_or_else(|e| panic!("{e}"));
    let runs = laid.runs(0);
    let title = run(&runs, "Invoice");
    assert!(title.bold && (title.size - 18.0).abs() < 0.01 && title.color == "#1f5fbf", "{title:?}");
    let row = run(&runs, "Item 1");
    assert!(!row.bold && (row.size - 9.0).abs() < 0.01, "the body's look: {row:?}");
    // A class the stylesheet does not have is said.
    let said = clarity_pdf::report(&report, &data(1), TEXTS, "", "").unwrap_err().to_string();
    assert!(said.contains("title"), "{said}");
    // A class is a name, and a stylesheet's errors are said with their lines.
    assert!(clarity_pdf::report(&report, &data(1), TEXTS, ".title { colr: red }", "").unwrap_err().to_string().contains("colr"));
}

#[test]
fn a_page_shows_as_svg_for_a_preview() {
    let laid = laid(2);
    let svg = laid.svg(0);
    assert!(svg.starts_with("<svg") && svg.contains("</svg>"), "{}", &svg[..80.min(svg.len())]);
    assert_eq!(svg, laid.svg(0), "the same every time");
}

/// A 4 x 2 PNG, red.
fn png() -> Vec<u8> {
    vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xf0, 0xca, 0xea, 0x34, 0x00, 0x00, 0x00, 0x10, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x47, 0x0c, 0xc8, 0x1c, 0x00, 0x6f, 0xaa, 0x07, 0xf9, 0x80, 0xdc, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82]
}

#[test]
fn an_image_shows_a_picture_of_the_resources_or_a_record_s() {
    let report = r#"<Report name="R" paper="A5"><Data name="book" type="Book" /><ReportHeader height="40mm"><Image at="0,0" size="40,20" source="logo.png" /><Image at="50,0" size="40,20" bind="book.cover" /></ReportHeader></Report>"#;
    let images = std::collections::BTreeMap::from([("logo.png".to_string(), png())]);
    let cover = base64_of(&png());
    let laid = clarity_pdf::report_layout_with(report, &format!(r#"{{ "book": {{ "cover": "{cover}" }} }}"#), "", "", "", &clarity_pdf::ReportExtras { images, ..Default::default() }).unwrap_or_else(|e| panic!("{e}"));
    let shown: Vec<_> = laid.items(0).into_iter().filter_map(|i| match i {
        clarity_pdf::Item::Image { width, height, .. } => Some((width, height)),
        _ => None,
    }).collect();
    assert_eq!(shown.len(), 2, "both pictures");
    // A 2:1 picture fits a 40 x 20 mm box exactly.
    let mm = |pt: f32| pt as f64 * 25.4 / 72.0;
    assert!((mm(shown[0].0) - 40.0).abs() < 0.5 && (mm(shown[0].1) - 20.0).abs() < 0.5, "{shown:?}");
    // A picture the program does not carry is said.
    let said = clarity_pdf::report_layout_with(report, &format!(r#"{{ "book": {{ "cover": "{cover}" }} }}"#), "", "", "", &clarity_pdf::ReportExtras::default()).err().unwrap().to_string();
    assert!(said.contains("logo.png"), "{said}");
}

fn base64_of(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() { out.push(TABLE[(n >> (18 - 6 * i) & 63) as usize] as char) } else { out.push('=') }
        }
    }
    out
}

#[test]
fn groups_print_their_header_where_the_value_changes_and_sum_their_own_items() {
    let report = r#"<Report name="R" paper="A4" margin="20mm">
  <Data name="order" type="Order" />
  <GroupHeader by="line.category" height="8mm"><Field at="0,0" size="80,6" bind="line.category" /></GroupHeader>
  <Detail list="order.lines" height="7mm"><Field at="10,0" size="80,6" bind="line.product" /></Detail>
  <GroupFooter by="line.category" height="8mm"><Sum at="140,0" size="30,6" of="line.price" align="right" /></GroupFooter>
  <ReportFooter height="10mm"><Sum at="140,0" size="30,6" of="line.price" align="right" /></ReportFooter>
</Report>"#;
    let data = r#"{ "order": { "lines": [
        { "category": "Tools", "product": "Hammer", "price": "10.00" },
        { "category": "Tools", "product": "Saw", "price": "5.50" },
        { "category": "Paper", "product": "Notebook", "price": "2.25" } ] } }"#;
    let laid = clarity_pdf::report_layout(report, data, "", "", "").unwrap_or_else(|e| panic!("{e}"));
    let runs = laid.runs(0);
    let y = |text: &str| run(&runs, text).y;
    // Tools, its two lines, its subtotal; then Paper, its line, its subtotal; then the total.
    let order = [y("Tools"), y("Hammer"), y("Saw"), y("15.50"), y("Paper"), y("Notebook"), y("2.25"), y("17.75")];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    assert_eq!(runs.iter().filter(|r| r.text == "Tools").count(), 1, "one header per group, not per line");
}

#[test]
fn a_value_shows_as_its_type_says_in_the_language_or_as_its_format() {
    let report = r#"<Report name="R" paper="A5"><Data name="order" type="Order" />
  <ReportHeader height="30mm">
    <Field at="0,0" size="60,6" bind="order.total" />
    <Field at="0,8" size="60,6" bind="order.date" />
    <Field at="0,16" size="60,6" bind="order.date" format="d MMMM yyyy" />
    <Field at="70,0" size="40,6" bind="order.total" format="0.0" />
  </ReportHeader></Report>"#;
    let extras = clarity_pdf::ReportExtras {
        language: Some("nl-NL".into()),
        currency: Some("EUR".into()),
        types: [("order.total".to_string(), "money".to_string()), ("order.date".to_string(), "date".to_string())].into_iter().collect(),
        ..Default::default()
    };
    let laid = clarity_pdf::report_layout_with(report, r#"{ "order": { "total": "1234.5", "date": "2026-11-19" } }"#, "", "", "", &extras).unwrap_or_else(|e| panic!("{e}"));
    let texts: Vec<String> = laid.runs(0).into_iter().map(|r| r.text).collect();
    let all = texts.join(" | ");
    assert!(all.contains("1.234,50"), "money in Dutch: {all}");
    assert!(all.contains("19-11-2026"), "a Dutch date: {all}");
    assert!(all.contains("19 november 2026"), "{all}");
    assert!(all.contains("1234,5"), "its own format: {all}");
}

#[test]
fn lines_and_boxes_are_drawn_in_their_class_s_colours() {
    let report = r#"<Report name="R" paper="A5" margin="10mm" stylesheet="s.css"><ReportHeader height="30mm">
  <Line at="0,10" size="100,0" class="rule" />
  <Box at="0,15" size="50,10" class="frame" />
</ReportHeader></Report>"#;
    let sheet = ".rule { color: #aa0000 } .frame { color: #0000aa; background-color: #eeeeee }";
    let laid = clarity_pdf::report_layout(report, "{}", "", sheet, "").unwrap_or_else(|e| panic!("{e}"));
    let areas = laid.areas(0);
    let line = areas.iter().find(|a| a.color == "#aa0000").unwrap_or_else(|| panic!("{areas:#?}"));
    assert!((line.width - 100.0).abs() < 1.0 && line.height < 1.0 && (line.x - 10.0).abs() < 0.5, "{line:?}");
    let fill = areas.iter().find(|a| a.color == "#eeeeee").unwrap_or_else(|| panic!("{areas:#?}"));
    assert!((fill.width - 50.0).abs() < 1.0 && (fill.height - 10.0).abs() < 1.0, "{fill:?}");
}

#[test]
fn an_image_with_a_barcode_draws_the_code_of_its_value() {
    let report = r#"<Report name="R" paper="A5"><Data name="order" type="Order" /><ReportHeader height="40mm">
  <Image at="0,0" size="30,30" barcode="qr" bind="order.number" />
  <Image at="40,0" size="60,15" barcode="code128" bind="order.number" />
  <Image at="40,20" size="60,15" barcode="ean13" bind="order.ean" />
</ReportHeader></Report>"#;
    let laid = clarity_pdf::report_layout(report, r#"{ "order": { "number": "A-17", "ean": "590123412345" } }"#, "", "", "").unwrap_or_else(|e| panic!("{e}"));
    let codes = laid.items(0).into_iter().filter(|i| matches!(i, clarity_pdf::Item::Image { svg: true, .. })).count();
    assert_eq!(codes, 3, "each a picture of its code");
    let said = clarity_pdf::report(&report.replace(r#"barcode="qr""#, r#"barcode="aztec""#), r#"{ "order": { "number": "A-17", "ean": "590123412345" } }"#, "", "", "").unwrap_err().to_string();
    assert!(said.contains("aztec") && said.contains("qr"), "{said}");
    let said = clarity_pdf::report(report, r#"{ "order": { "number": "A-17", "ean": "12" } }"#, "", "", "").unwrap_err().to_string();
    assert!(said.contains("ean13"), "{said}");
}

#[test]
fn a_text_fills_its_slots_from_the_report_s_data() {
    let report = r#"<Report name="R" paper="A5"><Data name="order" type="Order" /><ReportHeader height="20mm"><Text at="0,0" size="120,8" key="title" /></ReportHeader></Report>"#;
    let laid = clarity_pdf::report_layout(report, r#"{ "order": { "number": "A-17", "customer": "Ada" } }"#, r#"{ "title": "Invoice {order.number} for {order.customer}" }"#, "", "").unwrap();
    assert!(laid.runs(0).iter().any(|r| r.text.contains("Invoice A-17 for Ada")), "{:?}", laid.runs(0));
}
