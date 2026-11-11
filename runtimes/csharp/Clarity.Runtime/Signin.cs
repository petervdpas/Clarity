// Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4), as the
// database handle knows it. The package that signs people in (Clarity.Auth,
// Signins.AddSignin) gives the handle its people (IPeople) after the
// databases open; Db.Signin then finds and checks the person through it and
// makes them the current person. On the desktop and the command line that
// is one person for the program; served, each call is the person of its
// session cookie (Db.Served), so `me` is read per call (Db.MeId). Refusals
// are Clarity's own words, by key (conformance/signin.json).

using System.Text.Json;
using System.Text.Json.Nodes;

namespace Clarity.Runtime;

/// <summary>Clarity's words and checks of sign-in that generated code uses without Clarity.Auth.</summary>
public static class Signin
{
    /// <summary>The name of the session cookie a served program sets.</summary>
    public const string Cookie = "clarity_session";

    /// <summary>The name of the remembered desktop sign-in among the program's secrets.</summary>
    public const string Remembered = "clarityRemembered";

    /// <summary>Clarity's words when signing in did not work.</summary>
    public static Text Refused() => new("Signing in did not work. Check what you typed and try again.") { Key = "signinRefused" };

    /// <summary>Clarity's words when too many tries failed.</summary>
    public static Text TooManyTries() => new("Too many tries. Wait a few minutes and try again.") { Key = "signinTooManyTries" };

    /// <summary>Clarity's words when <c>allow when</c> does not hold.</summary>
    public static Text NotAllowed() => new("You are not allowed to do this.") { Key = "notAllowed" };

    /// <summary>Clarity's words when <c>readable when</c> or <c>writable when</c> does not hold.</summary>
    public static Text NotYours() => new("This record is not yours.") { Key = "notYours" };

    /// <summary>Clarity's words when text assigned to a hashed field with <c>minimum</c> is shorter.</summary>
    public static Text SecretTooShort(long minimum) =>
        ClarityException.Own("secretTooShort", "This needs at least {minimum} characters.", ("minimum", minimum));

    /// <summary><c>allow when &lt;condition&gt;</c> on an action, command or window: refused with <c>notAllowed</c> when it does not hold.</summary>
    public static void Allow(bool holds)
    {
        if (!holds)
            throw new RefusedException(NotAllowed());
    }

    /// <summary><c>readable when</c> / <c>writable when</c> of an entity, checked on a find or a save: refused with <c>notYours</c> when it does not hold.</summary>
    public static void Yours(bool holds)
    {
        if (!holds)
            throw new RefusedException(NotYours());
    }

    /// <summary>
    /// The password of <c>--user</c>, typed on the terminal without echo, <paramref name="prompt"/>
    /// written to standard error first; read as a line when the input is no terminal; null at
    /// the end of the input.
    /// </summary>
    public static string? ReadPassword(string prompt)
    {
        if (Console.IsInputRedirected)
            return Console.In.ReadLine();
        Console.Error.Write(prompt);
        var typed = new System.Text.StringBuilder();
        while (true)
        {
            var key = Console.ReadKey(intercept: true);
            if (key.Key == ConsoleKey.Enter)
                break;
            if (key.Key == ConsoleKey.Backspace)
            {
                if (typed.Length > 0)
                    typed.Length--;
                continue;
            }
            if (!char.IsControl(key.KeyChar))
                typed.Append(key.KeyChar);
        }
        Console.Error.WriteLine();
        return typed.ToString();
    }

    /// <summary>The session token a request's <c>Cookie</c> header carries; null for none.</summary>
    public static string? SessionOf(string? cookieHeader)
    {
        foreach (var pair in (cookieHeader ?? "").Split(';'))
        {
            var at = pair.IndexOf('=');
            if (at > 0 && pair[..at].Trim() == Cookie && pair[(at + 1)..].Trim() is { Length: > 0 } value)
                return value;
        }
        return null;
    }

    /// <summary>The <c>Set-Cookie</c> value of a new session that lasts <paramref name="seconds"/>.</summary>
    internal static string Opened(string token, long seconds) => $"{Cookie}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={seconds}";

