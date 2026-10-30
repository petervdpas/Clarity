// Lists of values, against the cases every stack's runtime shares
// (conformance/lists.json).

#include "check.hpp"
#include "clarity/code.hpp"
#include "conformance.hpp"

using namespace clarity;

TEST(an_item_is_read_by_its_position_from_1) {
    for (const auto& c : conformance("lists.json")["items"]) {
        auto list = c["list"].get<std::vector<int64_t>>();
        auto position = c["at"].get<int64_t>();
        if (c.contains("gives")) {
            CHECK_EQ(at(list, position), c["gives"].get<int64_t>());
            continue;
        }
        try {
            (void)at(list, position);
            CHECK(false);
        } catch (const ValueError& e) {
            CHECK_EQ(std::string(e.what()), c["fails"].get<std::string>());
        }
    }
}

TEST(an_item_is_set_by_its_position_from_1) {
    std::vector<int64_t> list{10, 20, 30};
    at(list, 2) = 99;
    CHECK(list == (std::vector<int64_t>{10, 99, 30}));
    CHECK_THROWS(ValueError, at(list, 4) = 1);
}

TEST(ranges_and_joins_are_as_every_runtime_gives_them) {
    auto file = conformance("lists.json");
    for (const auto& c : file["ranges"]) CHECK(range(c["from"].get<int64_t>(), c["to"].get<int64_t>()) == c["gives"].get<std::vector<int64_t>>());
    for (const auto& c : file["joins"]) CHECK_EQ(joined(c["texts"].get<std::vector<std::string>>(), c["separator"].get<std::string>()), c["gives"].get<std::string>());
}
