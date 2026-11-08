/*
 * The picture functions of code (docs/IMAGE-PLAN.md) through Clarity.Image's
 * library (clarity_image.h): each takes a picture in hand and gives a new one
 * as PNG in the scratch arena, leaving its arguments as they were. Every
 * stack gives the same pixels (conformance/picture-functions.json). A program
 * carries this folder only when its code calls one, and links clarity_image
 * (clarity_use_image in clarity.cmake).
 *
 * Each returns false with clarity_fail when it cannot: no picture, one not
 * read from its record, bytes that are no picture, or a size it cannot make.
 */

#ifndef CLARITY_PICTURES_H
#define CLARITY_PICTURES_H

#include <stdbool.h>
#include <stdint.h>

#include "clarity/picture.h"

bool clarity_picture_resize(const ClarityPicture* p, int64_t width, int64_t height, ClarityPicture** out);
bool clarity_picture_crop(const ClarityPicture* p, int64_t x, int64_t y, int64_t width, int64_t height, ClarityPicture** out);
bool clarity_picture_rotate(const ClarityPicture* p, double degrees, ClarityPicture** out);
bool clarity_picture_flip(const ClarityPicture* p, bool horizontal, ClarityPicture** out);
bool clarity_picture_blend(const ClarityPicture* base, const ClarityPicture* over, int64_t x, int64_t y, int64_t opacity, ClarityPicture** out);
bool clarity_picture_adjust(const ClarityPicture* p, double hue, double saturation, double brightness, double contrast, ClarityPicture** out);
bool clarity_picture_width(const ClarityPicture* p, int64_t* out);
bool clarity_picture_height(const ClarityPicture* p, int64_t* out);

#endif
