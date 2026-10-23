// The binding between a window on the screen and its handlers: the state
// the screen sends, and the reply it gets, in the same JSON as from the
// other runtimes.

#include "check.hpp"
#include "conformance.hpp"
#include "clarity/ui.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

struct Order {
    int64_t id = 0;
    std::string number;
};

void to_json(json& out, const Order& o) { out = {{"id", o.id}, {"number", o.number}}; }
void from_json(const json& in, Order& o) { o = {in.value("id", int64_t(0)), in.value("number", "")}; }

}  // namespace

TEST(a_handler_replies_as_the_other_runtimes_do) {
    WindowState state = json::parse(R"({"window":"Card","data":{"order":{"id":7,"number":"ORD-7"},"orders":[{"id":1,"number":"A"},{"id":2,"number":"B"}]},"parts":{"name_filter":{"text":"Ada"},"mine":{"checked":true}},"selected":{"grid":1}})").get<WindowState>();
    Order order = state.data["order"].get<Order>();
    auto orders = state.data["orders"].get<std::vector<Order>>();
    CHECK_EQ(order.number, "ORD-7");

    Ui ui(state);
    CHECK_EQ(ui.text("name_filter"), "Ada");
    CHECK_EQ(ui.text("other"), "");
    CHECK(ui.checked("mine"));
    CHECK(ui.visible("save"));
    CHECK(ui.readonly("name", true));
    CHECK_EQ(ui.caption("save", "Save"), "Save");
    CHECK_EQ(ui.selected("grid", orders).number, "B");
    CHECK_THROWS(InvalidError, ui.selected("other", orders));

    ui.set_enabled("save", false);
    ui.set_error("name", Text::plain("Name is required"));
    ui.set_readonly("name", false);
    ui.set_text("name_filter", "");
    ui.message(Level::Info, Text("Saved {n}").with("n", slot_text("ORD-7")));
    ui.open("Order", {{"order", order}});
    ui.close();
    order.number = "ORD-8";
    Reply reply = ui.reply("Card", {{"order", order}, {"orders", orders}});
    CHECK_EQ(json(reply), json::parse(R"({"state":{"window":"Card","instance":0,"data":{"order":{"id":7,"number":"ORD-8"},"orders":[{"id":1,"number":"A"},{"id":2,"number":"B"}]},"parts":{"mine":{"checked":true},"name":{"error":"Name is required","readonly":false},"name_filter":{"text":""},"save":{"enabled":false}},"selected":{"grid":1}},"messages":[{"level":"info","text":{"text":"Saved {n}","values":{"n":{"kind":"text","value":"ORD-7"}}}}],"open":[{"window":"Order","with":{"order":{"id":7,"number":"ORD-7"}}}],"close":true})"));
    // A new window's state has no parts and no selected rows yet.
    CHECK_EQ(json(WindowState{"Card"}).dump(), R"({"data":{},"instance":0,"parts":{},"selected":{},"window":"Card"})");
}

// A text value a handler sets travels as the text as written with its slots,
// so the screen shows it in the person's language and format.
TEST(a_reply_carries_the_texts_shown_in_text_values) {
    Ui ui(WindowState{"Game", json::object(), {}, {}, 0});
    CHECK_EQ(json(ui.reply("Game", json::object())).contains("shown"), false);
    ui.set_shown("rules", Text("Your move, {name}").with("name", slot_text("Ada")));
    ui.set_shown("hint", Text::plain("Pick a square"));
    CHECK_EQ(json(ui.reply("Game", json::object()))["shown"], json::parse(R"({"rules":{"text":"Your move, {name}","values":{"name":{"kind":"text","value":"Ada"}}},"hint":"Pick a square"})"));
}

TEST(a_question_travels_in_the_reply_as_from_the_other_runtimes) {
    Ui ui(WindowState{"Albums", json::object(), {}, {}, 0});
    CHECK_EQ(json(ui.reply("Albums", json::object())).contains("asks"), false);
    ui.ask(Text::key("deleteAlbum").with("title", slot_text("Blue")), ask_confirm(Text::key("delete"), std::nullopt), "deleteConfirmed", json::array());
    ui.ask(Text::key("lendTo"), ask_value("record", "Friend", json::array({3, 5}), nullptr), "lend", json::array({json::parse(R"({"id":7,"title":"Blue"})")}));
    CHECK_EQ(json(ui.reply("Albums", json::object()))["asks"], json::parse(R"([
        {"question":{"key":"deleteAlbum","values":{"title":{"kind":"text","value":"Blue"}}},"form":{"kind":"confirm","yes":{"key":"delete"},"no":null},"handler":"deleteConfirmed","values":[]},
        {"question":{"key":"lendTo"},"form":{"kind":"value","type":"record","of":"Friend","only":[3,5],"default":null},"handler":"lend","values":[{"id":7,"title":"Blue"}]}])"));
}

