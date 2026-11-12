// Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4) on the
// C++ runtime: signing in and out through Clarity.Auth as every runtime does
// (conformance/signin.json), the program's start, the wire's commands, the
// served session cookie and the command line.

#include <cstdlib>
#include <map>

#include "auth/auth.hpp"
#include "check.hpp"
#include "clarity/cli.hpp"
#include "clarity/settings.hpp"
#include "clarity/sql.hpp"
#include "conformance.hpp"
#include "shell/serve.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

/// The table of people of the cases, as the generator writes a record.
struct Member {
    int64_t id = 0;
    std::string name, email, login, password_hash, api_token, recovery_codes;
    static constexpr const char* table = "members";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> all{"name", "email", "login", "password_hash", "api_token", "recovery_codes"};
        return all;
    }
    std::vector<SqlValue> values() const { return {sql::text(name), sql::text(email), sql::text(login), sql::text(password_hash), sql::text(api_token), sql::text(recovery_codes)}; }
    static Member from_values(int64_t id, const Row& row) {
        auto text = [&](size_t at) { return std::holds_alternative<std::string>(row.at(at)) ? std::get<std::string>(row.at(at)) : std::string(); };
        return {id, text(0), text(1), text(2), text(3), text(4), text(5)};
    }
    /// The field of a column, as a save step names it.
    std::string& field(const std::string& column) {
        if (column == "name") return name;
        if (column == "email") return email;
        if (column == "login") return login;
        if (column == "password_hash") return password_hash;
        if (column == "api_token") return api_token;
        return recovery_codes;
    }
};

/// A value of a case's row: text, or {hash, text} stored as that hash.
std::string stored_value(const json& value) { return value.is_object() ? hashed(value["hash"], value["text"]) : value.get<std::string>(); }

SigninSource source_of(const json& block, const json& people) {
    SigninSource source;
    source.table = people["table"];
    source.key = people["key"];
    source.page = block["page"];
    source.targets = block["targets"].get<std::vector<std::string>>();
    for (const auto& s : block["strategies"]) source.strategies.push_back({s["name"], s["columns"].get<std::vector<std::string>>()});
    source.session_seconds = block["sessionMinutes"].get<int64_t>() * 60;
    source.tries = block["tries"];
    source.tries_seconds = block["triesMinutes"].get<int64_t>() * 60;
    source.program = block["program"];
    return source;
}

/// A database with the cases' table of people and these rows, a value {hash, text} stored as that hash.
Db people_with(const json& rows) {
    Db db = Db::memory();
    const auto& c = conformance("signin.json");
    db.batch("", c["people"]["create"]);
    for (const auto& row : rows) {
        std::string names, values;
        for (const auto& [column, value] : row.items()) {
            std::string text = stored_value(value);
            std::string quoted;
            for (char ch : text) quoted += ch == '\'' ? std::string("''") : std::string(1, ch);
            names += (names.empty() ? "" : ", ") + column;
            values += (values.empty() ? "'" : ", '") + quoted + "'";
        }
        db.batch("", "INSERT INTO members (" + names + ") VALUES (" + values + ")");
    }
    return db;
}

/// The key of a refusal, or empty when it ran.
template <typename F>
std::string refusal(F run) {
    try {
        run();
    } catch (const RefusedError& e) {
        return e.text.key().value_or(e.text.text());
    }
    return "";
}

/// The session a Set-Cookie value sets; empty when it clears it.
std::string cookie_from(const std::string& set_cookie) {
    auto token = session_of(set_cookie.substr(0, set_cookie.find(';')));
    return token ? std::string(session_cookie) + "=" + *token : "";
}

struct Nothing : shell::Windows {
    WindowState open(Db&, const std::string& window, const json&) override {
        WindowState state;
        state.window = window;
        return state;
    }
    std::optional<std::string> handler(const std::string&, const std::string&) override { return std::nullopt; }
    Reply dispatch(Db&, const WindowState&, const std::string&) override { return {}; }
    Reply answer(Db&, const WindowState&, const std::string&, const json&) override { return {}; }
    Reply redraw(Db&, Reply reply) override { return reply; }
    std::vector<RefOption> options(Db&, const std::string&) override { return {}; }
    std::optional<Bytes> picture(Db&, const std::string&, int64_t, const std::string&) override { return std::nullopt; }
};

