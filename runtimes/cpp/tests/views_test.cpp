// The shown rows of a window's tables, trees and charts (DESIGN 1.7): the
// runtime fills each view of the window's state after every reply.

#include "check.hpp"
#include "clarity/views.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

WindowState window_with(json data, json views) {
    WindowState state;
    state.window = "Board";
    state.data = std::move(data);
    state.views = std::move(views);
    return state;
}

const json tasks = json::parse(R"([{"id":1,"title":"Milk","up":null},{"id":2,"title":"bread","up":1},{"id":3,"title":"Apples","up":2}])");

}  // namespace

TEST(a_table_view_shows_its_page_in_order) {
    auto state = window_with({{"tasks", tasks}}, json::parse(R"({"taskTable":{"kind":"table","list":"tasks","columns":[{"field":"title","kind":"text","choices":[]}],
        "order":{"field":"title","desc":false},"page":2,"at":9}})"));
    fill_views(state);
    json view = state.views["taskTable"];
    CHECK_EQ(view["shown"], json::parse("[0]"));
    CHECK_EQ(view["pages"], json::parse(R"({"page":2,"pages":2,"from":3,"to":3,"count":3})"));
    CHECK_EQ(view["at"], json(2));
    // A sort goes back to the first page; the same field again sorts the other way.
    view_action(state, "taskTable", {{"sort", "title"}});
    fill_views(state);
    CHECK_EQ(state.views["taskTable"]["order"], json::parse(R"({"field":"title","desc":true})"));
    CHECK_EQ(state.views["taskTable"]["shown"], json::parse("[0,1]"));
    // Without a page size every row shows, and there are no pages.
    state.views["taskTable"]["page"] = nullptr;
    fill_views(state);
    CHECK_EQ(state.views["taskTable"]["shown"], json::parse("[0,1,2]"));
    CHECK(!state.views["taskTable"].contains("pages"));
}

TEST(a_chosen_row_shows_its_page_and_its_parents_once) {
    auto state = window_with({{"tasks", tasks}}, json::parse(R"({"taskTable":{"kind":"table","list":"tasks","columns":[],"order":null,"page":1,"at":1},
        "taskTree":{"kind":"tree","list":"tasks","parent":"up","key":"id","open":[]}})"));
    state.selected = {{"taskTable", 2}, {"taskTree", 2}};
    fill_views(state);
    CHECK_EQ(state.views["taskTable"]["at"], json(3));
    CHECK_EQ(state.views["taskTree"]["open"], json::parse("[1,2]"));
    CHECK_EQ(state.views["taskTree"]["shown"], json::parse("[[0,0,true],[1,1,true],[2,2,false]]"));
    // The person turns away or closes a parent: in the reply to that, the chosen row the screen sent stays where it was.
    view_action(state, "taskTable", {{"page", 1}});
    view_action(state, "taskTree", {{"close", 1}});
    auto sent = state.selected;
    fill_views(state, &sent);
    CHECK_EQ(state.views["taskTable"]["at"], json(1));
    CHECK_EQ(state.views["taskTree"]["open"], json::parse("[2]"));
    CHECK_EQ(state.views["taskTree"]["shown"], json::parse("[[0,0,true]]"));
    view_action(state, "taskTree", {{"open", 1}});
    fill_views(state, &sent);
    CHECK_EQ(state.views["taskTree"]["shown"].size(), size_t(3));
    // Any other reply shows the chosen row: a move may have put it under a closed row.
    view_action(state, "taskTree", {{"close", 2}});
    fill_views(state);
    CHECK_EQ(state.views["taskTree"]["open"], json::parse("[1,2]"));
    CHECK_THROWS(InvalidError, view_action(state, "nothing", {{"page", 1}}));
    view_action(state, "nothing", json::object());
}

TEST(a_chart_view_shows_its_groups) {
    auto state = window_with({{"sales", json::parse(R"([{"city":"Gent","total":"2.5"},{"city":"Utrecht","total":4},{"city":"Gent","total":1}])")}},
                             json::parse(R"({"salesChart":{"kind":"chart","list":"sales","by":"city","byKind":"text","measure":"sum","of":"total","values":[]}})"));
    fill_views(state);
    CHECK_EQ(state.views["salesChart"]["groups"], json::parse(R"([{"key":"Gent","value":3.5},{"key":"Utrecht","value":4}])"));
}

// The views travel with the window's state, and only when it has some.
TEST(a_window_state_carries_its_views) {
    WindowState state = json::parse(R"({"window":"Board","views":{"t":{"kind":"table"}}})");
    CHECK_EQ(json(state)["views"], json::parse(R"({"t":{"kind":"table"}})"));
    CHECK(!json(WindowState{}).contains("views"));
    // A handler's reply keeps them.
    CHECK_EQ(Ui(state).reply("Board", json::object()).state.views, state.views);
}
