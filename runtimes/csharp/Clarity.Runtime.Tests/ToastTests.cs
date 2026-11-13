// A toast the project declares (`toast saved`) travels as a message with its
// look: its name, level, lifespan, icon and corner; a plain message carries
// none. The same wire as every other runtime's.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class ToastTests
{
    [Fact]
    public void A_toast_travels_with_its_look()
    {
        var plain = WireJson.Message(new Message(Level.Info, Text.Plain("Plain")));
        Assert.False(plain.ContainsKey("toast"));
        var saved = WireJson.Message(new Message(Level.Info, Text.Plain("Saved Ada"), new ToastLook("saved", "success", 3000, "check", "bottomRight")));
        Assert.Equal("""{"name":"saved","level":"success","lifespan":3000,"corner":"bottomRight","icon":"check"}""", saved["toast"]!.ToJsonString());
        var imported = WireJson.Message(new Message(Level.Info, Text.Plain("Imported"), new ToastLook("imported", "info", 8000, null, "topLeft")));
        Assert.Equal("""{"name":"imported","level":"info","lifespan":8000,"corner":"topLeft"}""", imported["toast"]!.ToJsonString());
    }
}
