// An application's screen gives each window what its host made for it: the
// system's file dialogs reach the parts of every window through their host.

using Clarity.Ui.Tests.Generated;
using Clarity.Ui.Tests.Parts;

namespace Clarity.Ui.Tests;

public class AppScreenTests
{
    [Fact]
    public void The_hosts_file_dialogs_reach_every_windows_parts()
    {
        var picker = new FilePartPicker();
        using var app = new AppScreen(name => name == "Windows/Orders.cxaml" ? """<Window name="Orders"><Folder at="1,1" span="4" name="export" /></Window>""" : null, systemDark: false, files: picker);
        app.Stack.Push(new OrdersViewModel());
        Shot.Of(app.Screen, 800, 600);
        var folder = Assert.IsType<FolderPart>(Assert.Single(app.Stack.View!.Parts));
        Assert.Same(picker, folder.Host.Files);
    }
}
