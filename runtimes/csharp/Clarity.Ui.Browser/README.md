# Clarity.Ui.Browser

[Clarity.UI](https://github.com/petervdpas/Clarity) in a browser page: the same screen as on the desktop, compiled to WebAssembly and drawn by Skia onto the page's canvas.

```csharp
await BrowserCanvas.Run(screen, "clarity");
```

The page needs a `<canvas id="clarity">`; without one, the screen takes the whole page.

A generated application runs its windows on the page as on the desktop:

```csharp
await BrowserCanvas.Run(new MainViewModel(databases));
```

A dialog and the keys window show over the window; web addresses open in a new tab, `mailto:` and `tel:` in the page. The theme follows the page's `prefers-color-scheme`.

The page is the program's `BrowserPage`: its title, downloads, files shown in a tab of their own, what the person keeps in the browser's storage, and sounds through Web Audio (a browser lets a page sound once the person pressed or typed on it). A file is opened through the page's file input and saved as a download; a page chooses no folder, so a folder is typed. Timers and playing sounds tick on the page's frames.

A program that reads what the browser keeps before it runs (its language, its theme) calls `BrowserCanvas.Prepare()` first.
