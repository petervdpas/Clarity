// Undo puts back what code set on parts in the edit it takes back: a custom
// part's values, a caption, visible, enabled, readonly and an error, as every
// runtime does (conformance/edit-bus.json, `states`).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

sealed class StatesViewModel(Db db) : WindowViewModel(db)
{
    public long Count { get; set; }
    public string Name { get; set; } = "";
    public PartState Board { get; } = new("board");
    public PartState NameBox { get; } = new("nameBox");
    public PartState SaveButton { get; } = new("saveButton");
    public PartState Hint { get; } = new("hint");
}

public sealed class EditStateTests
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "edit-bus.json"))).RootElement;

    static string Member(string name) => char.ToUpperInvariant(name[0]) + name[1..];

    static object? Value(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Number => value.GetInt64(),
        JsonValueKind.String => value.GetString(),
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        JsonValueKind.Array => value.EnumerateArray().Select(Value).ToList(),
        _ => null,
    };

    static PartState Part(StatesViewModel window, string name) => (PartState)typeof(StatesViewModel).GetProperty(Member(name))!.GetValue(window)!;

    /// Gives the window a state as a case writes it.
    static void Give(StatesViewModel window, JsonElement state)
    {
        foreach (var d in state.GetProperty("data").EnumerateObject())
            typeof(StatesViewModel).GetProperty(Member(d.Name))!.SetValue(window, d.Value.ValueKind == JsonValueKind.Number ? d.Value.GetInt64() : d.Value.GetString());
        foreach (var p in state.GetProperty("parts").EnumerateObject())
        {
            var part = Part(window, p.Name);
            foreach (var f in p.Value.EnumerateObject())
            {
                switch (f.Name)
                {
                    case "text": part.Text = f.Value.GetString()!; break;
                    case "caption": part.CaptionText = Text.Plain(f.Value.GetString()!); break;
                    case "error": part.ErrorText = Text.Plain(f.Value.GetString()!); break;
                    case "visible": part.IsVisible = f.Value.GetBoolean(); break;
                    case "enabled": part.IsEnabled = f.Value.GetBoolean(); break;
                    case "readonly": part.IsReadOnly = f.Value.GetBoolean(); break;
                    case "values":
                        foreach (var v in f.Value.EnumerateObject())
                            part.SetValue(v.Name, Value(v.Value));
                        break;
                }
            }
        }
    }

    /// Each part field the expected state names is as given; null is the design.
    static void Expect(StatesViewModel window, JsonElement state, string at)
    {
        foreach (var d in state.GetProperty("data").EnumerateObject())
            Assert.True(Equals(typeof(StatesViewModel).GetProperty(Member(d.Name))!.GetValue(window), d.Value.ValueKind == JsonValueKind.Number ? d.Value.GetInt64() : d.Value.GetString()), $"{at}: {d.Name}");
        foreach (var p in state.GetProperty("parts").EnumerateObject())
        {
            var part = Part(window, p.Name);
            foreach (var f in p.Value.EnumerateObject())
            {
                var none = f.Value.ValueKind == JsonValueKind.Null;
                var where = $"{at}: {p.Name}.{f.Name}";
                switch (f.Name)
                {
                    case "text": Assert.True(part.Text == f.Value.GetString(), where); break;
                    case "caption": Assert.True(part.Caption == (none ? "" : f.Value.GetString()), where); break;
                    case "error": Assert.True(part.Error == (none ? "" : f.Value.GetString()), where); break;
                    case "visible": Assert.True(part.IsVisible == (none || f.Value.GetBoolean()), where); break;
                    case "enabled": Assert.True(part.IsEnabled == (none || f.Value.GetBoolean()), where); break;
                    case "readonly": Assert.True(part.IsReadOnly == (!none && f.Value.GetBoolean()), where); break;
                    case "values":
                        foreach (var v in f.Value.EnumerateObject())
                            Assert.True(JsonSerializer.Serialize(part.Values.GetValueOrDefault(v.Name)) == v.Value.GetRawText().Replace(" ", ""), $"{where}.{v.Name}: {JsonSerializer.Serialize(part.Values.GetValueOrDefault(v.Name))}");
                        break;
                }
            }
        }
    }

    [Fact]
    public void An_undo_puts_back_what_code_set_on_parts_as_every_runtime_does()
    {
        foreach (var c in Cases().GetProperty("states").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            using var db = Sqlite.Memory();
            var window = new StatesViewModel(db);
            Give(window, c.GetProperty("start"));
            window.Opened();
            var i = 0;
            foreach (var s in c.GetProperty("steps").EnumerateArray())
            {
                var at = $"{name}, step {++i}";
                if (s.TryGetProperty("edit", out var edit))
                {
                    Give(window, edit.GetProperty("state"));
                    window.Edited(edit.GetProperty("part").GetString()!, edit.GetProperty("scope").GetString()!);
                }
                else if (s.TryGetProperty("reply", out var reply))
                    window.Handle(() => Give(window, reply.GetProperty("state")));
                else
                {
                    var back = s.TryGetProperty("undo", out var scope);
                    if (!back)
                        scope = s.GetProperty("redo");
                    window.UndoEdit(scope.GetString()!, back);
                    Expect(window, s.GetProperty("state"), at);
                }
            }
        }
    }
}
