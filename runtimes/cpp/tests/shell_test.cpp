// The service's commands, run without a window: the commands and JSON of
// the screen's wire (manual/project.run-build.md).

#include <functional>

#include "apply_steps.hpp"
#include "check.hpp"
#include "clarity/code.hpp"
#include "clarity/watch.hpp"
#include "engine/engine.hpp"
#include "shell/shell.hpp"
#include "clarity/sql.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

/// A record a handler keeps, for the undo step its run makes.
struct Memo {
    int64_t id = 0;
    std::string text;
    static constexpr const char* table = "memo";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"text"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(text)}; }
    static Memo from_values(int64_t id, const Row& v) { return {id, sql::get_text("text", v[0])}; }
};

/// What a long handler runs while it runs, as the person's Stop does.
std::function<void()> while_planning;

class Cards final : public shell::Windows {
public:
    WindowState open(Db&, const std::string& window, const json& with) override {
        if (window != "Card") throw InvalidError("there is no window " + window);
        return opened(window, {{"name", "Ada"}}, with);
    }
    std::optional<std::string> handler(const std::string& window, const std::string& event) override {
        if (window == "Card" && event == "open") return "opened";
        return std::nullopt;
    }
    // Draws the card's canvas anew: a dot per letter of its name.
    Reply redraw(Db&, Reply reply) override {
        if (reply.state.window != "Card") return reply;
        Ui ui(reply.state);
        ui.begin_drawing("badge");
        auto name = reply.state.data.value("name", std::string());
        for (size_t i = 0; i < name.size(); ++i) ui.draw("badge", {{"op", "circle"}});
        reply.state.parts = ui.reply(reply.state.window, reply.state.data).state.parts;
        return reply;
    }

    Reply dispatch(Db& db, const WindowState& state, const std::string& handler) override {
        Ui ui(state);
        if (handler == "remember") {
            Memo memo{0, "kept"};
            db.save(memo);
        }
        if (handler == "opened") ui.set_caption("title", Text::plain("Card of " + state.data["name"].get<std::string>()));
        if (handler == "save") refuse(Text("{name} is taken").with("name", slot_text("Ada")));
        if (handler == "fail") throw StorageError("locked");
        if (handler == "divide") (void)whole_div(1, 0);
        // A wizard's on next that sends it to its last step.
        if (handler == "jump") ui.set_step("wizard", 3);
        // A task added and chosen, as a New button does.
        if (handler == "add") {
            json data = state.data;
            data["tasks"].push_back({{"id", 4}, {"title", "Zebra"}});
            Reply added = ui.reply(state.window, data);
            added.state.selected["taskTable"] = 3;
            return added;
        }
        if (handler == "plan") {
            // As a long engine call does: it says how far it is until it is stopped.
            const EngineWatch* watch = current_watch();
            for (int64_t day = 1;; ++day) {
                watch->progress(day, 365);
                if (day == 3 && while_planning) while_planning();
                if (watch->stop->load()) throw EngineError("engine Planner failed in plan: it was stopped");
            }
        }
        return ui.reply(state.window, state.data);
    }
    /// Renames the card to the answer, after the values written for it.
    Reply answer(Db&, const WindowState& state, const std::string& handler, const json& values) override {
        if (handler != "rename") throw InvalidError("Card has no handler " + handler + " to answer");
        json data = state.data;
        data["name"] = values.back();
        return Ui(state).reply(state.window, data);
    }
    std::vector<RefOption> options(Db&, const std::string& entity) override {
        if (entity == "Artist") return {{1, "Jansen", {{"city", "Utrecht"}}}, {2, "Janssens", {{"city", "Gent"}}}};
        return {{1, "Ada"}};
    }
    std::vector<std::string> values(Db&, const std::string& entity, const std::string& field) override {
        if (entity == "Album" && field == "city") return {"Utrecht", "amsterdam", "", "Amstelveen", "Utrecht"};
        return {};
    }
    std::optional<Bytes> picture(Db&, const std::string& entity, int64_t id, const std::string& field) override {
        if (entity != "Card" || field != "photo") throw InvalidError(entity + " has no picture " + field);
        if (id == 1) return Bytes{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
        return std::nullopt;
    }
};

}  // namespace

