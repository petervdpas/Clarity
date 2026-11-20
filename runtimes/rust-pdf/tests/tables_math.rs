//! Tables over pages and math in Markdown (docs/ledger/reports-pdf.md, step 3).

use clarity_pdf::{Laid, Run, Stylesheet};

fn laid(markdown: &str, sheet: &Stylesheet) -> Laid {
    clarity_pdf::layout(markdown, sheet).unwrap_or_else(|e| panic!("{markdown:?} did not lay out: {e}"))
}

fn sheet(css: &str) -> Stylesheet {
    Stylesheet::read(css, None).unwrap_or_else(|e| panic!("{css:?}: {e:?}"))
}

fn run<'a>(runs: &'a [Run], word: &str) -> &'a Run {
    runs.iter().find(|r| r.text.contains(word)).unwrap_or_else(|| panic!("no {word:?} in {runs:#?}"))
}

fn right(r: &Run) -> f64 {
    r.x + r.width
}

const TABLE: &str = "| Item | Count | Price |\n|:--|:-:|--:|\n| Paper | 2 | 4.50 |\n| Ink | 10 | 12.00 |";

#[test]
fn a_table_lies_in_columns_its_header_bold_and_a_line_under_each_row() {
    let laid = laid(TABLE, &Stylesheet::default());
    let runs = laid.runs(0);
    let (item, count, price, paper, ink) = (run(&runs, "Item"), run(&runs, "Count"), run(&runs, "Price"), run(&runs, "Paper"), run(&runs, "Ink"));
    assert!(item.bold && count.bold && price.bold && !paper.bold, "the header is bold");
    assert!(item.x < count.x && count.x < price.x, "columns side by side");
    assert!((paper.x - item.x).abs() < 0.1 && (ink.x - item.x).abs() < 0.1, "a left column starts its cells alike");
    assert!(item.y < paper.y && paper.y < ink.y, "rows below each other");
    let areas = laid.areas(0);
    let header_lines = areas.iter().filter(|a| a.color == "#b7bec9").count();
    let row_lines = areas.iter().filter(|a| a.color == "#d5d9e0").count();
    assert_eq!((header_lines, row_lines), (1, 2), "{areas:#?}");
    let under_header = areas.iter().find(|a| a.color == "#b7bec9").unwrap();
    assert!(under_header.y > item.y && under_header.y < paper.y);
}

#[test]
fn a_column_aligns_as_its_delimiter_says() {
    let laid = laid(TABLE, &Stylesheet::default());
    let runs = laid.runs(0);
    // Right: the cells end alike, so the shorter one starts further right.
    let (cheap, dear) = (run(&runs, "4.50"), run(&runs, "12.00"));
    assert!(cheap.x > dear.x + 0.5, "{cheap:?} {dear:?}");
    assert!((right(cheap) - right(dear)).abs() < 0.1, "{cheap:?} {dear:?}");
    // Centre: the narrower cell starts further in, by about half the difference.
    let (two, ten) = (run(&runs, "2"), run(&runs, "10"));
    assert!(two.x > ten.x, "{two:?} {ten:?}");
}

#[test]
fn a_long_table_runs_over_pages_with_its_header_on_each() {
    let mut table = String::from("| Line | Amount |\n|---|---:|\n");
    for i in 1..=120 {
        table.push_str(&format!("| Line {i} | {i}.00 |\n"));
    }
    let laid = laid(&table, &Stylesheet::default());
    assert!(laid.pages() >= 3, "{} pages", laid.pages());
    for page in 0..laid.pages() {
        let runs = laid.runs(page);
        let header = runs.iter().find(|r| r.text.contains("Amount")).unwrap_or_else(|| panic!("page {page} has no header"));
        assert!(header.bold);
        let first_row = runs.iter().find(|r| r.text.starts_with("Line ") && !r.bold).unwrap();
        assert!(header.y < first_row.y, "page {page}: the header is on top");
    }
    assert!(laid.runs(laid.pages() - 1).iter().any(|r| r.text.contains("120.00")));
}

