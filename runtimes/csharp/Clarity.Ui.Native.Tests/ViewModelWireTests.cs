// What the C# window model answers the library beside a window's values
// (ProgramWire): the values of fields in their own types, the records and
// values dropdowns offer, stored files, settings, the clipboard, the field
// facts of the generated Fields, and the unconnected flow. These were the
// ViewModelHost's answers before the library took the window (Clarity.Ui.Tests,
// ViewModelHostTests, DataPartTests, UnconnectedWindowTests).

using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui.Native.Tests;

public sealed class ViewModelWireTests
{
    static ProgramWire Wire(WindowViewModel main, Unconnected? unconnected = null, Text? problem = null)
    {
        var markups = new Dictionary<string, string> { ["Tickets"] = TicketsViewModel.Markup, ["Shelf"] = ShelfViewModel.Markup };
        return new ProgramWire(typeof(TicketsViewModel).Assembly, main, n => markups.GetValueOrDefault(n), unconnected, problem);
    }

    static JsonNode? Value(JsonObject? answer)
    {
        Assert.NotNull(answer);
        Assert.Null(answer["error"]);
        Assert.True(answer.ContainsKey("value"), answer.ToJsonString());
        return answer["value"];
    }

    static JsonObject Opened(ProgramWire wire, string window = "Tickets") =>
        Value(wire.Answer("open_window", new JsonObject { ["window"] = window, ["with"] = new JsonObject() }.ToJsonString()))!.AsObject();

    static JsonObject Send(ProgramWire wire, string command, JsonObject state, JsonObject more)
    {
        var args = new JsonObject { ["state"] = state.DeepClone() };
        foreach (var (key, value) in more)
            args[key] = value?.DeepClone();
        return Value(wire.Answer(command, args.ToJsonString()))!.AsObject();
    }

