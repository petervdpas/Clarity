# Clarity.Avalonia.Desktop

[![NuGet](https://img.shields.io/nuget/v/Clarity.Avalonia.Desktop.svg)](https://www.nuget.org/packages/Clarity.Avalonia.Desktop/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt)

**Clarity.Avalonia.Desktop** is what only a desktop program has of the C# applications that [Clarity](https://github.com/petervdpas/Clarity) generates. It stands on [Clarity.Avalonia](https://www.nuget.org/packages/Clarity.Avalonia/), which a program served to browsers takes alone.

## What it does

- **Native windows.** Through Avalonia.Desktop, on Windows, macOS and Linux.
- **The shell.** `Shell` shows the application's windows in a desktop window, in place or in a desktop window of their own, and dialogs in windows over it. `Unconnected` shows the window that keeps a connection when the databases did not open.
- **Sound.** `SoundFlowOutput` plays the sound part through the computer's speakers, with SoundFlow (miniaudio).
- **Quit and keys.** The built-in `quit` ends the application, and `keys` shows Clarity's keys window in a window of its own (`KeysWindow`).

## How it is used

A Clarity project with `stack csharp avalonia desktop` generates a .NET project that references this package and starts with it:

```csharp
public static AppBuilder BuildAvaloniaApp() => AppBuilder.Configure<App>().UseClarityDesktop().LogToTrace();
```

## License

MIT, see [LICENSE.txt](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt).
