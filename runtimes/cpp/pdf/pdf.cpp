#include "pdf/pdf.hpp"

#include "clarity/errors.hpp"
#include "clarity_pdf.h"

namespace clarity {

Bytes pdf_of(const std::string& text, const std::string& stylesheet, const std::string& theme) {
    static const ClarityPdfApi* api = clarity_pdf_api();
    uint8_t* bytes = nullptr;
    size_t len = 0;
    if (api->pdf(text.data(), text.size(), stylesheet.data(), stylesheet.size(), theme.data(), theme.size(), &bytes, &len) != CLARITY_PDF_OK) {
        char why[2048] = {};
        api->problem(why, sizeof why);
        throw InvalidError(why[0] ? why : "the PDF could not be made");
    }
    Bytes made(bytes, bytes + len);
    api->free(bytes, len);
    return made;
}

}  // namespace clarity
