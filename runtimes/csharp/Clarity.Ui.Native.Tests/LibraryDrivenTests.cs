// The C# layer and the C# window model through the native library itself,
// driven without a window (DESIGN 1.6): a C# custom part made, given its
// values and hears, drawn into the screen's pixels and telling its window;
// a part that throws staying in its place; undo, questions and messages
// coming back to the view model; and the lifetimes across the boundary
// (the program let go when the run ends, many runs one after another, the
// program's callback from several threads at once).

using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui.Native.Tests;

public sealed class LibraryDrivenTests
{
    static Driven Run(WindowViewModel main, string name, string markup)
    {
        var wire = new ProgramWire(typeof(LibraryDrivenTests).Assembly, main, n => n == name ? markup : null);
        var files = new NativeFiles();
        files.Window(name, markup);
        files.Title(name);
        return new Driven(wire.Answer, files);
    }

    static JsonObject Do(Driven driven, string step, JsonObject? more = null)
    {
        var json = new JsonObject { ["do"] = step };
        foreach (var (key, value) in more ?? [])
            json[key] = value?.DeepClone();
        var answer = driven.Step(json);
        Assert.True(answer["ok"]?.GetValue<bool>() ?? false, $"{json.ToJsonString()}: {answer.ToJsonString()}");
        return answer;
    }

    static JsonObject Click(Driven driven, string part) => Do(driven, "click", new() { ["part"] = part });

    static JsonObject Key(Driven driven, string key, bool ctrl = false) => Do(driven, "key", new() { ["key"] = key, ["ctrl"] = ctrl });

    /// <summary>A custom part kind of its own per test, so the parts one test made are its own.</summary>
    static string Kind(string test, bool breaks = false)
    {
        var kind = "WireGauge" + test;
        CustomParts.Add(kind, WireGauge.Maker(kind, breaks));
        return kind;
    }

    static WireGauge Gauge(string kind)
    {
        lock (WireGauge.Made)
            return Assert.Single(WireGauge.Made[kind]);
    }

    // -- Custom parts ----------------------------------------------------------------

    [Fact]
    public void A_csharp_custom_part_gets_its_line_s_values_what_it_binds_and_enabled_as_the_window_shows()
    {
        var kind = Kind("Shows");
        using var driven = Run(new GaugesViewModel(), "Gauges", GaugesViewModel.MarkupOf(kind));
        Do(driven, "read");
        var got = Gauge(kind).Got;
        Assert.Contains("set title Speed", got);
        Assert.Contains("set value 3", got);
        Assert.Contains("set enabled true", got);
    }

    [Fact]
    public void What_code_sets_and_tells_a_custom_part_reaches_it_through_the_library()
    {
        var kind = Kind("Hears");
        using var driven = Run(new GaugesViewModel(), "Gauges", GaugesViewModel.MarkupOf(kind));
        Do(driven, "read");
        var gauge = Gauge(kind);
        gauge.Got.Clear();
        Click(driven, "resetButton");
        Do(driven, "read");
        Assert.Contains("hear reset 2", gauge.Got);
        Assert.Contains("set speed 7", gauge.Got);
        // Given once: drawn again with nothing changed, nothing comes again.
        gauge.Got.Clear();
        Do(driven, "settle", new() { ["ms"] = 50 });
        Assert.Empty(gauge.Got);
    }

    [Fact]
    public void A_custom_part_s_change_and_tells_reach_the_view_model_and_undo_takes_the_change_back()
    {
        var kind = Kind("Tells");
        var gauges = new GaugesViewModel();
        using var driven = Run(gauges, "Gauges", GaugesViewModel.MarkupOf(kind));
        Click(driven, "gauge");
        Assert.Equal(9, gauges.Level);
        Assert.Equal(1, gauges.Changes);
        Assert.Equal("3 high", gauges.Reached);
        // The C# Told event hears every tell, the one no handler is wired to too.
        Assert.Equal(["reached 3,\"high\"", "dropped 1"], Gauge(kind).Heard);
        Key(driven, "Z", ctrl: true);
        Assert.Equal(3, gauges.Level);
    }

