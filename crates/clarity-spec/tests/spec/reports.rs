//! A project's reports (`reports/*.report`, docs/ledger/reports-pdf.md, step 6, R2)
//! checked against its entities, its texts and the stylesheet each names.

use clarity_spec::reports::{self, ReportFile};
use clarity_spec::load_converted;

const SHOP: &str = "module shop

entity Order
    number: text, label @orderNumber
    notes: text, label @orderNotes
    customer: ref Customer, label @orderCustomer
    lines: list<Line>, label @orderLines

entity Line
    product: text, label @lineProduct
    price: money, label @linePrice
    order: ref Order, label @lineOrder

entity Customer
    name: text, label @customerName
";

const INVOICE: &str = r#"<Report name="Invoice" paper="A4" margin="20mm" stylesheet="invoice.css">
  <Data name="order" type="Order" />
  <ReportHeader height="30mm">
    <Text at="0,0" size="80,10" key="invoiceTitle" class="title" />
    <Field at="120,0" size="60,6" bind="order.number" />
  </ReportHeader>
  <Detail list="order.lines" height="7mm">
    <Field at="0,0" size="100,6" bind="line.product" />
    <Field at="150,0" size="30,6" bind="line.price" align="right" />
  </Detail>
  <ReportFooter height="20mm">
    <Sum at="150,0" size="30,6" of="line.price" />
    <Markdown at="0,8" size="180,10" bind="order.notes" />
  </ReportFooter>
  <PageFooter height="8mm">
    <PageNumber at="80,0" size="20,5" />
  </PageFooter>
</Report>"#;

fn said(report: &str, stylesheet: Option<&str>) -> Vec<String> {
    let mut project = load_converted([("shop/shop.clarity", SHOP)]);
    project.translations.entry("en".into()).or_default().insert("invoiceTitle".into(), "Invoice".into());
    project.locale.language = Some("en".into());
    project.reports = vec![ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: report.into(), stylesheet: stylesheet.map(str::to_string) }];
    reports::check(&project).into_iter().map(|d| d.message).collect()
}

const SHEET: Option<&str> = Some(".title { font-size: 18pt; font-weight: bold }");

#[test]
fn a_report_that_fits_its_data_checks_clean() {
    assert!(said(INVOICE, SHEET).is_empty(), "{:#?}", said(INVOICE, SHEET));
}

