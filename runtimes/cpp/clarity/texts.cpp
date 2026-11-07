#include "clarity/texts.hpp"

#include <algorithm>

namespace clarity {

namespace {

// The code point at `at` and how many bytes it takes; a broken sequence is one byte.
uint32_t utf8_at(const std::string& text, size_t at, size_t& size) {
    auto byte = [&](size_t i) { return i < text.size() ? static_cast<unsigned char>(text[i]) : 0u; };
    unsigned b0 = byte(at), b1 = byte(at + 1), b2 = byte(at + 2), b3 = byte(at + 3);
    if (b0 < 0x80) return size = 1, b0;
    if ((b0 & 0xE0) == 0xC0 && (b1 & 0xC0) == 0x80) return size = 2, ((b0 & 0x1F) << 6) | (b1 & 0x3F);
    if ((b0 & 0xF0) == 0xE0 && (b1 & 0xC0) == 0x80 && (b2 & 0xC0) == 0x80) return size = 3, ((b0 & 0x0F) << 12) | ((b1 & 0x3F) << 6) | (b2 & 0x3F);
    if ((b0 & 0xF8) == 0xF0 && (b1 & 0xC0) == 0x80 && (b2 & 0xC0) == 0x80 && (b3 & 0xC0) == 0x80)
        return size = 4, ((b0 & 0x07) << 18) | ((b1 & 0x3F) << 12) | ((b2 & 0x3F) << 6) | (b3 & 0x3F);
    return size = 1, b0;
}

void utf8_put(uint32_t cp, std::string& out) {
    if (cp < 0x80) {
        out += static_cast<char>(cp);
    } else if (cp < 0x800) {
        out += static_cast<char>(0xC0 | (cp >> 6));
        out += static_cast<char>(0x80 | (cp & 0x3F));
    } else if (cp < 0x10000) {
        out += static_cast<char>(0xE0 | (cp >> 12));
        out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
        out += static_cast<char>(0x80 | (cp & 0x3F));
    } else {
        out += static_cast<char>(0xF0 | (cp >> 18));
        out += static_cast<char>(0x80 | ((cp >> 12) & 0x3F));
        out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
        out += static_cast<char>(0x80 | (cp & 0x3F));
    }
}

bool is_space(uint32_t cp) {
    return cp == ' ' || (cp >= 0x09 && cp <= 0x0D) || cp == 0x85 || cp == 0xA0 || cp == 0x1680 || (cp >= 0x2000 && cp <= 0x200A) || cp == 0x2028 || cp == 0x2029 || cp == 0x202F || cp == 0x205F ||
           cp == 0x3000;
}

uint32_t to_lower(uint32_t c) {
    if (c >= 'A' && c <= 'Z') return c + 32;
    if (c >= 0xC0 && c <= 0xDE && c != 0xD7) return c + 32;
    if ((c >= 0x100 && c <= 0x137) || (c >= 0x14A && c <= 0x177)) return c % 2 == 0 ? c + 1 : c;
    if ((c >= 0x139 && c <= 0x148) || (c >= 0x179 && c <= 0x17E)) return c % 2 == 1 ? c + 1 : c;
    if (c == 0x178) return 0xFF;
    if (c >= 0x391 && c <= 0x3A9 && c != 0x3A2) return c + 32;
    if (c == 0x386) return 0x3AC;
    if (c >= 0x388 && c <= 0x38A) return c + 37;
    if (c == 0x38C) return 0x3CC;
    if (c == 0x38E || c == 0x38F) return c + 63;
    if (c >= 0x410 && c <= 0x42F) return c + 32;
    if (c >= 0x400 && c <= 0x40F) return c + 80;
    return c;
}

uint32_t to_upper(uint32_t c) {
    if (c >= 'a' && c <= 'z') return c - 32;
    if (c >= 0xE0 && c <= 0xFE && c != 0xF7) return c - 32;
    if (c == 0xFF) return 0x178;
    if ((c >= 0x100 && c <= 0x137) || (c >= 0x14A && c <= 0x177)) return c % 2 == 1 ? c - 1 : c;
    if ((c >= 0x139 && c <= 0x148) || (c >= 0x179 && c <= 0x17E)) return c % 2 == 0 ? c - 1 : c;
    if (c == 0x3C2) return 0x3A3;
    if (c >= 0x3B1 && c <= 0x3C9) return c - 32;
    if (c == 0x3AC) return 0x386;
    if (c >= 0x3AD && c <= 0x3AF) return c - 37;
    if (c == 0x3CC) return 0x38C;
    if (c == 0x3CD || c == 0x3CE) return c - 63;
    if (c >= 0x430 && c <= 0x44F) return c - 32;
    if (c >= 0x450 && c <= 0x45F) return c - 80;
    return c;
}

std::string cased(const std::string& text, uint32_t (*change)(uint32_t)) {
    std::string out;
    for (size_t at = 0, size = 0; at < text.size(); at += size) utf8_put(change(utf8_at(text, at, size)), out);
    return out;
}

// The number of characters in `text`.
int64_t characters(const std::string& text) {
    int64_t count = 0;
    for (unsigned char c : text)
        if ((c & 0xC0) != 0x80) ++count;
    return count;
}

// The byte offset of the character at `position` (from 1), the end for one past it.
size_t offset_of(const std::string& text, int64_t position) {
    size_t at = 0, size = 0;
    for (int64_t i = 1; i < position && at < text.size(); ++i) {
        utf8_at(text, at, size);
        at += size;
    }
    return at;
}

}  // namespace

std::vector<std::string> words(const std::string& text) {
    std::vector<std::string> out;
    size_t start = std::string::npos;
    size_t at = 0, size = 0;
    for (; at < text.size(); at += size) {
        if (is_space(utf8_at(text, at, size))) {
            if (start != std::string::npos) out.push_back(text.substr(start, at - start));
            start = std::string::npos;
        } else if (start == std::string::npos) {
            start = at;
        }
    }
    if (start != std::string::npos) out.push_back(text.substr(start, at - start));
    return out;
}

std::vector<std::string> split(const std::string& text, const std::string& separator) {
    std::vector<std::string> out;
    if (separator.empty()) {
        for (size_t at = 0, size = 0; at < text.size(); at += size) {
            utf8_at(text, at, size);
            out.push_back(text.substr(at, size));
        }
        return out;
    }
    size_t start = 0;
    for (size_t found; (found = text.find(separator, start)) != std::string::npos; start = found + separator.size()) out.push_back(text.substr(start, found - start));
    out.push_back(text.substr(start));
    return out;
}

std::string trim(const std::string& text) {
    size_t first = std::string::npos, end = 0;
    for (size_t at = 0, size = 0; at < text.size(); at += size) {
        if (!is_space(utf8_at(text, at, size))) {
            if (first == std::string::npos) first = at;
            end = at + size;
        }
    }
    return first == std::string::npos ? std::string() : text.substr(first, end - first);
}

std::string lower(const std::string& text) { return cased(text, to_lower); }

std::string upper(const std::string& text) { return cased(text, to_upper); }

std::string replace(const std::string& text, const std::string& find, const std::string& with) {
    if (find.empty()) return text;
    std::string out;
    size_t start = 0;
    for (size_t found; (found = text.find(find, start)) != std::string::npos; start = found + find.size()) {
        out.append(text, start, found - start);
        out += with;
    }
    out.append(text, start, std::string::npos);
    return out;
}

bool starts_with(const std::string& text, const std::string& prefix) { return text.compare(0, prefix.size(), prefix) == 0; }

bool ends_with(const std::string& text, const std::string& suffix) {
    return suffix.size() <= text.size() && text.compare(text.size() - suffix.size(), suffix.size(), suffix) == 0;
}

std::string slice(const std::string& text, int64_t from, int64_t count) {
    int64_t start = from > 1 ? from : 1;
    int64_t end = from + (count > 0 ? count : 0);
    if (end > characters(text) + 1) end = characters(text) + 1;
    if (end <= start) return std::string();
    size_t a = offset_of(text, start), b = offset_of(text, end);
    return text.substr(a, b - a);
}

int64_t index_of(const std::string& text, const std::string& find) {
    size_t found = text.find(find);
    return found == std::string::npos ? 0 : characters(text.substr(0, found)) + 1;
}

std::vector<uint8_t> zip(const std::vector<std::string>& paths, const std::vector<std::string>& texts) {
    std::vector<uint8_t> out, central;
    auto u16 = [](std::vector<uint8_t>& v, uint32_t x) { v.push_back(uint8_t(x)); v.push_back(uint8_t(x >> 8)); };
    auto u32 = [](std::vector<uint8_t>& v, uint32_t x) { for (int i = 0; i < 4; ++i) v.push_back(uint8_t(x >> (8 * i))); };
    auto crc32 = [](const std::string& data) {
        uint32_t crc = 0xFFFFFFFFu;
        for (unsigned char b : data) {
            crc ^= b;
            for (int k = 0; k < 8; ++k) crc = (crc & 1u) ? (crc >> 1) ^ 0xEDB88320u : crc >> 1;
        }
        return ~crc;
    };
    size_t count = std::min(paths.size(), texts.size());
    for (size_t i = 0; i < count; ++i) {
        const std::string& name = paths[i];
        const std::string& data = texts[i];
        uint32_t crc = crc32(data), size = uint32_t(data.size()), offset = uint32_t(out.size());
        // Version 20, UTF-8 names, stored, 1980-01-01 00:00.
        u32(out, 0x04034b50u);
        for (uint32_t v : {20u, 0x0800u, 0u, 0u, 0x21u}) u16(out, v);
        u32(out, crc); u32(out, size); u32(out, size);
        u16(out, uint32_t(name.size())); u16(out, 0);
        out.insert(out.end(), name.begin(), name.end());
        out.insert(out.end(), data.begin(), data.end());
        u32(central, 0x02014b50u);
        for (uint32_t v : {20u, 20u, 0x0800u, 0u, 0u, 0x21u}) u16(central, v);
        u32(central, crc); u32(central, size); u32(central, size);
        for (uint32_t v : {uint32_t(name.size()), 0u, 0u, 0u, 0u}) u16(central, v);
        u32(central, 0); u32(central, offset);
        central.insert(central.end(), name.begin(), name.end());
    }
    uint32_t central_size = uint32_t(central.size()), central_at = uint32_t(out.size());
    out.insert(out.end(), central.begin(), central.end());
    u32(out, 0x06054b50u);
    for (uint32_t v : {0u, 0u, uint32_t(count), uint32_t(count)}) u16(out, v);
    u32(out, central_size); u32(out, central_at); u16(out, 0);
    return out;
}

}  // namespace clarity
