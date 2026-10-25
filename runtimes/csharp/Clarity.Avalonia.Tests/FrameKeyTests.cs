// The keys of a window's frame (DESIGN 5.28): its menu items and its toolbar
// and ribbon buttons, a ribbon's whichever tab is open. The window's own
// buttons hear a key first; what code hid or disabled does not answer.

using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Clarity.Avalonia;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class FrameKeyTests
{
    sealed class Ran
    {
        public readonly List<string> Names = [];
        public RelayCommand Of(string name) => new(() => Names.Add(name));
    }

    static (Window Window, Ran Ran, Button Save, TabItem Insert, MenuItem Open) Shown()
    {
        var ran = new Ran();
        var open = new MenuItem { Header = "Open", Command = ran.Of("open") };
        Keys.SetFrame(open, KeyGesture.Parse("Ctrl+O"));
        var save = new Button { Content = "Save", Command = ran.Of("save") };
        Keys.SetFrame(save, KeyGesture.Parse("Ctrl+S"));
        var picture = new Button { Content = "Picture", Command = ran.Of("picture") };
        Keys.SetFrame(picture, KeyGesture.Parse("Ctrl+P"));
        var newFromFrame = new Button { Content = "New", Command = ran.Of("frame new") };
        Keys.SetFrame(newFromFrame, KeyGesture.Parse("Ctrl+N"));
        var insert = new TabItem { Header = "Insert", Content = new StackPanel { Children = { picture } } };
        var ribbon = new TabControl { ItemsSource = new[] { new TabItem { Header = "Home", Content = new StackPanel { Children = { save, newFromFrame } } }, insert } };
        var menu = new Menu { ItemsSource = new[] { new MenuItem { Header = "File", ItemsSource = new[] { open } } } };
        // The window's own button with the frame's key: it goes first.
        var mine = new Button { Content = "Mine", Command = ran.Of("window new"), HotKey = KeyGesture.Parse("Ctrl+N") };
        var body = new StackPanel { Children = { mine, new TextBox() } };
        var frame = new DockPanel { Children = { menu, ribbon, body } };
        Keys.SetFrameRoot(frame, true);
        var window = new Window { Content = frame };
        window.Show();
        body.Children[1].Focus();
        return (window, ran, save, insert, open);
    }

    [AvaloniaFact]
    public void A_frame_command_runs_by_its_key_a_ribbon_button_on_any_tab()
    {
        var (window, ran, _, _, _) = Shown();
        window.KeyPressQwerty(PhysicalKey.S, RawInputModifiers.Control);
        // On the Insert tab, which is not open.
        window.KeyPressQwerty(PhysicalKey.P, RawInputModifiers.Control);
        // In a menu that is not open.
        window.KeyPressQwerty(PhysicalKey.O, RawInputModifiers.Control);
        Assert.Equal(["save", "picture", "open"], ran.Names);
    }

    [AvaloniaFact]
    public void The_windows_own_button_hears_its_key_first()
    {
        var (window, ran, _, _, _) = Shown();
        window.KeyPressQwerty(PhysicalKey.N, RawInputModifiers.Control);
        Assert.Equal(["window new"], ran.Names);
    }

    [AvaloniaFact]
    public void What_code_hid_or_disabled_does_not_answer()
    {
        var (window, ran, save, insert, open) = Shown();
        save.IsEnabled = false;
        insert.IsVisible = false;
        open.IsEnabled = false;
        window.KeyPressQwerty(PhysicalKey.S, RawInputModifiers.Control);
        window.KeyPressQwerty(PhysicalKey.P, RawInputModifiers.Control);
        window.KeyPressQwerty(PhysicalKey.O, RawInputModifiers.Control);
        Assert.Empty(ran.Names);
    }
}
