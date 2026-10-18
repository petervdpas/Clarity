#include "clarity/picture.h"

#include <stdio.h>
#include <string.h>

ClarityPicture* clarity_picture_stored(void) {
    ClarityPicture* p = CLARITY_NEW(ClarityPicture, 1);
    p->state = CLARITY_PICTURE_STORED;
    return p;
}

ClarityPicture* clarity_picture_new(ClarityBytes bytes) {
    ClarityPicture* p = CLARITY_NEW(ClarityPicture, 1);
    p->state = CLARITY_PICTURE_NEW;
    p->bytes = bytes;
    return p;
}

ClarityPicture* clarity_picture_in_store(const char* path) {
    ClarityPicture* p = CLARITY_NEW(ClarityPicture, 1);
    p->state = CLARITY_PICTURE_IN_STORE;
    p->path = clarity_copy(path);
    return p;
}

static bool same_bytes(ClarityBytes a, ClarityBytes b) { return a.size == b.size && (a.size == 0 || memcmp(a.data, b.data, a.size) == 0); }

bool clarity_picture_same(const ClarityPicture* a, const ClarityPicture* b) {
    if (!a || !b) return a == b;
    if (a->state != b->state) return false;
    if (a->state == CLARITY_PICTURE_NEW) return same_bytes(a->bytes, b->bytes);
    if (a->state == CLARITY_PICTURE_IN_STORE) return strcmp(a->path, b->path) == 0;
    return true;
}

static bool starts(ClarityBytes b, size_t at, const char* prefix, size_t length) {
    return b.size >= at + length && memcmp(b.data + at, prefix, length) == 0;
}

bool clarity_picture_kind(ClarityBytes b, ClarityPictureKind* out) {
    if (starts(b, 0, "\x89PNG", 4)) *out = (ClarityPictureKind){"png", "image/png"};
    else if (starts(b, 0, "\xFF\xD8\xFF", 3)) *out = (ClarityPictureKind){"jpg", "image/jpeg"};
    else if (starts(b, 0, "GIF8", 4)) *out = (ClarityPictureKind){"gif", "image/gif"};
    else if (starts(b, 0, "BM", 2)) *out = (ClarityPictureKind){"bmp", "image/bmp"};
    else if (starts(b, 0, "RIFF", 4) && starts(b, 8, "WEBP", 4)) *out = (ClarityPictureKind){"webp", "image/webp"};
    else return false;
    return true;
}

bool clarity_picture_checked(ClarityBytes bytes, ClarityPictureKind* out) {
    if (clarity_picture_kind(bytes, out)) return true;
    return clarity_fail(CLARITY_INVALID, "this file is not a picture; use a png, jpg, gif, bmp or webp file");
}

bool clarity_picture_file_name(ClarityBytes bytes, const char** out) {
    ClarityPictureKind kind = {0};
    if (!clarity_picture_checked(bytes, &kind)) return false;
    *out = clarity_format("%.32s.%s", clarity_sha256_hex(bytes), kind.extension);
    return true;
}

bool clarity_data_url(ClarityBytes bytes, const char** out) {
    ClarityPictureKind kind = {0};
    if (!clarity_picture_checked(bytes, &kind)) return false;
    *out = clarity_format("data:%s;base64,%s", kind.media_type, clarity_base64(bytes));
    return true;
}

const char* clarity_presence_sql(ClarityDialect d, const char* column) {
    return clarity_format("CASE WHEN %s IS NULL THEN 0 ELSE 1 END", clarity_quote(d, column));
}

bool clarity_in_store(const char* root, const char* path, const char** out) {
    ClarityBuffer file = {0};
    clarity_buffer_add(&file, root);
    const char* start = path ? path : "";
    bool any = false;
    while (true) {
        size_t length = strcspn(start, "/\\");
        const char* part = clarity_copy_n(start, length);
        if (length == 0 || strcmp(part, ".") == 0 || strcmp(part, "..") == 0 || strchr(part, ':'))
            return clarity_fail(CLARITY_INVALID, "'%s' is not a picture of its file store", path ? path : "");
        clarity_buffer_add_char(&file, '/');
        clarity_buffer_add(&file, part);
        any = true;
        if (start[length] == '\0') break;
        start += length + 1;
    }
    if (!any) return clarity_fail(CLARITY_INVALID, "'%s' is not a picture of its file store", path ? path : "");
    *out = clarity_buffer_text(&file);
    return true;
}

