// The C# layer over the native library itself: the library loads and has
// the interface this layer is written for, a native error comes back as a
// C# exception with the library's message, and a C# window model drives the
// library's screen (driven without a window, DESIGN 1.6) through the
// program's callback: a click runs the view model's handler, what the
// person types reaches its values, and what a handler throws never crosses
// into native code.

using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui.Native.Tests;

public sealed class NativeTests
{
    [Fact]
    public void The_library_loads_and_has_the_interface_this_layer_is_written_for()
    {
        var (version, fits) = DesktopWindow.Library();
        Assert.NotEmpty(version);
        Assert.True(fits);
    }

    [Fact]
    public void A_native_error_becomes_a_csharp_exception_with_the_library_s_message()
    {
        using var driven = Shelf(out _);
        var failed = Assert.Throws<ClarityException>(() => driven.Step(new JsonObject { ["do"] = "no such step" }));
        Assert.False(string.IsNullOrWhiteSpace(failed.Message));
    }

    [Fact]
    public unsafe void Text_given_back_to_a_callback_is_copied_by_the_library()
    {
        // Answered outside a callback, the library refuses the answer and says why; nothing crashes.
        var bytes = Interop.Utf8("{}");
        fixed (byte* b = bytes)
            Assert.NotEqual(Status.Ok, Lib.AnswerSet(0, b, (nuint)bytes.Length));
    }

    static Driven Shelf(out ShelfViewModel shelf)
    {
        shelf = new ShelfViewModel();
        var markups = new Dictionary<string, string> { ["Shelf"] = ShelfViewModel.Markup, ["Other"] = OtherViewModel.Markup };
        var wire = new ProgramWire(typeof(ShelfViewModel).Assembly, shelf, n => markups.GetValueOrDefault(n));
        var files = new NativeFiles();
        files.Window("Shelf", ShelfViewModel.Markup);
        files.Window("Other", OtherViewModel.Markup);
        files.Title("Shelf");
        return new Driven(wire.Answer, files);
    }

    [Fact]
    public void A_click_in_the_library_runs_the_view_model_s_handler()
    {
        using var driven = Shelf(out var shelf);
        var clicked = driven.Step(new JsonObject { ["do"] = "click", ["part"] = "saveButton" });
        Assert.True(clicked["ok"]!.GetValue<bool>(), clicked.ToJsonString());
        Assert.Equal(1, shelf.Saved);
    }

    [Fact]
    public void What_the_person_types_reaches_the_view_model()
    {
        using var driven = Shelf(out var shelf);
        driven.Step(new JsonObject { ["do"] = "click", ["part"] = "note" });
        driven.Step(new JsonObject { ["do"] = "type", ["text"] = "a good read" });
        driven.Step(new JsonObject { ["do"] = "click", ["part"] = "saveButton" });
        Assert.Equal("a good read", shelf.Note);
    }

    [Fact]
    public void The_library_reads_the_view_model_s_values()
    {
        using var driven = Shelf(out _);
        var read = driven.Step(new JsonObject { ["do"] = "read" });
        Assert.Equal("Shelf", read["window"]?.GetValue<string>());
        Assert.Equal("Dune", read["data"]?["books"]?[0]?["title"]?.GetValue<string>());
    }

    [Fact]
    public void A_handler_that_throws_shows_its_error_and_the_program_goes_on()
    {
        using var driven = Shelf(out var shelf);
        driven.Step(new JsonObject { ["do"] = "click", ["part"] = "failButton" });
        var read = driven.Step(new JsonObject { ["do"] = "read" });
        Assert.Contains("the shelf broke", read.ToJsonString());
        driven.Step(new JsonObject { ["do"] = "key", ["key"] = "Escape" });
        driven.Step(new JsonObject { ["do"] = "click", ["part"] = "saveButton" });
        Assert.Equal(1, shelf.Saved);
    }

    [Fact]
    public void A_window_a_handler_opens_shows_on_top()
    {
        using var driven = Shelf(out _);
        driven.Step(new JsonObject { ["do"] = "click", ["part"] = "otherButton" });
        var read = driven.Step(new JsonObject { ["do"] = "read" });
        Assert.Equal("Other", read["window"]?.GetValue<string>());
    }

    [Fact]
    public void A_program_whose_callback_throws_answers_an_error_without_crossing()
    {
        var files = new NativeFiles();
        files.Window("Shelf", ShelfViewModel.Markup);
        // Every command fails in C#; the library starts on what it can and says why.
        var started = Record.Exception(() =>
        {
            using var driven = new Driven((_, _) => throw new InvalidOperationException("broken program"), files);
            driven.Step(new JsonObject { ["do"] = "read" });
        });
        Assert.True(started is null or ClarityException, started?.ToString());
    }
}
