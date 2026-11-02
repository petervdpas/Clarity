// Hosting rows of the Avalonia parity work: a custom part hears its bound
// value and enabled when they change and raises Told for every tell; the
// system's light or dark choice is followed by its change event, read every
// so often only where there is none; a language change remakes a dialog over
// the window too, and a stack no longer shown no longer listens; a
// character the theme's font lacks is drawn in a fallback font.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class HostsParityTests
{
    const string GaugeLine = """<Custom name="gauge" at="1,1" span="4" rows="3" part="Gauge" bind="speed" tells='[{"tell":"reached","handler":"gaugeReached"}]' />""";

    static Gauge GaugeIn(TestHost host)
    {
        var (_, _, parts) = Made.Window(GaugeLine, host);
        return Assert.IsType<Gauge>(Assert.IsType<CustomPlace>(parts[0]).Inner);
    }

    [Fact]
    public void A_custom_part_hears_its_bound_value_and_enabled_when_they_change()
    {
        var host = new TestHost();
        host.Values["speed"] = 3L;
        var gauge = GaugeIn(host);
        gauge.Got.Clear();
        // The view model changed: the value reaches it now, not at the next layout.
        host.Values["speed"] = 4L;
        host.Change();
        Assert.Equal(["set value 4"], gauge.Got);
        gauge.Got.Clear();
        host.State("gauge")!.IsEnabled = false;
        Assert.Equal(["set enabled False"], gauge.Got);
        // Nothing changed: nothing is given again.
        gauge.Got.Clear();
        host.Change();
        Assert.Empty(gauge.Got);
    }

    sealed class TellingHost : TestHost
    {
        public override void Told(string part, string tell, JsonElement[] values) => Log.Add($"told {part} {tell}");
    }

    [Fact]
    public void A_tell_no_handler_is_wired_to_still_raises_Told()
    {
        var host = new TellingHost();
        var gauge = GaugeIn(host);
        var told = new List<string>();
        gauge.Told += (tell, values) => told.Add($"{tell} {string.Join(",", values.Select(v => v.GetRawText()))}");
        gauge.TellUnwired();
        Assert.Equal(["dropped 1"], told);
        // No handler runs for it.
        Assert.DoesNotContain(host.Log, l => l.StartsWith("told", StringComparison.Ordinal));
        gauge.OnClick();
        Assert.Equal(["dropped 1", "reached 3,\"high\""], told);
        Assert.Contains("told gauge reached", host.Log);
    }

    // -- The system's light or dark choice ------------------------------------------------

    /// <summary>A Linux whose portal says what <paramref name="dark"/> holds now.</summary>
    static SystemTheme Linux(StrongBox<bool> dark) =>
        new((program, _) => program == "gdbus" ? (dark.Value ? "(<<uint32 1>>,)" : "(<<uint32 2>>,)") : null, "linux");

    sealed class StrongBox<T>(T value)
    {
        public T Value = value;
    }

    sealed class Listening : IDisposable
    {
        public Action? Heard;
        public bool Disposed;

        public IDisposable? Listen(Action heard)
        {
            Heard = heard;
            return this;
        }

        public void Dispose() => Disposed = true;
    }

    [Fact]
    public async Task The_system_theme_is_followed_by_its_change_event()
    {
        var dark = new StrongBox<bool>(false);
        var listening = new Listening();
        var changed = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        // A long poll: only the event can tell it in time.
        using (var following = Linux(dark).Follow(false, listening.Listen, TimeSpan.FromHours(1), d => changed.TrySetResult(d)))
        {
            await following.Started.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
            Assert.True(following.ByEvent);
            dark.Value = true;
            listening.Heard!();
            Assert.True(await changed.Task.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        }
        Assert.True(listening.Disposed);
    }

    [Fact]
    public async Task Without_a_change_event_the_system_theme_is_read_every_so_often()
    {
        var dark = new StrongBox<bool>(false);
        var changed = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        using var following = Linux(dark).Follow(false, _ => null, TimeSpan.FromMilliseconds(50), d => changed.TrySetResult(d));
        await following.Started.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
        Assert.False(following.ByEvent);
        dark.Value = true;
        Assert.True(await changed.Task.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task A_change_event_that_cannot_be_listened_to_leaves_reading_every_so_often()
    {
        var dark = new StrongBox<bool>(false);
        using var following = Linux(dark).Follow(false, _ => throw new InvalidOperationException("no bus"), TimeSpan.FromMinutes(1), _ => { });
        await following.Started.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
        Assert.False(following.ByEvent);
    }

    // -- Fonts -----------------------------------------------------------------------------

    const string Japanese = "日本";

    [Fact]
    public void A_character_the_theme_s_font_lacks_is_drawn_in_a_fallback_font()
    {
        // The system's fallback, or a carried one where the system has none for it.
        const string droid = "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf";
        if (File.Exists(droid))
            Assert.True(Fonts.AddFallback(File.ReadAllBytes(droid)));
        var fonts = new Fonts(ThemeLook.Clarity(dark: false));
        var inter = Fonts.Carried(bold: false);
        Assert.False(Fonts.Has(inter, char.ConvertToUtf32(Japanese, 0)));
        var runs = fonts.Runs($"Hi {Japanese}!", "text", bold: false);
        var cjk = Assert.Single(runs, r => r.Text.Contains(Japanese));
        Assert.True(Fonts.Has(cjk.Typeface, Japanese));
        Assert.Equal($"Hi {Japanese}!", string.Concat(runs.Select(r => r.Text)));
        // Text the font has stays in one run of its own typeface.
        var plain = Assert.Single(fonts.Runs("Hello there", "text", bold: false));
        Assert.Same(fonts.Typeface("text", bold: false), plain.Typeface);
        // Measured in the fallback, as drawn.
        Assert.True(fonts.Measure(Japanese, "text", 14, false) > 20);
    }

    [Fact]
    public void Data_that_is_no_font_is_no_fallback()
    {
        Assert.False(Fonts.AddFallback([1, 2, 3, 4]));
        Assert.Equal(0, Fonts.CarryFallbacks(typeof(HostsParityTests).Assembly));
    }
}

[Collection("Language")]
public class LanguageDialogRemakeTests
{
    const string Greet = """<Window name="PickFriend"><Label name="hi" at="1,1" span="6" caption="@hello" /></Window>""";

    [Fact]
    public void ALanguageChangeRebuildsTheViewsOfEveryWindow_dialogs_included_and_a_closed_stack_no_longer_listens()
    {
        string? Lookup(string key, CultureInfo culture) => (key, culture.TwoLetterISOLanguageName) switch
        {
            ("hello", "nl") => "Hallo",
            ("hello", _) => "Hello",
            _ => null,
        };
        Locale.Start("en", ["en", "nl"], null, Lookup, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
        try
        {
            var built = 0;
            var stack = new WindowStack(model =>
            {
                built++;
                return model is PickFriendViewModel ? Greet : "<Window name=\"Lender\" />";
            });
            var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
            screen.Resize(800, 600, 1);
            var lender = new LenderViewModel();
            stack.Push(lender);
            lender.OpenHere(new PickFriendViewModel());
            DialogOverlay Dialog() => screen.Overlays.Select(o => o.Content).OfType<DialogOverlay>().Single();
            var before = Dialog().View;
            Assert.Equal("Hello", before.Parts[0].Caption());
            Assert.Equal(2, built);

            Locale.Choose("nl");

            Assert.Equal(4, built);
            Assert.NotSame(before, Dialog().View);
            Assert.Equal("Hallo", Dialog().View.Parts[0].Caption());

            // A stack no longer shown no longer listens.
            stack.Dispose();
            Locale.Choose("en");
            Assert.Equal(4, built);
        }
        finally
        {
            Locale.Choose("en");
            Locale.Reset();
        }
    }
}