#[test]
fn the_stylesheet_styles_header_and_cells() {
    let css = "th { background-color: #eeeeee; color: #aa0000; text-align: center } td { border-bottom: none; color: #0000aa } table { font-size: 9pt; width: 100% }";
    let laid = laid("| Name | Note |\n|---|---|\n| Ada | first |", &sheet(css));
    let runs = laid.runs(0);
    assert_eq!(run(&runs, "Name").color, "#aa0000");
    assert_eq!(run(&runs, "Ada").color, "#0000aa");
    assert!((run(&runs, "Ada").size - 9.0).abs() < 0.01);
    let areas = laid.areas(0);
    let header = areas.iter().find(|a| a.color == "#eeeeee").expect("the header's background");
    assert!((header.width - 80.0).abs() < 1.0, "a 100% wide table's first of two columns: {header:?}");
    assert!(!areas.iter().any(|a| a.color == "#d5d9e0"), "no lines under cells: {areas:#?}");
    // A column with no alignment of its own takes the header's text-align.
    let name = run(&runs, "Name");
    assert!(name.x > 25.0 + 20.0, "centred in its column: {name:?}");
    assert!((run(&runs, "Ada").x - 25.0).abs() < 5.0, "cells keep left");
}

#[test]
fn inline_and_block_math_are_set_as_formulas() {
    let laid = laid("Area $\\pi r^2$ of a circle.\n\n$$\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}$$\n\nAfter.", &Stylesheet::default());
    let runs = laid.runs(0);
    let math: Vec<&Run> = runs.iter().filter(|r| r.font == "New Computer Modern Math").collect();
    assert!(math.iter().any(|r| r.text.contains('π') || r.text.contains('𝜋')), "pi as a math letter: {runs:#?}");
    assert!(math.iter().any(|r| r.text.contains('∑')), "a sum sign: {runs:#?}");
    let area = run(&runs, "Area");
    let pi = math.iter().find(|r| r.text.contains('π') || r.text.contains('𝜋')).unwrap();
    assert!((pi.y - area.y).abs() < 1.0, "inline math sits on the line");
    let sum = math.iter().find(|r| r.text.contains('∑')).unwrap();
    assert!(sum.x > 60.0, "a formula on its own is centred: {sum:?}");
    assert!(!runs.iter().any(|r| r.font == "DejaVu Sans Mono"), "no formula fell back to its TeX");
}

#[test]
fn what_latex_writes_widely_is_set() {
    for tex in ["\\sqrt{x}", "\\mathbf{v}", "\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}", "\\xrightarrow{f}", "\\text{if } x > 0", "\\left( \\frac{a}{b} \\right)", "\\int_0^1 f(x)\\,dx", "\\operatorname{max}(a, b)", "\\alpha \\cdot \\beta"] {
        let laid = laid(&format!("$${tex}$$"), &Stylesheet::default());
        let runs = laid.runs(0);
        assert!(!runs.iter().any(|r| r.font == "DejaVu Sans Mono"), "{tex} fell back to its TeX: {runs:#?}");
        assert!(!runs.is_empty(), "{tex} shows nothing");
    }
}

#[test]
fn a_formula_that_cannot_be_set_shows_its_tex_and_the_rest_still_lays_out() {
    let laid = laid("Before $\\frac{1$ and $x^2$.\n\n$$\\begin{nosuch} y$$\n\nAfter.", &Stylesheet::default());
    let runs = laid.runs(0);
    let broken = run(&runs, "\\frac{1");
    assert_eq!(broken.font, "DejaVu Sans Mono", "{broken:?}");
    assert!(runs.iter().any(|r| r.font == "New Computer Modern Math"), "the good formula is still set");
    assert!(run(&runs, "After").font == "Inter");
}

#[test]
fn typst_code_in_a_formula_does_not_run() {
    let laid = laid("$\\text{#panic(\"no\")}$ and $\\text{\"); panic(\"no\"); (\"}$", &Stylesheet::default());
    assert_eq!(laid.pages(), 1);
}
