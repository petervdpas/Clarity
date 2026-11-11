// Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4): the
// generated program names its entity of people and its strategies once,
// after its databases open (db.AddSignin); the database handle then signs
// people in through it (Db.Signin, Db.SigninFirst, Db.SigninCli). The
// strategies' work (finding, checking, hashes, tries, sessions) is
// Clarity.Auth's, the one implementation every stack calls; this reads the
// person from the table of people and makes the first account.
//
// The first account: when the table of people has no rows and the strategy
// can make one (its catalog entry says firstAccount), the record is
// inserted with only the fields the strategy names (the match field, the
// secret as its hash); every other column takes its default from the
// table, so a required field without a default makes the insert fail.

using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Auth;

/// <summary>
/// One strategy the project uses: its name in Clarity.Auth's catalog and the columns of the
/// table of people it names, in the catalog's order of roles (<c>password email passwordHash</c>
/// is <c>["email", "password_hash"]</c>).
/// </summary>
public sealed record SigninStrategy(string Name, IReadOnlyList<string> Columns);

/// <summary>
/// The project's <c>signin</c> block as the program uses it: the people in <paramref name="Table"/>,
/// signing in on the window <paramref name="Page"/> (of the kind <c>signin</c>); on the desktop,
/// 8 hours a session and 5 failed tries per 15 minutes unless it says otherwise.
/// </summary>
public sealed record SigninSource(string Table, string Page)
{
    /// <summary>The data source of the entity of people; empty for the first.</summary>
    public string Source { get; init; } = "";

    /// <summary>The key column of the table.</summary>
    public string Key { get; init; } = "id";

    /// <summary>Where people sign in: <c>desktop</c>, <c>web</c>, <c>cli</c>.</summary>
    public IReadOnlyList<string> Targets { get; init; } = ["desktop"];

    /// <summary>The strategies the project names, in its order.</summary>
    public IReadOnlyList<SigninStrategy> Strategies { get; init; } = [];

    /// <summary>How long a served session lasts.</summary>
    public long SessionSeconds { get; init; } = 8 * 3600;

    /// <summary>Failed tries per key allowed within <see cref="TriesSeconds"/>.</summary>
    public int Tries { get; init; } = 5;

    /// <summary>The window failed tries are counted in.</summary>
    public long TriesSeconds { get; init; } = 15 * 60;

    /// <summary>The program's name, under which its secrets are kept (the remembered sign-in).</summary>
    public string Program { get; init; } = "";

    /// <summary>
    /// How long the desktop program keeps a sign-in between runs (<c>remember N days</c>) in the
    /// system's protected store; null without the line.
    /// </summary>
    public long? RememberSeconds { get; init; }

    /// <summary>
    /// Whether the sign-in part asks whether to remember (<c>remember N days when asked</c>): then
    /// only a sign-in whose input <c>remember</c> is "true" is remembered; else every one is.
    /// </summary>
    public bool RememberAsks { get; init; }

    /// <summary>Whether a sign-in with these inputs is remembered: always, unless the sign-in part asks and its box's answer is not "true".</summary>
    public bool Remembers(JsonObject inputs) =>
        !RememberAsks || (inputs["remember"] is JsonValue v && v.TryGetValue<string>(out var answer) && answer == "true");
}

/// <summary>Sign-in on a program's databases, and what generated code calls of Clarity.Auth.</summary>
public static class Signins
{
    /// <summary>Names the people of the program and how they sign in, after the databases opened.</summary>
    public static void AddSignin(this Db db, SigninSource source) => db.UsePeople(new People(source));

    /// <summary>
    /// Text assigned to a <c>hashed</c> field: what is stored, by the field's method
    /// (<c>argon2id</c>, <c>sha256</c> for a token field, <c>sha256-lines</c> for recovery codes).
    /// With the strategy's <c>minimum</c>, shorter text is refused with Clarity's <c>secretTooShort</c>.
    /// </summary>
    public static string Hashed(string method, string text, long? minimum = null)
    {
        if (minimum is { } least && !Bool(AuthCalls.Call("secretFits", new JsonObject { ["text"] = text, ["minimum"] = least }.ToJsonString())))
            throw new RefusedException(Clarity.Runtime.Signin.SecretTooShort(least));
        return Text(AuthCalls.Call("hash", new JsonObject { ["method"] = method, ["text"] = text }.ToJsonString()));
    }