#[test]
fn what_does_not_fit_is_said() {
    let says = |from: &str, to: &str, wanted: &str| {
        let e = said(&INVOICE.replace(from, to), SHEET);
        assert!(e.iter().any(|m| m.contains(wanted)), "{from} -> {to}: wanted {wanted:?} in {e:#?}");
    };
    says(r#"type="Order""#, r#"type="Ordr""#, "unknown entity 'Ordr'");
    says("order.number", "order.nummer", "Order has no field 'nummer'");
    says("line.product", "line.produce", "Line has no field 'produce'");
    says("order.number", "invoice.number", "'invoice.number' names no data of the report");
    says("order.number", "order.customer.name", "a reference to Customer");
    says("order.number", "order.lines", "holds records");
    says(r#"list="order.lines""#, r#"list="order.number""#, "a Detail repeats for a list of records");
    says(r#"of="line.price""#, r#"of="line.product""#, "a Sum adds numbers");
    says(r#"key="invoiceTitle""#, r#"key="invoiceTitel""#, "there is no text 'invoiceTitel'");
    says(r#"class="title""#, r#"class="heading""#, "no class 'heading'");
    says("PageFooter", "Pagefooter", "a report has no 'Pagefooter'");
    says("<PageNumber", "<PageNo", "a band has no 'PageNo'");
    says(r#"at="120,0""#, r#"at="120""#, "'at' of Field is two numbers");
    says(r#"align="right""#, r#"align="end""#, "'align' of Field is left, center or right");
    says(r#"name="Invoice""#, r#"name="Bill""#, "named Invoice");
    says(r#"margin="20mm""#, r#"margin="20mm" headerFirst="yes""#, "'headerFirst' of Report is true or false, not 'yes'");
    // The papers the Reports tab offers, written as it writes them, turned or not.
    says(r#"paper="A4""#, r#"paper="a4""#, "'paper' of Report is A3, A4, A5, Letter or Legal, with landscape or portrait after it, not 'a4'");
    says(r#"paper="A4""#, r#"paper="A6""#, "not 'A6'");
    says(r#"paper="A4""#, r#"paper="A4 sideways""#, "not 'A4 sideways'");
    for paper in ["A3", "A5 landscape", "Letter portrait", "Legal"] {
        assert!(said(&INVOICE.replace(r#"paper="A4""#, &format!(r#"paper="{paper}""#)), SHEET).is_empty(), "{paper} checks clean");
    }
}

#[test]
fn a_stylesheet_named_but_missing_is_said() {
    let e = said(INVOICE, None);
    assert!(e.iter().any(|m| m.contains("invoice.css is not in the reports folder")), "{e:#?}");
}

#[test]
fn a_report_that_is_no_xml_is_said_in_its_file() {
    let mut project = load_converted([("shop/shop.clarity", SHOP)]);
    project.reports = vec![ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: "<Report".into(), stylesheet: None }];
    let found = reports::check(&project);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].path, "reports/Invoice.report");
}

#[test]
fn a_new_report_checks_clean_and_its_sample_data_fills_every_field() {
    let mut project = load_converted([("shop/shop.clarity", SHOP)]);
    project.reports = vec![ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: reports::starter("Invoice"), stylesheet: None }];
    assert!(reports::check(&project).is_empty(), "{:#?}", reports::check(&project));
    project.reports[0].text = INVOICE.into();
    let sample = reports::sample_data(&project, "Invoice");
    assert_eq!(sample["order"]["number"], "number 1");
    assert_eq!(sample["order"]["lines"].as_array().map(Vec::len), Some(3));
    assert_eq!(sample["order"]["lines"][2]["price"], "3.50");
    assert!(sample["order"]["customer"].is_null(), "a reference is no record of its own");
}

#[test]
fn groups_check_their_field_and_their_header() {
    let grouped = INVOICE.replace("<ReportFooter", r#"<GroupHeader by="line.product" height="8mm"><Field at="0,0" size="80,6" bind="line.product" /></GroupHeader><GroupFooter by="line.product" height="8mm"><Sum at="0,0" size="30,6" of="line.price" /></GroupFooter><ReportFooter"#);
    assert!(said(&grouped, SHEET).is_empty(), "{:#?}", said(&grouped, SHEET));
    let e = said(&grouped.replace(r#"<GroupHeader by="line.product""#, r#"<GroupHeader by="line.produce""#), SHEET);
    assert!(e.iter().any(|m| m.contains("Line has no field 'produce'")), "{e:#?}");
    assert!(e.iter().any(|m| m.contains("has no GroupHeader by line.product")), "{e:#?}");
}

#[test]
fn each_value_a_report_shows_has_its_field_s_format() {
    let mut project = load_converted([("shop/shop.clarity", SHOP)]);
    project.reports = vec![ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: INVOICE.into(), stylesheet: None }];
    let formats = reports::formats_of(&project, "Invoice");
    assert_eq!(formats.get("line.price").map(String::as_str), Some("money"), "{formats:?}");
    assert!(!formats.contains_key("order.number"), "a text has no format: {formats:?}");
}

#[test]
fn a_report_s_text_keys_count_as_used_in_the_report() {
    let mut project = load_converted([("shop/shop.clarity", SHOP)]);
    project.reports = vec![ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: INVOICE.into(), stylesheet: None }];
    let keys = clarity_spec::translations::keys(&project);
    let title = keys.iter().find(|k| k.key == "invoiceTitle").expect("the report's key is used");
    assert!(title.places.iter().any(|p| p == "report Invoice"), "{:?}", title.places);
}

#[test]
fn code_s_pdf_of_a_report_gives_the_data_it_takes() {
    let code = |line: &str| format!("{SHOP}\naction printIt(o: Order) -> bytes\n    return {line}\n");
    let said = |line: &str| {
        let text = code(line);
        let mut project = load_converted([("shop/shop.clarity", text.as_str())]);
        project.reports = vec![ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: INVOICE.into(), stylesheet: None }];
        let mut all: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
        all.extend(reports::check(&project).into_iter().map(|d| d.message));
        all.retain(|m| !m.contains("invoiceTitle") && !m.contains("class") && !m.contains("stylesheet"));
        all
    };
    assert!(said("pdf of Invoice with order = o").is_empty(), "{:#?}", said("pdf of Invoice with order = o"));
    assert!(said("pdf of Invoce with order = o").iter().any(|m| m.contains("there is no report 'Invoce'")));
    assert!(said("pdf of Invoice with ordr = o").iter().any(|m| m.contains("takes no data 'ordr'")));
    assert!(said("pdf of Invoice with ordr = o").iter().any(|m| m.contains("takes order, a Order")));
    assert!(said("pdf of Invoice with order = 12").iter().any(|m| m.contains("a report's data is a record")));
}
