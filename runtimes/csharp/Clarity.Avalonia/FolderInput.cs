// Clarity's folder input on Avalonia (X2): the path of a folder setting,
// where a file store keeps its files. It is typed like any text, or chosen
// with the system's folder picker through its button, named in Clarity's
// own words; a chosen folder is taken at once. The same as on the web.

using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Platform.Storage;

namespace Clarity.Avalonia;

public class FolderInput : TextInput
{
    public static readonly StyledProperty<string> ChooseTextProperty =
        AvaloniaProperty.Register<FolderInput, string>(nameof(ChooseText), "Choose a folder");

    /// <summary>The words of the button, in the language shown.</summary>
    public string ChooseText
    {
        get => GetValue(ChooseTextProperty);
        set => SetValue(ChooseTextProperty, value);
    }

    /// <summary>The button that opens the system's folder picker.</summary>
    public Button ChooseButton { get; }

    public FolderInput()
    {
        ChooseButton = new Button { Content = "...", Focusable = false };
        AutomationProperties.SetName(ChooseButton, ChooseText);
        ToolTip.SetTip(ChooseButton, ChooseText);
        ChooseButton.Click += async (_, _) => await Choose();
        InnerRightContent = ChooseButton;
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ChooseTextProperty)
        {
            AutomationProperties.SetName(ChooseButton, ChooseText);
            ToolTip.SetTip(ChooseButton, ChooseText);
        }
        else if (change.Property == IsReadOnlyProperty)
            ChooseButton.IsEnabled = !IsReadOnly;
    }

    async Task Choose()
    {
        if (IsReadOnly || TopLevel.GetTopLevel(this)?.StorageProvider is not { } storage || !storage.CanPickFolder)
            return;
        var folders = await storage.OpenFolderPickerAsync(new FolderPickerOpenOptions { AllowMultiple = false, Title = ChooseText });
        if (folders.Count > 0 && folders[0].TryGetLocalPath() is { } path)
            Chosen(path);
    }

    /// <summary>Takes a folder the picker gave: a change when it differs from the value.</summary>
    public void Chosen(string path)
    {
        if (IsReadOnly)
            return;
        Text = path;
        Take();
    }
}