TEST(a_dialog_is_asked_for_and_closes_with_its_value) {
    Ui ui(WindowState{"Albums", json::object(), {}, {}, 0});
    ui.ask(Text::plain(""), ask_window("PickFriend", {{"friends", json::array()}}), "lend", json::array());
    json reply = ui.reply("Albums", json::object());
    CHECK_EQ(reply["asks"][0]["form"], json::parse(R"({"kind":"window","window":"PickFriend","with":{"friends":[]}})"));
    CHECK_EQ(reply.contains("result"), false);
    Ui dialog(WindowState{"PickFriend", json::object(), {}, {}, 0});
    dialog.close_with(json::parse(R"({"id":3,"name":"Ada"})"));
    json closed = dialog.reply("PickFriend", json::object());
    CHECK_EQ(closed["close"], json(true));
    CHECK_EQ(closed["result"], json::parse(R"({"id":3,"name":"Ada"})"));
}

TEST(rows_read_again_have_no_selected_row) {
    WindowState state = json::parse(R"({"window":"Orders","data":{},"parts":{},"selected":{"grid":1,"other":0}})").get<WindowState>();
    Ui ui(state);
    std::vector<Order> orders{{1, "A"}, {2, "B"}};
    ui.deselect("grid");
    CHECK_THROWS(InvalidError, ui.selected("grid", orders));
    CHECK_EQ(ui.selected("other", orders).number, "A");
    CHECK_EQ(json(ui.reply("Orders", json::object()).state.selected), json::parse(R"({"other":0})"));
}

TEST(a_window_opens_with_its_data_and_with) {
    WindowState state = opened("Card", {{"order", Order{0, "new"}}}, {{"order", {{"id", 7}, {"number", "ORD-7"}}}});
    CHECK_EQ(state.data["order"].get<Order>().number, "ORD-7");
    CHECK_EQ(json(Decimal::parse("12.50")), json("12.50"));
    CHECK(json("12.50").get<Decimal>() == Decimal::parse("12.5"));
    CHECK_EQ(json(*Date::parse("2026-10-17")), json("2026-10-17"));
    CHECK_THROWS(InvalidError, json("17-10-2026").get<Date>());
}

