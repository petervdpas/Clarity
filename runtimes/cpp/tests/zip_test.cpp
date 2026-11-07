// zip(paths, texts) alike in every runtime (conformance/zip.json).

#include <cstdio>

#include "check.hpp"
#include "clarity/texts.hpp"
#include "conformance.hpp"

using nlohmann::json;

TEST(a_zip_is_written_as_every_runtime_writes_it) {
    const json& cases = conformance("zip.json");
    for (const auto& item : cases["cases"]) {
        auto got = clarity::zip(item["paths"].get<std::vector<std::string>>(), item["texts"].get<std::vector<std::string>>());
        std::string hex;
        char two[3];
        for (uint8_t b : got) {
            std::snprintf(two, sizeof two, "%02x", b);
            hex += two;
        }
        CHECK_EQ(hex, item["hex"].get<std::string>());
    }
}
