// Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4). The
// generated program names its entity of people and its strategies once,
// after its databases open (`db.add_signin`, in auth/auth.cpp, carried with
// clarity_use_auth); `db.signin` then finds the person by what Clarity.Auth
// says, checks them and makes them the current person. On the desktop and
// the command line that is one person for the program; served, each call is
// the person of its session cookie (`served`), so `me` is read per call
// (`db.me_id()`). Refusals are Clarity's own words, by key
// (conformance/signin.json).
//
// This part holds what the service, the web server and the command line
// need without Clarity.Auth's library: the block's shape, the served call's
// person and the cookie. What needs the library is behind People.

#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/text.hpp"

namespace clarity {

class Db;
namespace cli {
class Terminal;
}  // namespace cli

/// The name of the session cookie a served program sets.
inline constexpr const char* session_cookie = "clarity_session";

/// One strategy the project uses: its name in Clarity.Auth's catalog and
/// the columns of the table of people it names, in the catalog's order of
/// roles (`password email passwordHash` is {"email", "password_hash"}).
/// The name of the remembered desktop sign-in among the program's secrets.
inline constexpr const char* remembered_name = "clarityRemembered";

struct SigninStrategy {
    std::string name;
    std::vector<std::string> columns;
};

/// The project's `signin` block as the program uses it: where its people
/// are kept, how they sign in, how long a sign-in lasts and how many failed
/// tries are allowed.
struct SigninSource {
    /// The data source of the entity of people; empty for the first.
    std::string source;
    std::string table;
    /// Its key column.
    std::string key = "id";
    /// The window of the kind `signin`.
    std::string page;
    /// Where people sign in: "desktop", "web", "cli".
    std::vector<std::string> targets{"desktop"};
    std::vector<SigninStrategy> strategies;
    /// How long a served session lasts.
    int64_t session_seconds = 8 * 3600;
    /// Failed tries per key allowed within `tries_seconds`.
    int64_t tries = 5;
    int64_t tries_seconds = 15 * 60;
    /// The program's name, under which its secrets are kept (the remembered sign-in).
    std::string program;
    /// How long the desktop program keeps a sign-in between runs (`remember
    /// N days`) in the system's protected store (secret_store); 0 without the line.
    int64_t remember_seconds = 0;
    /// Whether the sign-in part asks whether to remember (`remember N days
    /// when asked`): then only a sign-in whose input `remember` is "true"
    /// is remembered; else every one is.
    bool remember_asks = false;

    /// Whether people sign in on `target`.
    bool on(const std::string& target) const;
    /// Whether the project names strategy `name`.
    bool uses(const std::string& name) const;
};

/// Clarity's words when signing in did not work: signinRefused.
Text signin_refused();
/// Clarity's words when too many tries failed: signinTooManyTries.
Text signin_too_many_tries();
/// Clarity's words when `allow when` does not hold, or nobody is signed in: notAllowed.
Text not_allowed();
/// Clarity's words when `readable when` or `writable when` does not hold: notYours.
Text not_yours();
/// Clarity's words when a secret is shorter than its field's `minimum`: secretTooShort.
Text secret_too_short(int64_t minimum);

/// `allow when <condition>` on an action, command or window: refused with
/// notAllowed when it does not hold.
void allow(bool holds);
/// `readable when` / `writable when` of an entity, checked on a find or a
/// save: refused with notYours when it does not hold.
void yours(bool holds);

/// The served call this thread answers (`served`): its person, the session
/// and the address it came with, and what signing in or out changed of its session.
struct ServedCall {
    std::optional<int64_t> me;
    /// The session token; empty for none.
    std::string token;
    std::string address;
    /// A session opened by this call: its token and seconds.
    std::optional<std::pair<std::string, int64_t>> opened;
    /// Whether this call signed out.
    bool closed = false;
};

/// The served call this thread answers; none off a served call (desktop, command line).
ServedCall* served_call();

/// What signing in needs of Clarity.Auth, which auth/auth.cpp gives a
/// database handle (Db::add_signin).
class People {
public:
    virtual ~People() = default;
    virtual const SigninSource& source() const = 0;
    /// Signs a person in with `strategy` on `target` ("desktop", "web",
    /// "cli"): the record signed in, or a RefusedError with signinRefused or
    /// signinTooManyTries. Failed tries count when `counted`.
    virtual int64_t attempt(Db& db, const std::string& target, const std::string& strategy, const nlohmann::json& inputs, bool counted) = 0;
    /// Signs the current person out; on the desktop the remembered sign-in is forgotten.
    virtual void signout(Db& db) = 0;
    /// The record a remembered desktop sign-in (`remember N days`) signs in
    /// when it still holds; none for anything else (none remembered, no
    /// store, the record gone, the secret changed).
    virtual std::optional<int64_t> remembered(Db& db) = 0;
    /// The record session `token` names now; none when it names none.
    virtual std::optional<int64_t> session_find(const std::string& token) = 0;
    /// The columns of `table` in data source `source` that keep a secret of
    /// a strategy; none when it is not the table of people.
    virtual std::vector<std::string> secret_columns(const std::string& source, const std::string& table) const = 0;
    /// A save changed the secret of person `record`: served, their other
    /// sessions close when they changed their own, all of them when someone
    /// else did; on the desktop nothing.
    virtual void secret_changed(Db& db, int64_t record) = 0;
};

/// The people of the served program, whose sign-in window a call without a
/// session reaches alone: set by add_signin when people sign in on the web,
/// forgotten once their database handle is gone.
void serve_people(const std::shared_ptr<People>& people);

/// The session token a request's `Cookie` header carries; none without one.
std::optional<std::string> session_of(const std::string& cookie_header);

/// Whether a served call without a session may run, while people sign in
/// on the web: the program's start, its language and facts, signing in and
/// out, and the sign-in window `page` itself.
bool open_without_session(const std::string& command, const nlohmann::json& args, const std::string& page);

/// The answer of one served call and the `Set-Cookie` header's value, when
/// the call signed in (a new session) or out (the cookie cleared).
struct ServedAnswer {
    std::optional<std::string> answer;
    std::optional<std::string> set_cookie;
};

/// Runs one served call (`run`, which gives the answer as wire_answer does)
/// as the person of its session: `cookie` is the request's `Cookie` header
/// (empty for none), `address` where it came from. Without a valid session,
/// while people sign in on the web, only the sign-in window and its commands
/// are answered; the rest is refused with notAllowed.
ServedAnswer served(const std::string& cookie, const std::string& address, const std::string& command, const nlohmann::json& args,
                    const std::function<std::optional<std::string>()>& run);

/// What a command line says of signing in: `--user name` and `--token text`.
struct CliSignin {
    std::optional<std::string> user;
    std::optional<std::string> token;
};

/// `--user name` and `--token text` among `args` (the words after the
/// program's name), the token else from the environment's CLARITY_TOKEN.
CliSignin cli_signin_of(const std::vector<std::string>& args);

/// Signs the person of a command line in, before its command runs, when the
/// project's people sign in on `cli`: `--token` with `token`; `--user` with
/// `password`, the password read from `terminal` without echo after
/// `prompt` (the program's translated words; refused with nobody at a terminal); else
/// `account`, the computer's user (a flag of a strategy the project does not
/// use is passed over). The record signed in, none when people do not sign
/// in on the command line; a RefusedError (signinRefused,
/// signinTooManyTries) when it did not work. Later sign-ins are on `cli` too.
std::optional<int64_t> cli_signin(Db& db, const CliSignin& flags, cli::Terminal& terminal, const std::string& prompt);

}  // namespace clarity
