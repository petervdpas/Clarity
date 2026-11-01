// Why the databases did not open, in Clarity's own words that the project
// translates, as every runtime says it (conformance/unconnected.json).

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public sealed class UnconnectedTests
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "unconnected.json"))).RootElement;

    static void SameText(Text got, JsonElement want)
    {
        Assert.Equal(want.GetProperty("key").GetString(), got.Key);
        var values = want.TryGetProperty("values", out var v) ? v.EnumerateObject().ToDictionary(p => p.Name, p => p.Value.GetProperty("value").GetString()) : [];
        Assert.Equal(values, got.Values.ToDictionary(p => p.Key, p => (string?)p.Value.Value));
    }

    // Why the databases did not open, in Clarity's own words that the project
    // translates, as every runtime says it (conformance/unconnected.json).
    [Fact]
    public void A_secret_connection_not_kept_says_so()
    {
        var notKept = Cases().GetProperty("notKept");
        var e = Assert.ThrowsAny<Exception>(() => Db.Connect([("main", Dialect.Sqlite, notKept.GetProperty("location").GetString()!)]));
        SameText(Unconnecting.Why(e), notKept.GetProperty("reason"));
        // Its message stays as it was, for logs.
        Assert.Contains("the secret setting neverKept is not kept yet", e.Message);
    }

    [Fact]
    public void Any_other_failure_says_the_runtimes_message()
    {
        var failed = Cases().GetProperty("failed");
        SameText(Unconnecting.Why(new StorageException(failed.GetProperty("storage").GetString()!)), failed.GetProperty("reason"));
    }

    // Until the project gives them a value, Clarity's words read as their English.
    [Fact]
    public void Why_reads_in_english_until_the_project_translates_it()
    {
        var words = Cases().GetProperty("words");
        Assert.Equal(words.GetProperty("unconnectedNotKept").GetString(), Locale.Show(Unconnecting.Why(new NotKeptException("shopDb"))));
        Assert.Equal(words.GetProperty("unconnectedFailed").GetString()!.Replace("{problem}", "server down"), Locale.Show(Unconnecting.Why(new StorageException("server down"))));
    }
}
