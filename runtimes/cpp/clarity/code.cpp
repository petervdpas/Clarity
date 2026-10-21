#include "clarity/code.hpp"

#include <algorithm>
#include <cstdio>
#include <mutex>
#include <random>

namespace clarity {

std::string show(const std::optional<DateTime>& value) {
    if (!value) return "";
    char time[8];
    std::snprintf(time, sizeof time, " %02d:%02d", value->hour, value->minute);
    return value->date.str() + time;
}

std::vector<FileEntry> filter_files(std::vector<FileEntry> files, const std::function<bool(const FileEntry&)>& keep) {
    std::erase_if(files, [&](const FileEntry& f) { return !keep(f); });
    return files;
}

static int compare_files(const FileEntry& a, const FileEntry& b, const std::string& detail) {
    if (detail == "name") return a.name.compare(b.name);
    if (detail == "extension") return a.extension.compare(b.extension);
    if (detail == "size") return a.size < b.size ? -1 : a.size > b.size;
    if (detail == "modified") return compare(a.modified, b.modified);
    return a.path.compare(b.path);
}

std::vector<FileEntry> sort_files(std::vector<FileEntry> files, std::initializer_list<FileOrder> keys) {
    std::vector<FileOrder> order(keys);
    std::stable_sort(files.begin(), files.end(), [&](const FileEntry& a, const FileEntry& b) {
        for (const auto& k : order) {
            int c = compare_files(a, b, k.detail);
            if (k.descending) c = -c;
            if (c != 0) return c < 0;
        }
        return false;
    });
    return files;
}

int64_t Splitmix::next(int64_t max) {
    if (max < 1) throw ValueError("random takes a highest number of 1 or more, not " + std::to_string(max));
    state_ += 0x9E3779B97F4A7C15ULL;
    uint64_t z = state_;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    z ^= z >> 31;
    return 1 + int64_t(z % uint64_t(max));
}

int64_t random(int64_t max) {
    static std::mutex lock;
    static Splitmix shared{(uint64_t(std::random_device{}()) << 32) ^ std::random_device{}()};
    std::lock_guard<std::mutex> held(lock);
    return shared.next(max);
}

}  // namespace clarity