TEST(the_shell_answers_the_renderers_commands) {
    Db db = Db::memory();
    Cards cards;
    shell::Service service({"shop", "Shop", "Card", R"({"project":{}})", &db, &cards});
    CHECK_EQ(service.invoke("start", json::object()), json::parse(R"({"value":{"model":{"project":{}},"main":"Card"}})"));
    json opened = service.invoke("open_window", {{"window", "Card"}, {"with", {{"name", "Grace"}}}});
    CHECK_EQ(opened["value"]["state"]["parts"]["title"]["caption"], json("Card of Grace"));
    // Its canvases are drawn anew, when it opens and after every handler.
    CHECK_EQ(opened["value"]["state"]["parts"]["badge"]["drawing"].size(), size_t(5));
    json again = service.invoke("dispatch_event", {{"state", {{"window", "Card"}, {"data", {{"name", "Ada"}}}}}, {"handler", "noop"}});
    CHECK_EQ(again["value"]["state"]["parts"]["badge"]["drawing"].size(), size_t(3));
    // A refusal keeps its slots; other errors are text.
    CHECK_EQ(service.invoke("dispatch_event", {{"state", {{"window", "Card"}, {"data", json::object()}}}, {"handler", "save"}}),
             json::parse(R"({"error":{"text":"{name} is taken","values":{"name":{"kind":"text","value":"Ada"}}}})"));
    CHECK_EQ(service.invoke("dispatch_event", {{"state", {{"window", "Card"}}}, {"handler", "fail"}}), json::parse(R"({"error":"storage: locked"})"));
    // A value error is said in Clarity's own words by its key, which the screen translates.
    CHECK_EQ(service.invoke("dispatch_event", {{"state", {{"window", "Card"}}}, {"handler", "divide"}}), json::parse(R"({"error":{"key":"divisionByZero","text":"Division by zero"}})"));
    // The answer to a question runs the handler it goes to.
    json answered = service.invoke("answer_event", {{"state", {{"window", "Card"}, {"data", {{"name", "Ada"}}}}}, {"handler", "rename"}, {"values", {7, "Grace"}}});
    CHECK_EQ(answered["value"]["state"]["data"]["name"], json("Grace"));
    CHECK_EQ(service.invoke("ref_options", {{"entity", "Customer"}}), json::parse(R"({"value":[{"id":1,"text":"Ada"}]})"));
    CHECK_EQ(service.invoke("picture", {{"entity", "Card"}, {"id", 1}, {"field", "photo"}}), json::parse(R"({"value":"data:image/png;base64,iVBORw0KGgo="})"));
    CHECK_EQ(service.invoke("picture", {{"entity", "Card"}, {"id", 2}, {"field", "photo"}}), json::parse(R"({"value":null})"));
    CHECK_EQ(service.invoke("nonsense", json::object()), json::parse(R"({"error":"there is no command nonsense"})"));
    CHECK_EQ(service.invoke("start_app", json::object()), json::parse(R"({"value":{"state":{"window":"","instance":0,"data":{},"parts":{},"selected":{}},"messages":[],"open":[],"close":false}})"));

    // A link of a markdown part opens in the browser: only a web or a mail address.
    std::string opened_link;
    service.open_link = [&](const std::string& address) {
        opened_link = address;
        return true;
    };
    CHECK_EQ(service.invoke("open_link", {{"address", "https://example.com/rules"}}), json::parse(R"({"value":null})"));
    CHECK_EQ(opened_link, "https://example.com/rules");
    CHECK_EQ(service.invoke("open_link", {{"address", "file:///etc/passwd"}}),
             json::parse(R"({"error":"a link opens only an http, https or mailto address, not file:///etc/passwd"})"));
    CHECK_EQ(opened_link, "https://example.com/rules");

    // An engine beside the screen logs through the shell: into the application's log.
    std::string logged;
    service.log_line = [&](const std::string& line) { logged = line; };
    CHECK_EQ(service.invoke("log_line", {{"line", "engine Screen: level 2"}}), json::parse(R"({"value":null})"));
    CHECK_EQ(logged, "engine Screen: level 2");

}

// A window that closes is reported once its own `on close` has run, so an
// engine with state per window drops what it had of it.
TEST(a_closed_window_is_reported) {
    Db db = Db::memory();
    Cards cards;
    shell::Service service({"shop", "Shop", "Card", R"({"project":{}})", &db, &cards});
    uint64_t closed = 0;
    auto listening = on_window_closed([&](uint64_t instance) { closed = instance; });
    auto state = service.invoke("open_window", {{"window", "Card"}, {"with", json::object()}})["value"]["state"];
    service.invoke("close_window", {{"state", state}});
    CHECK(state["instance"].get<uint64_t>() > 0);
    CHECK_EQ(closed, state["instance"].get<uint64_t>());
}

// The databases did not open (manual/data.settings.md): start says what to show and why,
// and connect opens them once the person has kept their connection.
TEST(an_unconnected_start_says_why_and_connects_later) {
    Db db = Db::memory();
    Cards cards;
    shell::Options options{"shop", "Shop", "Card", R"({"project":{}})", &db, &cards};
    options.unconnected = Text::key("unconnectedNotKept");
    options.unconnected_window = "Preferences";
    int tries = 0;
    options.connect = [&tries] {
        if (++tries == 1) throw StorageError("still not kept");
        return std::make_unique<Db>(Db::memory());
    };
    shell::Service service(options);
    CHECK_EQ(service.invoke("start", json::object()),
             json::parse(R"({"value":{"model":{"project":{}},"main":"Card","unconnected":{"handler":null,"window":"Preferences","problem":{"key":"unconnectedNotKept"}}}})"));
    CHECK_EQ(service.invoke("connect", json::object()),
             json::parse(R"({"error":{"key":"unconnectedFailed","values":{"problem":{"kind":"text","value":"still not kept"}}}})"));
    CHECK_EQ(service.invoke("connect", json::object()), json::parse(R"({"value":null})"));
    CHECK_EQ(service.invoke("start", json::object()), json::parse(R"({"value":{"model":{"project":{}},"main":"Card"}})"));
}

// Undo (manual/code.handlers.md): the runtime keeps the window's edits, each with the
// database steps its handler made, and Ctrl+Z takes them back together.
TEST(the_runtime_keeps_the_edits_and_ctrl_z_takes_them_back) {
    Db db = Db::memory();
    apply_steps(db, {{"CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"}});
    Cards cards;
    shell::Service service({"shop", "Shop", "Card", R"({"project":{}})", &db, &cards});
    auto card = [](const char* name) { return json{{"window", "Card"}, {"instance", 7}, {"data", {{"name", name}}}}; };
    // A handler that failed made no step.
    CHECK(!service.invoke("dispatch_event", {{"state", card("Ada")}, {"handler", "fail"}}).contains("value"));
    // A reply: the edits start from the window as it is.
    CHECK(!service.invoke("dispatch_event", {{"state", card("Ada")}, {"handler", "noop"}})["value"].contains("step"));
    // The person typed a name, then pressed a part whose handler saves: two edits of the form.
    service.invoke("edit_event", {{"state", card("Ann")}, {"part", "nameBox"}, {"scope", "form"}, {"ends", true}});
    json saved = service.invoke("edit_event", {{"state", card("Ann")}, {"handler", "remember"}, {"part", "saveBox"}, {"scope", "form"}, {"ends", true}});
    CHECK_EQ(saved["value"]["step"], json(1));
    CHECK_EQ(db.all<Memo>().size(), size_t(1));
    json back = {{"state", card("Ann")}, {"scope", "form"}, {"back", true}};
    CHECK(service.invoke("undo_edit", back).contains("value"));
    CHECK(db.all<Memo>().empty());
    CHECK_EQ(service.invoke("undo_edit", back)["value"]["state"]["data"], json::parse(R"({"name":"Ada"})"));
    json again = {{"state", card("Ada")}, {"scope", "form"}, {"back", false}};
    CHECK_EQ(service.invoke("undo_edit", again)["value"]["state"]["data"], json::parse(R"({"name":"Ann"})"));
    service.invoke("undo_edit", again);
    CHECK_EQ(db.all<Memo>().size(), size_t(1));
    // A closed window's edits go.
    service.invoke("close_window", {{"state", card("Ann")}});
    CHECK_EQ(service.invoke("undo_edit", back)["value"]["state"]["data"], json::parse(R"({"name":"Ann"})"));
    // The old command is gone.
    CHECK(service.invoke("undo_step", {{"state", card("Ann")}, {"step", 1}, {"back", true}}).contains("error"));
}

namespace {

/// A shell with the memo table a handler keeps a record in.
struct Running {
    Db db = Db::memory();
    Cards cards;
    shell::Service service{{"shop", "Shop", "Card", R"({"project":{}})", &db, &cards}};
    Running() { apply_steps(db, {{"CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"}}); }
};

json card_with(json data) { return {{"window", "Card"}, {"instance", 8}, {"data", std::move(data)}}; }

}  // namespace

// A canvas (manual/parts.graphics.md): the runtime moves an object to its grid and edits
// a drawing, chooses what it touched and runs on change, as one edit.
TEST(the_runtime_moves_an_object_and_edits_a_drawing) {
    Running app;
    json types = json::parse(R"([{"id":1,"x":0,"y":0},{"id":2,"x":40,"y":40}])");
    json state = card_with({{"types", types}, {"note", {{"drawing", ""}}}});
    app.service.invoke("dispatch_event", {{"state", state}, {"handler", "noop"}});
    json drop = {{"state", state}, {"part", "diagram"}, {"scope", "diagram"}, {"mode", "objects"}, {"list", "types"}, {"at", {"x", "y"}}, {"grid", 20}, {"snap", true},
                 {"action", {{"move", {{"id", 2}, {"x", 67}, {"y", 31}}}}}, {"handler", "remember"}};
    json moved = app.service.invoke("canvas_event", drop)["value"];
    CHECK_EQ(moved["state"]["data"]["types"], json::parse(R"([{"id":1,"x":0,"y":0},{"id":2,"x":60,"y":40}])"));
    CHECK_EQ(moved["state"]["selected"], json::parse(R"({"diagram":1})"));
    CHECK_EQ(moved["step"], json(1));
    CHECK_EQ(app.db.all<Memo>().size(), size_t(1));
    // Ctrl+Z takes the move back with its handler's step.
    json back = app.service.invoke("undo_edit", {{"state", moved["state"]}, {"scope", "diagram"}, {"back", true}})["value"];
    CHECK_EQ(back["state"]["data"]["types"], types);
    CHECK(app.db.all<Memo>().empty());
    // Dropped where it is: refused, running nothing.
    drop["action"] = {{"move", {{"id", 1}, {"x", 5}, {"y", -4}}}};
    json refused = app.service.invoke("canvas_event", drop)["value"];
    CHECK_EQ(refused["refused"], json(true));
    CHECK(!refused.contains("step"));
    CHECK(app.db.all<Memo>().empty());
    json add = {{"state", state}, {"part", "drawing"}, {"scope", "drawing"}, {"mode", "vector"}, {"path", "note.drawing"}, {"units", {400, 300}},
                {"action", {{"add", {{"op", "rect"}, {"x", 1}, {"y", 2}, {"width", 3}, {"height", 4}, {"color", "#ff0000"}}}}}};
    json drawn = app.service.invoke("canvas_event", add)["value"];
    CHECK_EQ(drawn["state"]["data"]["note"]["drawing"],
             json("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 300\">\n<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#ff0000\"/>\n</svg>\n"));
    CHECK_EQ(drawn["state"]["selected"]["drawing"], json(0));
    // A readonly canvas refuses every action.
    add["state"] = drawn["state"];
    add["readonly"] = true;
    json kept = app.service.invoke("canvas_event", add)["value"];
    CHECK_EQ(kept["refused"], json(true));
    CHECK_EQ(kept["state"]["data"]["note"], drawn["state"]["data"]["note"]);
}

// A wizard's Next, Back and Finish (manual/parts.containers.md), decided by the runtime.
TEST(the_runtime_walks_a_wizard) {
    Running app;
    auto on_step = [](int64_t step) {
        json state = card_with({{"name", "Ada"}});
        state["parts"] = {{"wizard", {{"step", step}}}};
        return state;
    };
    json next = {{"state", card_with({{"name", "Ada"}})}, {"part", "wizard"}, {"button", "next"}, {"steps", 3}, {"next", "remember"}, {"finish", "remember"}};
    json moved = app.service.invoke("step_event", next)["value"];
    CHECK_EQ(moved["state"]["parts"]["wizard"]["step"], json(2));
    CHECK_EQ(moved["step"], json(1));
    // A refusal keeps the step: the screen shows it and keeps its state.
    next["next"] = "save";
    CHECK(app.service.invoke("step_event", next).contains("error"));
    // A handler that sets the step decides where the wizard goes.
    next["next"] = "jump";
    CHECK_EQ(app.service.invoke("step_event", next)["value"]["state"]["parts"]["wizard"]["step"], json(3));
    // Next on the last step stays; Back runs nothing.
    next["next"] = "";
    next["state"] = on_step(3);
    CHECK_EQ(app.service.invoke("step_event", next)["value"]["state"]["parts"]["wizard"]["step"], json(3));
    json back = {{"state", on_step(3)}, {"part", "wizard"}, {"button", "back"}, {"steps", 3}, {"next", "remember"}};
    json went = app.service.invoke("step_event", back)["value"];
    CHECK_EQ(went["state"]["parts"]["wizard"]["step"], json(2));
    CHECK_EQ(app.db.all<Memo>().size(), size_t(1));
    // Finish runs on finish on the step shown.
    json finish = {{"state", on_step(3)}, {"part", "wizard"}, {"button", "finish"}, {"steps", 3}, {"finish", "remember"}};
    json finished = app.service.invoke("step_event", finish)["value"];
    CHECK_EQ(finished["state"]["parts"]["wizard"]["step"], json(3));
    CHECK_EQ(app.db.all<Memo>().size(), size_t(2));
    finish["button"] = "sideways";
    CHECK(app.service.invoke("step_event", finish).contains("error"));
}

// What a dropdown offers for typed text (U1c), found by the runtime.
TEST(the_runtime_finds_a_dropdowns_options) {
    Running app;
    // The records of the entity; the screen searches them as the person types.
    CHECK_EQ(app.service.invoke("find_options", {{"entity", "Artist"}, {"show", {"city"}}})["value"].size(), size_t(2));
}

// A readonly part (manual/start.how-it-works.md): whatever the screen changed is taken back to
// the window's data as the last reply left it, no handler runs, no edit is
// kept, and the reply says refused.
TEST(the_runtime_takes_back_a_change_of_a_readonly_part) {
    Running app;
    json tasks = json::parse(R"([{"id":1,"rank":1},{"id":2,"rank":2}])");
    json shown = {{"window", "Card"}, {"instance", 10}, {"data", {{"name", "Ada"}, {"volume", 5}, {"tags", {"jazz"}}, {"tasks", tasks}}}};
    app.service.invoke("dispatch_event", {{"state", shown}, {"handler", "noop"}});
    json typed = shown;
    typed["data"]["name"] = "Ann";
    std::vector<json> commands{
        {{"command", "edit_event"}, {"args", {{"state", typed}, {"part", "nameBox"}, {"scope", "form"}, {"ends", true}, {"handler", "remember"}, {"readonly", true}}}},
    };
    for (const auto& c : commands) {
        std::string command = c["command"];
        json reply = app.service.invoke(command, c["args"])["value"];
        if (reply["refused"] != json(true)) check::fail(__FILE__, __LINE__, command + ": not refused");
        if (reply["state"]["data"] != shown["data"]) check::fail(__FILE__, __LINE__, command + ": " + reply["state"]["data"].dump());
    }
    CHECK(app.db.all<Memo>().empty());
    // Nothing went on the window's edits.
    CHECK_EQ(app.service.invoke("undo_edit", {{"state", shown}, {"scope", "form"}, {"back", true}})["value"]["state"]["data"], shown["data"]);
    // Not readonly, the edit is taken.
    json taken = app.service.invoke("edit_event", {{"state", typed}, {"part", "nameBox"}, {"scope", "form"}, {"ends", true}, {"readonly", false}})["value"];
    CHECK(!taken.contains("refused"));
    CHECK_EQ(taken["state"]["data"]["name"], json("Ann"));
}

// A table's edit keeps to its editable columns: any other change of its rows is taken back.
TEST(the_runtime_takes_back_a_table_edit_outside_its_editable_fields) {
    Running app;
    json rows = json::parse(R"([{"id":1,"title":"Milk","done":false},{"id":2,"title":"Tea","done":false}])");
    json shown = {{"window", "Card"}, {"instance", 11}, {"data", {{"tasks", rows}}}};
    app.service.invoke("dispatch_event", {{"state", shown}, {"handler", "noop"}});
    auto edit = [&](const char* changed) {
        json state = shown;
        state["data"]["tasks"] = json::parse(changed);
        return app.service.invoke("edit_event", {{"state", state}, {"part", "taskTable"}, {"scope", "taskTable"}, {"ends", true}, {"handler", "remember"},
                                                 {"path", "tasks"}, {"editable", {"done"}}})["value"];
    };
    json outside = edit(R"([{"id":1,"title":"Oats","done":false},{"id":2,"title":"Tea","done":false}])");
    CHECK_EQ(outside["refused"], json(true));
    CHECK_EQ(outside["state"]["data"]["tasks"], rows);
    CHECK(app.db.all<Memo>().empty());
    json inside = edit(R"([{"id":1,"title":"Milk","done":true},{"id":2,"title":"Tea","done":false}])");
    CHECK(!inside.contains("refused"));
    CHECK_EQ(inside["state"]["data"]["tasks"][0]["done"], json(true));
    CHECK_EQ(app.db.all<Memo>().size(), size_t(1));
}
