// Text functions alike in every runtime (conformance/texts.json).

#include "check.hpp"
#include "clarity/texts.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(text_functions_read_as_every_runtime_reads_them) {
    const json& cases = conformance("texts.json");
    for (const auto& item : cases["cases"]) {
        std::string function = item["function"].get<std::string>();
        const json& a = item["args"];
        const json& want = item["result"];
        auto s = [&](size_t i) { return a[i].get<std::string>(); };
        auto n = [&](size_t i) { return a[i].get<int64_t>(); };
        json got;
        if (function == "words") got = words(s(0));
        else if (function == "split") got = split(s(0), s(1));
        else if (function == "trim") got = trim(s(0));
        else if (function == "lower") got = lower(s(0));
        else if (function == "upper") got = upper(s(0));
        else if (function == "replace") got = replace(s(0), s(1), s(2));
        else if (function == "startsWith") got = starts_with(s(0), s(1));
        else if (function == "endsWith") got = ends_with(s(0), s(1));
        else if (function == "slice") got = slice(s(0), n(1), n(2));
        else if (function == "indexOf") got = index_of(s(0), s(1));
        CHECK_EQ(got.dump(), want.dump());
    }
}
