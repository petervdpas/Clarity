// The folder input (X2): a folder setting's path, typed or chosen with the
// system's folder picker, through the real FolderInput.

using Avalonia.Automation;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class FolderInputTests
{
    [AvaloniaFact]
    public void A_typed_or_chosen_folder_is_a_change()
    {
        var input = new FolderInput { ChooseText = "Choose a folder" };
        var changed = 0;
        input.ValueChanged += (_, _) => changed++;
        input.Text = "/mnt/photos";
        input.Take();
        Assert.Equal("/mnt/photos", input.Value);
        // A folder chosen with the picker is taken at once.
        input.Chosen("/home/ada/Pictures");
        Assert.Equal("/home/ada/Pictures", input.Value);
        Assert.Equal("/home/ada/Pictures", input.Text);
        input.Chosen("/home/ada/Pictures");
        Assert.Equal(2, changed);
        // Its button is named in Clarity's words.
        Assert.Equal("Choose a folder", AutomationProperties.GetName(input.ChooseButton));
        input.IsReadOnly = true;
        input.Chosen("/elsewhere");
        Assert.Equal("/home/ada/Pictures", input.Value);
    }
}
