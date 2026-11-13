# PDF and reports

Status: not built. A `clarity-pdf` crate on Typst (Markdown and Clarity content to PDF, from code, on every stack), then a band report designer with a flow part and a formula part.

Progress: 0%

## Code
- Spec: none (no `<Report>` page kind, no `pdf of`)
- Generators: none
- Runtimes: none (planned `runtimes/rust-pdf` as crate `clarity-pdf`, with `rust-pdf-ffi` for Go, C#, C and C++, as Clarity.Audio and Clarity.Video have)
- Clarity.UI: none (planned: a painter of Typst frames with Skia, after `typst-render`)
- IDE: none (planned: report designer on a page-sized canvas, preview with sample data)
- Docs: none

## Tests
- none

## Open
- Step 1, clarity-pdf: Markdown to PDF with paragraphs, headings, emphasis, page size, margins, page numbers: NOT BUILT.
- Step 2, lists, quotes, code with colors, images, links; the theme as a Typst style; carried fonts (no Typst packages, so builds stay offline): NOT BUILT.
- Step 3, tables over pages; `$...$` and `$$...$$` math in Markdown: NOT BUILT.
- Step 4, the C interface and the other stacks; a call in the language that makes a PDF from a Markdown text or resource: NOT BUILT.
- Step 5, the frame-to-Skia painter; the formula part on windows (`latex` through `mitex`, `mathml` through Clarity's XML reader mapped to Typst math, `typst`): NOT BUILT.
- Step 6, the report page kind (`design/InvoiceReport.cxaml`, `<Report class="shop.InvoiceReport">`), bands (report, page, group header and footer; detail per record), the designer and preview: NOT BUILT.
- Step 7, the flow part, groups, sums and counts, page number and count; the agent and the manual: NOT BUILT.
- In code: `let file = pdf of InvoiceReport(order)`, then save, show or print, on every stack: NOT BUILT.
- To confirm with the user before building (needs decision): the crate's place and name (`runtimes/rust-pdf`, `clarity-pdf`); which mono and math fonts and their licences; how much MathML the first version reads (`mfrac`, `msup`, `msub`, `msqrt`, `mrow`, `mtable`); how a report prints on each system; the Typst version (new dependencies, agreed in principle).
- Order beside the questionnaire ([mcp-tools](mcp-tools.md)): the user's choice.

## Conflicts
- DESIGN.md section 20, open question 5, asks which engine or format the `report` screen pattern targets; the plan of 2026-11-09 chose Typst and PDF.

## Decisions
- 2026-11-09: a pdf crate that works like pandoc. User: "how hard (set aside) would it be to make a clarity-pdf crate? It's should function like pandoc.", "there isnt a rust latex crate or something?"
- 2026-11-09: a PDF design module in the IDE. User: "clarity wouldnt be clarity if it didnt have a pdf design module in the ide, right?"
- 2026-11-09: bands with a flow part; Typst; the crate first. User: "both, bands with a flow part, write the plan after release. 2 yes Typst, 3. think crate first"
- 2026-11-09: formulas in a PDF design (the formula part, still to confirm). User: "still would be cool if a pdf design can hold latex (mathml, etc) and such?"
- 2026-11-09: Typst lays out and writes PDF files (`typst-pdf`); Skia only draws Typst's frames on screen. User: "maybe via skia", then "keep typst as the main engine then"
- 2026-11-09: after the MCP server's first five steps and the `.cxaml` refactor (both built), since a report is a `.cxaml` page. User: "agree"

## Was
- DESIGN section 20, open question 5
- REPORT-PLAN.md (all: decisions, step one, step two, the formula part, order 1-7, to confirm)
- memory: report-plan.md, roadmap-order.md (items 9 and 11)