std::string post(const std::string& body, const std::string& cookie = "") {
    return "POST /clarity/invoke HTTP/1.1\r\nHost: x\r\n" + (cookie.empty() ? "" : "Cookie: theme=dark; " + cookie + "\r\n") + "Content-Length: " + std::to_string(body.size()) + "\r\n\r\n" + body;
}

json body_of(const std::string& reply) { return json::parse(reply.substr(reply.find("\r\n\r\n") + 4)); }

std::string set_cookie_of(const std::string& reply) {
    auto at = reply.find("Set-Cookie: ");
    if (at == std::string::npos || at > reply.find("\r\n\r\n")) return "";
    return reply.substr(at + 12, reply.find("\r\n", at) - at - 12);
}

/// A terminal that answers with a line of its own and keeps what was written.
struct Typed : cli::Terminal {
    std::optional<std::string> secret;
    std::string written;
    bool hidden = false;
    bool is_terminal() override { return true; }
    std::optional<std::string> read_line() override { return std::nullopt; }
    std::optional<std::string> read_secret() override {
        hidden = true;
        return secret;
    }
    void write(const std::string& text) override { written += text; }
};

}  // namespace

TEST(signin_signs_people_in_as_every_stack_does) {
    const auto& c = conformance("signin.json");
    for (const auto& one : c["cases"]) {
        std::string name = one["name"];
        // remember N days keeps its entry in the secret store, here in memory, empty per case.
        SigninSource source = source_of(c["signin"], c["people"]);
        source.remember_seconds = one.value("rememberDays", int64_t{0}) * 24 * 3600;
        // remember N days when asked keeps it only when the input remember is "true".
        source.remember_asks = one.value("rememberAsks", false);
        auto kept = std::make_shared<std::map<std::string, std::string>>();
        secret_store.get = [kept](const std::string& program, const std::string& key) -> std::optional<std::string> {
            auto found = kept->find(program + ":" + key);
            return found == kept->end() ? std::nullopt : std::optional(found->second);
        };
        secret_store.set = [kept](const std::string& program, const std::string& key, const std::string& value) { (*kept)[program + ":" + key] = value; };
        Db db = people_with(one["rows"]);
        db.add_signin(source);
        bool serving = one.value("served", false);
        std::string address = one.value("address", "");
        // Served, each browser carries the session cookie the program set it last.
        std::map<std::string, std::string> cookies;
        auto as_browser = [&](const json& s, const std::string& command, const json& args, const std::function<std::string()>& run) {
            std::string& cookie = cookies[s.value("browser", "a")];
            auto done = served(cookie, address, command, args, [&] { return std::optional<std::string>(run()); });
            if (done.set_cookie) cookie = cookie_from(*done.set_cookie);
            return json::parse(*done.answer);
        };
        // A refusal goes on the wire as the service sends it: its key with its English.
        auto ran = [](const std::function<void()>& run) {
            try {
                run();
            } catch (const RefusedError& e) {
                return json{{"error", e.text}}.dump();
            }
            return json{{"value", nullptr}}.dump();
        };
        /// Runs a step's call as the program would: served as one call of its browser.
        auto call = [&](const json& s, const std::string& command, const json& args, const std::function<void()>& run) -> std::string {
            json answered = serving ? as_browser(s, command, args, [&] { return ran(run); }) : json::parse(ran(run));
            if (!answered.contains("error")) return "";
            std::string key = answered["error"].value("key", "");
            if (!c["refusals"].contains(key) || answered["error"].value("text", "") != c["refusals"][key].get<std::string>())
                check::fail(__FILE__, __LINE__, name + ": a refusal without Clarity's words: " + answered["error"].dump());
            return key;
        };
        auto me_now = [&](const json& s) -> json {
            std::optional<int64_t> me;
            call(s, "started", json::object(), [&] { me = db.me_id(); });
            return me ? json(*me) : json(nullptr);
        };
        int step = 0;
        for (const auto& s : one["steps"]) {
            ++step;
            std::string at = name + ", step " + std::to_string(step);
            std::string got;
            if (s.contains("signin")) {
                got = call(s, "signin", {{"strategy", s["signin"]}, {"inputs", s["inputs"]}}, [&] { db.signin(s["signin"], s["inputs"]); });
            } else if (s.contains("signout")) {
                call(s, "signout", json::object(), [&] { db.signout(); });
            } else if (s.contains("hashed")) {
                const json& h = s["hashed"];
                auto minimum = h.contains("minimum") ? std::optional<int64_t>(h["minimum"]) : std::nullopt;
                std::string kept;
                try {
                    kept = hashed(h["method"], h["text"], minimum);
                } catch (const RefusedError& e) {
                    got = e.text.key().value_or("");
                    if (json(e.text)["text"] != c["refusals"]["secretTooShort"]) check::fail(__FILE__, __LINE__, at + ": " + json(e.text).dump());
                    auto slot = json(e.text)["values"]["minimum"]["value"];
                    if (json(slot) != s["minimum"]) check::fail(__FILE__, __LINE__, at + ": slot minimum " + json(slot).dump());
                }
                if (got.empty() && (kept.empty() || kept == h["text"].get<std::string>())) check::fail(__FILE__, __LINE__, at + ": not kept as a hash");
            } else if (s.contains("save")) {
                // A dispatch of the browser's window: a handler that saves the record.
                got = call(s, "dispatch_event", {{"state", {{"window", "Main"}}}}, [&] {
                    Member member = db.get<Member>(s["save"]["id"]);
                    for (const auto& [column, value] : s["save"].items())
                        if (column != "id") member.field(column) = stored_value(value);
                    db.save(member);
                });
            } else if (s.contains("restart")) {
                // The program starts anew on the same database and secret store.
                db.add_signin(source);
                auto page = db.signin_first();
                if ((page ? json(*page) : json(nullptr)) != s["page"]) check::fail(__FILE__, __LINE__, at + ": starts with " + page.value_or("nothing"));
            } else if (s.contains("remembered")) {
                bool held = !(*kept)[source.program + ":" + remembered_name].empty();
                if (held != s["remembered"].get<bool>()) check::fail(__FILE__, __LINE__, at + ": remembered " + std::to_string(held));
            } else if (s.contains("count")) {
                if (json(db.all<Member>().size()) != s["count"]) check::fail(__FILE__, __LINE__, at + ": count " + std::to_string(db.all<Member>().size()));
            }
            if (got != s.value("refused", "")) check::fail(__FILE__, __LINE__, at + ": refused '" + got + "', not '" + s.value("refused", "") + "'");
            if (s.contains("me") && me_now(s) != s["me"]) check::fail(__FILE__, __LINE__, at + ": me is " + me_now(s).dump() + ", not " + s["me"].dump());
        }
    }
    secret_store = {};
}