    /// <summary>The <c>Set-Cookie</c> value that clears the session cookie.</summary>
    internal static string Cleared() => $"{Cookie}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
}

/// <summary>
/// How a command line signs in: <c>--user name</c> and <c>--token value</c>, the token else
/// from the environment's CLARITY_TOKEN.
/// </summary>
public sealed record CliSignin(string? User, string? Token)
{
    /// <summary><c>--user</c> and <c>--token</c> among <paramref name="args"/>, the token else from CLARITY_TOKEN.</summary>
    public static CliSignin FromArgs(IReadOnlyList<string> args)
    {
        string? After(string flag) => args.ToList().IndexOf(flag) is var at and >= 0 && at + 1 < args.Count ? args[at + 1] : null;
        var token = After("--token") ?? Environment.GetEnvironmentVariable("CLARITY_TOKEN");
        return new CliSignin(After("--user"), string.IsNullOrEmpty(token) ? null : token);
    }
}

/// <summary>
/// The people of a program and how they sign in, as the package that signs
/// people in (Clarity.Auth) gives them to the database handle.
/// </summary>
public interface IPeople
{
    /// <summary>The window of the kind <c>signin</c>.</summary>
    string Page { get; }

    /// <summary>Whether people sign in on <paramref name="target"/> (<c>desktop</c>, <c>web</c>, <c>cli</c>).</summary>
    bool On(string target);

    /// <summary>Whether the project names the strategy <paramref name="strategy"/>.</summary>
    bool Uses(string strategy);

    /// <summary>
    /// Finds and checks the person signing in with <paramref name="strategy"/> and its
    /// <paramref name="inputs"/> on <paramref name="target"/>, failed tries counted per account
    /// and per <paramref name="address"/> (none off the web) when <paramref name="counted"/>:
    /// the record, or a <see cref="RefusedException"/> with Clarity's words.
    /// </summary>
    long Signin(Db db, string strategy, JsonObject inputs, string target, string? address, bool counted);

    /// <summary>How long a served session lasts.</summary>
    long SessionSeconds { get; }

    /// <summary>A new session for <paramref name="record"/>: its token.</summary>
    string OpenSession(long record);

    /// <summary>The record of a session, 0 for none (unknown or expired).</summary>
    long FindSession(string token);

    void CloseSession(string token);

    /// <summary>Closes every session of <paramref name="record"/> but <paramref name="token"/>.</summary>
    void CloseOtherSessions(long record, string token);

    /// <summary>Closes every session of <paramref name="record"/>.</summary>
    void CloseAllSessions(long record);

    /// <summary>The data source of the table of people; empty for the first.</summary>
    string Source { get; }

    /// <summary>The table of people.</summary>
    string Table { get; }

    /// <summary>The columns of the table that keep a secret (a role <c>secret</c> of a strategy the project names).</summary>
    IReadOnlyList<string> SecretColumns { get; }

    /// <summary>
    /// The record a remembered desktop sign-in (<c>remember N days</c>) signs in when it still
    /// holds; 0 for anything else (none remembered, no store, the record gone, the secret changed).
    /// </summary>
    long Remembered(Db db);

    /// <summary>Forgets the remembered sign-in, when the program remembers one (signing out).</summary>
    void Forget();
}

/// <summary>
/// The served call that runs now: its person, the session and the address it came
/// with, and what signing in or out changed of its session.
/// </summary>
sealed class SigninCall
{
    public long? Me;
    public string? Token;
    public string Address = "";
    public string? Opened;
    public bool Closed;
}

public sealed partial class Db
{
    IPeople? people;

    /// <summary>The person signed in on the desktop or the command line.</summary>
    long? me;

    /// <summary>The command line signs in as <c>cli</c> (<see cref="SigninCli"/>); else <c>desktop</c>.</summary>
    bool cli;

    /// <summary>The served call this thread (and what it starts) answers; none off the web.</summary>
    static readonly AsyncLocal<SigninCall?> call = new();

    /// <summary>Gives the handle its people: what Clarity.Auth's AddSignin does after the databases open. Nobody is signed in yet.</summary>
    internal void UsePeople(IPeople given)
    {
        people = given;
        me = null;
    }

