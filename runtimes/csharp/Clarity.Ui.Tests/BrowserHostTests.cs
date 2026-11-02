// The browser host (DESIGN 7.9): a page shows the application's windows as
// the desktop does, through the same stack, from the markup the application
// carries; it opens no window of its own, so a dialog and the keys window
// show over the window. An address opens in a new tab (http, https) or in
// the page itself (mailto, tel); the theme follows prefers-color-scheme.

using Clarity.Runtime;
using Clarity.Ui.Browser;

namespace Clarity.Ui.Tests;

public class BrowserHostTests
{
    static string? Carried(string name) => name switch
    {
        "Windows/Lender.cxaml" => "<Window name=\"Lender\"><Label name=\"hello\" at=\"1,1\" span=\"4\" caption=\"Hello\" /></Window>",
        "Windows/PickFriend.cxaml" => "<Window name=\"PickFriend\"><Button name=\"cancelButton\" at=\"1,1\" span=\"6\" caption=\"Cancel\" cancel=\"true\" onclick=\"close\" /></Window>",
        _ => null,
    };

    [Fact]
    public void An_address_opens_in_a_new_tab_or_in_the_page()
    {
        Assert.Equal(BrowserAddress.Way.Tab, BrowserAddress.How("https://clarity.dev"));
        Assert.Equal(BrowserAddress.Way.Tab, BrowserAddress.How("HTTP://example.org/a?b=c"));
        Assert.Equal(BrowserAddress.Way.Page, BrowserAddress.How("mailto:ada@example.org"));
        Assert.Equal(BrowserAddress.Way.Page, BrowserAddress.How("tel:+31201234567"));
        // Anything else is not opened: no script, no file of the computer.
        Assert.Equal(BrowserAddress.Way.None, BrowserAddress.How("javascript:alert(1)"));
        Assert.Equal(BrowserAddress.Way.None, BrowserAddress.How("file:///etc/passwd"));
        Assert.Equal(BrowserAddress.Way.None, BrowserAddress.How("not an address"));
    }

    [Fact]
    public void The_page_shows_the_main_window_from_its_markup_and_dialogs_over_it()
    {
        using var app = new AppScreen(Carried, systemDark: false);
        var lender = new LenderViewModel();
        app.Stack.Push(lender);
        Shot.Of(app.Screen, 800, 600);
        Assert.Single(app.Stack.View!.Parts);
        lender.LendClicked();
        Assert.IsType<DialogOverlay>(Assert.Single(app.Screen.Overlays).Content);
        app.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Empty(app.Screen.Overlays);
        Assert.Equal(1, lender.ReturnedTimes);
    }

    [Fact]
    public void The_keys_window_shows_over_the_window_and_closes()
    {
        using var app = new AppScreen(Carried, systemDark: false);
        app.Stack.Push(new LenderViewModel());
        Shot.Of(app.Screen, 800, 600);
        app.ShowKeys();
        var keys = Assert.IsType<KeysWindow>(Assert.Single(app.Screen.Overlays).Content);
        Assert.True(keys.Focused);
        app.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Empty(app.Screen.Overlays);
    }

    [Fact]
    public void The_look_follows_the_system_s_light_or_dark()
    {
        using var app = new AppScreen(Carried, systemDark: false);
        app.Stack.Push(new LenderViewModel());
        var redrawn = 0;
        app.Screen.Invalidated += () => redrawn++;
        Assert.False(app.Screen.Look.Dark);
        app.SystemDark = true;
        Assert.True(app.Screen.Look.Dark);
        Assert.True(redrawn > 0);
    }

    /// <summary>A page that keeps what it was asked to download and show.</summary>
    sealed class FilesPage : IBrowserPage
    {
        public readonly List<string> Asked = [];
        public void SetTitle(string title) { }
        public void Download(string name, byte[] data) => Asked.Add($"download {name} {data.Length}");
        public void Show(string name, byte[] data) => Asked.Add($"show {name} {data.Length}");
        public string? Read(string key) => null;
        public void Write(string key, string value) { }
        public int Play(byte[] file, double volume, bool repeat) => 0;
        public void Stop(int sound) { }
    }

    [Fact]
    public async Task A_page_opens_files_from_its_file_input_and_saves_them_as_downloads()
    {
        var page = new FilesPage();
        var accepts = new List<string>();
        var picker = new PagePicker(page, accept =>
        {
            accepts.Add(accept);
            return Task.FromResult<ChosenFile?>(new ChosenFile("a.png", [1]));
        });
        Assert.Equal("a.png", (await picker.OpenFile("Choose", new FileFilter("Pictures", ["*.png", "*.jpg"])))!.Name);
        await picker.OpenFile("Choose");
        await picker.OpenFile("Choose", new FileFilter("All", ["*.*"]));
        Assert.Equal([".png,.jpg", "", ""], accepts);
        Assert.Equal("notes.txt", await picker.SaveFile("Save", "notes.txt", [1, 2]));
        picker.Show("report.pdf", [1, 2, 3]);
        Assert.Equal(["download notes.txt 2", "show report.pdf 3"], page.Asked);
        // A page chooses no folder: a folder is typed there.
        Assert.False(picker.PicksFolders);
        Assert.Null(await picker.ChooseFolder("Choose a folder"));
    }

    [Fact]
    public void A_file_shown_in_a_tab_is_given_a_kind_that_cannot_run_script()
    {
        Assert.Equal("application/pdf", PagePicker.TypeOf("Report.PDF"));
        Assert.Equal("image/png", PagePicker.TypeOf("a.png"));
        Assert.Equal("text/plain", PagePicker.TypeOf("data.csv"));
        Assert.Equal("application/octet-stream", PagePicker.TypeOf("page.html"));
        Assert.Equal("application/octet-stream", PagePicker.TypeOf("drawing.svg"));
        Assert.Equal("application/octet-stream", PagePicker.TypeOf("no-extension"));
    }
}