// SHA-256 (FIPS 180-4), small enough to carry rather than depend on a library.
const char* clarity_sha256_hex(ClarityBytes bytes) {
    static const uint32_t k[64] = {
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be,
        0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa,
        0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85,
        0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f,
        0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2};
    uint32_t h[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};
    // The message, a 1 bit, zeros and its length in bits, to a multiple of 64 bytes.
    size_t length = (bytes.size + 9 + 63) / 64 * 64;
    uint8_t* m = clarity_alloc(length);
    if (bytes.size) memcpy(m, bytes.data, bytes.size);
    m[bytes.size] = 0x80;
    uint64_t bits = (uint64_t)bytes.size * 8;
    for (int i = 0; i < 8; ++i) m[length - 1 - (size_t)i] = (uint8_t)(bits >> (i * 8));
#define ROT(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
    for (size_t chunk = 0; chunk < length; chunk += 64) {
        uint32_t w[64];
        for (int i = 0; i < 16; ++i) {
            const uint8_t* at = m + chunk + (size_t)i * 4;
            w[i] = (uint32_t)at[0] << 24 | (uint32_t)at[1] << 16 | (uint32_t)at[2] << 8 | at[3];
        }
        for (int i = 16; i < 64; ++i) {
            uint32_t s0 = ROT(w[i - 15], 7) ^ ROT(w[i - 15], 18) ^ (w[i - 15] >> 3);
            uint32_t s1 = ROT(w[i - 2], 17) ^ ROT(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16] + s0 + w[i - 7] + s1;
        }
        uint32_t a = h[0], b = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], hh = h[7];
        for (int i = 0; i < 64; ++i) {
            uint32_t t1 = hh + (ROT(e, 6) ^ ROT(e, 11) ^ ROT(e, 25)) + ((e & f) ^ (~e & g)) + k[i] + w[i];
            uint32_t t2 = (ROT(a, 2) ^ ROT(a, 13) ^ ROT(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));
            hh = g;
            g = f;
            f = e;
            e = d + t1;
            d = c;
            c = b;
            b = a;
            a = t1 + t2;
        }
        h[0] += a;
        h[1] += b;
        h[2] += c;
        h[3] += d;
        h[4] += e;
        h[5] += f;
        h[6] += g;
        h[7] += hh;
    }
#undef ROT
    char out[65];
    for (int i = 0; i < 8; ++i) snprintf(out + i * 8, 9, "%08x", h[i]);
    return clarity_copy(out);
}

static const char letters[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const char* clarity_base64(ClarityBytes bytes) {
    char* out = clarity_alloc((bytes.size + 2) / 3 * 4 + 1);
    size_t at = 0;
    for (size_t i = 0; i < bytes.size; i += 3) {
        uint32_t n = (uint32_t)bytes.data[i] << 16;
        if (i + 1 < bytes.size) n |= (uint32_t)bytes.data[i + 1] << 8;
        if (i + 2 < bytes.size) n |= bytes.data[i + 2];
        out[at++] = letters[(n >> 18) & 63];
        out[at++] = letters[(n >> 12) & 63];
        out[at++] = i + 1 < bytes.size ? letters[(n >> 6) & 63] : '=';
        out[at++] = i + 2 < bytes.size ? letters[n & 63] : '=';
    }
    return out;
}

ClarityBytes clarity_from_base64(const char* text) {
    size_t length = text ? strlen(text) : 0;
    uint8_t* out = clarity_alloc(length / 4 * 3 + 3);
    size_t size = 0;
    uint32_t n = 0;
    int bits = 0;
    for (size_t i = 0; i < length; ++i) {
        const char* at = strchr(letters, text[i]);
        if (text[i] == '=' || !at) continue;
        n = n << 6 | (uint32_t)(at - letters);
        bits += 6;
        if (bits >= 8) {
            bits -= 8;
            out[size++] = (uint8_t)(n >> bits);
        }
    }
    return (ClarityBytes){out, size};
}

ClarityValue clarity_picture_value(const ClarityPicture* picture) {
    if (!picture) return clarity_null();
    switch (picture->state) {
    case CLARITY_PICTURE_STORED: return (ClarityValue){.kind = CLARITY_KEPT};
    case CLARITY_PICTURE_NEW: return clarity_bytes_value(picture->bytes);
    default: return clarity_text_value(picture->path);
    }
}

cJSON* clarity_picture_json(const ClarityPicture* picture) {
    if (!picture) return cJSON_CreateNull();
    cJSON* out = cJSON_CreateObject();
    switch (picture->state) {
    case CLARITY_PICTURE_STORED: cJSON_AddTrueToObject(out, "stored"); break;
    case CLARITY_PICTURE_NEW: cJSON_AddStringToObject(out, "data", clarity_base64(picture->bytes)); break;
    case CLARITY_PICTURE_IN_STORE: cJSON_AddStringToObject(out, "file", picture->path); break;
    }
    return out;
}

ClarityPicture* clarity_picture_from_json(const cJSON* in) {
    if (!cJSON_IsObject(in)) return NULL;
    const cJSON* data = cJSON_GetObjectItemCaseSensitive(in, "data");
    if (cJSON_IsString(data)) return clarity_picture_new(clarity_from_base64(data->valuestring));
    const cJSON* file = cJSON_GetObjectItemCaseSensitive(in, "file");
    if (cJSON_IsString(file)) return clarity_picture_in_store(file->valuestring);
    return clarity_picture_stored();
}
