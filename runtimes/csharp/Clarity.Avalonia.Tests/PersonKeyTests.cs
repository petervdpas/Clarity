// The person's keys on Avalonia (DESIGN 5.28): a key pressed reads as Clarity
// writes keys; the keys are kept in the settings file beside the settings;
// a button and a frame command answer the person's key, not the design's.

using System.Text.Json.Nodes;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Clarity.Avalonia;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class PersonKeyTests
{
    [Fact]
    public void A_pressed_key_reads_as_clarity_writes_keys_and_back()
    {
        Assert.Equal("Ctrl+Shift+N", KeyRules.Written(Key.N, KeyModifiers.Control | KeyModifiers.Shift));
        Assert.Equal("Ctrl+1", KeyRules.Written(Key.D1, KeyModifiers.Control));
        Assert.Equal("Alt+Left", KeyRules.Written(Key.Left, KeyModifiers.Alt));
        Assert.Equal("Backspace", KeyRules.Written(Key.Back, KeyModifiers.None));
        Assert.Equal("Ctrl++", KeyRules.Written(Key.OemPlus, KeyModifiers.Control));
        Assert.Null(KeyRules.Written(Key.LeftCtrl, KeyModifiers.Control));
        Assert.Equal(KeyGesture.Parse("Ctrl+D1"), KeyRules.Gesture("Ctrl+1"));
        Assert.Equal(KeyGesture.Parse("Ctrl+OemPlus"), KeyRules.Gesture("Ctrl++"));
        Assert.Equal(KeyGesture.Parse("Shift+F5"), KeyRules.Gesture("Shift+F5"));
    }

    // On the UI thread: setting keys walks the buttons other tests showed there.
    [AvaloniaFact]
    public void The_persons_keys_are_kept_beside_the_settings()
    {
        var home = Directory.CreateTempSubdirectory("clarity-keys-");
        var was = Environment.GetEnvironmentVariable("XDG_CONFIG_HOME");
        Environment.SetEnvironmentVariable("XDG_CONFIG_HOME", home.FullName);
        try
        {
            KeptSettings.Keep("keys-test", new JsonObject { ["pageSize"] = 20 });
            PersonKeys.Start("keys-test");
            Assert.Empty(PersonKeys.Choices);
            PersonKeys.Set(new Dictionary<string, IReadOnlyList<string>> { ["Card.saveButton"] = ["Ctrl+K"] });
            var kept = KeptSettings.Read("keys-test");
            Assert.Equal(20, kept["pageSize"]!.GetValue<int>());
            Assert.Equal("Ctrl+K", kept["keys"]!["Card.saveButton"]![0]!.GetValue<string>());
            PersonKeys.Start("keys-test");
            Assert.Equal(["Ctrl+K"], PersonKeys.Of("Card.saveButton", "Ctrl+S"));
            Assert.Equal(["Ctrl+Q"], PersonKeys.Of("quitItem", "Ctrl+Q"));
        }
        finally
        {
            PersonKeys.Stop();
            Environment.SetEnvironmentVariable("XDG_CONFIG_HOME", was);
            home.Delete(true);
        }
    }

    [AvaloniaFact]
    public void A_button_and_a_frame_command_answer_the_persons_key()
    {
        var ran = new List<string>();
        var save = new Button { Content = "Save", Command = new RelayCommand(() => ran.Add("save")), HotKey = KeyGesture.Parse("Ctrl+S") };
        Keys.SetId(save, "Card.saveButton");
        var print = new Button { Content = "Print", Command = new RelayCommand(() => ran.Add("print")) };
        Keys.SetFrame(print, KeyGesture.Parse("Ctrl+P"));
        Keys.SetId(print, "printButton");
        var frame = new DockPanel { Children = { print, new StackPanel { Children = { save, new TextBox() } } } };
        Keys.SetFrameRoot(frame, true);
        var window = new Window { Content = frame };
        window.Show();
        try
        {
            PersonKeys.Set(new Dictionary<string, IReadOnlyList<string>> { ["Card.saveButton"] = ["Ctrl+K"], ["printButton"] = ["Ctrl+L"] });
            window.KeyPressQwerty(PhysicalKey.S, RawInputModifiers.Control);
            window.KeyPressQwerty(PhysicalKey.K, RawInputModifiers.Control);
            window.KeyPressQwerty(PhysicalKey.P, RawInputModifiers.Control);
            window.KeyPressQwerty(PhysicalKey.L, RawInputModifiers.Control);
            Assert.Equal(["save", "print"], ran);
            // Reset: the design's keys again.
            PersonKeys.Set(new Dictionary<string, IReadOnlyList<string>>());
            window.KeyPressQwerty(PhysicalKey.S, RawInputModifiers.Control);
            window.KeyPressQwerty(PhysicalKey.P, RawInputModifiers.Control);
            Assert.Equal(["save", "print", "save", "print"], ran);
        }
        finally
        {
            PersonKeys.Stop();
            window.Close();
        }
    }
}
