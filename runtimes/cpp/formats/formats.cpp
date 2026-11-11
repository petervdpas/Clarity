#include "formats/formats.hpp"

#include "clarity_formats.h"

namespace clarity {

namespace {

const ClarityFormatsApi* api() {
    static const ClarityFormatsApi* table = clarity_formats_api();
    return table;
}

/// Runs a call into the library: what it made, or the problem it kept, by `ok`.
std::string call(bool reading, const std::string& format, const std::string& input, const std::string& options, const std::string& kind, bool& ok) {
    uint8_t* made = nullptr;
    size_t length = 0;
    int32_t code = reading
        ? api()->read(format.data(), format.size(), reinterpret_cast<const uint8_t*>(input.data()), input.size(), options.data(), options.size(), kind.data(), kind.size(), &made, &length)
        : api()->write(format.data(), format.size(), input.data(), input.size(), options.data(), options.size(), kind.data(), kind.size(), &made, &length);
    ok = code == CLARITY_FORMATS_OK;
    if (!ok) {
        char why[1024] = {0};
        api()->problem(why, sizeof why);
        return why;
    }
    std::string out(reinterpret_cast<const char*>(made), length);
    api()->free(made, length);
    return out;
}

/// The table calls came with interface 2; an older library has no such fields.
void tables_known() {
    if (api()->interface < 2) throw ValueError("this Clarity.Formats library is too old to read a file as a table");
}

/// What the library made, or a ValueError with its problem.
std::string made(int32_t code, uint8_t* bytes, size_t length) {
    if (code != CLARITY_FORMATS_OK) {
        char why[1024] = {0};
        api()->problem(why, sizeof why);
        throw ValueError(why[0] ? why : "what was read does not fit");
    }
    std::string out(reinterpret_cast<const char*>(bytes), length);
    api()->free(bytes, length);
    return out;
}

}  // namespace

std::string formats_read_table(const std::string& format, const std::string& input, const std::string& options, const std::string& fields) {
    tables_known();
    uint8_t* bytes = nullptr;
    size_t length = 0;
    int32_t code = api()->read_table(format.data(), format.size(), reinterpret_cast<const uint8_t*>(input.data()), input.size(), options.data(), options.size(), fields.data(), fields.size(), &bytes, &length);
    return made(code, bytes, length);
}

std::string formats_write_table(const std::string& format, const std::string& original, const std::string& table, const std::string& options, const std::string& fields) {
    tables_known();
    uint8_t* bytes = nullptr;
    size_t length = 0;
    int32_t code = api()->write_table(format.data(), format.size(), reinterpret_cast<const uint8_t*>(original.data()), original.size(), table.data(), table.size(), options.data(), options.size(), fields.data(), fields.size(), &bytes, &length);
    return made(code, bytes, length);
}

std::string formats_fitted(const std::string& format, const std::string& input, const std::string& options, const std::string& kind) {
    bool ok = false;
    std::string out = call(true, format, input, options, kind, ok);
    if (!ok) throw ValueError(out);
    return out;
}

std::string formats_written(const std::string& format, const std::string& value, const std::string& options, const std::string& kind) {
    bool ok = false;
    std::string out = call(false, format, value, options, kind, ok);
    // A kind the generator wrote always describes the value; should it not, the value as to_json wrote it.
    return ok ? out : value;
}

}  // namespace clarity