TEST(a_draw_starts_an_empty_drawing_that_travels_with_the_part) {
    Ui ui(WindowState{"Main", json::object(), {}, {}});
    ui.begin_drawing("dial");
    ui.draw("dial", {{"op", "clear"}, {"color", "#101810"}});
    ui.draw("dial", {{"op", "circle"}, {"r", 40.5}});
    auto reply = ui.reply("Main", json::object());
    CHECK_EQ(json(reply.state)["parts"]["dial"]["drawing"], json::parse(R"([{"op":"clear","color":"#101810"},{"op":"circle","r":40.5}])"));
    // The next draw starts anew.
    Ui again(reply.state);
    again.begin_drawing("dial");
    CHECK_EQ(json(again.reply("Main", json::object()).state)["parts"]["dial"]["drawing"], json::array());
}

TEST(a_canvas_says_where_it_was_clicked_as_every_runtime_does) {
    auto points = conformance("drawing.json")["points"];
    CHECK(!points.empty());
    for (const auto& c : points) {
        WindowState state = json{{"window", "Main"}, {"data", json::object()}, {"parts", {{"board", c["part"]}}}}.get<WindowState>();
        Ui ui(state);
        CHECK_EQ(ui.x("board"), c["x"].get<int64_t>());
        CHECK_EQ(ui.y("board"), c["y"].get<int64_t>());
        // What the pointer holds besides x and y travels back unchanged.
        CHECK_EQ(json(ui.reply("Main", json::object()).state)["parts"]["board"], c["part"]);
    }
}

TEST(a_foldable_part_is_folded_as_every_runtime_says) {
    auto file = conformance("folding.json");
    CHECK(!file["cases"].empty());
    for (const auto& c : file["cases"]) {
        Ui ui(json{{"window", "Main"}, {"data", json::object()}, {"parts", {{"details", c["part"]}}}}.get<WindowState>());
        CHECK_EQ(ui.folded("details", c["design"].get<bool>()), c["folded"].get<bool>());
    }
    for (const auto& c : file["set"]) {
        Ui ui(WindowState{"Main", json::object(), {}, {}});
        ui.set_folded("details", c["folded"].get<bool>());
        CHECK_EQ(json(ui.reply("Main", json::object()).state)["parts"]["details"], c["part"]);
    }
}

TEST(a_sidebar_is_collapsed_and_maximized_as_every_runtime_says) {
    auto file = conformance("sidebars.json");
    auto with = [](const json& part) { return Ui(json{{"window", "Main"}, {"data", json::object()}, {"parts", {{"nav", part}}}}.get<WindowState>()); };
    for (const auto& c : file["collapsed"]) CHECK_EQ(with(c["part"]).collapsed("nav", c["design"].get<bool>()), c["collapsed"].get<bool>());
    for (const auto& c : file["maximized"]) CHECK_EQ(with(c["part"]).maximized("nav"), c["maximized"].get<bool>());
    for (const auto& c : file["set"]) {
        Ui ui(WindowState{"Main", json::object(), {}, {}});
        if (c.contains("collapsed")) ui.set_collapsed("nav", c["collapsed"].get<bool>());
        if (c.contains("maximized")) ui.set_maximized("nav", c["maximized"].get<bool>());
        CHECK_EQ(json(ui.reply("Main", json::object()).state)["parts"]["nav"], c["part"]);
    }
}

TEST(a_wizards_step_is_as_every_runtime_says) {
    auto file = conformance("steps.json");
    for (const auto& c : file["step"]) {
        Ui ui(json{{"window", "Main"}, {"data", json::object()}, {"parts", {{"wizard", c["part"]}}}}.get<WindowState>());
        CHECK_EQ(ui.step("wizard"), c["step"].get<int64_t>());
    }
    for (const auto& c : file["set"]) {
        Ui ui(WindowState{"Main", json::object(), {}, {}});
        ui.set_step("wizard", c["step"].get<int64_t>());
        CHECK_EQ(json(ui.reply("Main", json::object()).state)["parts"]["wizard"], c["part"]);
    }
}

TEST(a_custom_parts_values_and_hears_are_as_every_runtime_says) {
    auto file = conformance("custom-parts.json");
    for (const auto& c : file["read"]) {
        Ui ui(json{{"window", "Main"}, {"data", json::object()}, {"parts", {{"arena", c["part"]}}}}.get<WindowState>());
        CHECK_EQ(ui.part_value("arena", c["name"].get<std::string>()).value_or(json(nullptr)), c["value"]);
    }
    for (const auto& c : file["set"]) {
        Ui ui(WindowState{"Main", json::object(), {}, {}});
        for (const auto& set : c["sets"]) ui.set_part_value("arena", set[0].get<std::string>(), set[1]);
        CHECK_EQ(json(ui.reply("Main", json::object()).state)["parts"]["arena"], c["part"]);
    }
    for (const auto& c : file["hear"]) {
        Ui ui(WindowState{"Main", json::object(), {}, {}});
        for (const auto& hear : c["hears"]) ui.hear(hear[0].get<std::string>(), hear[1].get<std::string>(), hear[2]);
        json reply = ui.reply("Main", json::object());
        CHECK_EQ(reply.contains("hears") ? reply["hears"] : json(nullptr), c["reply"]);
    }
}

TEST(an_accordions_open_section_is_as_every_runtime_says) {
    auto file = conformance("accordions.json");
    for (const auto& c : file["open"]) {
        Ui ui(json{{"window", "Main"}, {"data", json::object()}, {"parts", {{"faq", c["part"]}}}}.get<WindowState>());
        CHECK_EQ(ui.open_section("faq"), c["open"].get<int64_t>());
    }
    for (const auto& c : file["set"]) {
        Ui ui(WindowState{"Main", json::object(), {}, {}});
        ui.set_open_section("faq", c["open"].get<int64_t>());
        CHECK_EQ(json(ui.reply("Main", json::object()).state)["parts"]["faq"], c["part"]);
    }
}

TEST(a_ribbons_open_tab_is_as_every_runtime_says) {
    auto file = conformance("ribbons.json");
    for (const auto& c : file["tab"]) {
        Ui ui(json{{"window", "Main"}, {"data", json::object()}, {"parts", {{"mainRibbon", c["part"]}}}}.get<WindowState>());
        CHECK_EQ(ui.tab("mainRibbon"), c["tab"].get<int64_t>());
    }
    for (const auto& c : file["set"]) {
        Ui ui(WindowState{"Main", json::object(), {}, {}});
        ui.set_tab("mainRibbon", c["tab"].get<int64_t>());
        CHECK_EQ(json(ui.reply("Main", json::object()).state)["parts"]["mainRibbon"], c["part"]);
    }
}

// Every window that opens has a number of its own, which its handlers'
// replies keep, so what belongs to one open window (an engine's state) is
// found again and dropped when it closes.
TEST(every_open_window_has_its_own_number) {
    auto first = opened("Orders", json::object(), json::object());
    auto second = opened("Orders", json::object(), json::object());
    CHECK(first.instance > 0 && second.instance > 0 && first.instance != second.instance);
    Ui ui(first);
    CHECK_EQ(ui.instance(), first.instance);
    CHECK_EQ(ui.reply("Orders", json::object()).state.instance, first.instance);
    // The number travels with the state to the screen and back.
    CHECK_EQ(json(first).get<WindowState>().instance, first.instance);
}

TEST(a_closed_window_is_reported_to_what_listens) {
    uint64_t heard = 0;
    auto listening = on_window_closed([&](uint64_t instance) { heard = instance; });
    window_closed(7);
    CHECK_EQ(heard, uint64_t(7));
    listening.reset();
    window_closed(8);
    CHECK_EQ(heard, uint64_t(7));
}