    /// <summary>The people of the program, when it names them.</summary>
    public IPeople? People => people;

    /// <summary>The database of <paramref name="source"/> (empty for the first), for a package that reads its people.</summary>
    internal IDriver Driver(string source) => Conn(source);

    /// <summary>Where this call runs: <c>web</c> served, else <paramref name="otherwise"/>.</summary>
    static string Target(string otherwise) => call.Value is null ? otherwise : "web";

    /// <summary>The record signed in for this call: served, the one of its session; else the program's. Null for nobody.</summary>
    public long? MeId => call.Value is { } served ? served.Me : me;

    /// <summary><c>me</c> in code: the record signed in now; refused with <c>notAllowed</c> when nobody is.</summary>
    public R Me<R>() where R : IRecord<R> =>
        MeId is { } id ? Get<R>(id) : throw new RefusedException(Clarity.Runtime.Signin.NotAllowed());

    /// <summary>
    /// The sign-in window the program starts with, as <c>started</c> says it: none when it
    /// names no people, when its people do not sign in where it runs (<c>web</c> served, else
    /// <c>desktop</c>), or when someone is signed in. On the desktop a sign-in remembered
    /// (<c>remember N days</c>) that still holds signs its person in, else the <c>account</c>
    /// strategy signs the computer's user in by itself when they have a record (no try counted).
    /// </summary>
    public string? SigninFirst()
    {
        if (people is null)
            return null;
        var target = Target("desktop");
        if (!people.On(target) || MeId is not null)
            return null;
        if (target == "desktop" && people.Remembered(this) is var remembered and > 0)
        {
            me = remembered;
            return null;
        }
        if (target == "desktop" && people.Uses("account"))
        {
            try
            {
                Attempt("account", [], target, counted: false);
                return null;
            }
            catch (RefusedException)
            {
            }
        }
        return people.Page;
    }

    /// <summary>
    /// Signs a person in with <paramref name="strategy"/> and its <paramref name="inputs"/>
    /// (<c>name</c>, <c>password</c>, <c>token</c>, as the catalog names them): the record
    /// signed in, or Clarity's refusal (<c>signinRefused</c>, <c>signinTooManyTries</c>).
    /// </summary>
    public long Signin(string strategy, JsonObject inputs) => Attempt(strategy, inputs, Target(cli ? "cli" : "desktop"), counted: true);

    long Attempt(string strategy, JsonObject inputs, string target, bool counted)
    {
        var them = people ?? throw new StorageException("the program names no people who sign in");
        var id = them.Signin(this, strategy, inputs, target, call.Value?.Address is { Length: > 0 } address ? address : null, counted);
        if (call.Value is { } served)
        {
            // A new session for the new sign-in; the one it came with is closed.
            if (served.Token is { } old)
                them.CloseSession(old);
            served.Token = them.OpenSession(id);
            served.Opened = Clarity.Runtime.Signin.Opened(served.Token, them.SessionSeconds);
            served.Closed = false;
            served.Me = id;
        }
        else
            me = id;
        return id;
    }

    /// <summary>
    /// Signs the command line's person in before a command runs, when the program's people
    /// sign in on <c>cli</c>: <see cref="CliSignin.Token"/> with the <c>token</c> strategy,
    /// <see cref="CliSignin.User"/> with the <c>password</c> strategy and the password
    /// <paramref name="password"/> reads (typed without echo, <see cref="Clarity.Runtime.Signin.ReadPassword"/>;
    /// null refuses), else the computer's user by <c>account</c> when they have a record. Null
    /// when the command line needs no sign-in; else the record signed in, or Clarity's refusal.
    /// </summary>
    public long? SigninCli(CliSignin given, Func<string?> password)
    {
        if (people is null || !people.On("cli"))
            return null;
        cli = true;
        if (given.Token is { } token && people.Uses("token"))
            return Signin("token", new JsonObject { ["token"] = token });
        if (given.User is { } user && people.Uses("password"))
        {
            var typed = password() ?? throw new RefusedException(Clarity.Runtime.Signin.Refused());
            return Signin("password", new JsonObject { ["name"] = user, ["password"] = typed });
        }
        if (people.Uses("account"))
            return Signin("account", []);
        throw new RefusedException(Clarity.Runtime.Signin.Refused());
    }