    [Fact]
    public void A_custom_part_draws_into_the_screen_s_pixels()
    {
        var kind = Kind("Draws");
        using var driven = Run(new GaugesViewModel(), "Gauges", GaugesViewModel.MarkupOf(kind));
        var at = Do(driven, "show", new() { ["part"] = "gauge" })["at"]!.AsArray().Select(v => v!.GetValue<double>()).ToArray();
        var path = Path.Combine(Path.GetTempPath(), $"clarity-native-{Guid.NewGuid():N}.png");
        try
        {
            Do(driven, "shot", new() { ["path"] = path });
            var png = Png.Read(path);
            Assert.Equal((1280, 800), (png.Width, png.Height));
            var (r, g, b, _) = png.At((int)(at[0] + at[2] / 2), (int)(at[1] + at[3] / 2));
            Assert.Equal(((byte)200, (byte)30, (byte)40), (r, g, b));
            // Beside it the window's own background, not the gauge's red.
            var beside = png.At((int)(at[0] + at[2] + 2), (int)(at[1] + at[3] / 2));
            Assert.NotEqual((r, g, b), (beside.R, beside.G, beside.B));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void A_custom_part_that_throws_keeps_its_place_and_the_window_goes_on()
    {
        var kind = Kind("Breaks", breaks: true);
        var gauges = new GaugesViewModel();
        using var driven = Run(gauges, "Gauges", GaugesViewModel.MarkupOf(kind));
        var path = Path.Combine(Path.GetTempPath(), $"clarity-native-{Guid.NewGuid():N}.png");
        try
        {
            Do(driven, "shot", new() { ["path"] = path });
        }
        finally
        {
            File.Delete(path);
        }
        Click(driven, "gauge");
        Assert.Equal(3, gauges.Level);
        Click(driven, "countButton");
        Assert.Equal(1, gauges.Counted);
    }

    [Fact]
    public void A_part_no_library_brings_says_so_in_its_place_and_the_window_goes_on()
    {
        var gauges = new GaugesViewModel();
        using var driven = Run(gauges, "Gauges", GaugesViewModel.MarkupOf("NoLibraryBringsThis"));
        Do(driven, "read");
        Click(driven, "countButton");
        Assert.Equal(1, gauges.Counted);
    }

    // -- The window model through the library ----------------------------------------------

    [Fact]
    public void What_the_person_typed_is_one_edit_that_ctrl_z_takes_back_in_the_view_model()
    {
        var tickets = new TicketsViewModel();
        using var driven = Run(tickets, "Tickets", TicketsViewModel.Markup);
        Click(driven, "titleBox");
        Key(driven, "End");
        Do(driven, "type", new() { ["text"] = " pipe" });
        Key(driven, "Tab");
        Assert.Equal("Leak pipe", tickets.Ticket.Title);
        Do(driven, "focus", new() { ["part"] = "titleBox" });
        Key(driven, "Z", ctrl: true);
        Assert.Equal("Leak", tickets.Ticket.Title);
    }

    [Fact]
    public void A_question_shows_in_the_library_s_dialog_and_its_answer_runs_the_view_model_s_continuation()
    {
        var tickets = new TicketsViewModel();
        using var driven = Run(tickets, "Tickets", TicketsViewModel.Markup);
        Click(driven, "sureButton");
        Assert.True(Do(driven, "read")["overlays"]!.GetValue<int>() > 0);
        Key(driven, "Enter");
        Assert.Equal(1, tickets.Confirmed);
        Assert.Equal(0, Do(driven, "read")["overlays"]!.GetValue<int>());
    }

    [Fact]
    public void Errors_a_handler_says_are_shown_one_at_a_time()
    {
        using var driven = Run(new TicketsViewModel(), "Tickets", TicketsViewModel.Markup);
        Click(driven, "warnButton");
        var first = Do(driven, "read").ToJsonString();
        Assert.Contains("\"one\"", first);
        Key(driven, "Enter");
        var second = Do(driven, "read").ToJsonString();
        Assert.Contains("\"two\"", second);
        Key(driven, "Enter");
        Assert.Equal(0, Do(driven, "read")["overlays"]!.GetValue<int>());
    }

    // -- Lifetimes and threads ----------------------------------------------------------------

    sealed class Holder
    {
        public int Calls;
    }

    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
    static WeakReference RunAndEnd()
    {
        var holder = new Holder();
        var wire = new ProgramWire(typeof(LibraryDrivenTests).Assembly, new TicketsViewModel(), n => n == "Tickets" ? TicketsViewModel.Markup : null);
        var files = new NativeFiles();
        files.Window("Tickets", TicketsViewModel.Markup);
        using (var driven = new Driven((command, args) =>
               {
                   holder.Calls++;
                   return wire.Answer(command, args);
               }, files))
            Do(driven, "read");
        Assert.True(holder.Calls > 0);
        return new WeakReference(holder);
    }

    [Fact]
    public void The_library_lets_go_of_the_program_when_the_run_ends()
    {
        var held = RunAndEnd();
        for (var i = 0; i < 5 && held.IsAlive; i++)
        {
            GC.Collect();
            GC.WaitForPendingFinalizers();
        }
        Assert.False(held.IsAlive, "the program's callback is still held after the run ended");
    }

    [Fact]
    public void Many_runs_one_after_another_each_start_and_end_cleanly()
    {
        for (var i = 0; i < 12; i++)
        {
            var tickets = new TicketsViewModel();
            using var driven = Run(tickets, "Tickets", TicketsViewModel.Markup);
            Click(driven, "countButton");
            Assert.Equal(1, tickets.Counted);
        }
    }

    [Fact]
    public void The_program_answers_the_library_from_several_threads_at_once_one_call_at_a_time()
    {
        var tickets = new TicketsViewModel();
        var wire = new ProgramWire(typeof(LibraryDrivenTests).Assembly, tickets, n => n == "Tickets" ? TicketsViewModel.Markup : null);
        var opened = wire.Answer("open_window", """{"window":"Tickets","with":{}}""")!["value"]!["state"]!.ToJsonString();
        var args = $$"""{"state":{{opened}},"handler":"count"}""";
        Parallel.For(0, 200, new ParallelOptions { MaxDegreeOfParallelism = 8 }, _ => Assert.Null(wire.Answer("dispatch_event", args)?["error"]));
        Assert.Equal(200, tickets.Counted);
    }
}
