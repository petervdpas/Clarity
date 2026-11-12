# PDF and reports

A plan, worked out with the user 2026-11-09; nothing is built yet.

## What the user said (2026-11-09, verbatim)

- "how hard (set aside) would it be to make a clarity-pdf crate? It's should function like pandoc."
- "there isnt a rust latex crate or something?"
- "clarity wouldnt be clarity if it didnt have a pdf design module in the ide, right?"
- To the three questions (bands or flow, Typst as the engine, crate or designer first): "both, bands with a flow part, write the plan after release. 2 yes Typst, 3. think crate first"
- "still would be cool if a pdf design can hold latex (mathml, etc) and such?"
- "maybe via skia"
- "keep typst as the main engine then"

## Decisions

- **Typst is the engine.** The `typst` and `typst-pdf` crates (Rust, Apache-2.0) lay out every page and write the PDF files: page breaking, tables over pages, footnotes, a table of contents, headers and footers, math, links and bookmarks, tagged PDF and PDF/A.
- **Skia only draws on screen.** Where a formula or a report shows inside Clarity (a window, the IDE's preview), Clarity.UI draws Typst's laid-out frames with Skia, as it draws everything else, so it is sharp at any zoom and the same on every stack and in the browser.
- **The crate comes first**, then the designer.
- **Reports are bands with a flow part**: the classic bands for invoices and lists, and a part inside a band that flows text over pages for letters and long Markdown.

## Step one: the clarity-pdf crate

- `runtimes/rust-pdf` (crate `clarity-pdf`): Markdown and Clarity content in, a PDF out, from code.
- **Markdown to Typst markup**: headings, paragraphs, emphasis, lists, quotes, code with colors, tables, images, links, `$...$` and `$$...$$` math.
- **The theme to a Typst style**: the app's fonts, colors and spacing, so a PDF looks like its app.
- **Offline**: the fonts are carried in the crate (Inter, a mono font, a math font); no Typst packages, which would come from the internet.
- **Every stack**: a C interface (`rust-pdf-ffi`), as Clarity.Audio and Clarity.Video have, for Go, C#, C and C++; the browser through the server or WebAssembly.
- **In the language**: a call that makes a PDF from a Markdown text or resource, to save, show or hand to the person.

## Step two: the report designer

- **A report is a page kind** in `.cxaml`: `design/InvoiceReport.cxaml` with `<Report class="shop.InvoiceReport">`, designed in the window designer on a page-sized canvas (A4 or Letter, margins, mm or pt).
- **Bands**: report header and footer, page header and footer, group header and footer per group, and a detail band that repeats for each record of the list it binds.
- **The flow part**: lies in a band and flows its text (Markdown, a bound text) over as many pages as it needs.
- **Parts**: the window parts that fit paper (text and fields, images, charts, tables, Markdown, lines and boxes) and the report's own (page number, page count, sums and counts over a band, the formula part).
- **In code**: something like `let file = pdf of InvoiceReport(order)`, then save, show or print it, on every stack.
- **Preview** in the IDE with sample data, drawn through Skia from Typst's frames.
- **Texts are keys**, numbers and dates in the person's format; the look comes from the theme.
- As every part: checks, part options, designer properties, manual pages, and the agent knows how to make a report.

## The formula part

- A formula in `latex`, `mathml` or `typst`, in the flow part, a band, Markdown, and on windows.
- LaTeX math through `mitex`; MathML read with Clarity's own XML reader and mapped onto Typst math; Typst math as it is.

## Order

1. clarity-pdf: Markdown to PDF with paragraphs, headings, emphasis, page size, margins and page numbers.
2. Lists, quotes, code, images, links; the theme's style; carried fonts.
3. Tables over pages; math in Markdown.
4. The C interface and the other stacks; the call in the language.
5. The frame-to-Skia painter (after `typst-render`); the formula part on windows.
6. The report page kind, bands and the designer, with the preview.
7. The flow part, groups, sums, page numbers; the agent and the manual.

In the agreed work order (2026-11-09) this comes after the MCP server's first five steps and the `.cxaml` refactor, since a report is a `.cxaml` page.

## To confirm with the user before building

- The crate's place and name: `runtimes/rust-pdf` as `clarity-pdf`, beside rust-audio and rust-video.
- The carried fonts (which mono and which math font), and their licenses.
- MathML: how much of it the first version reads (the common elements: `mfrac`, `msup`, `msub`, `msqrt`, `mrow`, `mtable`).
- How a report is printed on each system, beyond saving and showing the PDF.
- The Typst version to start from; the crates are new dependencies (agreed in principle: "yes Typst").
