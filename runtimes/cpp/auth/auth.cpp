#include "auth/auth.hpp"

#include <algorithm>

#include "clarity_auth.h"

#include "clarity/code.hpp"
#include "clarity/settings.hpp"
#include "clarity/sql.hpp"

namespace clarity {

using nlohmann::json;

json auth_call(const std::string& op, const json& args) {
    static const ClarityAuthApi* api = clarity_auth_api();
    std::string sent = args.dump();
    uint8_t* out = nullptr;
    size_t length = 0;
    if (api->call(op.data(), op.size(), sent.data(), sent.size(), &out, &length) != CLARITY_AUTH_OK) {
        char why[1024] = {0};
        api->problem(why, sizeof why);
        throw ValueError(why[0] ? why : "Clarity.Auth failed: " + op);
    }
    std::string made(reinterpret_cast<const char*>(out), length);
    api->free(out, length);
    auto answer = json::parse(made, nullptr, false);
    if (answer.is_discarded()) throw ValueError("Clarity.Auth answered no JSON: " + op);
    return answer;
}

std::string hashed(const std::string& method, const std::string& text, std::optional<int64_t> minimum) {
    if (minimum && !auth_call("secretFits", {{"text", text}, {"minimum", *minimum}}).get<bool>()) refuse(secret_too_short(*minimum));
    return auth_call("hash", {{"method", method}, {"text", text}}).get<std::string>();
}

std::string new_recovery_codes(int64_t count) { return auth_call("newRecoveryCodes", {{"count", count}}).get<std::string>(); }

std::string new_token() { return auth_call("newToken", json::object()).get<std::string>(); }

namespace {

/// The strategy `name` of Clarity.Auth's catalog; null when it has none.
json strategy_info(const std::string& name) {
    static const json catalog = [] {
        try {
            return auth_call("catalog", json::object());
        } catch (const std::exception&) {
            return json::array();
        }
    }();
    for (const auto& s : catalog)
        if (s.is_object() && s.value("name", "") == name) return s;
    return nullptr;
}

bool listed(const json& list, const std::string& item) {
    if (!list.is_array()) return false;
    for (const auto& x : list)
        if (x.is_string() && x.get<std::string>() == item) return true;
    return false;
}

}  // namespace

/// The sign-in of a database handle (Db::add_signin): it asks Clarity.Auth
/// and reads and adds people in their table.
class AuthPeople final : public People {
public:
    explicit AuthPeople(SigninSource source) : source_(std::move(source)) {}

    const SigninSource& source() const override { return source_; }

    int64_t attempt(Db& db, const std::string& target, const std::string& strategy, const json& inputs, bool counted) override {
        auto refused = [] { return RefusedError(signin_refused()); };
        // Only a strategy the project names, on a target the catalog lets it work on.
        const SigninStrategy* used = nullptr;
        for (const auto& s : source_.strategies)
            if (s.name == strategy) used = &s;
        json info = strategy_info(strategy);
        if (!used || !info.is_object() || !listed(info.value("targets", json::array()), target)) throw refused();
        json found;
        try {
            found = auth_call("find", {{"strategy", strategy}, {"inputs", inputs}});
        } catch (const std::exception&) {
            throw refused();
        }
        std::string role = found.value("role", ""), equals = found.value("equals", "");
        std::vector<std::string> keys{"account:" + equals};
        if (const ServedCall* call = served_call(); call && !call->address.empty()) keys.push_back("address:" + call->address);
        if (counted) {
            for (const auto& key : keys)
                if (!auth_call("triesAllowed", {{"key", key}, {"tries", source_.tries}, {"window", source_.tries_seconds}}).get<bool>()) refuse(signin_too_many_tries());
        }
        auto failed = [&] {
            if (counted)
                for (const auto& key : keys) auth_call("triesFailed", {{"key", key}});
            return refused();
        };
        // The column of a role: the strategy's columns are in the catalog's order of its fields.
        auto column = [&](const std::string& wanted) -> std::optional<std::string> {
            const json& fields = info.value("fields", json::array());
            for (size_t at = 0; at < fields.size(); ++at)
                if (fields[at].value("role", "") == wanted) return at < used->columns.size() ? std::optional(used->columns[at]) : std::nullopt;
            return std::nullopt;
        };
        auto by = column(role);
        if (!by) throw failed();
        int64_t id = 0;
        if (auto person = person_by(db, *by, column("secret"), equals)) {
            bool checked = false;
            try {
                checked = auth_call("check", {{"strategy", strategy}, {"inputs", inputs}, {"stored", {{"secret", person->second}}}}).get<bool>();
            } catch (const std::exception&) {
            }
            if (!checked) throw failed();
            id = person->first;
            // What the strategy keeps after it (a recovery code used up), written back to the person's row.
            json after = auth_call("afterSignin", {{"strategy", strategy}, {"inputs", inputs}, {"stored", {{"secret", person->second}}}});
            if (after.is_object())
                for (const auto& [kept_role, value] : after.items())
                    if (auto kept = column(kept_role); kept && value.is_string()) keep(db, *kept, value.get<std::string>(), id);
        } else if (info.value("firstAccount", false) && no_people(db)) {
            try {
                id = first_account(db, info, *used, strategy, inputs);
            } catch (const std::exception&) {
                throw failed();
            }
        } else {
            throw failed();
        }
        if (counted)
            for (const auto& key : keys) auth_call("triesCleared", {{"key", key}});
        signed_in(db, id);
        if (target == "desktop" && remembers(inputs)) remember(db, column("secret"), id);
        return id;
    }

