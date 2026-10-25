// Clarity's keys window on Avalonia (DESIGN 5.28): every command per place
// with its keys; Change, then press the new key; a key another command has
// in the same place offers a swap; Reset and Reset all; refused keys say why.

using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class KeysWindowTests
{
    static readonly KeyLayerInfo[] Catalog =
    [
        new("", null, [new("keysItem", Text.Plain("Keys"), []), new("quitItem", Text.Plain("Quit"), ["Ctrl+Q"])]),
        new("Card", Text.Plain("Card"), [new("Card.saveButton", Text.Plain("Save"), ["Ctrl+S"])]),
    ];

    static (Window Window, KeysView View) Shown()
    {
        PersonKeys.Stop();
        var view = new KeysView(Catalog);
        var window = new Window { Content = view, Width = 500, Height = 400 };
        window.Show();
        return (window, view);
    }

    // What the window shows once it has laid itself out again.
    static Button Named(KeysView view, string name)
    {
        view.UpdateLayout();
        return view.GetVisualDescendants().OfType<Button>().First(b => global::Avalonia.Automation.AutomationProperties.GetName(b) == name);
    }

    static IEnumerable<string> Texts(KeysView view)
    {
        view.UpdateLayout();
        return view.GetVisualDescendants().OfType<TextBlock>().Select(t => t.Text ?? "").ToList();
    }

    [AvaloniaFact]
    public void Lists_every_command_by_place_with_its_keys()
    {
        var (window, view) = Shown();
        var texts = Texts(view).ToList();
        foreach (var expected in new[] { "Keys", "Everywhere", "Quit", "Ctrl+Q", "Card", "Save", "Ctrl+S", "None" })
            Assert.Contains(expected, texts);
        window.Close();
    }

    [AvaloniaFact]
    public void A_new_key_is_taken_and_kept_and_reset_puts_it_back()
    {
        var (window, view) = Shown();
        Named(view, "Change Save").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        window.KeyPressQwerty(PhysicalKey.K, RawInputModifiers.Control);
        Assert.Equal(["Ctrl+K"], PersonKeys.Choices["Card.saveButton"]);
        Assert.Contains("Ctrl+K", Texts(view));
        Named(view, "Reset Save").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.Empty(PersonKeys.Choices);
        window.Close();
    }

    [AvaloniaFact]
    public void A_key_another_command_has_in_the_same_place_offers_a_swap()
    {
        var (window, view) = Shown();
        Named(view, "Change Keys").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        window.KeyPressQwerty(PhysicalKey.Q, RawInputModifiers.Control);
        Assert.Empty(PersonKeys.Choices);
        Assert.Contains("Ctrl+Q is the key of Quit. Swap them?", Texts(view));
        Named(view, "Swap").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.Equal(["Ctrl+Q"], PersonKeys.Choices["keysItem"]);
        Assert.Empty(PersonKeys.Choices["quitItem"]);
        Named(view, "Reset all").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.Empty(PersonKeys.Choices);
        window.Close();
    }

    [AvaloniaFact]
    public void A_key_the_inputs_need_is_refused_saying_why()
    {
        var (window, view) = Shown();
        Named(view, "Change Save").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        window.KeyPressQwerty(PhysicalKey.A, RawInputModifiers.None);
        Assert.Empty(PersonKeys.Choices);
        Assert.Contains("A cannot be used: a key without Ctrl or Alt is typing", Texts(view));
        window.Close();
    }
}
