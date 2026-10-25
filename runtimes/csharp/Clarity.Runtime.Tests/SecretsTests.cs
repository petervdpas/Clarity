// Secret settings in the system's protected store (DESIGN 5.27): kept, read
// back and forgotten, apart from settings.json. Needs a store, such as the
// Secret Service of a desktop session; without one it checks nothing.

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class SecretsTests
{
    const string Program = "clarity-runtime-test";

    static bool StoreWorks()
    {
        try
        {
            Secrets.Set(Program, "probe", "1");
            var works = Secrets.Get(Program, "probe") == "1";
            Secrets.Set(Program, "probe", "");
            return works;
        }
        catch (ValueException)
        {
            return false;
        }
    }

    /// <summary>With CLARITY_SECRET_STORE=1 the machine has a store, so the tests must reach it.</summary>
    [Fact]
    public void The_store_is_there_where_it_is_said_to_be()
    {
        if (Environment.GetEnvironmentVariable("CLARITY_SECRET_STORE") == "1")
            Assert.True(StoreWorks(), "the system's protected store cannot be reached");
    }

    /// <summary>On Windows a secret is kept as every stack keeps it (conformance/secret-store.json): target, user and UTF-8 blob.</summary>
    [Fact]
    public void The_credential_manager_shape_is_every_stack_s()
    {
        var cases = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "secret-store.json"))).RootElement.GetProperty("windows");
        foreach (var c in cases.EnumerateArray())
        {
            var value = c.GetProperty("value").GetString()!;
            Assert.Equal(c.GetProperty("target").GetString(), Secrets.WindowsTarget(c.GetProperty("program").GetString()!, c.GetProperty("name").GetString()!));
            var blob = Secrets.WindowsBlob(value);
            Assert.Equal(c.GetProperty("blob").GetString(), Convert.ToHexStringLower(blob));
            Assert.Equal(value, Secrets.FromWindowsBlob(blob));
        }
    }

    [Fact]
    public void A_secret_is_kept_read_and_forgotten()
    {
        if (!StoreWorks())
            return;
        Secrets.Set(Program, "token", "s3cr3t");
        Assert.Equal("s3cr3t", Secrets.Get(Program, "token"));
        Secrets.Set(Program, "token", "");
        Assert.Null(Secrets.Get(Program, "token"));
        // Forgetting what was never kept is no error.
        Secrets.Set(Program, "token", "");
    }

    [Fact]
    public void Settings_keep_their_secrets_out_of_the_file()
    {
        if (!StoreWorks())
            return;
        var home = Path.Combine(AppContext.BaseDirectory, "secret-settings");
        // Windows keeps settings under APPDATA, the others under XDG_CONFIG_HOME.
        var was = Environment.GetEnvironmentVariable("APPDATA");
        Environment.SetEnvironmentVariable("XDG_CONFIG_HOME", home);
        Environment.SetEnvironmentVariable("APPDATA", home);
        try
        {
            var program = "clarity-runtime-test-settings";
            KeptSettings.Keep(program, new JsonObject { ["pageSize"] = 20, ["apiToken"] = "t0k3n" }, ["apiToken"]);
            var file = File.ReadAllText(Path.Combine(home, program, "settings.json"));
            Assert.DoesNotContain("t0k3n", file);
            Assert.DoesNotContain("apiToken", file);
            var read = KeptSettings.Read(program, ["apiToken"]);
            Assert.Equal("t0k3n", read["apiToken"]!.GetValue<string>());
            Assert.Equal(20, read["pageSize"]!.GetValue<int>());
            Secrets.Set(program, "apiToken", "");
        }
        finally
        {
            Environment.SetEnvironmentVariable("APPDATA", was);
        }
    }

    /// <summary>A database connects with a secret setting (DESIGN 5.27): its connection, password and all, kept in the protected store.</summary>
    [Fact]
    public void A_database_connects_with_a_secret_setting()
    {
        if (!StoreWorks())
            return;
        var file = Path.Combine(AppContext.BaseDirectory, "secret-connection.db");
        Secrets.Set(Program, "shopDb", file);
        try
        {
            using var db = Db.Connect(("main", Dialect.Sqlite, $"setting:{Program}/shopDb"));
        }
        finally
        {
            Secrets.Set(Program, "shopDb", "");
        }
        var missing = Assert.Throws<NotKeptException>(() => Db.Connect(("main", Dialect.Sqlite, $"setting:{Program}/shopDb")));
        Assert.Contains("the secret setting shopDb is not kept yet", missing.Message);
    }
}
