#include "pictures/pictures.h"

#include <string.h>

#include "clarity/core.h"
#include "clarity_image.h"

static const ClarityImageApi* api(void) {
    static const ClarityImageApi* table;
    if (!table) table = clarity_image_api();
    return table;
}

/* The bytes of a picture in hand; none, or one not read from its record, is refused. */
static bool in_hand(const ClarityPicture* p, ClarityBytes* out) {
    if (!p) return clarity_fail(CLARITY_INVALID, "there is no picture");
    if (p->state != CLARITY_PICTURE_NEW) return clarity_fail(CLARITY_INVALID, "the picture is not read from its record");
    *out = p->bytes;
    return true;
}

/* Why Clarity.Image could not make the picture. */
static bool failed(void) {
    char why[512] = {0};
    api()->problem(why, sizeof why);
    return clarity_fail(CLARITY_INVALID, "%s", why[0] ? why : "the picture could not be made");
}

/* The PNG Clarity.Image made, copied into the arena and given back to it. */
static bool made(int32_t code, uint8_t* bytes, size_t len, ClarityPicture** out) {
    if (code != CLARITY_IMAGE_OK) return failed();
    uint8_t* copy = CLARITY_NEW(uint8_t, len ? len : 1);
    memcpy(copy, bytes, len);
    api()->free(bytes, len);
    *out = clarity_picture_new((ClarityBytes){copy, len});
    return true;
}

bool clarity_picture_resize(const ClarityPicture* p, int64_t width, int64_t height, ClarityPicture** out) {
    ClarityBytes b;
    if (!in_hand(p, &b)) return false;
    uint8_t* bytes = NULL;
    size_t len = 0;
    int32_t code = api()->resize(b.data, b.size, width, height, &bytes, &len);
    return made(code, bytes, len, out);
}

bool clarity_picture_crop(const ClarityPicture* p, int64_t x, int64_t y, int64_t width, int64_t height, ClarityPicture** out) {
    ClarityBytes b;
    if (!in_hand(p, &b)) return false;
    uint8_t* bytes = NULL;
    size_t len = 0;
    int32_t code = api()->crop(b.data, b.size, x, y, width, height, &bytes, &len);
    return made(code, bytes, len, out);
}

bool clarity_picture_rotate(const ClarityPicture* p, double degrees, ClarityPicture** out) {
    ClarityBytes b;
    if (!in_hand(p, &b)) return false;
    uint8_t* bytes = NULL;
    size_t len = 0;
    int32_t code = api()->rotate(b.data, b.size, degrees, &bytes, &len);
    return made(code, bytes, len, out);
}

bool clarity_picture_flip(const ClarityPicture* p, bool horizontal, ClarityPicture** out) {
    ClarityBytes b;
    if (!in_hand(p, &b)) return false;
    uint8_t* bytes = NULL;
    size_t len = 0;
    int32_t code = api()->flip(b.data, b.size, horizontal ? 1 : 0, &bytes, &len);
    return made(code, bytes, len, out);
}

bool clarity_picture_blend(const ClarityPicture* base, const ClarityPicture* over, int64_t x, int64_t y, int64_t opacity, ClarityPicture** out) {
    ClarityBytes under, top;
    if (!in_hand(base, &under) || !in_hand(over, &top)) return false;
    uint8_t* bytes = NULL;
    size_t len = 0;
    int32_t code = api()->blend(under.data, under.size, top.data, top.size, x, y, opacity, &bytes, &len);
    return made(code, bytes, len, out);
}

bool clarity_picture_adjust(const ClarityPicture* p, double hue, double saturation, double brightness, double contrast, ClarityPicture** out) {
    ClarityBytes b;
    if (!in_hand(p, &b)) return false;
    uint8_t* bytes = NULL;
    size_t len = 0;
    int32_t code = api()->adjust(b.data, b.size, hue, saturation, brightness, contrast, &bytes, &len);
    return made(code, bytes, len, out);
}

static bool size_of(const ClarityPicture* p, uint32_t* width, uint32_t* height) {
    ClarityBytes b;
    if (!in_hand(p, &b)) return false;
    if (api()->size(b.data, b.size, width, height) != CLARITY_IMAGE_OK) return failed();
    return true;
}

bool clarity_picture_width(const ClarityPicture* p, int64_t* out) {
    uint32_t width = 0, height = 0;
    if (!size_of(p, &width, &height)) return false;
    *out = width;
    return true;
}

bool clarity_picture_height(const ClarityPicture* p, int64_t* out) {
    uint32_t width = 0, height = 0;
    if (!size_of(p, &width, &height)) return false;
    *out = height;
    return true;
}
