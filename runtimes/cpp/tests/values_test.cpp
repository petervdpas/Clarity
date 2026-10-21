// The values records hold: exact decimals, dates and times, and text with
// slots as it travels to the screen.

#include "check.hpp"
#include "conformance.hpp"
#include "clarity/errors.hpp"
#include "clarity/text.hpp"

using namespace clarity;

TEST(a_decimal_keeps_its_decimals) {
    CHECK_EQ(Decimal::parse("1250.50").str(), "1250.50");
    CHECK_EQ(Decimal::parse("-0.05").str(), "-0.05");
    CHECK_EQ(Decimal::parse("7").str(), "7");
    CHECK_EQ(Decimal(42).str(), "42");
    CHECK_THROWS(InvalidError, Decimal::parse("12,5"));
    CHECK_THROWS(InvalidError, Decimal::parse(""));
}

TEST(decimals_add_multiply_divide_and_compare_by_value) {
    CHECK_EQ((Decimal::parse("1.25") + Decimal::parse("2.5")).str(), "3.75");
    CHECK_EQ((Decimal::parse("1.25") - Decimal(2)).str(), "-0.75");
    CHECK_EQ((Decimal::parse("19.99") * Decimal(3)).str(), "59.97");
    CHECK_EQ((Decimal(10) / Decimal(4)).str(), "2.5");
    CHECK_EQ((Decimal(1) / Decimal(3)).str(), "0.3333333333333333");
    CHECK_EQ((Decimal(2) / Decimal(3)).str(), "0.6666666666666667");
    CHECK_THROWS(InvalidError, Decimal(1) / Decimal(0));
    CHECK(Decimal::parse("1.50") == Decimal::parse("1.5"));
    CHECK(Decimal::parse("99.5") < Decimal::parse("1250.50"));
    CHECK_EQ(Decimal::parse("2.345").rescaled(2).str(), "2.35");
    CHECK_EQ(Decimal::parse("-2.345").rescaled(2).str(), "-2.35");
    // Money as SQL Server and the other databases keep it.
    CHECK_EQ(Decimal::parse("922337203685477.5807").str(), "922337203685477.5807");
}

TEST(dates_and_times_read_and_write_as_text) {
    CHECK_EQ(Date::parse("2026-10-17")->str(), "2026-10-17");
    CHECK(!Date::parse("2026-02-30"));
    CHECK(!Date::parse("17-10-2026"));
    CHECK_EQ(DateTime::parse("2026-10-17T09:30:15")->str(), "2026-10-17T09:30:15");
    CHECK_EQ(DateTime::parse("2026-10-17 09:30")->str(), "2026-10-17T09:30:00");
    CHECK(!DateTime::parse("2026-10-17T25:00:00"));
    CHECK(Date::parse("2026-01-02") < Date::parse("2026-10-01"));
    CHECK_EQ(compare(std::optional<Date>{}, Date::parse("2026-01-02")), -1);
    CHECK(today().year >= 2026);
}

TEST(text_travels_as_the_other_runtimes_send_it) {
    CHECK_EQ(nlohmann::json(Text::plain("Brace { yourself }")).dump(), "\"Brace { yourself }\"");
    auto since = Date::parse("2026-10-17");
    Text saved = Text("Saved {customer.name} on {when}: {total}, {paid}, {kind}")
                     .with("customer.name", slot_text("Ada"))
                     .with("when", slot_date(since, "long"))
                     .with("total", slot_decimal(Decimal::parse("1250.50"), 2))
                     .with("paid", slot_bool(true))
                     .with("kind", slot_enum("Business", "CustomerKind"));
    auto expected = nlohmann::json::parse(R"({"text":"Saved {customer.name} on {when}: {total}, {paid}, {kind}","values":{"customer.name":{"kind":"text","value":"Ada"},"kind":{"kind":"enum","value":"Business","enum":"CustomerKind"},"paid":{"kind":"bool","value":true},"total":{"kind":"decimal","value":"1250.50","scale":2},"when":{"kind":"date","value":"2026-10-17","format":"long"}}})");
    CHECK_EQ(nlohmann::json(saved), expected);
    CHECK_EQ(saved.show(), "Saved Ada on 2026-10-17: 1250.50, yes, Business");
    CHECK_EQ(nlohmann::json(saved).get<Text>().show(), saved.show());
    CHECK_EQ(Text("{{braces}} and }").show(), "{braces} and }");
    // A key travels with its slot values; the screen looks up its value.
    auto keyed = Text::key("orderApproved").with("number", slot_text("A-7"));
    CHECK_EQ(nlohmann::json(keyed), nlohmann::json::parse(R"({"key":"orderApproved","values":{"number":{"kind":"text","value":"A-7"}}})"));
    CHECK_EQ(nlohmann::json(Text::key("save")), nlohmann::json::parse(R"({"key":"save"})"));
    CHECK_EQ(*nlohmann::json(keyed).get<Text>().key(), std::string("orderApproved"));
}

TEST(every_rounding_case_rounds_as_every_runtime_does) {
    auto cases = conformance("rounding.json")["cases"];
    CHECK(!cases.empty());
    for (const auto& c : cases) {
        auto rounded = Decimal::parse(c["value"].get<std::string>()).rounded(c["places"].get<int>());
        CHECK(rounded == Decimal::parse(c["gives"].get<std::string>()));
    }
}

TEST(sin_and_cos_take_degrees_as_every_runtime_does) {
    auto angles = conformance("drawing.json")["angles"];
    CHECK(!angles.empty());
    for (const auto& c : angles) {
        auto degrees = Decimal::parse(c["degrees"].get<std::string>());
        CHECK(clarity::sin(degrees) == Decimal::parse(c["sin"].get<std::string>()));
        CHECK(clarity::cos(degrees) == Decimal::parse(c["cos"].get<std::string>()));
    }
    CHECK(units(Decimal::parse("40.5")) == 40.5);
}

// Text code makes is translated in the backend, in the language the screen
// picked, as every runtime does (conformance/translations.json, `code`).
TEST(code_texts_are_translated_as_every_runtime_does) {
    auto code = conformance("translations.json")["code"];
    load_translations(code["model"].dump());
    CHECK(!code["cases"].empty());
    for (const auto& c : code["cases"]) {
        set_language(c["language"].is_string() ? std::optional(c["language"].get<std::string>()) : std::nullopt);
        Text text = Text::key(c["key"].get<std::string>());
        for (const auto& [path, value] : c["values"].items()) text = text.with(path, slot_text(value.get<std::string>()));
        CHECK_EQ(text.show(), c["gives"].get<std::string>());
    }
    set_language(std::nullopt);
}
