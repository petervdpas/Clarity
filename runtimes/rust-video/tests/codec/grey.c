/* A codec for the tests: it takes "avc1" and makes each frame a grey picture
 * whose first byte counts the frames, so a test sees it decoding. */
#include <stdlib.h>
#include <string.h>
#include "../../include/clarity_codec.h"

typedef struct { uint32_t width, height; uint8_t count; } Grey;

static void *decoder_new(const char *format, uint32_t width, uint32_t height, const uint8_t *setup, size_t setup_len) {
    (void)setup; (void)setup_len;
    if (strcmp(format, "avc1") != 0) return NULL;
    Grey *grey = calloc(1, sizeof(Grey));
    grey->width = width;
    grey->height = height;
    return grey;
}

static void decoder_free(void *decoder) { free(decoder); }

static int32_t decode(void *decoder, const uint8_t *frame, size_t frame_len, uint8_t *rgba, size_t size, uint32_t *width, uint32_t *height) {
    (void)frame; (void)frame_len;
    Grey *grey = decoder;
    size_t wanted = (size_t)grey->width * grey->height * 4;
    if (size < wanted) return CLARITY_CODEC_ERROR;
    memset(rgba, 128, wanted);
    rgba[0] = grey->count++;
    *width = grey->width;
    *height = grey->height;
    return CLARITY_CODEC_PICTURE;
}

static const ClarityCodecApi API = { CLARITY_CODEC_INTERFACE, "grey", "avc1", decoder_new, decoder_free, decode };

const ClarityCodecApi *clarity_codec_api(void) { return &API; }