    /// <summary><c>newRecoveryCodes(count)</c> in code: new recovery codes as lines of text, shown once.</summary>
    public static string NewRecoveryCodes(long count) => Text(AuthCalls.Call("newRecoveryCodes", new JsonObject { ["count"] = count }.ToJsonString()));

    static bool Bool(string json) => JsonNode.Parse(json)?.GetValue<bool>() ?? false;

    /// <summary><c>newToken()</c> in code.</summary>
    public static string NewToken() => Text(AuthCalls.Call("newToken", "{}"));

    static string Text(string json) => JsonNode.Parse(json)?.GetValue<string>() ?? "";
}

/// <summary>A strategy of Clarity.Auth's catalog: its fields' roles in order, its targets, whether it makes the first account.</summary>
sealed record Strategy(string Name, IReadOnlyList<string> Roles, IReadOnlyList<string> Targets, bool FirstAccount)
{
    /// <summary>The catalog, read once from Clarity.Auth.</summary>
    public static readonly Lazy<IReadOnlyList<Strategy>> Catalog = new(() =>
    [
        .. JsonNode.Parse(AuthCalls.Call("catalog", "{}"))!.AsArray().Select(s => new Strategy(
            s!["name"]!.GetValue<string>(),
            [.. s["fields"]!.AsArray().Select(f => f!["role"]!.GetValue<string>())],
            [.. s["targets"]!.AsArray().Select(t => t!.GetValue<string>())],
            s["firstAccount"]?.GetValue<bool>() ?? false)),
    ]);
}

/// <summary>The people of a program as the database handle asks for them (IPeople).</summary>
sealed class People(SigninSource source) : IPeople
{
    public string Page => source.Page;

    public long SessionSeconds => source.SessionSeconds;

    public bool On(string target) => source.Targets.Contains(target);

    public bool Uses(string strategy) => source.Strategies.Any(s => s.Name == strategy);

    static RefusedException Refuse() => new(Clarity.Runtime.Signin.Refused());

