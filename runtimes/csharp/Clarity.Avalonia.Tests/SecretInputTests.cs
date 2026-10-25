// A password input shows what is typed or hides it again, and copies it,
// with its buttons named in Clarity's own words (showSecret, hideSecret,
// copySecret), the same as on the web.

using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Input.Platform;
using Avalonia.Interactivity;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class SecretInputTests
{
    static (Window Window, TextInput Input) Shown(char passwordChar)
    {
        var input = new TextInput { PasswordChar = passwordChar, Value = "s3cr3t" };
        var window = new Window { Width = 400, Height = 100, Content = input };
        window.Show();
        return (window, input);
    }

    static Button Named(TextInput input, string name) =>
        Assert.Single(((Panel)input.InnerRightContent!).Children.OfType<Button>(), b => AutomationProperties.GetName(b) == name);

    [AvaloniaFact]
    public void The_eye_shows_what_is_typed_and_hides_it_again()
    {
        var (window, input) = Shown('*');
        Assert.False(input.RevealPassword);
        Named(input, "Show").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.True(input.RevealPassword);
        // Pressed, it says what it does next.
        Named(input, "Hide").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.False(input.RevealPassword);
        window.Close();
    }

    [AvaloniaFact]
    public async Task Copy_puts_what_is_in_the_input_on_the_clipboard_typed_or_not_yet_taken()
    {
        var (window, input) = Shown('*');
        input.Text = "typed";
        Named(input, "Copy").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        var clipboard = TopLevel.GetTopLevel(input)!.Clipboard!;
        Assert.Equal("typed", await clipboard.TryGetTextAsync());
        window.Close();
    }

    [AvaloniaFact]
    public void A_text_input_has_neither()
    {
        var (window, input) = Shown('\0');
        Assert.Null(input.InnerRightContent);
        window.Close();
    }
}
