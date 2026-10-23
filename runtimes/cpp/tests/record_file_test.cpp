// Files in records (U1c), against the cases every runtime shares
// (conformance/record-files.json), and kept in SQLite or in a file store.

#include <algorithm>
#include <filesystem>
#include <fstream>

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/record_file.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(a_file_in_a_store_is_kept_under_its_name_in_a_folder_of_its_contents) {
    json cases = conformance("record-files.json");
    for (const auto& c : cases["storePaths"]) {
        std::string text = c["text"];
        CHECK(store_path(Bytes(text.begin(), text.end()), c["chosen"]) == c["path"].get<std::string>());
    }
    for (const auto& c : cases["names"]) CHECK(name_of(c["path"]) == c["name"].get<std::string>());
}

TEST(a_copy_goes_to_the_downloads_folder_under_a_free_name) {
    json cases = conformance("record-files.json");
    for (const auto& c : cases["downloads"]) {
        const json& env = c["env"];
        auto found = downloads_in(c["os"], [&](const std::string& n) { return env.contains(n) ? std::optional<std::string>(env[n].get<std::string>()) : std::nullopt; });
        if (c["folder"].is_null()) CHECK(!found);
        else CHECK(found && *found == c["folder"].get<std::string>());
    }
    for (const auto& c : cases["copies"]) {
        auto taken = c["taken"].get<std::vector<std::string>>();
        CHECK(free_name(c["name"], [&](const std::string& n) { return std::find(taken.begin(), taken.end(), n) != taken.end(); }) == c["saved"].get<std::string>());
    }
}

TEST(a_file_travels_with_its_name) {
    json stored = RecordFile{"manual.pdf", Picture::stored()};
    CHECK(stored == json({{"stored", true}, {"name", "manual.pdf"}}));
    auto read = json({{"name", "C:\\docs\\a.pdf"}, {"data", "JVBERg=="}}).get<RecordFile>();
    CHECK(read.name == "a.pdf" && read.content.bytes.size() == 4);
}
