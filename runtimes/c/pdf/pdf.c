#include "pdf/pdf.h"

#include <string.h>

#include "clarity_pdf.h"

static const ClarityPdfApi* api(void) {
    static const ClarityPdfApi* table;
    if (!table) table = clarity_pdf_api();
    return table;
}

bool clarity_pdf_of(const char* text, const char* stylesheet, const char* theme, ClarityBytes* out) {
    text = text ? text : "";
    stylesheet = stylesheet ? stylesheet : "";
    theme = theme ? theme : "";
    uint8_t* bytes = NULL;
    size_t len = 0;
    if (api()->pdf(text, strlen(text), stylesheet, strlen(stylesheet), theme, strlen(theme), &bytes, &len) != CLARITY_PDF_OK) {
        char why[2048] = {0};
        api()->problem(why, sizeof why);
        return clarity_fail(CLARITY_INVALID, "%s", why[0] ? why : "the PDF could not be made");
    }
    /* The PDF copied into the arena and given back to Clarity.Pdf. */
    uint8_t* copy = CLARITY_NEW(uint8_t, len ? len : 1);
    memcpy(copy, bytes, len);
    api()->free(bytes, len);
    *out = (ClarityBytes){copy, len};
    return true;
}
