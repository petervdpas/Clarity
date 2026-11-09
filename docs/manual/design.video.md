# Video

How a window plays a video with the video part, on every stack.

## The video part

A video stands on a window in its place in the grid, as large as its span and rows, and shows the video as large as fits, centred, on black. Drag **Video** from the Media group of the palette onto the window and set it in the inspector.

In the window's file, `design/Screen.cxaml`:

```xml
<Window class="cinema.Screen" title="@screenTitle">
  <Video name="film" at="1,1" span="8" rows="6" source="intro.mp4" volume="80" onend="filmEnded" onfail="filmFailed" />
</Window>
```

The handlers `filmEnded` and `filmFailed` are in the module's `code Screen` block.

- **Source**: an `.mp4` video of the resources folder, which `film.play()` plays. Add one under Resources with **+ New resource**, Video.
- **Volume**: 0 to 100, 100 unless set.
- **Repeat**: starts again at its end until code stops it.

## Telling it what to play

| Code | What it does |
|---|---|
| `film.play()` | plays its source from the start |
| `film.play(path)` | plays the video at `path` from disk, such as a file of a folder the person picked |
| `film.pause()` and `film.resume()` | holds the video where it is, and plays on |
| `film.seek(90)` | moves the video to 90 seconds from its start |
| `film.stop()` | stops it: no picture, no sound |
| `film.volume(50)` | how loud its sound plays from now on |
| `film.playing`, `film.paused` | whether it plays now, or is held |
| `film.position`, `film.length` | whole seconds played, and its length in whole seconds |
| `film.file` | the path of the video that plays; empty for none or a resource |
| `film.problem` | why the last video did not play |

Wire **On end** (it played to its end) and **On fail** (it could not play; `problem` says why) in the inspector.

## Which decoder plays it

Clarity adds Clarity.Video to a program with video parts, on every stack, and Clarity.Audio for its sound; its pictures follow the sound, so they stay together. The first of these plays a video: a codec the project added, the system's own decoder (Media Foundation on Windows, AVFoundation on macOS, GStreamer on Linux when it is installed), or Cisco's openh264, which every app with video carries beside its program, so videos play offline. Clarity provides openh264 itself and keeps it up to date from Cisco.

## Codecs your project adds

A video in a format the system and openh264 do not play (VP9, AV1) plays through a codec library the project adds, written in any language that makes a C library (C, C++, Rust, Go, C#'s native AOT), often around libvpx, dav1d or FFmpeg. It gives Clarity.Video one table, `clarity_codec_api`, which `runtimes/rust-video/include/clarity_codec.h` describes: its name, the formats it takes, and a decoder that turns a frame into an RGBA picture. Name it in the project file:

```clarity
project Films
    stack go desktop
    codec vp9 from "codecs/libvp9"
```

Clarity carries the library for the system it builds on in the program's `codecs` folder and adds it as the program starts; for the formats it takes it plays before the system's decoder and openh264. A library that is not there is an error when the project is generated. In Rust a crate can add a codec too, with `clarity_video::register`.

An app that carries openh264 makes its maker the holder of the H.264 licence (through Via LA; free up to 100,000 copies a year, but signed). Cisco covers the royalties only for its binary downloaded from Cisco, which an app that must play offline cannot do.
