# Clarity.Ui.Desktop

[Clarity.UI](https://github.com/petervdpas/Clarity) on the desktop: a screen of Clarity's UI framework in the system's own window, drawn by Skia through OpenGL, on Linux, Windows and macOS.

```csharp
DesktopWindow.Run(screen, "Hello", 800, 600);
```

A generated application runs its windows from its main window's view model:

```csharp
DesktopWindow.Run(new MainViewModel(databases));
```

Windows open in place; a dialog, the keys window and a window marked `in new window` get a desktop window of their own. The theme follows the system's light or dark. A window draws at the system's content scale (150 or 200 percent on Windows and X11 too), shows the application's icon (a resource `icon.png` it carries, else Clarity's), and the mouse pointer takes the shape of what is under it.

Drawing goes through OpenGL 3.3 core where the system has it, else an older OpenGL (2.1, down to Windows' own 1.1, as over Remote Desktop); where Skia cannot draw through it, it draws in memory and the window shows the pixels. `CLARITY_RENDER=software` asks for drawing in software, for a driver that draws wrongly.

Whatever fails on the window's loop (what the person did, drawing, laying out after a resize, time going on) is said as an error message in the window, and the program goes on.

Sounds play through [SoundFlow](https://www.nuget.org/packages/SoundFlow) (WAV, MP3 and FLAC, and the melodies and tones the sound part makes); a computer without a sound device runs on, silent. Timers and playing sounds tick from the window's frame loop, on the screen's thread.

The file dialogs are the system's own, with no package of their own, each over the window it was asked from; while one is open the windows take no input:

| System | Open, save and folder dialogs |
| --- | --- |
| Linux | the desktop portal over D-Bus (`org.freedesktop.portal.FileChooser`), else `zenity`, else `kdialog` |
| macOS | `osascript`: AppleScript's `choose file`, `choose file name` and `choose folder` |
| Windows | the common item dialog (`IFileDialog`) through COM |

A file opened in the system's program for its kind goes through `xdg-open`, `open` or the Windows shell.
