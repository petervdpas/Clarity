// The binding between a window on the screen and its handlers: the state
// the screen sends, and the reply it gets, in the same JSON as from the
// other runtimes.

#include "check.hpp"
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
    CHECK_EQ(json(reply), json::parse(R"({"state":{"window":"Card","data":{"order":{"id":7,"number":"ORD-8"},"orders":[{"id":1,"number":"A"},{"id":2,"number":"B"}]},"parts":{"mine":{"checked":true},"name":{"error":"Name is required","readonly":false},"name_filter":{"text":""},"save":{"enabled":false}},"selected":{"grid":1}},"messages":[{"level":"info","text":{"text":"Saved {n}","values":{"n":{"kind":"text","value":"ORD-7"}}}}],"open":[{"window":"Order","with":{"order":{"id":7,"number":"ORD-7"}}}],"close":true})"));
    // A new window's state has no parts and no selected rows yet.
    CHECK_EQ(json(WindowState{"Card"}).dump(), R"({"data":{},"parts":{},"selected":{},"window":"Card"})");
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