    std::optional<int64_t> remembered(Db& db) override {
        if (source_.remember_seconds <= 0 || !secret_store.get) return std::nullopt;
        try {
            auto entry = secret_store.get(source_.program, remembered_name);
            if (!entry || entry->empty()) return std::nullopt;
            int64_t record = auth_call("rememberRecord", {{"entry", *entry}}).get<int64_t>();
            if (record <= 0) return std::nullopt;
            for (const auto& column : secrets_)
                if (auto stored = stored_secret(db, column, record); stored && auth_call("rememberCheck", {{"entry", *entry}, {"stored", *stored}}).get<int64_t>() == record) return record;
        } catch (const std::exception&) {
        }
        return std::nullopt;
    }

    void signout(Db& db) override {
        if (ServedCall* call = served_call()) {
            if (!call->token.empty()) close(call->token);
            call->token.clear();
            call->me.reset();
            call->opened.reset();
            call->closed = true;
            return;
        }
        db.me_.reset();
        if (source_.remember_seconds > 0 && secret_store.set) {
            try {
                secret_store.set(source_.program, remembered_name, "");
            } catch (const std::exception&) {
            }
        }
    }

    std::optional<int64_t> session_find(const std::string& token) override {
        try {
            int64_t id = auth_call("sessionFind", {{"token", token}}).get<int64_t>();
            return id > 0 ? std::optional(id) : std::nullopt;
        } catch (const std::exception&) {
            return std::nullopt;
        }
    }

    std::vector<std::string> secret_columns(const std::string& source, const std::string& table) const override {
        return source == source_.source && table == source_.table ? secrets_ : std::vector<std::string>{};
    }

    void secret_changed(Db&, int64_t record) override {
        ServedCall* call = served_call();
        if (!call) return;
        try {
            // Their own change keeps the session they made it in; someone else's closes every one.
            if (call->me == record && !call->token.empty()) auth_call("sessionCloseOthers", {{"record", record}, {"token", call->token}});
            else auth_call("sessionCloseAll", {{"record", record}});
        } catch (const std::exception&) {
        }
    }

private:
    SigninSource source_;
    /// The columns that keep a secret of one of the strategies, each once.
    std::vector<std::string> secrets_ = [this] {
        std::vector<std::string> out;
        for (const auto& used : source_.strategies) {
            json info = strategy_info(used.name);
            json fields = info.is_object() ? info.value("fields", json::array()) : json::array();
            for (size_t at = 0; at < fields.size() && at < used.columns.size(); ++at)
                if (fields[at].value("role", "") == "secret" && std::find(out.begin(), out.end(), used.columns[at]) == out.end()) out.push_back(used.columns[at]);
        }
        return out;
    }();

    /// Whether a sign-in with these inputs is remembered: always, unless the
    /// sign-in part asks (`when asked`) and its box's answer is not "true".
    bool remembers(const json& inputs) const {
        if (!source_.remember_asks) return true;
        auto answer = inputs.is_object() ? inputs.find("remember") : inputs.end();
        return answer != inputs.end() && answer->is_string() && answer->get<std::string>() == "true";
    }

    /// After a sign-in on the desktop with `remember N days`: an entry signed
    /// with the person's stored secret in `column` (of the strategy used),
    /// kept in the system's protected store. A strategy without a secret
    /// (`account`) remembers nothing; a store that cannot keep it leaves the
    /// sign-in as it is.
    void remember(Db& db, const std::optional<std::string>& column, int64_t id) {
        if (source_.remember_seconds <= 0 || !column || !secret_store.set) return;
        try {
            // Read after signing in: what signing in changed (a recovery code used up) is what holds.
            auto stored = stored_secret(db, *column, id);
            if (!stored) return;
            std::string entry = auth_call("rememberMake", {{"record", id}, {"seconds", source_.remember_seconds}, {"stored", *stored}}).get<std::string>();
            secret_store.set(source_.program, remembered_name, entry);
        } catch (const std::exception&) {
        }
    }