    public long Signin(Db db, string strategy, JsonObject inputs, string target, string? address, bool counted)
    {
        // Only a strategy the project names, on a target the catalog lets it work on.
        var used = source.Strategies.FirstOrDefault(s => s.Name == strategy) ?? throw Refuse();
        var info = Strategy.Catalog.Value.FirstOrDefault(s => s.Name == strategy) ?? throw Refuse();
        if (!info.Targets.Contains(target))
            throw Refuse();
        string role, equals;
        try
        {
            var found = JsonNode.Parse(AuthCalls.Call("find", new JsonObject { ["strategy"] = strategy, ["inputs"] = inputs.DeepClone() }.ToJsonString()))!;
            role = found["role"]!.GetValue<string>();
            equals = found["equals"]!.GetValue<string>();
        }
        catch (ValueException)
        {
            // An input missing, or no account to find by: nothing to count it against.
            throw Refuse();
        }
        List<string> keys = [$"account:{equals}"];
        if (address is not null)
            keys.Add($"address:{address}");
        if (counted && keys.Any(k => !Bool(AuthCalls.Call("triesAllowed", new JsonObject { ["key"] = k, ["tries"] = source.Tries, ["window"] = source.TriesSeconds }.ToJsonString()))))
            throw new RefusedException(Clarity.Runtime.Signin.TooManyTries());
        RefusedException Failed()
        {
            if (counted)
                foreach (var key in keys)
                    AuthCalls.Call("triesFailed", new JsonObject { ["key"] = key }.ToJsonString());
            return Refuse();
        }
        string? Column(string named) => info.Roles.ToList().IndexOf(named) is var at and >= 0 && at < used.Columns.Count ? used.Columns[at] : null;
        var by = Column(role) ?? throw Failed();
        var conn = db.Driver(source.Source);
        long id;
        if (PersonBy(conn, by, Column("secret"), equals) is ({ } person, { } stored))
        {
            var check = new JsonObject { ["strategy"] = strategy, ["inputs"] = inputs.DeepClone(), ["stored"] = new JsonObject { ["secret"] = stored } };
            bool holds;
            try
            {
                holds = Bool(AuthCalls.Call("check", check.ToJsonString()));
            }
            catch (ValueException)
            {
                holds = false;
            }
            if (!holds)
                throw Failed();
            id = person;
            // What Clarity.Auth says to store after it (a recovery code used up), by role.
            JsonObject after;
            try
            {
                after = JsonNode.Parse(AuthCalls.Call("afterSignin", check.ToJsonString()))!.AsObject();
            }
            catch (ValueException)
            {
                throw Failed();
            }
            foreach (var (changedRole, value) in after)
                if (Column(changedRole) is { } column && value is JsonValue v && v.TryGetValue<string>(out var text))
                {
                    var d = conn.Dialect;
                    conn.Execute($"UPDATE {d.Quote(source.Table)} SET {d.Quote(column)} = {d.Mark(1)} WHERE {d.Quote(source.Key)} = {d.Mark(2)}", [text, id]);
                }
        }
        else if (info.FirstAccount && NoPeople(conn))
        {
            try
            {
                id = FirstAccount(conn, info, used, strategy, inputs);
            }
            catch (ClarityException)
            {
                throw Failed();
            }
        }
        else
            throw Failed();
        if (counted)
            foreach (var key in keys)
                AuthCalls.Call("triesCleared", new JsonObject { ["key"] = key }.ToJsonString());
        if (target == "desktop" && source.Remembers(inputs))
            Remember(conn, Column("secret"), id);
        return id;
    }

    /// <summary>
    /// After a sign-in on the desktop with <c>remember N days</c>: an entry signed with the
    /// person's stored secret in <paramref name="column"/> (of the strategy used), kept in the
    /// system's protected store. A strategy without a secret (<c>account</c>) remembers nothing; a
    /// store that cannot keep it leaves the sign-in as it is.
    /// </summary>
    void Remember(IDriver conn, string? column, long id)
    {
        if (source.RememberSeconds is not { } seconds || column is null)
            return;
        // Read after signing in: what signing in changed (a recovery code used up) is what holds.
        if (StoredSecret(conn, column, id) is not { } stored)
            return;
        try
        {
            var entry = JsonNode.Parse(AuthCalls.Call("rememberMake", new JsonObject { ["record"] = id, ["seconds"] = seconds, ["stored"] = stored }.ToJsonString()))!.GetValue<string>();
            Secrets.Set(source.Program, Clarity.Runtime.Signin.Remembered, entry);
        }
        catch (ClarityException)
        {
        }
    }

    public long Remembered(Db db)
    {
        if (source.RememberSeconds is null)
            return 0;
        try
        {
            if (Secrets.Get(source.Program, Clarity.Runtime.Signin.Remembered) is not { Length: > 0 } entry)
                return 0;
            var record = JsonNode.Parse(AuthCalls.Call("rememberRecord", new JsonObject { ["entry"] = entry }.ToJsonString()))?.GetValue<long>() ?? 0;
            if (record <= 0)
                return 0;
            var conn = db.Driver(source.Source);
            foreach (var column in SecretColumns)
                if (StoredSecret(conn, column, record) is { } stored
                    && JsonNode.Parse(AuthCalls.Call("rememberCheck", new JsonObject { ["entry"] = entry, ["stored"] = stored }.ToJsonString()))?.GetValue<long>() == record)
                    return record;
        }
        catch (ClarityException)
        {
        }
        return 0;
    }

    public void Forget()
    {
        if (source.RememberSeconds is null)
            return;
        try
        {
            Secrets.Set(source.Program, Clarity.Runtime.Signin.Remembered, "");
        }
        catch (ClarityException)
        {
        }
    }

