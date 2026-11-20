# Reports

A report is a page of paper designed beforehand: bands that repeat as their names say, parts placed in them in millimetres, filled with the data code gathers. A pdfReport part shows it on a window ([Media parts](parts.media.md#pdfreport)), and saves or prints it as a PDF.

## The report's file

A report is a file of the project's `reports` folder, `reports/Invoice.report`, named by its file:

```xml
<Report name="Invoice" paper="A4" margin="20mm" stylesheet="invoice.css">
  <Data name="order" type="Order" />
  <ReportHeader height="30mm">
    <Text at="0,0" size="80,10" key="invoiceTitle" class="title" />
    <Field at="120,0" size="60,6" bind="order.number" align="right" />
  </ReportHeader>
  <PageHeader height="10mm">
    <Text at="0,0" size="80,6" key="shopName" />
  </PageHeader>
  <Detail list="order.lines" height="7mm">
    <Field at="0,0" size="100,6" bind="line.product" />
    <Field at="150,0" size="30,6" bind="line.price" align="right" />
  </Detail>
  <ReportFooter height="20mm">
    <Sum at="150,0" size="30,6" of="line.price" align="right" class="total" />
    <Markdown at="0,8" size="170,10" bind="order.notes" />
  </ReportFooter>
  <PageFooter height="8mm">
    <PageNumber at="80,0" size="20,5" align="center" />
  </PageFooter>
</Report>
```

- `Report` names its `paper` (`A3`, `A4`, `A5`, `Letter`, `Legal`, or two lengths, with `landscape`), its `margin` and the `stylesheet` in the reports folder that gives its parts their look. `headerFirst` says which header tops the first page: `true` (as written nothing) prints the report header first and the page header below it, as a title above its column headings, and every later page starts with the page header; `false` puts the page header above the report header on the first page too.
- `Data` is what it takes: a record of an entity, by a name its binds start with. A pdfReport part's handler sets it (`invoicePdf.order = current`).
- The bands, each with its `height`: `ReportHeader` once at the start, `PageHeader` and `PageFooter` on every page, `Detail` once per record of its `list` (running over as many pages as it needs), `ReportFooter` once at the end.
- Groups: `<GroupHeader by="line.category">` prints before the detail's records where that value changes in the list as code gathered it (nothing is sorted: gather them in order, `order by l.category`), and `<GroupFooter by="line.category">` after the group's records; a Sum in a group footer adds that group. Several groups nest, the first written outermost.
- The parts, each `at` its place and `size` in millimetres in its band, `align` left, center or right, and a `class` of the stylesheet:
  - `Text`: a text of the translations by its `key`, in the language shown.
  - `Field`: one value it `bind`s, as in `order.number`; in a detail its first name is the record it repeats for (`line.product`). A number, money or date shows as its field's type says, in the person's language (`€ 1.234,50`, `19-11-2026`); `format` writes it otherwise: `0.00`, `#,##0.00`, `0%`, `percent`, `dd-MM-yyyy`, `d MMMM yyyy`, `HH:mm`.
  - `Sum`: the detail's field it adds, `of="line.price"`, exactly, shown as that field is (or as its `format` says).
  - `Markdown`: a text field as Markdown, with tables and math.
  - `PageNumber`: the page's number.
  - `Line`: a line from its place to the far corner of its size (`size="100,0"` is a level line), in its class's `color`.
  - `Box`: a frame round its place and size, in its class's `color`, filled with its `background-color`.
  - `Image`: a picture of the resources folder by its `source` (`logo.png`, an SVG among them), or a record's image field it `bind`s; fitted to its box. With `barcode` it draws the value it binds as a barcode: `qr`, `code128` or `ean13` (12 or 13 digits).
- A Text's translation may hold named slots, `"Invoice {order.number} for {order.customer.name}"`, filled from the report's data, each value shown as its type says.

## Its look

The stylesheet is the same CSS a PDF of Markdown takes ([Expressions](code.expressions.md#pdfs)), laid over Clarity's own. A class gives a part its look:

```css
.title { font-size: 18pt; font-weight: bold; color: #1f5fbf }
.total { font-weight: bold }
body { font-size: 9pt }
```

## What the checker says

"unknown entity 'Ordr': a report's data is a record of an entity"; "'headerFirst' of Report is true or false, not 'yes'"; "Order has no field 'nummer' ('order.nummer')"; "'invoice.number' names no data of the report; it takes order"; "'order.customer.name' goes through 'customer', a reference to Customer: give the report that record as data of its own"; "Field shows one value; 'order.lines' holds records"; "a Detail repeats for a list of records; 'order.number' is none"; "a Sum adds numbers; 'line.product' is none"; "there is no text 'invoiceTitel' in the translations"; "the report's stylesheet has no class 'heading' (.heading { ... })"; "the stylesheet invoice.css is not in the reports folder"; "a report has no 'Pagefooter': its parts are Data, ReportHeader, PageHeader, Detail, ReportFooter, PageFooter"; "a band has no 'PageNo': its parts are Text, Field, Sum, Markdown, PageNumber"; "'at' of Field is two numbers in mm, as in "10,20", not '120'"; "Image needs 'source', a picture of the resources folder, or 'bind', an image field"; "there is no picture logo.png in the resources folder"; "an Image shows a picture; 'book.title' is no image field"; "a GroupHeader needs 'by', the detail's field it groups by, as in line.category"; "a group goes by one value; 'order.lines' holds records"; "the GroupFooter by line.category has no GroupHeader by line.category".

## Designing a report

The Reports tab designs a report on its paper, as report designers do: a bar above each band names it, the field list beside the palette holds the report's data, and the ruler measures in millimetres.

- Drag a part from the palette onto a band, or a field from the field list (it becomes a Field bound to it); drag a list of the field list onto the paper for a Detail band that repeats for its records.
- Drag a part to move it, its corner to size it, a band's lower edge to change its height; Snap keeps them on a grid. The wheel zooms.
- Shift+click picks several parts, as does a box drawn round them on a band, and Ctrl+A picks all of a band. The parts picked move together and line up with the one picked last: left, centre, right, top, middle or bottom, the same width or height, or spread evenly across or down.
- Ctrl+Z and Ctrl+Y undo and redo, Ctrl+X, Ctrl+C and Ctrl+V cut, copy and paste the picked parts into the picked band, Delete removes them, and the arrows move them by the snap step. Every change is written to the file as it is made.
- A part shows as it prints: a Text in the default language, a field by what it binds, each in its class's look.
- The palette has tabs: Parts, Bands, and the fields of each of the report's data; the properties of what is picked slide out over the paper. XML shows the report's file to read and change; Preview lays it out with sample data.
