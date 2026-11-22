# Invoices

Orders with their lines, each printed as an invoice. The invoice is a report
designed in the Reports tab (`reports/Invoice.report`, with its look in
`reports/invoice.css`); the window shows it as it prints, and code makes the
same invoice as a PDF file.

## What you see and can do

- Orders: a table of orders (number and notes) and, below it, the lines of the order picked (product and price), both edited in place. Add makes a new one to type over; the trash removes one.
- Beside them, the invoice of the order picked as it prints, with Save, Print, zoom and the page count of the pdfReport part.
- **Save PDF** writes the same invoice into `data/exports/invoice-<number>.pdf`, made by code.

## Stack

Go, desktop. An example builds only for the stack its project file names.

## What it shows

- A report of the `reports` folder: a report header with the title and the order's number, a page header, a detail band for each line, a report footer with the sum of the prices and the order's notes as Markdown, and the page number in the page footer.
- The report's texts come from the translations (English and Dutch), its prices show as money in the person's language.
- A pdfReport part (`invoicePdf`) whose `ongather` handler sets its data: `invoicePdf.order = current`.
- A PDF of the report made by code: `pdf of Invoice with order = current`, written to a file store.
