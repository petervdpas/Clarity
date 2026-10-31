// The person's settings as a window sees them: read whole and given back
// whole, so a setting a part changed is part of that part's edit and Ctrl+Z
// takes it back (DESIGN 5.29). Every generated app's Settings is one.

using System.Text.Json.Nodes;

namespace Clarity.Runtime;

public interface ISettings
{
    /// <summary>Every setting as it is now, by its name.</summary>
    JsonObject ToJson();

    /// <summary>Takes the settings <paramref name="sent"/> gives, as they were.</summary>
    void Apply(JsonObject sent);
}
