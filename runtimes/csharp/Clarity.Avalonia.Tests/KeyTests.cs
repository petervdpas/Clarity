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

    [AvaloniaFact]
    public void ATypedKeyIsTheCharacterFromTheKeyboardOrTheNumberPad()
    {
        var plus = 0;
        var seven = 0;
        var add = new Button { Command = new RelayCommand(() => plus++) };
        Keys.SetTyped(add, "+");
        var digit = new Button { Command = new RelayCommand(() => seven++) };
        Keys.SetTyped(digit, "7");
        var field = new TextBox();
        var window = new Window { Content = new StackPanel { Children = { field, add, digit } } };
        window.Show();
        add.Focus();

        window.KeyPressQwerty(PhysicalKey.NumPadAdd, RawInputModifiers.None);
        // Shift and = types + (the headless keyboard does not shift it itself).
        window.KeyPress(Key.OemPlus, RawInputModifiers.Shift, PhysicalKey.Equal, "+");
        Assert.Equal(2, plus);
        window.KeyPressQwerty(PhysicalKey.Digit7, RawInputModifiers.None);
        window.KeyPressQwerty(PhysicalKey.NumPad7, RawInputModifiers.None);
        Assert.Equal(2, seven);
        // With Ctrl it is another key.
        window.KeyPressQwerty(PhysicalKey.Digit7, RawInputModifiers.Control);
        Assert.Equal(2, seven);

        // Typing in a text input is the person's typing, not the buttons'.
        field.Focus();
        window.KeyPressQwerty(PhysicalKey.Digit7, RawInputModifiers.None);
        window.KeyPressQwerty(PhysicalKey.NumPadAdd, RawInputModifiers.None);
        Assert.Equal((2, 2), (plus, seven));
    }
}
