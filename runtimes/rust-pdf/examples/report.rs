//! Writes a sample invoice report as a PDF: `cargo run -p clarity-pdf --example report -- out.pdf`.

const REPORT: &str = r#"<Report name="Invoice" paper="A4" margin="20mm">
  <Data name="order" type="Order" />
  <ReportHeader height="30mm">
    <Text at="0,0" size="80,10" key="invoiceTitle" class="title" />
    <Field at="120,0" size="50,6" bind="order.number" align="right" />
    <Field at="0,12" size="100,6" bind="order.customer" />
  </ReportHeader>
  <PageHeader height="10mm">
    <Text at="0,0" size="80,6" key="shopName" />
  </PageHeader>
  <Detail list="order.lines" height="7mm">
    <Field at="0,0" size="120,6" bind="line.product" />
    <Field at="140,0" size="30,6" bind="line.price" align="right" />
  </Detail>
  <ReportFooter height="40mm">
    <Text at="100,2" size="30,6" key="totalLabel" />
    <Sum at="140,2" size="30,6" of="line.price" align="right" class="total" />
    <Markdown at="0,12" size="170,25" bind="order.notes" />
  </ReportFooter>
  <PageFooter height="8mm">
    <PageNumber at="80,0" size="10,5" align="center" />
  </PageFooter>
</Report>"#;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "report.pdf".into());
    let lines: Vec<String> = (1..=12).map(|i| format!(r#"{{ "product": "Notebook, ruled, A5 ({i})", "price": "{}.{:02}" }}"#, 3 + i, i * 7 % 100)).collect();
    let data = format!(r#"{{ "order": {{ "number": "Invoice A-17", "customer": "Ada Lovelace, 12 Analytical Lane", "notes": "Thank you for **your** order.\n\nPayment within 14 days; the area of the logo is $\\pi r^2$.", "lines": [{}] }} }}"#, lines.join(", "));
    let texts = r#"{ "invoiceTitle": "Invoice", "shopName": "Paper Shop", "totalLabel": "Total" }"#;
    let file = clarity_pdf::report(REPORT, &data, texts, ".title { font-size: 20pt; font-weight: bold; color: #1f5fbf } .total { font-weight: bold }", "").unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&out, file).unwrap();
}