    /// <summary>The stored secret in <paramref name="column"/> of the person <paramref name="id"/>; null when the record is gone or the column empty.</summary>
    string? StoredSecret(IDriver conn, string column, long id)
    {
        var d = conn.Dialect;
        var rows = conn.Rows($"SELECT {d.Quote(column)} FROM {d.Quote(source.Table)} WHERE {d.Quote(source.Key)} = {d.Mark(1)}", [id]);
        return rows.FirstOrDefault()?[0] is string { Length: > 0 } stored ? stored : null;
    }

    /// <summary>The key and the stored secret (empty without one) of the record whose <paramref name="column"/> equals <paramref name="equals"/>.</summary>
    (long Id, string Stored)? PersonBy(IDriver conn, string column, string? secret, string equals)
    {
        var d = conn.Dialect;
        var wanted = secret is null ? d.Quote(source.Key) : $"{d.Quote(source.Key)}, {d.Quote(secret)}";
        var rows = conn.Rows($"SELECT {wanted} FROM {d.Quote(source.Table)} WHERE {d.Quote(column)} = {d.Mark(1)}", [equals]);
        if (rows.Count == 0)
            return null;
        var row = rows[0];
        return (Sql.GetInteger(source.Key, row[0]), row.Length > 1 && row[1] is string stored ? stored : "");
    }

    bool NoPeople(IDriver conn)
    {
        var d = conn.Dialect;
        var rows = conn.Rows($"SELECT COUNT(*) FROM {d.Quote(source.Table)}", []);
        return Sql.GetInteger("count", rows.FirstOrDefault()?[0]) == 0;
    }

    /// <summary>Inserts the first person, with only the fields the strategy names.</summary>
    long FirstAccount(IDriver conn, Strategy info, SigninStrategy used, string strategy, JsonObject inputs)
    {
        var made = JsonNode.Parse(AuthCalls.Call("firstAccount", new JsonObject { ["strategy"] = strategy, ["inputs"] = inputs.DeepClone() }.ToJsonString()))!.AsObject();
        var columns = new List<string>();
        var values = new List<object?>();
        foreach (var (role, column) in info.Roles.Zip(used.Columns))
            if (made[role] is JsonValue value && value.TryGetValue<string>(out var text))
            {
                columns.Add(column);
                values.Add(text);
            }
        return conn.Insert(conn.Dialect.Insert(source.Table, source.Key, columns), values);
    }

    static bool Bool(string json) => JsonNode.Parse(json)?.GetValue<bool>() ?? false;

    public string OpenSession(long record) =>
        JsonNode.Parse(AuthCalls.Call("sessionOpen", new JsonObject { ["record"] = record, ["seconds"] = source.SessionSeconds }.ToJsonString()))!.GetValue<string>();

    public long FindSession(string token) =>
        JsonNode.Parse(AuthCalls.Call("sessionFind", new JsonObject { ["token"] = token }.ToJsonString()))?.GetValue<long>() ?? 0;

    public string Source => source.Source;

    public string Table => source.Table;

    public IReadOnlyList<string> SecretColumns { get; } =
    [
        .. source.Strategies
            .SelectMany(used => Strategy.Catalog.Value.FirstOrDefault(s => s.Name == used.Name) is { } info
                ? info.Roles.Zip(used.Columns).Where(p => p.First == "secret").Select(p => p.Second)
                : [])
            .Distinct(),
    ];

    public void CloseOtherSessions(long record, string token) =>
        AuthCalls.Call("sessionCloseOthers", new JsonObject { ["record"] = record, ["token"] = token }.ToJsonString());

    public void CloseAllSessions(long record) =>
        AuthCalls.Call("sessionCloseAll", new JsonObject { ["record"] = record }.ToJsonString());

    public void CloseSession(string token) => AuthCalls.Call("sessionClose", new JsonObject { ["token"] = token }.ToJsonString());
}
