// Pictures in records: an image field holds no picture, the one the record
// has stored, a new one loaded but not yet saved, or a file of the field's
// own store. Reading records never carries the pictures themselves; the
// screen fetches each on its own. A picture is kept in the database, or as
// a file in a file store named by the hash of its bytes, the column holding
// that name. The other runtimes do the same (conformance/pictures.json).

#pragma once

#include <filesystem>
#include <optional>
#include <string>

#include <nlohmann/json.hpp>

#include "clarity/dialect.hpp"
#include "clarity/value.hpp"

namespace clarity {

/// The picture of an image field that has one.
struct Picture {
    enum class State { Stored, New, InStore };
    State state = State::Stored;
    /// The bytes of a new picture.
    Bytes bytes;
    /// The path of a file of the field's own store.
    std::string path;

    /// The record has one stored; it is fetched on its own.
    static Picture stored() { return {}; }
    /// One loaded but not yet saved.
    static Picture of(Bytes bytes) { return {State::New, std::move(bytes), {}}; }
    /// A file of the field's own store, by its path there: saved as that path, nothing copied.
    static Picture in_store(std::string path) { return {State::InStore, {}, std::move(path)}; }

    friend bool operator==(const Picture&, const Picture&) = default;
};

/// An image field's column, and the file store it keeps its pictures in;
/// without one they are in the database.
struct PictureColumn {
    std::string column;
    std::optional<std::string> store;
    /// For a file field: the column beside it that holds the file's name.
    /// Any file is kept, not only a picture, and in a store under its name.
    std::optional<std::string> name = std::nullopt;
};

/// A kind of picture every stack shows.
struct PictureKind {
    std::string extension;
    std::string media_type;
};

/// The kind of picture `bytes` are, told by their first bytes.
std::optional<PictureKind> kind_of(const Bytes& bytes);
/// The kind of `bytes`, or InvalidError when they are no picture.
PictureKind checked(const Bytes& bytes);
/// The name of a picture's file in a file store: the first 32 hex digits of
/// the SHA-256 of its bytes, and the extension of its kind.
std::string file_name(const Bytes& bytes);
/// A picture as the page shows it.
std::string data_url(const Bytes& bytes);
/// What the select list reads of a picture kept in the database: whether there is one.
std::string presence(Dialect d, const std::string& column);
/// The file `path` (as a column holds it) in the store at `root`; a path
/// that would leave the store is refused.
std::filesystem::path in_store(const std::filesystem::path& root, const std::string& path);

std::string sha256_hex(const Bytes& bytes);
std::string base64(const Bytes& bytes);
Bytes from_base64(const std::string& text);

/// `{"stored": true}`, `{"data": base64}` or `{"file": path}`.
void to_json(nlohmann::json& out, const Picture& picture);
void from_json(const nlohmann::json& in, Picture& picture);

}  // namespace clarity

namespace nlohmann {
/// An empty picture is null on the wire.
template <>
struct adl_serializer<std::optional<clarity::Picture>> {
    static void to_json(json& out, const std::optional<clarity::Picture>& p) { out = p ? json(*p) : json(nullptr); }
    static void from_json(const json& in, std::optional<clarity::Picture>& p) {
        if (in.is_null()) p.reset();
        else p = in.get<clarity::Picture>();
    }
};
}  // namespace nlohmann
