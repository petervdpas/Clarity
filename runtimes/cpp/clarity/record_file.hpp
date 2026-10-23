// Files in records (U1c): a file field holds no file, the one the record
// has stored, a new one chosen but not yet saved, or a file of its file
// store; always with its name. Like a picture, reading records never
// carries the file itself; the screen fetches it on its own to open it or
// save a copy. In the database the bytes are in the field's column and the
// name in the column beside it; in a file store the file keeps its name in
// a folder named by its contents (conformance/record-files.json).

#pragma once

#include <functional>
#include <optional>
#include <string>

#include <nlohmann/json.hpp>

#include "clarity/picture.hpp"
#include "clarity/value.hpp"

namespace clarity {

/// The file of a file field that has one, and its name.
struct RecordFile {
    std::string name;
    Picture content;

    friend bool operator==(const RecordFile&, const RecordFile&) = default;
};

/// The name a chosen file is kept under: the last part of what was chosen, trimmed; "file" when nothing is left.
std::string clean_name(const std::string& chosen);
/// The path of a file in a file store: a folder named by the first 32 hex
/// digits of the SHA-256 of its bytes, and in it the file under its name.
std::string store_path(const Bytes& bytes, const std::string& name);
/// The name a path in a store shows: its last part.
std::string name_of(const std::string& path);
/// The extension of a file's name, in lower case without its dot; "" without one.
std::string extension_of(const std::string& name);
/// The name of a record's file, and its extension; "" when it has none.
std::string record_file_name(const std::optional<RecordFile>& file);
std::string record_file_extension(const std::optional<RecordFile>& file);
/// The folder a copy is saved in on system `os`, with `env` giving the environment; none when it names no home.
std::optional<std::string> downloads_in(const std::string& os, const std::function<std::optional<std::string>(const std::string&)>& env);
/// The name a copy gets where `taken` names are in use: the name itself, else " (2)", " (3)" and on before its extension.
std::string free_name(const std::string& name, const std::function<bool(const std::string&)>& taken);
/// Saves a copy of a file in the person's Downloads folder, under a name free there; the file it wrote.
std::string save_copy(const std::string& name, const Bytes& bytes);
/// Writes a copy of a file in a folder of its own under the system's temporary folder, to open it; the file.
std::string temporary_copy(const std::string& name, const Bytes& bytes);

/// What a file field stores in its own column (as a picture does), and in the column of its name ("" for none).
SqlValue file_content(const std::optional<RecordFile>& file);
SqlValue file_name_value(const std::optional<RecordFile>& file);
/// A record's file, from whether it has one (as for a picture) and its name.
std::optional<RecordFile> get_file(const std::string& column, const SqlValue& content, const SqlValue& name);

/// `{"name", "stored": true}`, `{"name", "data"}` or `{"name", "file"}`.
void to_json(nlohmann::json& out, const RecordFile& file);
void from_json(const nlohmann::json& in, RecordFile& file);

}  // namespace clarity

namespace nlohmann {
/// No file is null on the wire.
template <>
struct adl_serializer<std::optional<clarity::RecordFile>> {
    static void to_json(json& out, const std::optional<clarity::RecordFile>& f) { out = f ? json(*f) : json(nullptr); }
    static void from_json(const json& in, std::optional<clarity::RecordFile>& f) {
        if (in.is_null()) f.reset();
        else f = in.get<clarity::RecordFile>();
    }
};
}  // namespace nlohmann
