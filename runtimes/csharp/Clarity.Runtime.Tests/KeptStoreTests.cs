// Settings kept in a store instead of a file (DESIGN 7.6): in a browser, its
// own storage, where the person's settings stay without the secret ones.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

[Collection("kept settings")]
public class KeptStoreTests
{
    [Fact]
    public void Settings_are_kept_in_the_store_when_there_is_one_and_its_secrets_are_not()
    {
        var stored = new Dictionary<string, string>();
        KeptSettings.Store = (key => stored.GetValueOrDefault(key), (key, value) => stored[key] = value);
        try
        {
            Assert.Empty(KeptSettings.Read("walker"));
            KeptSettings.Keep("walker", new JsonObject { ["speed"] = 3, ["token"] = "s3cret" }, ["token"]);
            Assert.Equal(["settings:walker"], stored.Keys);
            Assert.DoesNotContain("s3cret", stored["settings:walker"]);
            var read = KeptSettings.Read("walker", ["token"]);
            Assert.Equal(3, read["speed"]!.GetValue<int>());
            Assert.False(read.ContainsKey("token"));
        }
        finally
        {
            KeptSettings.Store = null;
        }
    }
}