// A provider's settings go to Clarity.Auth, the secret read from the protected store, "" when not kept.
TEST(signin_a_provider_s_settings_go_to_clarity_auth) {
    SigninSource source;
    source.program = "club";
    SigninStrategy github{"github", {"email"}, {{"client", "app"}, {"secret", "setting:githubSecret"}, {"claim", "email"}}};
    auto kept = std::make_shared<std::map<std::string, std::string>>();
    secret_store.get = [kept](const std::string& program, const std::string& key) -> std::optional<std::string> {
        auto found = kept->find(program + ":" + key);
        return found == kept->end() ? std::nullopt : std::optional<std::string>(found->second);
    };
    secret_store.set = [kept](const std::string& program, const std::string& key, const std::string& value) { (*kept)[program + ":" + key] = value; };
    CHECK_EQ(source.options_of(github), json::parse(R"({"client":"app","secret":"","claim":"email"})"));
    (*kept)["club:githubSecret"] = "s3cret";
    CHECK_EQ(source.options_of(github), json::parse(R"({"client":"app","secret":"s3cret","claim":"email"})"));
    CHECK_EQ(source.options_of(SigninStrategy{"password", {"email", "password_hash"}}), json::object());
    secret_store = {};
}

TEST(me_allow_and_yours_refuse_in_clarity_words) {
    Db db = people_with(json::parse(R"([{"name":"Ann","email":"ann@me.test","password_hash":{"hash":"argon2id","text":"pw"}}])"));
    SigninSource source;
    source.table = "members";
    source.page = "SignIn";
    source.strategies = {{"password", {"email", "password_hash"}}};
    db.add_signin(source);
    CHECK_EQ(refusal([&] { db.me<Member>(); }), std::string("notAllowed"));
    CHECK_EQ(db.signin("password", {{"name", "ann@me.test"}, {"password", "pw"}}), int64_t(1));
    CHECK_EQ(db.me<Member>().name, std::string("Ann"));
    CHECK_EQ(refusal([] { allow(false); }), std::string("notAllowed"));
    CHECK_EQ(refusal([] { allow(true); }), std::string(""));
    CHECK_EQ(refusal([] { yours(false); }), std::string("notYours"));
    // A refusal carries its key and its English, both ways over the wire; with no translation it shows its English.
    json wire = not_allowed();
    CHECK_EQ(wire, json::parse(R"({"key":"notAllowed","text":"You are not allowed to do this."})"));
    CHECK_EQ(wire.get<Text>().show(), std::string("You are not allowed to do this."));
    CHECK_EQ(secret_too_short(8).show(), std::string("This needs at least 8 characters."));
    // A hashed field keeps the hash, never the text; a new token is new every time.
    CHECK(hashed("sha256", "a") != "a");
    CHECK_EQ(hashed("sha256", "a"), hashed("sha256", "a"));
    CHECK(new_token() != new_token());
}

