// A backup of an SQLite database before migration steps change it; see backup.hpp.

#include "clarity/backup.hpp"

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <filesystem>
#include <optional>
#include <tuple>
#include <vector>

#include "clarity/errors.hpp"

namespace clarity::backup {

namespace fs = std::filesystem;

std::string stamp(int64_t seconds) {
    int64_t days = seconds / 86400, rest = seconds % 86400;
    if (rest < 0) {
        rest += 86400;
        days -= 1;
    }
    // The civil date of a day count (Howard Hinnant's days_from_civil, turned around).
    int64_t z = days + 719468;
    int64_t era = (z >= 0 ? z : z - 146096) / 146097;
    int64_t doe = z - era * 146097;
    int64_t yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    int64_t doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    int64_t mp = (5 * doy + 2) / 153;
    int64_t day = doy - (153 * mp + 2) / 5 + 1;
    int64_t month = mp < 10 ? mp + 3 : mp - 9;
    int64_t year = yoe + era * 400 + (month <= 2);
    char text[96];
    std::snprintf(text, sizeof text, "%04lld%02lld%02lldT%02lld%02lld%02lldZ", (long long)year, (long long)month, (long long)day, (long long)(rest / 3600),
                  (long long)(rest % 3600 / 60), (long long)(rest % 60));
    return text;
}

std::string file_name(const std::string& database, int64_t seconds, int step) {
    char number[16];
    std::snprintf(number, sizeof number, "%04d", step);
    return database + "-" + stamp(seconds) + "-before-migration-" + number + ".db";
}

namespace {

struct Found {
    std::string when;
    int count;
    fs::path path;
};

/// A backup file of the database named `database`, when it is an automatic one: its stamp and count within its second.
std::optional<Found> automatic(const std::string& database, const fs::path& path) {
    std::string file = path.filename().string();
    if (file.size() < database.size() + 22 || file.compare(0, database.size() + 1, database + "-") != 0 || file.substr(file.size() - 3) != ".db") return std::nullopt;
    std::string rest = file.substr(database.size() + 1, file.size() - database.size() - 4);
    if (rest.size() < 18 || rest[15] != 'Z' || rest[16] != '-') return std::nullopt;
    std::string when = rest.substr(0, 16), why = rest.substr(17);
    int count = 1;
    if (auto under = why.find('_'); under != std::string::npos) {
        try {
            size_t used = 0;
            count = std::stoi(why.substr(under + 1), &used);
            if (used != why.size() - under - 1) return std::nullopt;
        } catch (const std::exception&) {
            return std::nullopt;
        }
        why = why.substr(0, under);
    }
    if (why == "manual") return std::nullopt;
    return Found{when, count, path};
}

}  // namespace

void prune(const std::string& database) {
    fs::path file(database);
    fs::path folder = file.parent_path() / "backups";
    std::error_code error;
    if (!fs::is_directory(folder, error)) return;
    std::vector<Found> all;
    for (const auto& entry : fs::directory_iterator(folder, error))
        if (auto found = automatic(file.stem().string(), entry.path())) all.push_back(*found);
    std::sort(all.begin(), all.end(), [](const Found& a, const Found& b) { return std::tie(b.when, b.count) < std::tie(a.when, a.count); });
    for (size_t i = kept_automatic; i < all.size(); ++i) fs::remove(all[i].path, error);
}

void before_migration(Driver& driver, int step) {
    if (driver.dialect() != Dialect::Sqlite) return;
    auto rows = driver.rows("SELECT file FROM pragma_database_list WHERE name = 'main'", {});
    if (rows.empty() || !std::holds_alternative<std::string>(rows[0][0]) || std::get<std::string>(rows[0][0]).empty()) return;
    fs::path file(std::get<std::string>(rows[0][0]));
    try {
        fs::path folder = file.parent_path() / "backups";
        fs::create_directories(folder);
        int64_t now = std::chrono::duration_cast<std::chrono::seconds>(std::chrono::system_clock::now().time_since_epoch()).count();
        fs::path first = folder / file_name(file.stem().string(), now, step);
        fs::path path = first;
        for (int n = 2; fs::exists(path); ++n) path = folder / (first.stem().string() + "_" + std::to_string(n) + ".db");
        driver.execute("VACUUM INTO ?1", {path.string()});
    } catch (const std::exception& e) {
        throw StorageError(std::string("the database was not migrated, since it could not be backed up first: ") + e.what());
    }
    prune(file.string());
}

}  // namespace clarity::backup
