// The shell's commands, run without a window: the same commands and JSON as
// the Tauri and Wails shells.

#include "check.hpp"
#include "shell/shell.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

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

    Reply dispatch(Db&, const WindowState& state, const std::string& handler) override {
        Ui ui(state);
        if (handler == "opened") ui.set_caption("title", Text::plain("Card of " + state.data["name"].get<std::string>()));
        if (handler == "save") refuse(Text("{name} is taken").with("name", slot_text("Ada")));
        if (handler == "fail") throw StorageError("locked");
        return ui.reply(state.window, state.data);
    }
    std::vector<RefOption> options(Db&, const std::string&) override { return {{1, "Ada"}}; }
    std::optional<Bytes> picture(Db&, const std::string& entity, int64_t id, const std::string& field) override {
        if (entity != "Card" || field != "photo") throw InvalidError(entity + " has no picture " + field);
        if (id == 1) return Bytes{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
        return std::nullopt;
    }
};

class Fake final : public shell::Desktop {
public:
    std::string title;
    bool closed = false;
    std::string name() const override { return "main"; }
    void set_title(const std::string& t) override { title = t; }
    void close() override { closed = true; }
};

}  // namespace

TEST(the_shell_answers_the_renderers_commands) {
    Db db = Db::memory();
    Cards cards;
    shell::Service service({"shop", "Shop", "Card", R"({"project":{}})", "", &db, &cards});
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

    // The window a command came from takes its title, and a window of its own starts where it was opened.
    Fake main;
    service.invoke("set_title", {{"title", "Customer card"}}, &main);
    CHECK_EQ(main.title, "Customer card");
    std::string opened_name;
    shell::Size opened_size{};
    service.open_desktop = [&](shell::Desktop&, const std::string& name, shell::Size size) {
        opened_name = name;
        opened_size = size;
    };
    service.invoke("open_apart", {{"window", "Card"}, {"with", {{"name", "Linus"}}}}, &main);
    CHECK_EQ(opened_name, "apart-1");
    // Without a size from the renderer, the standard one; with one, that.
    CHECK_EQ(opened_size.width, 1100);
    CHECK_EQ(opened_size.height, 760);
    service.invoke("open_apart", {{"window", "Card"}, {"with", json::object()}, {"width", 320}, {"height", 460}}, &main);
    CHECK_EQ(opened_size.width, 320);
    CHECK_EQ(opened_size.height, 460);
    CHECK(service.closed("apart-1") == std::optional<std::string>("main"));
    service.invoke("close_apart", json::object(), &main);
    CHECK(main.closed);
}

// A window that closes is reported once its own `on close` has run, so an
// engine with state per window drops what it had of it.
TEST(a_closed_window_is_reported) {
    Db db = Db::memory();
    Cards cards;
    shell::Service service({"shop", "Shop", "Card", R"({"project":{}})", "", &db, &cards});
    uint64_t closed = 0;
    auto listening = on_window_closed([&](uint64_t instance) { closed = instance; });
    auto state = service.invoke("open_window", {{"window", "Card"}, {"with", json::object()}})["value"]["state"];
    service.invoke("close_window", {{"state", state}});
    CHECK(state["instance"].get<uint64_t>() > 0);
    CHECK_EQ(closed, state["instance"].get<uint64_t>());
}
