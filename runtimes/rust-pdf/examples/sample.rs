//! Writes a sample PDF: `cargo run -p clarity-pdf --example sample -- out.pdf`.
fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "sample.pdf".into());
    let text = "# Invoice 2026-114\n\nThank you for your order, **Ada**. The *total* is `EUR 120.00`.\n\n## Lines\n\n- Paper, 2 boxes\n- Ink\n  - black\n\n1. Pay within 30 days\n2. Quote the number\n\n> Questions? Mail us.\n\n---\n\n```\nlet total = 120\n```\n\nSee [our site](https://example.org).\n\n## Totals\n\n| Item | Count | Price |\n|:--|:-:|--:|\n| Paper | 2 | 9.00 |\n| Ink | 1 | 111.00 |\n\nThe area is $\\pi r^2$, and\n\n$$\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}$$\n";
    let sheet = clarity_pdf::Stylesheet::read("@page { @bottom-center { content: counter(page) \" / \" counter(pages) } }", None).unwrap();
    let pdf = clarity_pdf::pdf(text, &sheet).unwrap();
    std::fs::write(&path, pdf).unwrap();
}
