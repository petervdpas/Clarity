// The C# window model answering the library's commands (ProgramWire), as
// the library sends them: without the library, so each answer's JSON is read
// as the library reads it.

using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui.Native.Tests;

public sealed class ProgramWireTests
{
    static (ProgramWire Wire, ShelfViewModel Shelf) Started()
    {
        var shelf = new ShelfViewModel();
        var markups = new Dictionary<string, string> { ["Shelf"] = ShelfViewModel.Markup, ["Other"] = OtherViewModel.Markup };
        return (new ProgramWire(typeof(ShelfViewModel).Assembly, shelf, n => markups.GetValueOrDefault(n)), shelf);
    }

    static JsonObject Value(JsonObject? answer)
    {
        Assert.NotNull(answer);
        Assert.Null(answer["error"]);
        return Assert.IsType<JsonObject>(answer["value"]);
    }

    static JsonObject Open(ProgramWire wire) =>
        Value(wire.Answer("open_window", """{"window":"Shelf","with":{}}"""));

    static JsonObject Send(ProgramWire wire, string command, JsonObject state, JsonObject? more = null)
    {
        var args = new JsonObject { ["state"] = state.DeepClone() };
        foreach (var (key, value) in more ?? [])
            args[key] = value?.DeepClone();
        return Value(wire.Answer(command, args.ToJsonString()));
    }

    [Fact]
    public void Started_names_the_main_window_by_its_view_model()
    {
        var (wire, _) = Started();
        var started = Value(wire.Answer("started", "{}"));
        Assert.Equal("Shelf", started["main"]?.GetValue<string>());
        Assert.Null(started["unconnected"]);
    }

