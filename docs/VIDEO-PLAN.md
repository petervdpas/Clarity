# Camera and video in Clarity

A plan, agreed 2026-11-05; nothing is built yet.

## Goal

A Clarity app can show a webcam, take a still from it, record video with sound, and play video back, on every stack (Rust, Go, C#, C, C++), on Linux, Windows and macOS, desktop and browser.

## Decisions

- **Built-in parts, not a custom canvas.** A custom part is a library per language, so a webcam part would have to be written again for each stack. The `camera` and `video` parts are Clarity.UI's, drawn as every part is.
- **Clarity.Video, a library of its own** (agreed 2026-11-06). Capture, the codecs and the container live in Clarity.Video, beside Clarity.UI, so an app without video does not carry them; it plays its sound through Clarity.Audio (docs/AUDIO-PLAN.md), on the same clock. Crate `clarity-video` (`runtimes/rust-video`), C interface `libclarity_video` handed to Clarity.UI with `clarity_ui_use_video(clarity_video_api())`, NuGet `Clarity.Video`; the generator adds it when a project has a `video` field or part. The browser needs none of it.
- **Pluggable codecs, openh264 by default.** The parts talk to a codec interface in Clarity.Video. The project picks the codec by name; without a `video` line it is openh264: H.264 in an MP4 file, which plays nearly everywhere.
- **Codecs load at run time.** The one prebuilt Clarity.Video library serves every choice, so no stack needs its own build. openh264 is Cisco's prebuilt binary, fetched per system the way Clarity fetches Clarity.UI, which keeps the H.264 royalties covered (a build from source is not covered).
- **Sound in Opus.** AAC, the usual sound of an MP4, has patents of its own; Opus in MP4 is free and plays in browsers and most players.

## The language

```clarity
project Studio
    stack go desktop
    video openh264
    filestore clips "clips"

entity Clip
    title: text(80)
    video: video, store clips
    still: image

page Recorder: window
    data clip: Clip
    camera name cam at 1,1 span 8 rows 8 bind clip.video audio on recorded: clipRecorded on denied: noCamera on missing: noCamera
    button @recordCaption name recordButton at 9,1 span 4 on click: recordClicked
    video name player at 1,10 span 8 rows 6 bind clip.video on ended: playEnded
```

| What | Meaning |
|---|---|
| `video openh264` | The codec, in the project file: `openh264` (the default when none is written), `ffmpeg` (the system's FFmpeg: also WebM, VP9, AV1) or `system` (the system's own encoders: Media Foundation, AVFoundation, GStreamer). |
| `video` field | A recorded video, always kept in a file store (`store clips`), never in the database. |
| `camera` part | The live picture of a camera; `bind` a video field it records into. Options: `device`, `audio` (record the microphone too), `resolution 1280 x 720`. |
| `cam.record()`, `cam.stop()` | Start and stop recording into the bound field. |
| `cam.snapshot()` | A still of what it shows, as a picture for an `image` field. |
| `on recorded`, `on denied`, `on missing` | Recording ended and the field holds it; the person or the system refused the camera; no camera, or no codec, with the reason. |
| `video` part | Plays a video field: play, pause, seek; `player.playing`, `player.position`; `on ended`. |

The checker holds a camera to a video field with a store, a codec name to the ones Clarity knows, and a format to what its codec writes. Clarity's own words name the buttons, the recording sign and "no camera", in every language.

## Clarity.Video

- **Capture** per system: V4L2 or PipeWire on Linux, AVFoundation on macOS, Media Foundation on Windows, through one Rust crate (such as `nokhwa`). Frames are drawn into the part as pictures.
- **Sound** through Clarity.Audio: played on its player, recorded through `cpal`, which it already uses.
- **The codec interface** (in Clarity.Video): an encoder, a decoder and a container writer, fed frames and sound. Each codec is a backend behind it; a new one is a name more, and the language and parts stay as they are.
- **Browser**: getUserMedia for the camera and MediaRecorder for recording, through the browser host.
- **When it fails**: a missing camera or codec runs `on missing` with the reason, a refusal `on denied`; the preview and stills work without a codec. Nothing crashes.

## Every system

- macOS wants a text saying why the app uses the camera and microphone; the generator writes it into the app's Info.plist.
- Windows asks through its privacy settings; Linux needs access to `/dev/video` or the PipeWire portal.
- A recording sign shows while recording, and screen readers hear the part's state.

## Testing

- A fake camera that shows a moving test pattern, and a fake codec, so every part is tested headless and under Xvfb, test first, with no webcam.
- Conformance cases every renderer shares: recording states, file names in the store, the events and what they give.

## The IDE

Both parts in the palette, a still placeholder frame in the designer, the inspector's options (device, audio, resolution, codec in the project settings), and the docs (LANGUAGE.md, DESIGN.md).

## Order

1. **The `video` field and the player.** Decoding only; starts after Clarity.Audio's player (AUDIO-PLAN step 2).
2. **The camera: preview and stills.** No encoder, so quick and already useful (a profile photo, scanning a page).
3. **Recording** with openh264 and MP4, sound in Opus.
4. **The other codecs:** `ffmpeg` and `system`.
5. **The browser target.**

## To settle when building

- The crates: the camera crate, the openh264 binding (loading Cisco's binary), an MP4 writer and an Opus encoder. Each is a new dependency, asked for first.
- Where the fetched openh264 binary is kept per system, as Clarity.UI's own library is.
- An example app that uses it: a small video diary, on a stack no example uses for this yet.
