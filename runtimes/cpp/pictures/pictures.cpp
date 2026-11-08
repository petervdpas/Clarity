#include "pictures/pictures.hpp"

#include <string>

#include "clarity/errors.hpp"
#include "clarity_image.h"

namespace clarity {

namespace {

const ClarityImageApi* api() {
    static const ClarityImageApi* table = clarity_image_api();
    return table;
}

/// The bytes of a picture in hand; none, or one not read from its record, is refused.
const Bytes& in_hand(const std::optional<Picture>& p) {
    if (!p) throw InvalidError("there is no picture");
    if (p->state != Picture::State::New) throw InvalidError("the picture is not read from its record");
    return p->bytes;
}

/// Why Clarity.Image could not make the picture.
[[noreturn]] void failed() {
    char why[512] = {};
    api()->problem(why, sizeof why);
    throw InvalidError(why[0] ? why : "the picture could not be made");
}

/// The PNG Clarity.Image made, copied and given back to it.
std::optional<Picture> made(int32_t code, uint8_t* bytes, size_t len) {
    if (code != CLARITY_IMAGE_OK) failed();
    Bytes copy(bytes, bytes + len);
    api()->free(bytes, len);
    return Picture::of(std::move(copy));
}

}  // namespace

std::optional<Picture> picture_resize(const Bytes& p, int64_t width, int64_t height) {
    uint8_t* bytes = nullptr;
    size_t len = 0;
    int32_t code = api()->resize(p.data(), p.size(), width, height, &bytes, &len);
    return made(code, bytes, len);
}

std::optional<Picture> picture_resize(const std::optional<Picture>& p, int64_t width, int64_t height) {
    return picture_resize(in_hand(p), width, height);
}

std::optional<Picture> picture_crop(const Bytes& p, int64_t x, int64_t y, int64_t width, int64_t height) {
    uint8_t* bytes = nullptr;
    size_t len = 0;
    int32_t code = api()->crop(p.data(), p.size(), x, y, width, height, &bytes, &len);
    return made(code, bytes, len);
}

std::optional<Picture> picture_crop(const std::optional<Picture>& p, int64_t x, int64_t y, int64_t width, int64_t height) {
    return picture_crop(in_hand(p), x, y, width, height);
}

std::optional<Picture> picture_rotate(const Bytes& p, const Decimal& degrees) {
    uint8_t* bytes = nullptr;
    size_t len = 0;
    int32_t code = api()->rotate(p.data(), p.size(), units(degrees), &bytes, &len);
    return made(code, bytes, len);
}

std::optional<Picture> picture_rotate(const std::optional<Picture>& p, const Decimal& degrees) {
    return picture_rotate(in_hand(p), degrees);
}

std::optional<Picture> picture_flip(const Bytes& p, bool horizontal) {
    uint8_t* bytes = nullptr;
    size_t len = 0;
    int32_t code = api()->flip(p.data(), p.size(), horizontal ? 1 : 0, &bytes, &len);
    return made(code, bytes, len);
}

std::optional<Picture> picture_flip(const std::optional<Picture>& p, bool horizontal) {
    return picture_flip(in_hand(p), horizontal);
}

std::optional<Picture> picture_blend(const Bytes& base, const Bytes& over, int64_t x, int64_t y, int64_t opacity) {
    uint8_t* bytes = nullptr;
    size_t len = 0;
    int32_t code = api()->blend(base.data(), base.size(), over.data(), over.size(), x, y, opacity, &bytes, &len);
    return made(code, bytes, len);
}

std::optional<Picture> picture_blend(const std::optional<Picture>& base, const std::optional<Picture>& over, int64_t x, int64_t y, int64_t opacity) {
    return picture_blend(in_hand(base), in_hand(over), x, y, opacity);
}

std::optional<Picture> picture_adjust(const Bytes& p, const Decimal& hue, const Decimal& saturation, const Decimal& brightness, const Decimal& contrast) {
    uint8_t* bytes = nullptr;
    size_t len = 0;
    int32_t code = api()->adjust(p.data(), p.size(), units(hue), units(saturation), units(brightness), units(contrast), &bytes, &len);
    return made(code, bytes, len);
}

std::optional<Picture> picture_adjust(const std::optional<Picture>& p, const Decimal& hue, const Decimal& saturation, const Decimal& brightness, const Decimal& contrast) {
    return picture_adjust(in_hand(p), hue, saturation, brightness, contrast);
}

namespace {

std::pair<int64_t, int64_t> size_of(const Bytes& p) {
    uint32_t width = 0, height = 0;
    if (api()->size(p.data(), p.size(), &width, &height) != CLARITY_IMAGE_OK) failed();
    return {width, height};
}

}  // namespace

int64_t picture_width(const Bytes& p) { return size_of(p).first; }
int64_t picture_width(const std::optional<Picture>& p) { return size_of(in_hand(p)).first; }
int64_t picture_height(const Bytes& p) { return size_of(p).second; }
int64_t picture_height(const std::optional<Picture>& p) { return size_of(in_hand(p)).second; }

}  // namespace clarity