TEST(the_start_says_signin_comes_first_and_the_wire_signs_in_and_out) {
    Db db = people_with(json::parse(R"([{"name":"Gus","email":"gus@wire.test","password_hash":{"hash":"argon2id","text":"pw"}}])"));
    SigninSource source;
    source.table = "members";
    source.page = "SignIn";
    source.strategies = {{"password", {"email", "password_hash"}}};
    db.add_signin(source);
    Nothing windows;
    shell::Service service({"club", "Club", "Main", R"({"project":{}})", &db, &windows});
    auto started = [&] { return json::parse(*shell::wire_answer(service, "started", json::object()))["value"]["signin"]; };
    CHECK_EQ(started(), json("SignIn"));
    CHECK_EQ(json::parse(*shell::wire_answer(service, "signin", {{"strategy", "password"}, {"inputs", {{"name", "gus@wire.test"}, {"password", "no"}}}}))["error"]["key"], json("signinRefused"));
    CHECK_EQ(json::parse(*shell::wire_answer(service, "signin", {{"strategy", "password"}, {"inputs", {{"name", "gus@wire.test"}, {"password", "pw"}}}})), json::parse(R"({"value":null})"));
    CHECK_EQ(db.me_id(), std::optional<int64_t>(1));
    CHECK_EQ(started(), json(nullptr));
    CHECK_EQ(json::parse(*shell::wire_answer(service, "signout", json::object())), json::parse(R"({"value":null})"));
    CHECK_EQ(db.me_id(), std::optional<int64_t>());
    CHECK_EQ(started(), json("SignIn"));

    // A program whose people do not sign in leaves both to the library's default.
    Db plain = Db::memory();
    shell::Service without({"club", "Club", "Main", R"({"project":{}})", &plain, &windows});
    CHECK(!shell::wire_answer(without, "signout", json::object()));
    CHECK(!shell::wire_answer(without, "signin", {{"strategy", "password"}}));
    CHECK_EQ(json::parse(*shell::wire_answer(without, "started", json::object()))["value"]["signin"], json(nullptr));
}

TEST(on_the_desktop_the_account_strategy_signs_the_computers_user_in_at_the_start) {
    Db db = people_with(json::array());
    SigninSource source;
    source.table = "members";
    source.page = "SignIn";
    source.strategies = {{"account", {"login"}}};
    db.add_signin(source);
    // The first account is the computer's user's, signed in at the start;
    // after a sign-out the sign-in window shows.
    CHECK_EQ(db.signin_first(), std::optional<std::string>());
    CHECK_EQ(db.me_id(), std::optional<int64_t>(1));
    db.signout();
    CHECK_EQ(db.signin_first(), std::optional<std::string>("SignIn"));
    CHECK_EQ(db.all<Member>().size(), size_t(1));
    // A new start (the handle named its people again) signs them in by themselves.
    db.add_signin(source);
    CHECK_EQ(db.signin_first(), std::optional<std::string>());
    CHECK_EQ(db.me_id(), std::optional<int64_t>(1));
}

