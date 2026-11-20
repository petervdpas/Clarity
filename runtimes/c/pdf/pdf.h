/*
 * The PDFs of code (docs/ledger/reports-pdf.md) through Clarity.Pdf's library
 * (clarity_pdf.h): `pdf of text stylesheet css theme true`, a Markdown text
 * laid out by Typst in a stylesheet's look, as bytes in the scratch arena.
 * A program carries this folder only when its code makes a PDF, and links
 * clarity_pdf (clarity_use_pdf in clarity.cmake).
 *
 * Returns false with clarity_fail when it cannot: a stylesheet with errors,
 * a theme that is no theme file, or pages Typst could not lay out.
 */

#ifndef CLARITY_PDF_OF_H
#define CLARITY_PDF_OF_H

#include <stdbool.h>

#include "clarity/core.h"

/* A PDF of `text` in `stylesheet`'s look (empty for Clarity's own sheet), reading the theme file `theme` (empty for none). */
bool clarity_pdf_of(const char* text, const char* stylesheet, const char* theme, ClarityBytes* out);

#endif
