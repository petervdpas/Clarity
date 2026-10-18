#include "clarity/picture.hpp"

#include <array>
#include <cstdio>
#include <cstring>

#include "clarity/errors.hpp"

namespace clarity {

std::optional<PictureKind> kind_of(const Bytes& b) {
    auto starts = [&](std::initializer_list<int> prefix, size_t at = 0) {
        if (b.size() < at + prefix.size()) return false;
        size_t i = at;
        for (int c : prefix)
            if (b[i++] != c) return false;
        return true;
    };
    if (starts({0x89, 'P', 'N', 'G'})) return PictureKind{"png", "image/png"};
    if (starts({0xFF, 0xD8, 0xFF})) return PictureKind{"jpg", "image/jpeg"};
    if (starts({'G', 'I', 'F', '8'})) return PictureKind{"gif", "image/gif"};
    if (starts({'B', 'M'})) return PictureKind{"bmp", "image/bmp"};
    if (starts({'R', 'I', 'F', 'F'}) && starts({'W', 'E', 'B', 'P'}, 8)) return PictureKind{"webp", "image/webp"};
    return std::nullopt;
}

PictureKind checked(const Bytes& bytes) {
    auto kind = kind_of(bytes);
    if (!kind) throw InvalidError("this file is not a picture; use a png, jpg, gif, bmp or webp file");
    return *kind;
}

std::string file_name(const Bytes& bytes) {
    PictureKind kind = checked(bytes);
    return sha256_hex(bytes).substr(0, 32) + "." + kind.extension;
}

std::string data_url(const Bytes& bytes) { return "data:" + checked(bytes).media_type + ";base64," + base64(bytes); }

std::string presence(Dialect d, const std::string& column) { return "CASE WHEN " + quote(d, column) + " IS NULL THEN 0 ELSE 1 END"; }

std::filesystem::path in_store(const std::filesystem::path& root, const std::string& path) {
    std::filesystem::path out = root;
    size_t start = 0;
    bool any = false;
    while (true) {
        size_t end = path.find_first_of("/\\", start);
        std::string part = path.substr(start, end == std::string::npos ? std::string::npos : end - start);
        if (part.empty() || part == "." || part == ".." || part.find(':') != std::string::npos)
            throw InvalidError("'" + path + "' is not a picture of its file store");
        out /= part;
        any = true;
        if (end == std::string::npos) break;
        start = end + 1;
    }
    if (!any) throw InvalidError("'" + path + "' is not a picture of its file store");
    return out;
}

// SHA-256 (FIPS 180-4), small enough to carry rather than depend on a library.
std::string sha256_hex(const Bytes& bytes) {
    static const uint32_t k[64] = {
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be,
        0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa,
        0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85,
        0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f,
        0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2};
    uint32_t h[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};
    Bytes m = bytes;
    uint64_t bits = uint64_t(bytes.size()) * 8;
    m.push_back(0x80);
    while (m.size() % 64 != 56) m.push_back(0);
    for (int i = 7; i >= 0; --i) m.push_back(uint8_t(bits >> (i * 8)));
    auto rot = [](uint32_t x, int n) { return (x >> n) | (x << (32 - n)); };
    for (size_t chunk = 0; chunk < m.size(); chunk += 64) {
        uint32_t w[64];
        for (int i = 0; i < 16; ++i)
            w[i] = uint32_t(m[chunk + i * 4]) << 24 | uint32_t(m[chunk + i * 4 + 1]) << 16 | uint32_t(m[chunk + i * 4 + 2]) << 8 | m[chunk + i * 4 + 3];
        for (int i = 16; i < 64; ++i) {
            uint32_t s0 = rot(w[i - 15], 7) ^ rot(w[i - 15], 18) ^ (w[i - 15] >> 3);
            uint32_t s1 = rot(w[i - 2], 17) ^ rot(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16] + s0 + w[i - 7] + s1;
        }
        uint32_t a = h[0], b = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], hh = h[7];
        for (int i = 0; i < 64; ++i) {
            uint32_t t1 = hh + (rot(e, 6) ^ rot(e, 11) ^ rot(e, 25)) + ((e & f) ^ (~e & g)) + k[i] + w[i];
            uint32_t t2 = (rot(a, 2) ^ rot(a, 13) ^ rot(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));
            hh = g;
            g = f;
            f = e;
            e = d + t1;
            d = c;
            c = b;
            b = a;
            a = t1 + t2;
        }
        h[0] += a; h[1] += b; h[2] += c; h[3] += d; h[4] += e; h[5] += f; h[6] += g; h[7] += hh;
    }
    std::string out;
    char hex[9];
    for (uint32_t v : h) {
        std::snprintf(hex, sizeof hex, "%08x", v);
        out += hex;
    }
    return out;
}

static const char* const base64_letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

std::string base64(const Bytes& bytes) {
    std::string out;
    for (size_t i = 0; i < bytes.size(); i += 3) {
        uint32_t n = uint32_t(bytes[i]) << 16;
        if (i + 1 < bytes.size()) n |= uint32_t(bytes[i + 1]) << 8;
        if (i + 2 < bytes.size()) n |= bytes[i + 2];
        out += base64_letters[(n >> 18) & 63];
        out += base64_letters[(n >> 12) & 63];
        out += i + 1 < bytes.size() ? base64_letters[(n >> 6) & 63] : '=';
        out += i + 2 < bytes.size() ? base64_letters[n & 63] : '=';
    }
    return out;
}

Bytes from_base64(const std::string& text) {
    Bytes out;
    uint32_t n = 0;
    int bits = 0;
    for (char c : text) {
        const char* at = std::strchr(base64_letters, c);
        if (c == '=' || !at || !c) continue;
        n = n << 6 | uint32_t(at - base64_letters);
        bits += 6;
        if (bits >= 8) {
            bits -= 8;
            out.push_back(uint8_t(n >> bits));
        }
    }
    return out;
}

void to_json(nlohmann::json& out, const Picture& picture) {
    switch (picture.state) {
    case Picture::State::Stored: out = {{"stored", true}}; break;
    case Picture::State::New: out = {{"data", base64(picture.bytes)}}; break;
    case Picture::State::InStore: out = {{"file", picture.path}}; break;
    }
}

void from_json(const nlohmann::json& in, Picture& picture) {
    if (in.contains("data")) picture = Picture::of(from_base64(in["data"].get<std::string>()));
    else if (in.contains("file")) picture = Picture::in_store(in["file"].get<std::string>());
    else picture = Picture::stored();
}

}  // namespace clarity
