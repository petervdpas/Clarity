// The edit bus (DESIGN 5.29) against the cases every runtime shares
// (conformance/edit-bus.json).

#include "check.hpp"
#include "clarity/edits.hpp"
#include "clarity/errors.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(the_edit_bus_keeps_the_record_as_every_runtime_does) {
    for (const auto& c : conformance("edit-bus.json")["cases"]) {
        std::string name = c["name"];
        EditBus bus;
        json data = c["start"];
        bus.reset(data);
        int at = 0;
        for (const auto& s : c["steps"]) {
            ++at;
            std::string where = name + ", step " + std::to_string(at);
            auto steps_of = [](const json& made) { return made.value("steps", std::vector<int64_t>{}); };
            if (s.contains("edit")) {
                const auto& made = s["edit"];
                data = made["data"];
                bus.edit(made["part"], made["scope"], data, steps_of(made));
                continue;
            }
            if (s.contains("drag")) {
                const auto& made = s["drag"];
                data = made["data"];
                bus.drag(made["part"], made["scope"], steps_of(made));
                continue;
            }
            if (s.contains("reply")) {
                data = s["reply"]["data"];
                bus.reset(data);
                continue;
            }
            bool back = s.contains("undo");
            std::string scope = s[back ? "undo" : "redo"];
            bool refuse = s.contains("refused");
            std::vector<int64_t> taken;
            auto take = [&](const std::vector<int64_t>& steps, bool) {
                if (refuse) throw StorageError("changed since");
                taken = steps;
            };
            if (refuse) {
                CHECK_THROWS(StorageError, bus.undo(scope, data, back, take));
                continue;
            }
            auto after = bus.undo(scope, data, back, take);
            if (s.contains("nothing")) {
                if (after) check::fail(__FILE__, __LINE__, where + ": something to take back");
                continue;
            }
            if (!after) {
                check::fail(__FILE__, __LINE__, where + ": nothing taken back");
                continue;
            }
            if (*after != s["data"]) check::fail(__FILE__, __LINE__, where + ": data " + after->dump());
            if (taken != steps_of(s)) check::fail(__FILE__, __LINE__, where + ": steps differ");
            data = *after;
        }
    }
}

// A part's own text and tick travel with the window's data, and come back from it.
TEST(a_parts_own_text_and_tick_are_edit_data) {
    WindowState state = json::parse(R"({"window":"Card","data":{"name":"Ada"},"parts":{"pinBox":{"text":"12"},"okBox":{"checked":true},"title":{}}})");
    CHECK_EQ(edit_data(state), json::parse(R"({"name":"Ada","#parts.pinBox":{"text":"12"},"#parts.okBox":{"checked":true}})"));
    WindowState back = with_edit_data(state, json::parse(R"({"name":"Ann","#parts.pinBox":{"text":""}})"));
    CHECK_EQ(back.data, json::parse(R"({"name":"Ann"})"));
    CHECK_EQ(back.parts["pinBox"].text, std::optional<std::string>(""));
    CHECK(!back.parts["okBox"].checked);
    CHECK(back.parts.contains("title"));
}

// Where a refused change goes back to: the window's data as the last reply left it.
TEST(the_edit_bus_gives_its_baseline) {
    EditBus bus;
    CHECK(!bus.baseline());
    bus.reset(json::parse(R"({"name":"Ada"})"));
    CHECK_EQ(bus.baseline().value_or(json()), json::parse(R"({"name":"Ada"})"));
}

// A table's edit keeps to its editable fields: the same rows in the same order, every other field as it was.
TEST(a_table_edit_fits_its_editable_fields) {
    json before = json::parse(R"([{"id":1,"title":"Milk","done":false},{"id":2,"title":"Tea","done":false}])");
    std::vector<std::string> editable{"done"};
    CHECK(table_edit_fits(before, json::parse(R"([{"id":1,"title":"Milk","done":true},{"id":2,"title":"Tea","done":false}])"), editable));
    CHECK(!table_edit_fits(before, json::parse(R"([{"id":1,"title":"Oats","done":false},{"id":2,"title":"Tea","done":false}])"), editable));
    CHECK(!table_edit_fits(before, json::parse(R"([{"id":2,"title":"Tea","done":false},{"id":1,"title":"Milk","done":false}])"), editable));
    CHECK(!table_edit_fits(before, json::parse(R"([{"id":1,"title":"Milk","done":false}])"), editable));
    // A new or a removed field is a change of it.
    CHECK(!table_edit_fits(before, json::parse(R"([{"id":1,"title":"Milk","done":false,"x":1},{"id":2,"title":"Tea","done":false}])"), editable));
    CHECK(!table_edit_fits(before, json::parse(R"([{"id":1,"done":false},{"id":2,"title":"Tea","done":false}])"), editable));
    CHECK(table_edit_fits(json::parse("[1,2]"), json::parse("[1,2]"), {}));
    CHECK(!table_edit_fits(json::parse("[1,2]"), json::parse("[1,3]"), {}));
}

// The edit bus on whole window states (conformance/edit-bus.json, `states`):
// what code set on parts is compared and put back as data is.
TEST(the_edit_bus_puts_back_what_code_set_on_parts) {
    const auto& cases = conformance("edit-bus.json")["states"];
    CHECK(!cases.empty());
    auto state_of = [](json given) {
        given["window"] = "Case";
        return given.get<WindowState>();
    };
    for (const auto& c : cases) {
        std::string name = c["name"];
        EditBus bus;
        WindowState state = state_of(c["start"]);
        bus.reset(edit_data(state));
        int at = 0;
        for (const auto& s : c["steps"]) {
            ++at;
            std::string where = name + ", step " + std::to_string(at);
            if (s.contains("edit")) {
                const auto& made = s["edit"];
                state = state_of(made["state"]);
                bus.edit(made["part"], made["scope"], edit_data(state), {});
                continue;
            }
            if (s.contains("reply")) {
                state = state_of(s["reply"]["state"]);
                bus.reset(edit_data(state));
                continue;
            }
            bool back = s.contains("undo");
            auto after = bus.undo(s[back ? "undo" : "redo"], edit_data(state), back, [](const std::vector<int64_t>&, bool) {});
            if (!after) {
                check::fail(__FILE__, __LINE__, where + ": nothing taken back");
                continue;
            }
            state = with_edit_data(state, *after);
            json got = state;
            const auto& want = s["state"];
            if (got["data"] != want["data"]) check::fail(__FILE__, __LINE__, where + ": data " + got["data"].dump());
            for (const auto& [part, fields] : want["parts"].items()) {
                json has = got["parts"].value(part, json::object());
                for (const auto& [field, expected] : fields.items()) {
                    if (field == "values") {
                        json values = has.value("values", json::object());
                        for (const auto& [value, v] : expected.items())
                            if (values.value(value, json()) != v) check::fail(__FILE__, __LINE__, where + ": " + part + ".values." + value + " in " + has.dump());
                        continue;
                    }
                    if (has.value(field, json()) != expected) check::fail(__FILE__, __LINE__, where + ": " + part + "." + field + " in " + has.dump());
                }
            }
        }
    }
}
