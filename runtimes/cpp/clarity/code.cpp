#include "clarity/code.hpp"

#include <algorithm>
#include <cstdio>

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

}  // namespace clarity
