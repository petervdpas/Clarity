// Lists of values and random, against the cases every stack's runtime
// shares (conformance/lists.json).

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

TEST(random_gives_what_every_runtime_gives_from_a_seed) {
    for (const auto& c : conformance("lists.json")["random"]) {
        Splitmix random(c["seed"].get<uint64_t>());
        auto max = c["max"].get<int64_t>();
        if (!c.contains("gives")) {
            try {
                (void)random.next(max);
                CHECK(false);
            } catch (const ValueError& e) {
                CHECK_EQ(std::string(e.what()), c["fails"].get<std::string>());
            }
            continue;
        }
        std::vector<int64_t> got;
        for (size_t i = 0; i < c["gives"].size(); ++i) got.push_back(random.next(max));
        CHECK(got == c["gives"].get<std::vector<int64_t>>());
    }
}

TEST(random_stays_within_its_highest_number) {
    for (int i = 0; i < 200; ++i) {
        auto n = clarity::random(6);
        CHECK(n >= 1 && n <= 6);
    }
}
