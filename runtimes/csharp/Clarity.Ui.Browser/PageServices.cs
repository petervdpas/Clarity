// What a browser page gives the program before its windows show (DESIGN
// 7.6, 7.9): itself as the BrowserPage (its title, downloads, files shown in
// a tab), its storage for the person's settings, as the browser keeps them
// across reloads.

using Clarity.Runtime;

namespace Clarity.Ui.Browser;

public static class PageServices
{
    /// <summary>
    /// Makes <paramref name="page"/> the program's: the BrowserPage, the store
    /// of the person's settings (a file in WebAssembly's memory would be lost
    /// at every reload).
    /// </summary>
    public static void Use(IBrowserPage page)
    {
        BrowserPage.Current = page;
        KeptSettings.Store = (page.Read, page.Write);
    }
}
