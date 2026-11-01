# Clarity.UI samples

The same screen of Clarity.UI on the desktop and in a browser page (DESIGN 7.9).

- `Hello/`: the screen: a heading, a button drawn in the theme's `button` look (plain, hover, pressed, focus), a counter, and a panel that grows with the window.
- `Hello.Desktop/`: in the system's own window: `dotnet run --project Hello.Desktop`.
- `Hello.Browser/`: compiled to WebAssembly: `dotnet publish Hello.Browser`, then serve `bin/Debug/net10.0-browser/publish/wwwroot`. It needs .NET's WebAssembly tools once: `dotnet workload install wasm-tools`.
