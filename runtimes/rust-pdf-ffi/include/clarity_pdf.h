/*
 * clarity_pdf.h: the C interface of Clarity.Pdf (docs/ledger/reports-pdf.md).
 * The library (libclarity_pdf.so, clarity_pdf.dll, libclarity_pdf.dylib)
 * makes the PDFs of code: `pdf of text stylesheet css theme true`, a Markdown
 * text laid out by Typst in a stylesheet's look, alike on every stack; and
 * a report (`.report`, interface 2) laid out with its data and texts.
 *
 * Conventions
 * - A text is UTF-8 as a pointer and a length; an empty stylesheet is
 *   Clarity's own sheet alone, an empty theme is none.
 * - A made PDF goes into `*out` and `*out_len`, allocated by the library; the
 *   program gives it back with `free`.
 * - A failing call gives a CLARITY_PDF_* code; `problem` copies what it was
 *   about. No panic crosses the interface.
 */

#ifndef CLARITY_PDF_H
#define CLARITY_PDF_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_PDF_API __declspec(dllimport)
#else
#define CLARITY_PDF_API
#endif

/* The version of the table; it grows when a field is added at its end. */
#define CLARITY_PDF_INTERFACE 2

#define CLARITY_PDF_OK 0
/* The stylesheet has errors, each with its line. */
#define CLARITY_PDF_STYLESHEET 1
/* The theme is no theme file. */
#define CLARITY_PDF_THEME 2
/* Typst laid out no pages. */
#define CLARITY_PDF_LAYOUT 3
/* The report, its data or its texts are not right (interface 2). */
#define CLARITY_PDF_REPORT 5
/* A text that is no UTF-8, nowhere to give the PDF, or a fault inside Clarity.Pdf. */
#define CLARITY_PDF_FAILED 4

typedef struct ClarityPdfApi {
    uint32_t interface;
    int32_t (*pdf)(const char* text, size_t text_len, const char* stylesheet, size_t stylesheet_len, const char* theme, size_t theme_len, uint8_t** out, size_t* out_len);
    void (*free)(uint8_t* bytes, size_t len);
    /* What the last failing call on this thread was about, as UTF-8 into `into`; its code. */
    int32_t (*problem)(char* into, size_t size);
    /* Interface 2: a report's XML laid out with its data (JSON by its Data's names) and texts (JSON by key), in its stylesheet (empty for Clarity's own), reading the theme (empty for none). */
    int32_t (*report)(const char* report, size_t report_len, const char* data, size_t data_len, const char* texts, size_t texts_len, const char* stylesheet, size_t stylesheet_len, const char* theme, size_t theme_len, uint8_t** out, size_t* out_len);
} ClarityPdfApi;

/* The table, the same for the library's whole life. */
CLARITY_PDF_API const ClarityPdfApi* clarity_pdf_api(void);

#ifdef __cplusplus
}
#endif

#endif