TEST(served_a_session_cookie_names_the_person_and_only_the_signin_window_answers_without_one) {
    Db db = people_with(json::parse(R"([{"name":"Hal","email":"hal@serve.test","password_hash":{"hash":"argon2id","text":"pw"}}])"));
    SigninSource source;
    source.table = "members";
    source.page = "SignIn";
    source.targets = {"web"};
    source.session_seconds = 3600;
    source.strategies = {{"password", {"email", "password_hash"}}};
    db.add_signin(source);
    Nothing windows;
    shell::Server server(shell::Options{"club", "Club", "Main", R"({"project":{}})", &db, &windows});
    auto ask = [&](const std::string& body, const std::string& cookie = "") { return shell::serve_answer(server, post(body, cookie), "192.0.2.50"); };

    CHECK_EQ(body_of(ask(R"({"command":"started"})"))["value"]["signin"], json("SignIn"));
    CHECK_EQ(body_of(ask(R"({"command":"open_window","args":{"window":"Main"}})"))["error"]["key"], json("notAllowed"));
    CHECK(body_of(ask(R"({"command":"open_window","args":{"window":"SignIn"}})")).contains("value"));
    // A refused sign-in sets no cookie.
    std::string refused = ask(R"({"command":"signin","args":{"strategy":"password","inputs":{"name":"hal@serve.test","password":"no"}}})");
    CHECK_EQ(body_of(refused)["error"]["key"], json("signinRefused"));
    CHECK_EQ(set_cookie_of(refused), std::string(""));
    std::string signed_in = ask(R"({"command":"signin","args":{"strategy":"password","inputs":{"name":"hal@serve.test","password":"pw"}}})");
    std::string set = set_cookie_of(signed_in);
    CHECK(set.rfind("clarity_session=", 0) == 0);
    CHECK(set.size() > 40);
    CHECK(set.ends_with("; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600"));
    std::string cookie = cookie_from(set);
    CHECK(body_of(ask(R"({"command":"open_window","args":{"window":"Main"}})", cookie)).contains("value"));
    CHECK_EQ(body_of(ask(R"({"command":"started"})", cookie))["value"]["signin"], json(nullptr));
    // The desktop's person is untouched by a browser's sign-in.
    CHECK_EQ(db.me_id(), std::optional<int64_t>());
    std::string out = ask(R"({"command":"signout"})", cookie);
    CHECK_EQ(set_cookie_of(out), std::string("clarity_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"));
    // The closed session names nobody any more.
    CHECK_EQ(body_of(ask(R"({"command":"open_window","args":{"window":"Main"}})", cookie))["error"]["key"], json("notAllowed"));
}

TEST(a_command_line_signs_in_before_its_command_runs) {
    Db db = people_with(json::parse(R"([{"name":"Ivy","email":"ivy@cli.test","password_hash":{"hash":"argon2id","text":"pw"},"api_token":{"hash":"sha256","text":"ivy-cli-token"}}])"));
    SigninSource source;
    source.table = "members";
    source.page = "SignIn";
    source.targets = {"desktop"};
    source.strategies = {{"password", {"email", "password_hash"}}, {"token", {"api_token"}}};
    db.add_signin(source);
    Typed terminal;
    // People who do not sign in on the command line: nothing to do.
    CHECK_EQ(cli_signin(db, {}, terminal, "Password: "), std::optional<int64_t>());
    source.targets = {"cli"};
    db.add_signin(source);
    // --user: the password read without echo, after the program's prompt.
    terminal.secret = "pw";
    CHECK_EQ(cli_signin(db, {"ivy@cli.test", std::nullopt}, terminal, "Password: "), std::optional<int64_t>(1));
    CHECK(terminal.hidden);
    CHECK_EQ(terminal.written, std::string("Password: \n"));
    terminal.secret = "wrong";
    CHECK_EQ(refusal([&] { cli_signin(db, {"ivy@cli.test", std::nullopt}, terminal, "Password: "); }), std::string("signinRefused"));
    // --token, or CLARITY_TOKEN.
    CHECK_EQ(cli_signin(db, cli_signin_of({"list", "--token", "ivy-cli-token"}), terminal, "Password: "), std::optional<int64_t>(1));
#ifdef _WIN32
    _putenv_s("CLARITY_TOKEN", "ivy-cli-token");
#else
    setenv("CLARITY_TOKEN", "ivy-cli-token", 1);
#endif
    CHECK_EQ(cli_signin_of({"list"}).token, std::optional<std::string>("ivy-cli-token"));
    CHECK_EQ(cli_signin(db, cli_signin_of({"list"}), terminal, "Password: "), std::optional<int64_t>(1));
#ifdef _WIN32
    _putenv_s("CLARITY_TOKEN", "");
#else
    unsetenv("CLARITY_TOKEN");
#endif
    CHECK_EQ(cli_signin_of({"list", "--user", "ivy@cli.test"}).user, std::optional<std::string>("ivy@cli.test"));
    CHECK_EQ(cli_signin_of({"list"}).token, std::optional<std::string>());
    // Without a flag and without account: refused.
    CHECK_EQ(refusal([&] { cli_signin(db, {}, terminal, "Password: "); }), std::string("signinRefused"));
}
