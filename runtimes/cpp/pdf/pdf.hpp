// The PDFs of code (docs/ledger/reports-pdf.md) through Clarity.Pdf's library
// (clarity_pdf.h): `pdf of text stylesheet css theme true`, a Markdown text
// laid out by Typst in a stylesheet's look. A program carries this folder
// only when its code makes a PDF, and links clarity_pdf (clarity_use_pdf in
// clarity.cmake). Throws InvalidError when it cannot: a stylesheet with
// errors, a theme that is no theme file, or pages Typst could not lay out.

#pragma once

#include <string>

#include "clarity/value.hpp"

namespace clarity {

/// A PDF of `text` in `stylesheet`'s look (empty for Clarity's own sheet), reading the theme file `theme` (empty for none).
Bytes pdf_of(const std::string& text, const std::string& stylesheet, const std::string& theme);

}  // namespace clarity