    /// <summary>Signs the person out: served, the call's session is closed and its cookie cleared; else nobody is signed in any more.</summary>
    public void Signout()
    {
        if (call.Value is { } served)
        {
            if (served.Token is { } token)
                people?.CloseSession(token);
            served.Token = null;
            served.Me = null;
            served.Opened = null;
            served.Closed = true;
        }
        else
        {
            me = null;
            people?.Forget();
        }
    }

    /// <summary>
    /// Runs one served call (<paramref name="run"/>) as the person of its session:
    /// <paramref name="cookie"/> is the request's <c>Cookie</c> header, <paramref name="address"/>
    /// where it came from. Gives its answer and the <c>Set-Cookie</c> value when the call
    /// signed in (a new session) or out (the cookie cleared). Without people it just runs.
    /// </summary>
    internal (T Answer, string? SetCookie) Served<T>(string? cookie, string address, Func<T> run)
    {
        if (people is null)
            return (run(), null);
        var token = Clarity.Runtime.Signin.SessionOf(cookie);
        var record = token is null ? 0 : people.FindSession(token);
        var now = new SigninCall { Me = record > 0 ? record : null, Token = record > 0 ? token : null, Address = address };
        var was = call.Value;
        call.Value = now;
        try
        {
            var answer = run();
            return (answer, now.Opened ?? (now.Closed ? Clarity.Runtime.Signin.Cleared() : null));
        }
        finally
        {
            call.Value = was;
        }
    }

    /// <summary>
    /// Whether a save of <paramref name="table"/> in <paramref name="source"/> may change a
    /// person's secret: the secrets of record <paramref name="id"/> as stored before it, to give
    /// <see cref="SecretsSaved"/>; null when it cannot.
    /// </summary>
    internal object?[]? SecretsBefore(string source, string table, string key, long id)
    {
        if (people is not { SecretColumns.Count: > 0 } them || id == 0 || them.Table != table || (them.Source != source && sources.Count > 1))
            return null;
        var conn = Conn(source);
        var d = conn.Dialect;
        var rows = conn.Rows($"SELECT {string.Join(", ", them.SecretColumns.Select(c => d.Quote(c)))} FROM {d.Quote(table)} WHERE {d.Quote(key)} = {d.Mark(1)}", [id]);
        return rows.FirstOrDefault();
    }

    /// <summary>
    /// After a save that may have changed a person's secret: served, when it did, that
    /// person is signed out everywhere else (every session but this call's when they changed
    /// their own, all of them when someone else did). On the desktop nothing.
    /// </summary>
    internal void SecretsSaved(string source, string table, string key, long id, object?[]? before)
    {
        if (before is null || people is not { } them || call.Value is not { } served)
            return;
        var after = SecretsBefore(source, table, key, id);
        if (after is not null && before.SequenceEqual(after))
            return;
        if (served.Me == id && served.Token is { } token)
            them.CloseOtherSessions(id, token);
        else
            them.CloseAllSessions(id);
    }

    /// <summary>The sign-in window a served call without a session is held to; null when people do not sign in on the web or someone is signed in.</summary>
    internal string? ServedPage => people is { } them && them.On("web") && MeId is null ? them.Page : null;

    /// <summary>Whether a served call without a session may run <paramref name="command"/>: the program's start, its language and facts, signing in and out.</summary>
    internal static bool OpenWithoutSession(string command) =>
        command is "started" or "set_language" or "facts" or "splash_status" or "watches" or "read_settings" or "signin" or "signout";

    /// <summary>The inputs of a <c>signin</c> command: each given as text.</summary>
    internal static JsonObject Inputs(JsonNode? given)
    {
        var inputs = new JsonObject();
        if (given is JsonObject all)
            foreach (var (name, value) in all)
                if (value is JsonValue v && v.GetValueKind() == JsonValueKind.String)
                    inputs[name] = v.GetValue<string>();
        return inputs;
    }
}
