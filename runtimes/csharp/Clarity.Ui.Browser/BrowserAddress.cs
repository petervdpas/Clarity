// How a browser page opens an address a link or a handler opens (DESIGN
// 7.9): a web page in a new tab, mail and phone numbers in the page itself,
// which hands them to the system's program for them. Nothing else opens: no
// script, no file of the computer.

namespace Clarity.Ui.Browser;

public static class BrowserAddress
{
    /// <summary>Where an address opens.</summary>
    public enum Way
    {
        None,
        Tab,
        Page,
    }

    /// <summary>Where <paramref name="address"/> opens: http and https in a new tab, mailto and tel in the page, anything else not at all.</summary>
    public static Way How(string address)
    {
        if (!Uri.TryCreate(address.Trim(), UriKind.Absolute, out var uri))
            return Way.None;
        return uri.Scheme.ToLowerInvariant() switch
        {
            "http" or "https" => Way.Tab,
            "mailto" or "tel" => Way.Page,
            _ => Way.None,
        };
    }
}
