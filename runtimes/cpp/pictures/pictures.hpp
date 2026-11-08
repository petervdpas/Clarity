// The picture functions of code (docs/IMAGE-PLAN.md) through Clarity.Image's
// library (clarity_image.h): each takes a picture in hand, or bytes read from
// a file, and gives a new one as PNG, leaving its arguments as they were.
// Every stack gives the same pixels (conformance/picture-functions.json). A
// program carries this folder only when its code calls one, and links
// clarity_image (clarity_use_image in clarity.cmake). Each throws
// InvalidError when it cannot: no picture, one not read from its record,
// bytes that are no picture, or a size it cannot make.

#pragma once

#include <cstdint>
#include <optional>

#include "clarity/decimal.hpp"
#include "clarity/picture.hpp"
#include "clarity/value.hpp"

namespace clarity {

std::optional<Picture> picture_resize(const std::optional<Picture>& p, int64_t width, int64_t height);
std::optional<Picture> picture_resize(const Bytes& p, int64_t width, int64_t height);
std::optional<Picture> picture_crop(const std::optional<Picture>& p, int64_t x, int64_t y, int64_t width, int64_t height);
std::optional<Picture> picture_crop(const Bytes& p, int64_t x, int64_t y, int64_t width, int64_t height);
std::optional<Picture> picture_rotate(const std::optional<Picture>& p, const Decimal& degrees);
std::optional<Picture> picture_rotate(const Bytes& p, const Decimal& degrees);
std::optional<Picture> picture_flip(const std::optional<Picture>& p, bool horizontal);
std::optional<Picture> picture_flip(const Bytes& p, bool horizontal);
std::optional<Picture> picture_blend(const std::optional<Picture>& base, const std::optional<Picture>& over, int64_t x, int64_t y, int64_t opacity);
std::optional<Picture> picture_blend(const Bytes& base, const Bytes& over, int64_t x, int64_t y, int64_t opacity);
std::optional<Picture> picture_adjust(const std::optional<Picture>& p, const Decimal& hue, const Decimal& saturation, const Decimal& brightness, const Decimal& contrast);
std::optional<Picture> picture_adjust(const Bytes& p, const Decimal& hue, const Decimal& saturation, const Decimal& brightness, const Decimal& contrast);
int64_t picture_width(const std::optional<Picture>& p);
int64_t picture_width(const Bytes& p);
int64_t picture_height(const std::optional<Picture>& p);
int64_t picture_height(const Bytes& p);

}  // namespace clarity
