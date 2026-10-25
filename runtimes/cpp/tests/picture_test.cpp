// Pictures in records (X2), against the cases every runtime shares
// (conformance/pictures.json), and kept in SQLite or in a file store.

#include <filesystem>
#include <fstream>

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/sql.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

Bytes hex(const std::string& text) {
    Bytes out;
    for (size_t i = 0; i + 1 < text.size(); i += 2) out.push_back(uint8_t(std::stoi(text.substr(i, 2), nullptr, 16)));
    return out;
}

struct Album {
    int64_t id = 0;
    std::string title;
    std::optional<Picture> cover;
    std::optional<Picture> photo;

    static constexpr const char* table = "album";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"title", "cover", "photo"};
        return names;
    }
    static std::vector<PictureColumn> pictures() { return {{"cover", std::nullopt}, {"photo", "photos"}}; }
    std::vector<SqlValue> values() const { return {sql::text(title), sql::picture(cover), sql::picture(photo)}; }
    static Album from_values(int64_t id, const Row& v) {
        return {id, sql::get_text("title", v[0]), sql::get_picture("cover", v[1]), sql::get_picture("photo", v[2])};
    }
};

const Bytes png{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
const Bytes jpg{0xFF, 0xD8, 0xFF, 0xE0};

std::filesystem::path scratch(const std::string& name) {
    auto dir = std::filesystem::temp_directory_path() / ("clarity-pictures-cpp-" + name);
    std::filesystem::remove_all(dir);
    return dir;
}

Db albums(const std::filesystem::path& store) {
    Db db = Db::memory();
    db.migrate({{"CREATE TABLE album (id INTEGER PRIMARY KEY, title TEXT NOT NULL, cover BLOB, photo TEXT)"}});
    db.add_store("photos", store.string());
    return db;
}

}  // namespace

TEST(a_picture_is_told_by_its_first_bytes_and_named_by_them) {
    json cases = conformance("pictures.json");
    for (const auto& kind : cases["kinds"]) {
        auto found = kind_of(hex(kind["bytes"]));
        REQUIRE(found);
        CHECK_EQ(found->extension, kind["extension"].get<std::string>());
        CHECK_EQ(found->media_type, kind["type"].get<std::string>());
    }
    for (const auto& other : cases["notPictures"]) CHECK(!kind_of(hex(other)));
    for (const auto& c : cases["fileNames"]) CHECK_EQ(file_name(hex(c["bytes"])), c["name"].get<std::string>());
    CHECK_EQ(sha256_hex({}), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    CHECK_THROWS(InvalidError, file_name(Bytes{'%', 'P', 'D', 'F'}));
    std::pair<const char*, Dialect> dialects[] = {{"sqlite", Dialect::Sqlite}, {"postgres", Dialect::Postgres}, {"mysql", Dialect::Mysql}, {"mssql", Dialect::Mssql}};
    for (auto [word, d] : dialects) CHECK_EQ(presence(d, "cover"), cases["presence"][word].get<std::string>());
    CHECK_EQ(data_url(png), "data:image/png;base64,iVBORw0KGgo=");
}

TEST(a_picture_travels_to_the_renderer_as_every_runtime_sends_it) {
    json cases = conformance("pictures.json");
    for (const auto& c : cases["wire"]) {
        std::optional<Picture> value;
        if (c["value"] == "stored") value = Picture::stored();
        if (c["value"] == "new") value = Picture::of(hex(c["bytes"]));
        CHECK_EQ(json(value), c["json"]);
        CHECK(c["json"].get<std::optional<Picture>>() == value);
    }
    CHECK_EQ(json(Picture::in_store("2024/a.png")), json::parse(R"({"file":"2024/a.png"})"));
}

TEST(a_picture_is_kept_in_the_database_and_fetched_on_its_own) {
    auto store = scratch("db");
    Db db = albums(store);
    Album album{0, "Blue Train", Picture::of(png), std::nullopt};
    db.save(album);
    Album read = db.get<Album>(album.id);
    CHECK(read.cover == Picture::stored());
    CHECK(!read.photo);
    CHECK(db.picture<Album>(album.id, "cover") == png);
    // Saving the record again keeps the picture it has.
    read.title = "Blue Train (mono)";
    db.save(read);
    CHECK(db.picture<Album>(album.id, "cover") == png);
    // A new picture replaces it; none empties it.
    read.cover = Picture::of(jpg);
    db.save(read);
    CHECK(db.picture<Album>(album.id, "cover") == jpg);
    read.cover.reset();
    db.save(read);
    CHECK(!db.picture<Album>(album.id, "cover"));
    // Only a picture is kept.
    read.cover = Picture::of(Bytes{'%', 'P', 'D', 'F'});
    CHECK_THROWS(InvalidError, db.save(read));
    std::filesystem::remove_all(store);
}

TEST(a_picture_in_a_file_store_is_a_file_named_by_its_bytes) {
    auto store = scratch("store");
    Db db = albums(store);
    Album album{0, "Kind of Blue", std::nullopt, Picture::of(jpg)};
    db.save(album);
    std::ifstream written(store / file_name(jpg), std::ios::binary);
    CHECK(Bytes(std::istreambuf_iterator<char>(written), {}) == jpg);
    written.close(); // Windows removes no file that is open.
    CHECK(db.picture<Album>(album.id, "photo") == jpg);
    // A path that leaves the store is refused.
    db.batch("", "UPDATE album SET photo = '../secret.png' WHERE id = " + std::to_string(album.id));
    CHECK_THROWS(InvalidError, db.picture<Album>(album.id, "photo"));
    CHECK_THROWS(StorageError, Db::memory().add_store("photos", "env:CLARITY_NO_SUCH_STORE"));
    std::filesystem::remove_all(store);
}
