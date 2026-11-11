// A served program whose people sign in (docs/APP-CAPABILITIES-PLAN.md,
// step 4): the browser's Cookie header reaches the program, signing in sets
// the session cookie and signing out clears it, and without a session only
// the sign-in window and its commands are answered.

using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;
using Clarity.Auth;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Ui.Server.Tests;

public class WebServerSigninTests
{
    static Db People()
    {
        Sqlite.Use();
        var db = Sqlite.Memory();
        db.Batch("", "CREATE TABLE members (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL DEFAULT '', email TEXT, password_hash TEXT)");
        db.Batch("", $"INSERT INTO members (name, email, password_hash) VALUES ('Lou', 'lou@signin.server', '{Signins.Hashed("argon2id", "lou")}')");
        db.AddSignin(new SigninSource("members", "SignIn") { Targets = ["web"], Strategies = [new SigninStrategy("password", ["email", "password_hash"])] });
        return db;
    }

    static string Post(string body, string? cookie = null) =>
        $"POST {WebServer.InvokePath} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n" + (cookie is null ? "" : $"Cookie: {cookie}\r\n") + $"Content-Length: {Encoding.UTF8.GetByteCount(body)}\r\n\r\n{body}";

    static async Task<string> Ask(int port, string request)
    {
        using var client = new TcpClient();
        await client.ConnectAsync("127.0.0.1", port);
        var stream = client.GetStream();
        await stream.WriteAsync(Encoding.UTF8.GetBytes(request));
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return await reader.ReadToEndAsync();
    }

    static string? SetCookie(string answer) =>
        answer.Split("\r\n").FirstOrDefault(l => l.StartsWith("Set-Cookie: ", StringComparison.Ordinal))?["Set-Cookie: ".Length..];

    [Fact]
    public async Task Signin_sets_the_session_cookie_the_next_requests_carry_and_signout_clears_it()
    {
        using var db = People();
        var server = new WebServer(new ShelfViewModel(db));
        using var listening = server.Start("127.0.0.1:0");
        var open = Wire.Command("open_window", new JsonObject { ["window"] = "Shelf" }, "a");
        Assert.Contains("\"notAllowed\"", await Ask(listening.Port, Post(open)));
        var refused = await Ask(listening.Port, Post(Wire.Command("signin", new JsonObject { ["strategy"] = "password", ["inputs"] = new JsonObject { ["name"] = "lou@signin.server", ["password"] = "no" } }, "a")));
        Assert.Contains("\"signinRefused\"", refused);
        Assert.Null(SetCookie(refused));
        var signed = await Ask(listening.Port, Post(Wire.Command("signin", new JsonObject { ["strategy"] = "password", ["inputs"] = new JsonObject { ["name"] = "lou@signin.server", ["password"] = "lou" } }, "a")));
        var cookie = SetCookie(signed);
        Assert.NotNull(cookie);
        Assert.Contains("HttpOnly; SameSite=Strict; Path=/; Max-Age=28800", cookie);
        var session = cookie.Split(';')[0];
        Assert.Contains("\"instance\"", await Ask(listening.Port, Post(open, session)));
        var gone = await Ask(listening.Port, Post(Wire.Command("signout", null, "a"), session));
        Assert.Equal("clarity_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0", SetCookie(gone));
        Assert.Contains("\"notAllowed\"", await Ask(listening.Port, Post(open, session)));
    }

    [Fact]
    public void Signin_failed_tries_count_per_address_the_request_came_from()
    {
        using var db = People();
        var server = new WebServer(new ShelfViewModel(db));
        string Try(string name, string password, string address) =>
            Encoding.UTF8.GetString(server.Answer("POST", WebServer.InvokePath, Wire.Command("signin", new JsonObject { ["strategy"] = "password", ["inputs"] = new JsonObject { ["name"] = name, ["password"] = password } }, "a"), address: address).Body);
        for (var i = 0; i < 5; i++)
            Assert.Contains("\"signinRefused\"", Try($"x{i}@signin.server", "x", "192.0.2.40"));
        Assert.Contains("\"signinTooManyTries\"", Try("lou@signin.server", "lou", "192.0.2.40"));
        Assert.DoesNotContain("\"error\"", Try("lou@signin.server", "lou", "192.0.2.41"));
    }
}