    [Fact]
    public void An_open_window_sends_its_values_by_their_Clarity_names_in_Clarity_s_JSON()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        Assert.Equal("Shelf", state["window"]?.GetValue<string>());
        Assert.Equal(shelf.Instance, state["instance"]?.GetValue<long>());
        var data = state["data"]!.AsObject();
        Assert.Equal("nobody", data["finished_by"]?.GetValue<string>());
        Assert.Equal("Dune", data["books"]![0]!["title"]?.GetValue<string>());
        Assert.Equal(474, data["books"]![1]!["pages"]?.GetValue<long>());
        // Part states, choices and commands are no values of the window.
        Assert.Null(data["person_choices"]);
        Assert.Null(data["save_command"]);
        Assert.Null(data["save_button"]);
    }

    [Fact]
    public void A_part_state_goes_by_the_part_s_name_with_its_caption_by_its_key()
    {
        var (wire, shelf) = Started();
        shelf.SaveButton.IsEnabled = false;
        var parts = Open(wire)["state"]!["parts"]!.AsObject();
        Assert.False(parts["saveButton"]!["enabled"]!.GetValue<bool>());
        Assert.Equal("saveCaption", parts["saveButton"]!["caption"]!["key"]!.GetValue<string>());
        Assert.Null(parts["shelf"]!["caption"]);
    }

    [Fact]
    public void A_dispatched_handler_runs_on_the_view_model_and_its_messages_come_back()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "save" });
        Assert.Equal(1, shelf.Saved);
        Assert.Equal("info", reply["messages"]![0]!["level"]!.GetValue<string>());
        Assert.Equal("saved", reply["messages"]![0]!["text"]!.GetValue<string>());
        // A message the library has is not sent again.
        var again = Send(wire, "dispatch_event", reply["state"]!.AsObject(), new() { ["handler"] = "digit(7)" });
        Assert.Empty(again["messages"]!.AsArray());
        Assert.Equal(7, shelf.Digit);
    }

    [Fact]
    public void What_the_library_changed_reaches_the_view_model_before_the_handler_runs_keeping_its_records()
    {
        var (wire, shelf) = Started();
        var dune = shelf.Books[0];
        var state = Open(wire)["state"]!.AsObject();
        state["data"]!["note"] = "read soon";
        state["data"]!["books"]![0]!["pages"] = 500;
        Send(wire, "dispatch_event", state, new() { ["handler"] = "save" });
        Assert.Equal("read soon", shelf.Note);
        Assert.Same(dune, shelf.Books[0]);
        Assert.Equal(500, dune.Pages);
    }

    [Fact]
    public void An_edit_is_one_step_of_its_scope_which_undo_takes_back()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        state["data"]!["note"] = "first";
        var edited = Send(wire, "edit_event", state, new() { ["part"] = "note", ["scope"] = "note", ["ends"] = true, ["readonly"] = false });
        Assert.Equal("first", shelf.Note);
        var undone = Send(wire, "undo_edit", edited["state"]!.AsObject(), new() { ["scope"] = "note", ["back"] = true });
        Assert.Equal("", shelf.Note);
        Assert.Equal("", undone["state"]!["data"]!["note"]!.GetValue<string>());
    }

    [Fact]
    public void A_readonly_part_s_change_is_taken_back()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        state["data"]!["note"] = "not allowed";
        var reply = Send(wire, "edit_event", state, new() { ["part"] = "note", ["scope"] = "note", ["ends"] = true, ["readonly"] = true });
        Assert.Equal("", shelf.Note);
        Assert.Equal("", reply["state"]!["data"]!["note"]!.GetValue<string>());
    }

    [Fact]
    public void A_chosen_row_goes_between_the_library_s_place_and_the_view_model_s_record()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        state["selected"] = new JsonObject { ["shelf"] = 1 };
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "save" });
        Assert.Same(shelf.Books[1], shelf.Shelf.SelectedItem);
        Assert.Equal(1, reply["state"]!["selected"]!["shelf"]!.GetValue<long>());
    }

    [Fact]
    public void A_failing_handler_says_why_and_the_window_goes_on()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "fail" });
        Assert.Equal("error", reply["messages"]![0]!["level"]!.GetValue<string>());
        Assert.Equal("the shelf broke", reply["messages"]![0]!["text"]!.GetValue<string>());
        Send(wire, "dispatch_event", reply["state"]!.AsObject(), new() { ["handler"] = "save" });
        Assert.Equal(1, shelf.Saved);
    }

    [Fact]
    public void A_handler_the_window_has_no_command_for_says_so()
    {
        var (wire, _) = Started();
        var state = Open(wire)["state"]!.AsObject();
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "nothingHere" });
        Assert.Equal("error", reply["messages"]![0]!["level"]!.GetValue<string>());
    }

    [Fact]
    public void A_window_a_handler_opens_is_handed_over_by_its_token()
    {
        var (wire, _) = Started();
        var state = Open(wire)["state"]!.AsObject();
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "openOther" });
        var open = reply["open"]![0]!.AsObject();
        Assert.Equal("Other", open["window"]!.GetValue<string>());
        var other = Value(wire.Answer("open_window", new JsonObject { ["window"] = "Other", ["with"] = open["with"]!.DeepClone() }.ToJsonString()));
        Assert.Equal("Other", other["state"]!["window"]!.GetValue<string>());
    }

    [Fact]
    public void A_question_s_choices_go_by_their_places_and_its_answer_runs_its_handler()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "ask" });
        var ask = reply["asks"]![0]!.AsObject();
        Assert.Equal("record", ask["form"]!["type"]!.GetValue<string>());
        var token = ask["handler"]!.GetValue<string>();
        var options = Assert.IsType<JsonArray>(wire.Answer("ref_options", new JsonObject { ["entity"] = token }.ToJsonString())!["value"]);
        Assert.Equal("Ann", options[1]!["text"]!.GetValue<string>());
        Send(wire, "answer_event", reply["state"]!.AsObject(), new() { ["handler"] = token, ["values"] = new JsonArray(2) });
        Assert.Equal(2L, shelf.Answered);
    }

    [Fact]
    public void A_reference_offers_the_view_model_s_records_without_the_empty_one()
    {
        var (wire, _) = Started();
        Open(wire);
        var answer = wire.Answer("ref_options", """{"entity":"person"}""");
        var options = Assert.IsType<JsonArray>(answer!["value"]);
        Assert.Equal(2, options.Count);
        Assert.Equal(1, options[0]!["id"]!.GetValue<long>());
        Assert.Equal("Bob", options[1]!["text"]!.GetValue<string>());
    }

    [Fact]
    public void A_handler_told_with_values_runs_with_them()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        Send(wire, "answer_event", state, new() { ["handler"] = "counted", ["values"] = new JsonArray(4) });
        Assert.Equal(40, shelf.Digit);
    }

    [Fact]
    public void A_command_the_program_does_not_know_takes_the_library_s_default()
    {
        var (wire, _) = Started();
        Assert.Null(wire.Answer("a_command_of_a_later_library", "{}"));
        // A watch the program never started is answered, and stops nothing.
        Assert.Null(wire.Answer("start_watch", """{"id":1}""")?["value"]);
        Assert.False(wire.Answer("watches", "{}")!["value"]!.GetValue<bool>());
    }

    [Fact]
    public void Arguments_that_do_not_read_are_an_error_not_a_crash()
    {
        var (wire, _) = Started();
        Assert.NotNull(wire.Answer("dispatch_event", "{not json")!["error"]);
        Assert.NotNull(wire.Answer("dispatch_event", """{"state":{"instance":999},"handler":"save"}""")!["error"]);
    }

    [Fact]
    public void A_closed_window_hears_its_on_close_and_is_gone()
    {
        var (wire, _) = Started();
        var state = Open(wire)["state"]!.AsObject();
        Send(wire, "close_window", state);
        Assert.NotNull(wire.Answer("dispatch_event", new JsonObject { ["state"] = state.DeepClone(), ["handler"] = "save" }.ToJsonString())!["error"]);
    }

    [Fact]
    public void A_canvas_s_pointer_handler_runs_through_its_generated_command_with_where_the_pointer_was()
    {
        var (wire, shelf) = Started();
        var state = Open(wire)["state"]!.AsObject();
        state["parts"]!["pad"]!["pointer"] = new JsonObject { ["x"] = 30, ["y"] = 40, ["fromX"] = 10, ["fromY"] = 20 };
        Send(wire, "dispatch_event", state, new() { ["handler"] = "padClicked" });
        Assert.Equal("click", shelf.PadEvent);
        Assert.Equal((30L, 40L, 10L, 20L), (shelf.PadAt!.X, shelf.PadAt.Y, shelf.PadAt.FromX, shelf.PadAt.FromY));
        var reply = Send(wire, "edit_event", state, new() { ["handler"] = "padDragged", ["part"] = "pad", ["scope"] = "pad", ["ends"] = true, ["readonly"] = false });
        Assert.Equal("drag", shelf.PadEvent);
        Assert.Equal(10, reply["state"]!["parts"]!["pad"]!["pointer"]!["fromX"]!.GetValue<long>());
    }
}