    /// The stored secret in `column` of person `id`; none when the record is gone or the column empty.
    std::optional<std::string> stored_secret(Db& db, const std::string& column, int64_t id) {
        Dialect d = db.dialect(source_.source);
        auto rows = db.select(source_.source, "SELECT " + quote(d, column) + " FROM " + quote(d, source_.table) + " WHERE " + quote(d, source_.key) + " = " + mark(d, 1), {id});
        if (rows.empty() || rows[0].empty()) return std::nullopt;
        auto text = std::get_if<std::string>(&rows[0][0]);
        return text && !text->empty() ? std::optional(*text) : std::nullopt;
    }

    /// Writes `value` into `column` of person `id`.
    void keep(Db& db, const std::string& column, const std::string& value, int64_t id) {
        Dialect d = db.dialect(source_.source);
        db.at(source_.source).driver->execute("UPDATE " + quote(d, source_.table) + " SET " + quote(d, column) + " = " + mark(d, 1) + " WHERE " + quote(d, source_.key) + " = " + mark(d, 2), {sql::text(value), id});
    }

    static void close(const std::string& token) {
        try {
            auth_call("sessionClose", {{"token", token}});
        } catch (const std::exception&) {
        }
    }

    /// The key and the stored secret (empty without one) of the record whose `column` equals `equals`.
    std::optional<std::pair<int64_t, std::string>> person_by(Db& db, const std::string& column, const std::optional<std::string>& secret, const std::string& equals) {
        Dialect d = db.dialect(source_.source);
        std::string wanted = quote(d, source_.key) + (secret ? ", " + quote(d, *secret) : "");
        auto rows = db.select(source_.source, "SELECT " + wanted + " FROM " + quote(d, source_.table) + " WHERE " + quote(d, column) + " = " + mark(d, 1), {sql::text(equals)});
        if (rows.empty() || rows[0].empty()) return std::nullopt;
        int64_t id = sql::get_integer(source_.key, rows[0][0]);
        std::string stored;
        if (rows[0].size() > 1)
            if (auto text = std::get_if<std::string>(&rows[0][1])) stored = *text;
        return std::pair{id, stored};
    }

    bool no_people(Db& db) {
        Dialect d = db.dialect(source_.source);
        auto rows = db.select(source_.source, "SELECT COUNT(*) FROM " + quote(d, source_.table), {});
        return !rows.empty() && !rows[0].empty() && sql::get_integer("count", rows[0][0]) == 0;
    }

    /// Inserts the first person, with only the fields the strategy names;
    /// every other column takes its default from the table.
    int64_t first_account(Db& db, const json& info, const SigninStrategy& used, const std::string& strategy, const json& inputs) {
        json made = auth_call("firstAccount", {{"strategy", strategy}, {"inputs", inputs}});
        const json& fields = info.value("fields", json::array());
        std::vector<std::string> columns;
        std::vector<SqlValue> values;
        for (size_t at = 0; at < fields.size() && at < used.columns.size(); ++at) {
            std::string role = fields[at].value("role", "");
            if (made.contains(role) && made[role].is_string()) {
                columns.push_back(used.columns[at]);
                values.push_back(sql::text(made[role].get<std::string>()));
            }
        }
        Driver& driver = *db.at(source_.source).driver;
        return driver.insert(insert(driver.dialect(), source_.table, source_.key, columns), values);
    }

    void signed_in(Db& db, int64_t id) {
        if (ServedCall* call = served_call()) {
            // A new session for the new sign-in; the one it came with is closed.
            if (!call->token.empty()) close(call->token);
            call->token.clear();
            try {
                std::string token = auth_call("sessionOpen", {{"record", id}, {"seconds", source_.session_seconds}}).get<std::string>();
                call->opened = std::pair{token, source_.session_seconds};
                call->token = token;
            } catch (const std::exception&) {
            }
            call->me = id;
            return;
        }
        db.me_ = id;
    }
};

void Db::add_signin(const SigninSource& source) {
    auto people = std::make_shared<AuthPeople>(source);
    people_ = people;
    me_.reset();
    signin_started_ = false;
    signin_cli_ = false;
    if (source.on("web")) serve_people(people_);
}

}  // namespace clarity
