# Clarity.Ui.Desktop

[Clarity.UI](https://github.com/petervdpas/Clarity) on the desktop: a screen of Clarity's UI framework in the system's own window, drawn by Skia through OpenGL, on Linux, Windows and macOS.

```csharp
DesktopWindow.Run(screen, "Hello", 800, 600);
```

A generated application runs its windows from its main window's view model:

```csharp
DesktopWindow.Run(new MainViewModel(databases));
```

Windows open in place; a dialog, the keys window and a window marked `in new window` get a desktop window of their own. The theme follows the system's light or dark.

Sounds play through [SoundFlow](https://www.nuget.org/packages/SoundFlow) (WAV, MP3 and FLAC, and the melodies and tones the sound part makes); a computer without a sound device runs on, silent. Timers and playing sounds tick from the window's frame loop, on the screen's thread.

The file dialogs are the system's own, with no package of their own:

| System | Open, save and folder dialogs |
| --- | --- |
| Linux | the desktop portal over D-Bus (`org.freedesktop.portal.FileChooser`), else `zenity`, else `kdialog` |
| macOS | `osascript`: AppleScript's `choose file`, `choose file name` and `choose folder` |
| Windows | the common item dialog (`IFileDialog`) through COM |

A file opened in the system's program for its kind goes through `xdg-open`, `open` or the Windows shell.
