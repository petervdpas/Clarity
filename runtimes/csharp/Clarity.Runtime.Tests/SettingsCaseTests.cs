// The settings against the cases every stack shares
// (conformance/settings.json): which kept values fit, which a start takes
// back, and where an application keeps them.

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

// With the other tests of kept settings, one at a time: KeptSettings.Store is shared.
[Collection("kept settings")]
public class SettingsCaseTests
{
    static readonly JsonNode File = JsonNode.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "settings.json")))!;

    static Setting SettingOf(JsonNode s)
    {
        IReadOnlyList<string>? List(string name) => s[name]?.AsArray().Select(v => v!.GetValue<string>()).ToList();
        return new Setting(s["type"]!.GetValue<string>(), s["min"]?.GetValue<string>(), s["max"]?.GetValue<string>(), List("values") ?? List("languages") ?? List("themes"));
    }

    [Fact]
    public void A_kept_value_fits_its_setting_as_on_every_stack()
    {
        foreach (var c in File["fits"]!.AsArray())
            Assert.True(KeptSettings.Fits(SettingOf(c!["setting"]!), c["value"]) == c["fits"]!.GetValue<bool>(), c.ToJsonString());
    }

    [Fact]
    public void A_start_takes_back_only_the_kept_values_that_still_fit()
    {
        foreach (var c in File["kept"]!.AsArray())
        {
            var settings = c!["settings"]!.AsObject().ToDictionary(p => p.Key, p => SettingOf(p.Value!));
            var taken = KeptSettings.Taken(settings, c["kept"]!.AsObject());
            Assert.True(JsonNode.DeepEquals(taken, c["takes"]), $"{taken.ToJsonString()} for {c.ToJsonString()}");
        }
    }

    [Fact]
    public void The_settings_file_is_where_every_runtime_puts_it()
    {
        foreach (var c in File["file"]!.AsArray())
        {
            var env = c!["env"]!.AsObject();
            var found = KeptSettings.FileIn(c["os"]!.GetValue<string>(), name => env[name]?.GetValue<string>(), c["program"]!.GetValue<string>());
            Assert.Equal(c["file"]?.GetValue<string>(), found);
        }
    }

    [Fact]
    public void Settings_kept_are_read_back()
    {
        var folder = Path.Combine(Path.GetTempPath(), $"clarity-settings-{Environment.ProcessId}");
        Environment.SetEnvironmentVariable(OperatingSystem.IsWindows() ? "APPDATA" : "XDG_CONFIG_HOME", folder);
        const string program = "clarity-settings-test";
        Assert.Empty(KeptSettings.Read(program));
        var values = new JsonObject { ["pageSize"] = 20, ["language"] = "nl" };
        KeptSettings.Keep(program, values);
        Assert.True(JsonNode.DeepEquals(values, KeptSettings.Read(program)));
        Directory.Delete(folder, recursive: true);
    }
}
