// The data formats of code end to end on the C++ runtime
// (conformance/formats-runtime.json): read through Clarity.Formats, decoded
// by nlohmann into records as the generator writes them, written back.

#include "check.hpp"
#include "conformance.hpp"
#include "formats/formats.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

struct Line {
    int64_t id = 0;
    std::string product;
    int64_t quantity = 0;
};

struct Order {
    int64_t id = 0;
    std::string number;
    Decimal total;
    std::optional<Date> placed;
    std::vector<Line> lines;
};

void to_json(json& j, const Line& r) { j = json{{"id", r.id}, {"product", r.product}, {"quantity", r.quantity}}; }
void from_json(const json& j, Line& r) {
    j.at("id").get_to(r.id);
    j.at("product").get_to(r.product);
    j.at("quantity").get_to(r.quantity);
}
void to_json(json& j, const Order& r) { j = json{{"id", r.id}, {"number", r.number}, {"total", r.total}, {"placed", r.placed}, {"lines", r.lines}}; }
void from_json(const json& j, Order& r) {
    j.at("id").get_to(r.id);
    j.at("number").get_to(r.number);
    j.at("total").get_to(r.total);
    j.at("placed").get_to(r.placed);
    j.at("lines").get_to(r.lines);
}

}  // namespace

TEST(formats_read_decode_and_write_as_every_stack_does) {
    for (const auto& c : conformance("formats-runtime.json")["cases"]) {
        std::string kind = c["kind"].dump();
        std::string options = c.contains("options") ? c["options"].dump() : "";
        std::string input = c["input"].get<std::string>();
        bool many = c["kind"]["kind"] == "records";
        try {
            std::string written = many ? formats_write("json", formats_read<std::vector<Order>>("json", input, options, kind), "", kind)
                                       : formats_write("json", formats_read<Order>("json", input, options, kind), "", kind);
            CHECK(c.contains("written"));
            if (c.contains("written")) CHECK_EQ(written, c["written"].get<std::string>());
        } catch (const ValueError& e) {
            CHECK(c.contains("error"));
            if (c.contains("error")) CHECK_EQ(std::string(e.what()), c["error"].get<std::string>());
        }
    }
}
