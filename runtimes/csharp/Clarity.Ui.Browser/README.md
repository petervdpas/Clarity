# Clarity.Ui.Browser

[Clarity.UI](https://github.com/petervdpas/Clarity) in a browser page: the same screen as on the desktop, compiled to WebAssembly and drawn by Skia onto the page's canvas.

```csharp
await BrowserCanvas.Run(screen, "clarity");
```

The page needs a `<canvas id="clarity">`; without one, the screen takes the whole page.
