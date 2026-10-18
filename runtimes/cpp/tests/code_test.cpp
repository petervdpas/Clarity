// What generated code calls: contracts, values as text, `in`, and files
// filtered and sorted by their details.

#include "check.hpp"
#include "clarity/code.hpp"

using namespace clarity;

TEST(contracts_show_and_one_of) {
    CHECK_THROWS(ContractError, require(false, "order.total > 0"));
    require(true, "fine");
    try {
        require(false, "order.total > 0");
    } catch (const ContractError& e) {
        CHECK_EQ(std::string(e.what()), "condition failed: order.total > 0");
    }
    CHECK_EQ(show(int64_t(42)), "42");
    CHECK_EQ(show(true), "yes");
    CHECK_EQ(show(Decimal::parse("9.95")), "9.95");
    CHECK_EQ(show(DateTime::parse("2026-10-17T09:30:15")), "2026-10-17 09:30");
    CHECK_EQ(show(std::optional<Date>()), "");
    CHECK(one_of<std::string>("b", {"a", "b"}));
    CHECK(!one_of<int64_t>(3, {1, 2}));
}

TEST(files_are_filtered_and_sorted_by_their_details) {
    auto at = [](int day) { return DateTime::parse("2026-01-0" + std::to_string(day) + "T00:00:00"); };
    std::vector<FileEntry> files{{"s", "x/b.jpg", "b.jpg", "jpg", 2, at(3)}, {"s", "a.png", "a.png", "png", 9, at(1)}, {"s", "c.jpg", "c.jpg", "jpg", 2, at(2)}};
    auto names = [](const std::vector<FileEntry>& fs) {
        std::vector<std::string> out;
        for (const auto& f : fs) out.push_back(f.name);
        return out;
    };
    CHECK_EQ(filter_files(files, [](const FileEntry& f) { return f.extension == "jpg"; }).size(), size_t(2));
    CHECK(names(sort_files(files, {desc("size"), asc("path")})) == (std::vector<std::string>{"a.png", "c.jpg", "b.jpg"}));
    CHECK(names(sort_files(files, {asc("modified")})) == (std::vector<std::string>{"a.png", "c.jpg", "b.jpg"}));
    CHECK(names(sort_files(files, {desc("name")})) == (std::vector<std::string>{"c.jpg", "b.jpg", "a.png"}));
}
