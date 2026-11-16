# PDF and reports

Status: not built. A `clarity-pdf` crate on Typst (Markdown and Clarity content to PDF, from code, on every stack), then a band report designer with a flow part and a formula part.

Progress: 0%

## Code
- Spec: none (no `<Report>` root in `.cxaml`, no `pdf of`); `ResourceKind::Pdf` (`crates/clarity-spec/src/resources.rs:99`) is a PDF file among the resources, not PDF making
- Generators: none
- Runtimes: none
- Clarity.UI: none
- IDE: none
- Docs: none

## Tests
- none

## Open
- Step 1, clarity-pdf: Markdown to PDF with paragraphs, headings, emphasis, page size, margins, page numbers: NOT BUILT; see Design.
- Step 2, lists, quotes, code, images, links; the theme's style; carried fonts: NOT BUILT; see Design.
- Step 3, tables over pages; math in Markdown: NOT BUILT; see Design.
- Step 4, the C interface, the other stacks, the call in the language: NOT BUILT; see Design.
- Step 5, the frame-to-Skia painter; the formula part on windows: NOT BUILT; see Design.
- Step 6, reports, bands, the designer and preview: NOT BUILT; see Design.
- Step 7, the flow part, groups, sums, page numbers; the agent and the manual: NOT BUILT; see Design.
- To confirm with the user before building: needs decision; see Design.

## Design
Typst lays out every page and writes the PDF files (crates `typst` and `typst-pdf`, Apache-2.0): page breaking, tables over pages, footnotes, a table of contents, headers and footers, math, links and bookmarks, tagged PDF and PDF/A. Skia only draws on screen: where a formula or a report shows inside Clarity (a window, the IDE's preview), Clarity.UI draws Typst's laid-out frames with Skia, sharp at any zoom and the same on every stack and in the browser. The crate comes first, then the designer.

The clarity-pdf crate:
- `runtimes/rust-pdf` (crate `clarity-pdf`), beside rust-audio and rust-video: Markdown and Clarity content in, a PDF out, from code.
- Markdown to Typst markup: headings, paragraphs, emphasis, lists, quotes, code with colors, tables, images, links, `$...$` and `$$...$$` math.
- The theme to a Typst style: the app's fonts, colors and spacing, so a PDF looks like its app.
- Offline: the fonts are carried in the crate (Inter, a mono font, a math font); no Typst packages, which would come from the internet.
- Every stack: a C interface (`rust-pdf-ffi`), as Clarity.Audio and Clarity.Video have, for Go, C#, C and C++; the browser through the server or WebAssembly.
- In the language: a call that makes a PDF from a Markdown text or resource, to save, show or hand to the person.

The report designer:
- A report is its own kind in `.cxaml`, beside window, dialog and view: `design/InvoiceReport.cxaml` with `<Report class="shop.InvoiceReport">`, designed in the designer on a paper-sized canvas (A4 or Letter, margins, mm or pt).
- Bands: report header and footer, page header and footer, group header and footer per group, and a detail band that repeats for each record of the list it binds.
- The flow part lies in a band and flows its text (Markdown, a bound text) over as many pages as it needs.
- Parts: the window parts that fit paper (text and fields, images, charts, tables, Markdown, lines and boxes) and the report's own (page number, page count, sums and counts over a band, the formula part).
- In code: `let file = pdf of InvoiceReport(order)`, then save, show or print it, on every stack.
- Preview in the IDE with sample data, drawn through Skia from Typst's frames.
- Texts are keys, numbers and dates in the person's format; the look comes from the theme.
- As every part: checks, part options, designer properties, manual pages, and the agent knows how to make a report.

The formula part: a formula in `latex`, `mathml` or `typst`, in the flow part, a band, Markdown, and on windows. LaTeX math through `mitex`; MathML read with Clarity's own XML reader (`runtimes/rust-formats/src/xml.rs`) and mapped onto Typst math; Typst math as it is.

Order: 1 Markdown to PDF with paragraphs, headings, emphasis, page size, margins and page numbers; 2 lists, quotes, code, images, links, the theme's style, carried fonts; 3 tables over pages, math in Markdown; 4 the C interface and the other stacks, the call in the language; 5 the frame-to-Skia painter (after `typst-render`), the formula part on windows; 6 reports, bands and the designer, with the preview; 7 the flow part, groups, sums, page numbers, the agent and the manual.

To confirm with the user before building:
- The crate's place and name: `runtimes/rust-pdf` as `clarity-pdf`.
- The carried fonts (which mono and which math font), and their licences.
- How much MathML the first version reads (`mfrac`, `msup`, `msub`, `msqrt`, `mrow`, `mtable`).
- How a report is printed on each system, beyond saving and showing the PDF.
- The Typst version to start from; the crates are new dependencies (agreed in principle: "yes Typst").
- Where this falls beside the questionnaire ([mcp-tools](mcp-tools.md)): the user's choice.

## Conflicts
- none

## Decisions
- 2026-11-09: a pdf crate that works like pandoc. User: "how hard (set aside) would it be to make a clarity-pdf crate? It's should function like pandoc.", "there isnt a rust latex crate or something?"
- 2026-11-09: a PDF design module in the IDE. User: "clarity wouldnt be clarity if it didnt have a pdf design module in the ide, right?"
- 2026-11-09: bands with a flow part; Typst; the crate first. User: "both, bands with a flow part, write the plan after release. 2 yes Typst, 3. think crate first"
- 2026-11-09: formulas in a PDF design (the formula part, still to confirm). User: "still would be cool if a pdf design can hold latex (mathml, etc) and such?"
- 2026-11-09: Typst lays out and writes PDF files (`typst-pdf`); Skia only draws Typst's frames on screen. User: "maybe via skia", then "keep typst as the main engine then"
- 2026-11-09: after the MCP server's first five steps and the `.cxaml` refactor (both built), since a report is a `.cxaml` design. User: "agree"

## Was
- DESIGN section 20, open question 5
- REPORT-PLAN.md, all of it (Decisions, step one, step two, the formula part, order 1-7, to confirm): now Design; its order paragraph is dropped, as both things it waited for are built
- memory: report-plan.md, roadmap-order.md (items 9 and 11)
