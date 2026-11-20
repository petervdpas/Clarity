# clarity-pdf

Clarity.Pdf: Markdown laid out into pages and written as a PDF, for the programs Clarity
generates. [Typst](https://typst.app) lays out the pages and writes the PDF; the Markdown is read
the way every Clarity renderer reads it (`clarity_runtime::markdown`), and its text goes to Typst
as text, never as Typst markup. Tables run over pages with their header on each; TeX math (`$...$`,
`$$...$$`) is set through [MiTeX](https://github.com/mitex-rs/mitex), whose Typst definitions are
carried in `mitex/`; a formula Typst cannot set shows its TeX.

A PDF's look comes from a stylesheet: a subset of CSS (`body`, `h1` to `h6`, `p`, `ul`, `ol`,
`li`, `blockquote`, `pre`, `code`, `a`, `hr`, `strong`, `em`, `table`, `th`, `td` and `@page` with its margin boxes)
laid over Clarity's own sheet (`src/clarity.css`). What the stylesheet does not know is an error
with its line. Given the app's theme, the stylesheet reads its roles as custom properties
(`var(--accent)`, `var(--textFont)`), so a PDF looks like its app unless it says otherwise.

The fonts are carried in the crate (Inter in four faces, Libertinus Serif, New Computer Modern,
DejaVu Sans Mono), so making a PDF needs no internet, and the same text gives the same file every
time.

## Use

```rust
use clarity_pdf::{Stylesheet, Theme};

let theme = Theme::read(&std::fs::read_to_string("clarity.light.json")?)?;
let sheet = Stylesheet::read(r#"
    @page { size: A4; margin: 20mm; @bottom-center { content: counter(page) " / " counter(pages) } }
    h1 { color: var(--accent); }
"#, Some(&theme)).map_err(|errors| format!("{errors:?}"))?;
let file = clarity_pdf::pdf("# Invoice\n\nThank you for your order.", &sheet)?;
std::fs::write("invoice.pdf", file)?;
```

`clarity_pdf::layout` gives the laid-out pages: their number, their size, the runs of text on
each (place, size, font, look, color) and the areas drawn (backgrounds, bars, lines).
`cargo run -p clarity-pdf --example sample -- out.pdf` writes a sample.

## License

MIT. Typst and MiTeX are Apache-2.0 (`mitex/LICENSE`). Inter (`fonts/`, Inter 4.0) is under the SIL Open Font License
(`fonts/Inter-LICENSE.txt`); Typst's carried fonts are under the OFL and their own free licences.
