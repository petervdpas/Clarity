// The files of a file store (X2), against the cases every runtime shares
// (conformance/files.json).

#include <chrono>
#include <filesystem>
#include <fstream>

#include "check.hpp"
#include "clarity/db.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;
namespace fs = std::filesystem;

namespace {

fs::path planted() {
    auto root = fs::temp_directory_path() / "clarity-files-cpp";
    fs::remove_all(root);
    json cases = conformance("files.json");
    for (const auto& f : cases["tree"]) {
        fs::path path = root / f["path"].get<std::string>();
        fs::create_directories(path.parent_path());
        std::ofstream(path, std::ios::binary) << f["text"].get<std::string>();
        auto at = DateTime::parse(f["modified"].get<std::string>());
        std::chrono::sys_days day = std::chrono::year_month_day{std::chrono::year{at->date.year}, std::chrono::month{unsigned(at->date.month)}, std::chrono::day{unsigned(at->date.day)}};
        auto when = day + std::chrono::hours{at->hour} + std::chrono::minutes{at->minute} + std::chrono::seconds{at->second};
        fs::last_write_time(path, std::chrono::clock_cast<fs::file_time_type::clock>(when));
    }
    return root;
}

}  // namespace

TEST(a_store_lists_its_files_as_every_runtime_does) {
    fs::path root = planted();
    json cases = conformance("files.json");
    Db db = Db::memory();
    db.add_store("photos", root.string());
    json listed = json::array();
    for (const auto& f : db.files("photos"))
        listed.push_back({{"path", f.path}, {"name", f.name}, {"extension", f.extension}, {"size", f.size}, {"modified", f.modified->str()}});
    CHECK_EQ(listed, cases["listing"]);
    CHECK_THROWS(StorageError, db.files("films"));

    // Text is read and written inside the store.
    FileEntry notes;
    for (const auto& f : db.files("photos"))
        if (f.name == "notes.txt") notes = f;
    CHECK_EQ(db.read_text(notes), "Hello\nworld\n");
    db.write_text("photos", "exports/titles.txt", "Blue Train\n");
    std::ifstream written(root / "exports" / "titles.txt");
    CHECK_EQ(std::string(std::istreambuf_iterator<char>(written), {}), "Blue Train\n");
    for (const auto& path : cases["refused"]) CHECK_THROWS(InvalidError, db.write_text("photos", path.get<std::string>(), "x"));

    // A file becomes a picture: linked in its own store, copied elsewhere.
    Bytes png{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
    fs::create_directories(root / "covers");
    std::ofstream(root / "covers" / "cover.png", std::ios::binary).write(reinterpret_cast<const char*>(png.data()), std::streamsize(png.size()));
    FileEntry cover;
    for (const auto& f : db.files("photos"))
        if (f.name == "cover.png") cover = f;
    CHECK(db.picture_from(cover, "photos") == Picture::in_store("covers/cover.png"));
    CHECK(db.picture_from(cover, std::nullopt) == Picture::of(png));
    CHECK_THROWS(InvalidError, db.picture_from(notes, std::nullopt));
    CHECK(db.store_file("photos", "covers/cover.png") == png);
    CHECK_THROWS(InvalidError, db.store_file("photos", "../x.png"));
    fs::remove_all(root);
}
