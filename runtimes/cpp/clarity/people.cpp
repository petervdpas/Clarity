#include "clarity/people.hpp"

#include <algorithm>
#include <cstdlib>
#include <mutex>

#include "clarity/cli.hpp"
#include "clarity/db.hpp"

namespace clarity {

using nlohmann::json;

bool SigninSource::on(const std::string& target) const { return std::find(targets.begin(), targets.end(), target) != targets.end(); }

bool SigninSource::uses(const std::string& name) const {
    return std::any_of(strategies.begin(), strategies.end(), [&](const SigninStrategy& s) { return s.name == name; });
}

// Clarity's own words with their English (conformance/signin.json, "refusals").
Text signin_refused() { return Text::keyed("signinRefused", "Signing in did not work. Check what you typed and try again."); }
Text signin_too_many_tries() { return Text::keyed("signinTooManyTries", "Too many tries. Wait a few minutes and try again."); }
Text not_allowed() { return Text::keyed("notAllowed", "You are not allowed to do this."); }
Text not_yours() { return Text::keyed("notYours", "This record is not yours."); }
Text secret_too_short(int64_t minimum) { return Text::keyed("secretTooShort", "This needs at least {minimum} characters.").with("minimum", slot_integer(minimum)); }

void allow(bool holds) {
    if (!holds) refuse(not_allowed());
}

void yours(bool holds) {
    if (!holds) refuse(not_yours());
}

namespace {

/// The served call this thread answers (`served`); none off one.
thread_local ServedCall* calling = nullptr;

/// The people of the served program; empty once their database handle is gone.
std::mutex serving;
std::weak_ptr<People> served_people;

std::shared_ptr<People> people_served() {
    std::lock_guard lock(serving);
    return served_people.lock();
}

/// Puts back the served call a thread answered before, however `run` ends.
struct Calling {
    explicit Calling(ServedCall* call) : was(calling) { calling = call; }
    ~Calling() { calling = was; }
    Calling(const Calling&) = delete;
    Calling& operator=(const Calling&) = delete;
    ServedCall* was;
};

}  // namespace

ServedCall* served_call() { return calling; }

void serve_people(const std::shared_ptr<People>& people) {
    std::lock_guard lock(serving);
    served_people = people;
}

std::optional<int64_t> Db::me_id() const {
    if (const ServedCall* call = served_call()) return call->me;
    return me_;
}

std::optional<std::string> Db::signin_first() {
    if (!people_) return std::nullopt;
    const SigninSource& people = people_->source();
    std::string target = served_call() ? "web" : "desktop";
    if (!people.on(target) || me_id()) return std::nullopt;
    // Once, at the start: after a sign-out the sign-in window shows.
    bool first = !signin_started_;
    signin_started_ = true;
    // A sign-in remembered (`remember N days`) that still holds signs its person in.
    if (target == "desktop" && first) {
        if (auto id = people_->remembered(*this)) {
            me_ = *id;
            return std::nullopt;
        }
    }
    if (target == "desktop" && first && people.uses("account")) {
        try {
            people_->attempt(*this, target, "account", json::object(), false);
            return std::nullopt;
        } catch (const std::exception&) {
            // The computer's user has no record: the sign-in window then.
        }
    }
    return people.page;
}

int64_t Db::signin(const std::string& strategy, const json& inputs) { return signin_on(served_call() ? "web" : signin_cli_ ? "cli" : "desktop", strategy, inputs); }

int64_t Db::signin_on(const std::string& target, const std::string& strategy, const json& inputs) {
    if (!people_) throw StorageError("the program names no people who sign in");
    return people_->attempt(*this, target, strategy, inputs.is_object() ? inputs : json::object(), true);
}

void Db::signout() {
    if (people_) people_->signout(*this);
    else me_.reset();
}

std::optional<std::string> session_of(const std::string& cookie_header) {
    size_t at = 0;
    while (at <= cookie_header.size()) {
        size_t end = cookie_header.find(';', at);
        if (end == std::string::npos) end = cookie_header.size();
        std::string pair = cookie_header.substr(at, end - at);
        auto equals = pair.find('=');
        if (equals != std::string::npos) {
            auto trimmed = [](std::string text) {
                text.erase(0, text.find_first_not_of(" \t"));
                text.erase(text.find_last_not_of(" \t") + 1);
                return text;
            };
            std::string name = trimmed(pair.substr(0, equals)), value = trimmed(pair.substr(equals + 1));
            if (name == session_cookie && !value.empty()) return value;
        }
        at = end + 1;
    }
    return std::nullopt;
}

bool open_without_session(const std::string& command, const json& args, const std::string& page) {
    static const char* open[] = {"started", "start", "set_language", "facts", "splash_status", "watches", "read_settings", "signin", "signout"};
    for (const char* name : open)
        if (command == name) return true;
    auto text = [](const json& holder, const char* key) { return holder.is_object() && holder.contains(key) && holder[key].is_string() ? holder[key].get<std::string>() : std::string(); };
    if (command == "open_window") return text(args, "window") == page;
    return args.is_object() && args.contains("state") && text(args["state"], "window") == page;
}

ServedAnswer served(const std::string& cookie, const std::string& address, const std::string& command, const json& args,
                    const std::function<std::optional<std::string>()>& run) {
    auto people = people_served();
    auto token = session_of(cookie);
    std::optional<int64_t> me;
    if (people && token) me = people->session_find(*token);
    if (!me) token.reset();
    if (people && !me && !open_without_session(command, args, people->source().page)) return {json{{"error", not_allowed()}}.dump(), std::nullopt};
    ServedCall call{me, token.value_or(""), address, std::nullopt, false};
    ServedAnswer out;
    {
        Calling as(&call);
        out.answer = run();
    }
    auto answered = out.answer ? json::parse(*out.answer, nullptr, false) : json(nullptr);
    bool failed = answered.is_object() && answered.contains("error") && !answered["error"].is_null();
    std::string tail = "; HttpOnly; SameSite=Strict; Path=/; Max-Age=";
    if (call.opened && !failed) out.set_cookie = std::string(session_cookie) + "=" + call.opened->first + tail + std::to_string(call.opened->second);
    else if (call.closed) out.set_cookie = std::string(session_cookie) + "=" + tail + "0";
    return out;
}

CliSignin cli_signin_of(const std::vector<std::string>& args) {
    auto after = [&](const char* flag) -> std::optional<std::string> {
        for (size_t at = 0; at + 1 < args.size(); ++at)
            if (args[at] == flag) return args[at + 1];
        return std::nullopt;
    };
    CliSignin flags{after("--user"), after("--token")};
    if (!flags.token) {
        const char* told = std::getenv("CLARITY_TOKEN");
        if (told) flags.token = told;
    }
    if (flags.token && flags.token->empty()) flags.token.reset();
    return flags;
}

std::optional<int64_t> cli_signin(Db& db, const CliSignin& flags, cli::Terminal& terminal, const std::string& prompt) {
    const People* people = db.people();
    if (!people || !people->source().on("cli")) return std::nullopt;
    const SigninSource& source = people->source();
    db.signin_cli_ = true;
    if (flags.token && source.uses("token")) return db.signin_on("cli", "token", json{{"token", *flags.token}});
    if (flags.user && source.uses("password")) {
        if (!terminal.is_terminal()) refuse(signin_refused());
        terminal.write(prompt);
        auto typed = terminal.read_secret();
        terminal.write("\n");
        if (!typed) refuse(signin_refused());
        return db.signin_on("cli", "password", json{{"name", *flags.user}, {"password", *typed}});
    }
    if (source.uses("account")) return db.signin_on("cli", "account", json::object());
    refuse(signin_refused());
}

}  // namespace clarity
