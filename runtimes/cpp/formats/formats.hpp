// The data formats of code (docs/FORMATS-PLAN.md) through Clarity.Formats,
// the one implementation every stack calls: `read Order from json text`
// reads the format and fits it to the record, which the record's own
// from_json then decodes; `json of value` writes the value as its to_json
// writes it and the library writes it in the format, in Clarity's one
// form. Carried only when the code reads or writes a format
// (clarity_use_formats in clarity.cmake links the library).

#pragma once

#include <string>

#include <nlohmann/json.hpp>

#include "clarity/code.hpp"

namespace clarity {

/// The JSON of a value as its to_json writes it: a new record, which a kind carries as its base, or an option's value.
template <typename T>
std::string json_of(const T& value) {
    return nlohmann::json(value).dump();
}

/// Reads `input` in `format` with its `options` (a JSON object, "" for none)
/// as `kind` describes it: what was fitted, as JSON. Throws a ValueError with
/// Clarity.Formats' message, word for word as every stack says it.
std::string formats_fitted(const std::string& format, const std::string& input, const std::string& options, const std::string& kind);

/// `value` (as to_json wrote it) written in `format` with its `options`, as `kind` describes it.
std::string formats_written(const std::string& format, const std::string& value, const std::string& options, const std::string& kind);

/// Reads `input` in `format` and decodes what was fitted as a `T` (a record, or a vector of them).
template <typename T>
T formats_read(const std::string& format, const std::string& input, const std::string& options, const std::string& kind) {
    try {
        return nlohmann::json::parse(formats_fitted(format, input, options, kind)).get<T>();
    } catch (const nlohmann::json::exception& e) {
        throw ValueError(std::string("what was read does not fit: ") + e.what());
    }
}

/// `value` written in `format` with its `options`, as `kind` describes it.
template <typename T>
std::string formats_write(const std::string& format, const T& value, const std::string& options, const std::string& kind) {
    return formats_written(format, json_of(value), options, kind);
}

}  // namespace clarity
