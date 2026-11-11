// Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4): people
// signing in through Clarity.Auth, as every runtime does
// (conformance/signin.json), and the program's start and wire answering it.

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Auth;
using Clarity.Runtime;

namespace Clarity.Ui.Native.Tests;

sealed record Member : IRecord<Member>
{
    public static string SourceName => "";
    public static string TableName => "members";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name", "email"];
    public long Id { get; set; }
    public string Name { get; set; } = "";
    public string? Email { get; set; }
    public object?[] ToValues() => [Name, Email];
    public static Member FromValues(long id, object?[] values) => new() { Id = id, Name = Sql.GetText("name", values[0]), Email = values[1] as string };
}

public sealed class SigninTests
{
    static JsonObject Conformance()
    {
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir is not null; dir = dir.Parent)
        {
            var file = Path.Combine(dir.FullName, "conformance", "signin.json");
            if (File.Exists(file))
                return JsonNode.Parse(File.ReadAllText(file))!.AsObject();
        }
        throw new FileNotFoundException("signin.json");
    }

    /// <summary>The signin block of the conformance file, as the generator writes it.</summary>
    static SigninSource Source(JsonObject all, IReadOnlyList<string>? targets = null)
    {
        var block = all["signin"]!.AsObject();
        return new SigninSource(all["people"]!["table"]!.GetValue<string>(), block["page"]!.GetValue<string>())
        {
            Key = all["people"]!["key"]!.GetValue<string>(),
            Targets = targets ?? [.. block["targets"]!.AsArray().Select(t => t!.GetValue<string>())],
            Strategies = [.. block["strategies"]!.AsArray().Select(s => new SigninStrategy(s!["name"]!.GetValue<string>(), [.. s["columns"]!.AsArray().Select(c => c!.GetValue<string>())]))],
            SessionSeconds = block["sessionMinutes"]!.GetValue<long>() * 60,
            Tries = block["tries"]!.GetValue<int>(),
            TriesSeconds = block["triesMinutes"]!.GetValue<long>() * 60,
            Program = block["program"]!.GetValue<string>(),
        };
    }

    /// <summary>A database of the conformance file's people with <paramref name="rows"/>.</summary>
    static Db People(JsonObject all, JsonArray rows, IReadOnlyList<string>? targets = null)
    {
        Sqlite.Use();
        var db = Sqlite.Memory();
        db.Batch("", all["people"]!["create"]!.GetValue<string>());
        foreach (var row in rows.Select(r => r!.AsObject()))
        {
            var columns = row.Select(p => p.Key).ToList();
            var values = row.Select(p => p.Value switch
            {
                JsonObject hashed => "'" + Signins.Hashed(hashed["hash"]!.GetValue<string>(), hashed["text"]!.GetValue<string>()).Replace("'", "''") + "'",
                var v => "'" + v!.GetValue<string>().Replace("'", "''") + "'",
            });
            db.Batch("", $"INSERT INTO members ({string.Join(", ", columns)}) VALUES ({string.Join(", ", values)})");
        }
        db.AddSignin(Source(all, targets));
        return db;
    }

    static long Count(Db db) => db.All<Member>().Count;

    [Fact]
    public void Signin_conformance_every_case_as_every_runtime()
    {
        var all = Conformance();
        foreach (var c in all["cases"]!.AsArray().Select(c => c!.AsObject()))
        {
            var name = c["name"]!.GetValue<string>();
            using var db = People(all, c["rows"]!.AsArray());
            // remember N days keeps its entry in the secret store, here in memory, empty per case.
            // remember N days when asked keeps it only when the input remember is "true".
            var source = Source(all) with { RememberSeconds = c["rememberDays"]?.GetValue<long>() * 24 * 3600, RememberAsks = c["rememberAsks"]?.GetValue<bool>() ?? false };
            var kept = new Dictionary<string, string>();
            Secrets.Store = ((program, key) => kept.GetValueOrDefault($"{program}:{key}"), (program, key, value) => kept[$"{program}:{key}"] = value);
            db.AddSignin(source);
            var served = c["served"]?.GetValue<bool>() ?? false;
            var address = c["address"]?.GetValue<string>() ?? "";
            var cookies = new Dictionary<string, string?>();
            foreach (var step in c["steps"]!.AsArray().Select(s => s!.AsObject()))
            {
                // Each step one call; served, of its browser, carrying the cookie the program set it last.
                var browser = step["browser"]?.GetValue<string>() ?? "a";
                if (step.ContainsKey("restart"))
                {
                    // The program starts anew on the same database and secret store.
                    db.AddSignin(source);
                    var page = db.SigninFirst();
                    Assert.True(page == step["page"]?.GetValue<string>(), $"{name}: {step.ToJsonString()}: starts with {page}");
                    continue;
                }
                if (step["remembered"] is { } remembered)
                {
                    var held = kept.GetValueOrDefault($"{source.Program}:{Signin.Remembered}") is { Length: > 0 };
                    Assert.True(held == remembered.GetValue<bool>(), $"{name}: {step.ToJsonString()}: remembered {held}");
                    continue;
                }
                var (seen, setCookie) = served
                    ? db.Served(cookies.GetValueOrDefault(browser), address, () => Step(db, step))
                    : (Step(db, step), null);
                if (setCookie is not null)
                    cookies[browser] = setCookie.Contains("Max-Age=0") ? null : setCookie.Split(';')[0];
                Assert.True(seen.Ok, $"{name}: {step.ToJsonString()}: {seen.Why}");
            }
        }
        Secrets.Store = null;
    }

    static (bool Ok, string Why) Step(Db db, JsonObject step)
    {
        if (step["signin"] is { } strategy)
        {
            var refused = step["refused"]?.GetValue<string>();
            try
            {
                var id = db.Signin(strategy.GetValue<string>(), step["inputs"]!.AsObject());
                if (refused is not null)
                    return (false, $"signed {id} in instead of {refused}");
                return step["me"] is { } me ? (me.GetValue<long>() == id && db.MeId == id, $"signed {id} in") : (true, "");
            }
            catch (RefusedException e)
            {
                return (e.Text.Key == refused, $"refused with {e.Text.Key}");
            }
        }
        if (step.ContainsKey("signout"))
        {
            db.Signout();
            return (true, "");
        }
        if (step.ContainsKey("me"))
        {
            var wanted = step["me"]?.GetValue<long>();
            return (db.MeId == wanted, $"me is {db.MeId}");
        }
        if (step["hashed"] is JsonObject hashed)
        {
            var refused = step["refused"]?.GetValue<string>();
            try
            {
                var kept = Signins.Hashed(hashed["method"]!.GetValue<string>(), hashed["text"]!.GetValue<string>(), hashed["minimum"]?.GetValue<long>());
                return (refused is null && kept.Length > 0 && kept != hashed["text"]!.GetValue<string>(), $"kept {kept}");
            }
            catch (RefusedException e)
            {
                var minimum = e.Text.Values.TryGetValue("minimum", out var slot) ? slot.Value : null;
                return (e.Text.Key == refused && Equals(minimum, step["minimum"]?.GetValue<long>()), $"refused with {e.Text.Key} ({minimum})");
            }
        }
        if (step["save"] is JsonObject save)
        {
            var person = db.Get<Person>(save["id"]!.GetValue<long>());
            foreach (var (column, value) in save.Where(p => p.Key != "id"))
            {
                var text = value is JsonObject h ? Signins.Hashed(h["hash"]!.GetValue<string>(), h["text"]!.GetValue<string>()) : value!.GetValue<string>();
                person.Values[Person.ColumnNames.ToList().IndexOf(column)] = text;
            }
            db.Save(person);
            return (true, "");
        }
        if (step["count"] is { } count)
            return (Count(db) == count.GetValue<long>(), $"{Count(db)} rows");
        return (false, "an unknown step");
    }

    [Fact]
    public void Signin_refusals_are_Clarity_s_words_by_key()
    {
        var all = Conformance();
        foreach (var (key, english) in all["refusals"]!.AsObject())
        {
            var text = key switch
            {
                "signinRefused" => Signin.Refused(),
                "signinTooManyTries" => Signin.TooManyTries(),
                "notAllowed" => Signin.NotAllowed(),
                "notYours" => Signin.NotYours(),
                "secretTooShort" => Signin.SecretTooShort(8),
                _ => throw new InvalidOperationException(key),
            };
            Assert.Equal(key, text.Key);
            Assert.Equal(english!.GetValue<string>(), text.Template);
        }
        Assert.Equal("notAllowed", Assert.Throws<RefusedException>(() => Signin.Allow(false)).Text.Key);
        Assert.Equal("notYours", Assert.Throws<RefusedException>(() => Signin.Yours(false)).Text.Key);
        Signin.Allow(true);
        Signin.Yours(true);
    }

    [Fact]
    public void Signin_me_is_the_record_signed_in_and_nobody_is_refused()
    {
        var all = Conformance();
        using var db = People(all, [new JsonObject { ["name"] = "Gus", ["email"] = "gus@signin.csharp", ["password_hash"] = new JsonObject { ["hash"] = "argon2id", ["text"] = "g" } }]);
        Assert.Equal("notAllowed", Assert.Throws<RefusedException>(() => db.Me<Member>()).Text.Key);
        db.Signin("password", new JsonObject { ["name"] = "gus@signin.csharp", ["password"] = "g" });
        Assert.Equal("Gus", db.Me<Member>().Name);
    }

    [Fact]
    public void Signin_hashed_text_is_kept_by_its_method_and_tokens_are_new()
    {
        Assert.Equal(Signins.Hashed("sha256", "abc"), Signins.Hashed("sha256", "abc"));
        Assert.StartsWith("$argon2id$", Signins.Hashed("argon2id", "abc"));
        Assert.NotEqual(Signins.NewToken(), Signins.NewToken());
        Assert.Throws<ValueException>(() => Signins.Hashed("rot13", "abc"));
    }

    [Fact]
    public void Signin_first_on_the_desktop_signs_the_computer_s_account_in_by_itself()
    {
        var all = Conformance();
        // A record of the computer's user: the account strategy makes it the first time.
        using var db = People(all, []);
        Assert.Equal(1, db.Signin("account", []));
        db.Signout();
        Assert.Null(db.MeId);
        Assert.Null(db.SigninFirst());
        Assert.Equal(1, db.MeId);
    }

    [Fact]
    public void Signin_first_names_the_page_while_nobody_is_signed_in()
    {
        var all = Conformance();
        using var db = People(all, [new JsonObject { ["name"] = "Hal", ["login"] = "nobody-of-this-computer" }]);
        Assert.Equal("SignIn", db.SigninFirst());
        using var web = People(all, [], targets: ["web"]);
        Assert.Null(web.SigninFirst());
        Assert.Equal("SignIn", web.Served(null, "192.0.2.20", web.SigninFirst).Answer);
    }

    [Fact]
    public void Signin_on_the_command_line_by_token_user_or_account()
    {
        var all = Conformance();
        using var db = People(all,
        [
            new JsonObject { ["name"] = "Ivy", ["email"] = "ivy@signin.csharp", ["password_hash"] = new JsonObject { ["hash"] = "argon2id", ["text"] = "ivy" } },
            new JsonObject { ["name"] = "Bot", ["api_token"] = new JsonObject { ["hash"] = "sha256", ["text"] = "token-of-the-csharp-bot" } },
        ], targets: ["desktop", "cli"]);
        Assert.Equal(2, db.SigninCli(new CliSignin(null, "token-of-the-csharp-bot"), () => throw new InvalidOperationException("no password asked")));
        Assert.Equal(1, db.SigninCli(new CliSignin("ivy@signin.csharp", null), () => "ivy"));
        Assert.Equal("signinRefused", Assert.Throws<RefusedException>(() => db.SigninCli(new CliSignin("ivy@signin.csharp", null), () => "wrong")).Text.Key);
        // No password typed (the input ended) refuses.
        Assert.Equal("signinRefused", Assert.Throws<RefusedException>(() => db.SigninCli(new CliSignin("ivy@signin.csharp", null), () => null)).Text.Key);
        // There are people, none of them the computer's user.
        Assert.Equal("signinRefused", Assert.Throws<RefusedException>(() => db.SigninCli(new CliSignin(null, null), () => "")).Text.Key);
        using var desktop = People(all, []);
        Assert.Null(desktop.SigninCli(new CliSignin(null, null), () => ""));
    }

    [Fact]
    public void Signin_hashed_with_a_minimum_refuses_shorter_text()
    {
        var refused = Assert.Throws<RefusedException>(() => Signins.Hashed("argon2id", "short", 8));
        Assert.Equal("secretTooShort", refused.Text.Key);
        Assert.Equal(8L, refused.Text.Values["minimum"].Value);
        Assert.StartsWith("$argon2id$", Signins.Hashed("argon2id", "long enough", 8));
        Assert.Equal(3, Signins.NewRecoveryCodes(3).Split('\n', StringSplitOptions.RemoveEmptyEntries).Length);
    }

    [Fact]
    public void Signin_cli_reads_user_and_token_from_the_command_line()
    {
        var given = CliSignin.FromArgs(["tasks", "list", "--user", "ann", "--token", "t1"]);
        Assert.Equal(new CliSignin("ann", "t1"), given);
        Assert.Null(CliSignin.FromArgs(["tasks", "--user"]).User);
    }

    // -- The program's start and wire ----------------------------------------------------

    static ProgramWire Wire(Db db) =>
        new(typeof(SigninTests).Assembly, new PeopleViewModel(db), n => n == "People" ? PeopleViewModel.Markup : null);

    static JsonObject Answered(JsonObject? answer)
    {
        Assert.NotNull(answer);
        return answer;
    }

    [Fact]
    public void Signin_started_reports_the_signin_page_and_the_wire_signs_in_and_out()
    {
        var all = Conformance();
        using var db = People(all, [new JsonObject { ["name"] = "Jo", ["email"] = "jo@signin.csharp", ["password_hash"] = new JsonObject { ["hash"] = "argon2id", ["text"] = "jo" } }]);
        var wire = Wire(db);
        Assert.Equal("SignIn", Answered(wire.Answer("started", "{}"))["value"]!["signin"]!.GetValue<string>());
        var refused = Answered(wire.Answer("signin", """{"strategy":"password","inputs":{"name":"jo@signin.csharp","password":"no"}}"""));
        Assert.Equal("signinRefused", refused["error"]!["key"]!.GetValue<string>());
        var signed = Answered(wire.Answer("signin", """{"strategy":"password","inputs":{"name":"jo@signin.csharp","password":"jo"}}"""));
        Assert.Null(signed["error"]);
        Assert.Equal(1, db.MeId);
        Assert.Null(Answered(wire.Answer("started", "{}"))["value"]!["signin"]);
        Assert.Null(Answered(wire.Answer("signout", "{}"))["error"]);
        Assert.Null(db.MeId);
    }

    [Fact]
    public void Signin_started_without_people_reports_no_signin()
    {
        var wire = new ProgramWire(typeof(SigninTests).Assembly, new PeopleViewModel(Db.Empty()), _ => null);
        var started = Answered(wire.Answer("started", "{}"))["value"]!.AsObject();
        Assert.True(started.ContainsKey("signin"));
        Assert.Null(started["signin"]);
    }

    [Fact]
    public void Signin_served_without_a_session_answers_only_the_signin_page_and_its_commands()
    {
        var all = Conformance();
        using var db = People(all, [new JsonObject { ["name"] = "Kim", ["email"] = "kim@signin.csharp", ["password_hash"] = new JsonObject { ["hash"] = "argon2id", ["text"] = "kim" } }], targets: ["web"]);
        var wire = new ProgramWire(typeof(SigninTests).Assembly, new PeopleViewModel(db), n => n == "People" ? PeopleViewModel.Markup : null) { Served = true };
        var started = wire.AnswerFor("b1", null, "started", "{}", null, "192.0.2.30");
        Assert.Equal("SignIn", started.Answer!["value"]!["signin"]!.GetValue<string>());
        Assert.Null(started.SetCookie);
        var held = wire.AnswerFor("b1", null, "open_window", """{"window":"People","with":{}}""", null, "192.0.2.30");
        Assert.Equal("notAllowed", held.Answer!["error"]!["key"]!.GetValue<string>());
        var signed = wire.AnswerFor("b1", null, "signin", """{"strategy":"password","inputs":{"name":"kim@signin.csharp","password":"kim"}}""", null, "192.0.2.30");
        Assert.Null(signed.Answer!["error"]);
        Assert.Matches("^clarity_session=[^;]+; HttpOnly; SameSite=Strict; Path=/; Max-Age=28800$", signed.SetCookie);
        var cookie = signed.SetCookie!.Split(';')[0];
        var opened = wire.AnswerFor("b1", null, "open_window", """{"window":"People","with":{}}""", "other=1; " + cookie, "192.0.2.30");
        Assert.Null(opened.Answer!["error"]);
        Assert.Null(wire.AnswerFor("b1", null, "started", "{}", cookie, "192.0.2.30").Answer!["value"]!["signin"]);
        var gone = wire.AnswerFor("b1", null, "signout", "{}", cookie, "192.0.2.30");
        Assert.Equal("clarity_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0", gone.SetCookie);
        Assert.Equal("notAllowed", wire.AnswerFor("b1", null, "open_window", """{"window":"People","with":{}}""", cookie, "192.0.2.30").Answer!["error"]!["key"]!.GetValue<string>());
        // Nobody signed in on the server's own handle meanwhile.
        Assert.Null(db.MeId);
    }

    [Fact]
    public void Signin_a_window_s_allow_is_checked_before_it_opens()
    {
        var all = Conformance();
        using var db = People(all, [new JsonObject { ["name"] = "Lee", ["email"] = "lee@signin.csharp", ["password_hash"] = new JsonObject { ["hash"] = "argon2id", ["text"] = "lee" } }]);
        var wire = new ProgramWire(typeof(SigninTests).Assembly, new PeopleViewModel(db), n => n is "People" or "Guarded" ? PeopleViewModel.Markup : null);
        // Nobody signed in: the window refuses.
        Assert.Equal("notAllowed", Answered(wire.Answer("open_window", """{"window":"Guarded","with":{}}"""))["error"]!["key"]!.GetValue<string>());
        Assert.Null(Answered(wire.Answer("signin", """{"strategy":"password","inputs":{"name":"lee@signin.csharp","password":"lee"}}"""))["error"]);
        Assert.Null(Answered(wire.Answer("open_window", """{"window":"Guarded","with":{}}"""))["error"]);
        // A window without a condition opens for anyone.
        Assert.Null(Answered(wire.Answer("open_window", """{"window":"People","with":{}}"""))["error"]);
    }

    [Fact]
    public void Signin_session_of_reads_the_cookie_header()
    {
        Assert.Equal("abc", Signin.SessionOf("a=1; clarity_session=abc; b=2"));
        Assert.Null(Signin.SessionOf("clarity_session="));
        Assert.Null(Signin.SessionOf(null));
        Assert.Null(Signin.SessionOf("other_clarity_session=abc"));
    }
}

/// <summary>A window of a program whose people sign in.</summary>
public sealed class PeopleViewModel(Db db) : WindowViewModel(db)
{
    public const string Markup = """
        <Window name="People">
          <Label name="hello" at="1,1" span="6" caption="Hello" />
        </Window>
        """;
}

/// <summary>A window with <c>allow when</c>: only someone signed in opens it.</summary>
public sealed class GuardedViewModel(Db db) : WindowViewModel(db)
{
    public override void Allow() => Signin.Allow(Db.MeId is not null);
}

/// <summary>A record of the conformance file's table of people, every column as text.</summary>
sealed record Person : IRecord<Person>
{
    public static string SourceName => "";
    public static string TableName => "members";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name", "email", "login", "password_hash", "api_token", "recovery_codes"];
    public long Id { get; set; }
    public object?[] Values { get; init; } = new object?[6];
    public object?[] ToValues() => [.. Values];
    public static Person FromValues(long id, object?[] values) => new() { Id = id, Values = [.. values] };
}
