// A button's keys: its own (HotKey) and a second one, such as the number
// pad's 7 beside the keyboard's, which a browser treats alike.

using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Clarity.Avalonia;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class KeyTests
{
    [AvaloniaFact]
    public void ASecondKeyPressesTheButtonAnywhereInTheWindow()
    {
        var presses = 0;
        var seven = new Button { Command = new RelayCommand(() => presses++), HotKey = KeyGesture.Parse("D7") };
        Keys.SetAlso(seven, KeyGesture.Parse("NumPad7"));
        var other = new TextBox();
        var window = new Window { Content = new StackPanel { Children = { other, seven } } };
        window.Show();
        other.Focus();

        window.KeyPressQwerty(PhysicalKey.NumPad7, RawInputModifiers.None);
        Assert.Equal(1, presses);
        window.KeyPressQwerty(PhysicalKey.Digit7, RawInputModifiers.None);
        Assert.Equal(2, presses);

        // A button that cannot be pressed is not pressed by its key either.
        seven.IsEnabled = false;
        window.KeyPressQwerty(PhysicalKey.NumPad7, RawInputModifiers.None);
        Assert.Equal(2, presses);

        // Gone from the window, its key is gone too.
        seven.IsEnabled = true;
        ((StackPanel)window.Content!).Children.Remove(seven);
        window.KeyPressQwerty(PhysicalKey.NumPad7, RawInputModifiers.None);
        Assert.Equal(2, presses);
    }
}