    [Fact]
    public void Values_the_library_sends_back_take_the_types_of_their_properties()
    {
        var tickets = new TicketsViewModel();
        var wire = Wire(tickets);
        var state = Opened(wire)["state"]!.AsObject();
        var ticket = state["data"]!["ticket"]!.AsObject();
        // Clarity's JSON: snake case names, enums by their names, a decimal as exact text or a number.
        Assert.Equal("Open", ticket["status"]!.GetValue<string>());
        ticket["total"] = 19.75;
        ticket["status"] = "Done";
        ticket["finished_by"] = 2;
        ticket["due"] = "2026-11-03";
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "count" });
        Assert.Equal(1, tickets.Counted);
        Assert.Equal(19.75m, tickets.Ticket.Total);
        Assert.Equal(TicketStatus.Done, tickets.Ticket.Status);
        Assert.Equal(2L, tickets.Ticket.FinishedBy);
        Assert.Equal(new DateOnly(2026, 11, 3), tickets.Ticket.Due);
        // A date the person cleared is no date.
        state = reply["state"]!.AsObject();
        state["data"]!["ticket"]!["due"] = "";
        Send(wire, "dispatch_event", state, new() { ["handler"] = "count" });
        Assert.Null(tickets.Ticket.Due);
    }

    [Fact]
    public void A_reference_offers_the_records_and_a_typed_dropdown_the_field_s_values()
    {
        var wire = Wire(new TicketsViewModel());
        Opened(wire);
        var people = Value(wire.Answer("ref_options", """{"entity":"person"}"""))!.AsArray();
        Assert.Equal(["Ann", "Bob"], people.Select(p => p!["text"]!.GetValue<string>()));
        Assert.Equal([1L, 2L], people.Select(p => p!["id"]!.GetValue<long>()));
        var values = Value(wire.Answer("field_values", """{"entity":"ticket","field":"note"}"""))!.AsArray();
        Assert.Contains("second", values.Select(v => v!.GetValue<string>()));
        var typed = Value(wire.Answer("find_options", """{"kind":"typed","entity":"ticket","field":"note","typed":"se"}"""))!.AsArray();
        Assert.Equal(["second"], typed.Select(v => v!.GetValue<string>()));
        // An entity no window offers has no records, and that is no error.
        Assert.Empty(Value(wire.Answer("ref_options", """{"entity":"planet"}"""))!.AsArray());
    }

    [Fact]
    public void A_search_dropdown_offers_the_records_that_fit_with_the_fields_it_shows()
    {
        var wire = Wire(new TicketsViewModel());
        Opened(wire);
        var found = Value(wire.Answer("find_options", """{"kind":"search","entity":"person","show":["name","role"],"typed":"lea"}"""))!.AsArray();
        var cy = Assert.Single(found)!;
        Assert.Equal(7L, cy["id"]!.GetValue<long>());
        Assert.Equal("lead", cy["fields"]!["1"]!.GetValue<string>());
    }

    [Fact]
    public void A_stored_picture_and_a_store_s_file_are_fetched_through_the_generated_fetchers()
    {
        var wire = Wire(new TicketsViewModel());
        Opened(wire);
        var picture = Value(wire.Answer("field_bytes", """{"entity":"Ticket","id":5,"field":"photo"}"""));
        Assert.Equal([1, 2, 3], Convert.FromBase64String(picture!.GetValue<string>()));
        // Another record has none here.
        Assert.Null(Value(wire.Answer("field_bytes", """{"entity":"Ticket","id":6,"field":"photo"}""")));
        var file = Value(wire.Answer("store_file", """{"store":"scans","path":"a/b.pdf"}"""));
        Assert.Equal([2], Convert.FromBase64String(file!.GetValue<string>()));
        var missing = wire.Answer("store_file", """{"store":"scans","path":"no/such.pdf"}""");
        Assert.NotNull(missing?["error"]);
    }

    [Fact]
    public void The_settings_are_read_whole_and_a_changed_one_is_kept_on_the_settings_object()
    {
        var tickets = new TicketsViewModel();
        var wire = Wire(tickets);
        Opened(wire);
        var settings = Value(wire.Answer("read_settings", "{}"))!.AsObject();
        Assert.Contains(settings, s => s.Value?.ToJsonString() == "true");
        // The person turned the one setting that is on off.
        var changed = settings.DeepClone().AsObject();
        var on = changed.First(s => s.Value?.ToJsonString() == "true").Key;
        changed[on] = false;
        Value(wire.Answer("keep_settings", new JsonObject { ["settings"] = changed }.ToJsonString()));
        Assert.False(tickets.Settings.ShowDone);
        Assert.Equal("hello", tickets.Settings.Greeting);
        Assert.True(JsonNode.DeepEquals(changed, Value(wire.Answer("read_settings", "{}"))));
    }

    [Fact]
    public void What_a_handler_copies_goes_to_the_library_s_clipboard_once()
    {
        var wire = Wire(new TicketsViewModel());
        var state = Opened(wire)["state"]!.AsObject();
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "copyIt" });
        Assert.Contains("Name: Ada", reply["copy"]!.ToJsonString());
        var again = Send(wire, "dispatch_event", reply["state"]!.AsObject(), new() { ["handler"] = "count" });
        Assert.Null(again["copy"]);
    }

    [Fact]
    public void Errors_said_by_a_handler_all_come_back_in_their_order()
    {
        var wire = Wire(new TicketsViewModel());
        var state = Opened(wire)["state"]!.AsObject();
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "warnTwice" });
        Assert.Equal(["one", "two"], reply["messages"]!.AsArray().Select(m => m!["text"]!.GetValue<string>()));
        Assert.All(reply["messages"]!.AsArray(), m => Assert.Equal("error", m!["level"]!.GetValue<string>()));
    }

    [Fact]
    public void A_question_goes_out_with_a_token_that_its_answer_brings_back_once()
    {
        var tickets = new TicketsViewModel();
        var wire = Wire(tickets);
        var state = Opened(wire)["state"]!.AsObject();
        var reply = Send(wire, "dispatch_event", state, new() { ["handler"] = "makeSure" });
        var ask = reply["asks"]!.AsArray().Single()!;
        Assert.Equal("confirm", ask["form"]!["kind"]!.GetValue<string>());
        var token = ask["handler"]!.GetValue<string>();
        Send(wire, "answer_event", reply["state"]!.AsObject(), new() { ["handler"] = token, ["values"] = new JsonArray() });
        Assert.Equal(1, tickets.Confirmed);
        // The same token again is no question any more: nothing runs twice.
        wire.Answer("answer_event", new JsonObject { ["state"] = reply["state"]!.DeepClone(), ["handler"] = token, ["values"] = new JsonArray() }.ToJsonString());
        Assert.Equal(1, tickets.Confirmed);
    }

    [Fact]
    public void The_field_facts_are_the_generated_fields_of_the_records()
    {
        // Labels, formats, ranges and references of the generated Fields.All, as the library reads facts_json.
        var wire = Wire(new TicketsViewModel());
        var facts = Value(wire.Answer("facts", "{}"));
        Assert.NotNull(facts);
        var fields = facts["fields"]!.AsObject();
        Assert.Equal("@ticketTitleLabel", fields["Ticket.title"]!["label"]!.GetValue<string>());
        Assert.True(fields["Ticket.title"]!["required"]!.GetValue<bool>());
        Assert.Equal("money", fields["Ticket.total"]!["format"]!["kind"]!.GetValue<string>());
        Assert.Equal("500", fields["Ticket.total"]!["max"]!.GetValue<string>());
        Assert.Equal("Person", fields["Ticket.finished_by"]!["refers"]!.GetValue<string>());
    }

    [Fact]
    public void An_open_of_a_window_the_program_has_not_is_an_error()
    {
        var wire = Wire(new TicketsViewModel());
        var answer = wire.Answer("open_window", """{"window":"Nowhere","with":{}}""");
        Assert.Contains("Nowhere", answer?["error"]?.ToJsonString());
    }

    [Fact]
    public void A_command_on_a_window_that_closed_is_an_error_and_the_program_goes_on()
    {
        var wire = Wire(new TicketsViewModel());
        var state = Opened(wire)["state"]!.AsObject();
        Value(wire.Answer("close_window", new JsonObject { ["state"] = state.DeepClone() }.ToJsonString()));
        var late = wire.Answer("dispatch_event", new JsonObject { ["state"] = state.DeepClone(), ["handler"] = "count" }.ToJsonString());
        Assert.NotNull(late?["error"]);
        // Opened again, it is a new window: the closed one is not shown again.
        Assert.NotEqual(state["instance"]!.GetValue<long>(), Opened(wire)["state"]!["instance"]!.GetValue<long>());
    }

    // -- Unconnected (DESIGN 5.27) --------------------------------------------------------

    sealed class Counted
    {
        public int Opened;
    }

    static Unconnected Flow(Func<Db> connect, Counted counted) => new()
    {
        Connect = connect,
        Empty = Db.Empty,
        Main = db =>
        {
            counted.Opened++;
            return new TicketsViewModel();
        },
        Window = db => new ShelfViewModel(),
    };

    [Fact]
    public void Unconnected_the_program_starts_on_the_window_that_keeps_the_connection_saying_why()
    {
        var counted = new Counted();
        var wire = Wire(null!, Flow(Db.Empty, counted), Text.Plain("no server"));
        var started = Value(wire.Answer("started", "{}"))!.AsObject();
        var unconnected = started["unconnected"]!.AsObject();
        Assert.Equal("Shelf", unconnected["window"]!.GetValue<string>());
        Assert.Contains("no server", unconnected["problem"]!.ToJsonString());
        Assert.Null(unconnected["handler"]);
        // That window opens on the empty database before anything connected.
        Assert.Equal("Shelf", Opened(wire, "Shelf")["state"]!["window"]!.GetValue<string>());
        Assert.Equal(0, counted.Opened);
    }

    [Fact]
    public void Connecting_again_says_why_in_Clarity_s_words_or_opens_the_main_window_on_the_databases()
    {
        var counted = new Counted();
        var fails = true;
        var databases = Db.Empty();
        var wire = Wire(null!, Flow(() => fails ? throw new StorageException("the server did not answer") : databases, counted), Text.Plain("no server"));
        Value(wire.Answer("started", "{}"));
        var failed = wire.Answer("connect", "{}");
        Assert.Contains("unconnectedFailed", failed?["error"]?.ToJsonString());
        Assert.Contains("the server did not answer", failed?["error"]?.ToJsonString());
        Assert.Null(wire.Connected);
        fails = false;
        Value(wire.Answer("connect", "{}"));
        Assert.Same(databases, wire.Connected);
        Assert.Equal(1, counted.Opened);
        Assert.Equal("Tickets", Opened(wire)["state"]!["window"]!.GetValue<string>());
    }
}
