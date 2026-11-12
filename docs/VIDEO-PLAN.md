# Camera and video in Clarity

Open: the `video` field and the `video` project line, the camera (preview, stills, recording), the frame buffer, MP4 export, the `ffmpeg` and `system` codecs, the browser camera, picture functions in engines; the Windows and macOS decoders have not run yet. Built parts are in docs/IMPLEMENTED.md.

## Goal

A Clarity app can show a webcam, take a still from it, record video with sound, and play video back, on every stack (Rust, Go, C#, C, C++), on Linux, Windows and macOS, desktop and browser.

## Decisions

- **Built-in parts, not a custom canvas.** A custom part is a library per language, so a webcam part would have to be written again for each stack. The `camera` and `video` parts are Clarity.UI's, drawn as every part is.
- **Clarity.Video, a library of its own** (agreed 2026-11-06). Capture, the codecs and the container live in Clarity.Video, beside Clarity.UI, so an app without video does not carry them; it plays its sound through Clarity.Audio (docs/AUDIO-PLAN.md), on the same clock. Crate `clarity-video` (`runtimes/rust-video`), C interface `libclarity_video` handed to Clarity.UI with `clarity_ui_use_video(clarity_video_api())`, NuGet `Clarity.Video`; the generator adds it when a project has a `video` field or part. The browser needs none of it.
- **Which decoder plays** and **where openh264 comes from** (user's choices 2026-11-07): built; see docs/IMPLEMENTED.md section 6. The Windows and macOS decoders have not run yet.
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
- **The codec interface** (in Clarity.Video): a decoder (compressed frames in, pictures out) and later an encoder, with the container (MP4) read and written apart from them. Each codec is a backend registered by name for the formats it decodes (`avc1` for H.264); a new one is a name more, and the language and parts stay as they are.
- **Codecs people add** (user's wish, 2026-11-07): built; see docs/IMPLEMENTED.md section 6.
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

Working order (user's decision, 2026-11-07): step 1 first, then step 4 (image tools and the painting canvas), then steps 2 and 3, then the rest in order.

1. **The `video` field and the player.** The player is built; see docs/IMPLEMENTED.md section 6. Open: the `video` field, kept in a file store and played by a bound `video` part (see "The language").
2. **The camera: preview and stills.** No encoder, so quick and already useful (a profile photo, scanning a page).
3. **Recording** with openh264 and MP4, sound in Opus.
4. **Image tools and the painting canvas** (user's decision, 2026-11-07): the picture functions and the painting canvas's toolbox are built; see docs/IMPLEMENTED.md sections 3 and 4. What is open is in docs/IMAGE-PLAN.md and "Pictures" below.
5. **The frame buffer** (user's decision, 2026-11-07): a picture code and engines draw into per frame, shown on screen. See "Pictures" below.
6. **MP4 export** (user's decision, 2026-11-07): app code and engines hand Clarity.Video pictures and get an H.264 MP4 file, on the encoder and MP4 writer of step 3. See "Export" below.
7. **The other codecs:** `ffmpeg` and `system`.
8. **The browser target.**

## Export

Decided by the user on 2026-11-07, added to the plan only; playing comes first.

- **What it does.** A program makes a video of its own: code (or an engine) gives Clarity.Video one picture at a time, the same RGBA `Picture` playing gives, at a frame rate, with sound when it has some, and gets an H.264 MP4 file. All stacks (Rust, C#, Go, C, C++), through the same C table as playing.
- **On what is there.** The encoder and the MP4 writer of recording (step 3): openh264's encoder (the same carried binary; a system encoder where step 5 adds one), the MP4 writer, sound in Opus (or AAC from a system encoder). A codec a project adds may give an encoder too.
- **The language, to settle when building.** An export opened on a path with a size and a frame rate (`let film = export("out.mp4", 1080, 1080, 30)`), pictures added to it (`film.add(picture)`), sound added (`film.sound(samples)` or a sound file), and finished (`film.finish()`); the finished file goes where a file store or a path says. A long export runs as a long call, with the progress bar and Stop, as engines do.
- **Where the pictures come from:** the frame buffer, the painting canvas read back, image tools, a video's own pictures, an engine, a library or a custom part (see "Pictures").

## Pictures

Decided by the user on 2026-11-07 (relayed), added to the plan only; playing comes first. Everything here works on one type: the RGBA `Picture` that playing gives and export takes, so a picture goes from one to the other without conversion.

- **Image tools (step 4).** The functions in code are built on every stack; see docs/IMPLEMENTED.md section 3. Open: loading a video's frame as a picture, and engines getting the same functions, so heavy per-pixel work can run in an engine.
- **The painting canvas (step 4).** Its tools are built; see docs/IMPLEMENTED.md section 4. Open: layer opacity, and reading the result back as a picture (`let shot = board.picture()`) for the frame buffer, a picture field or export.
- **The frame buffer (step 5).** A picture a program and its engines draw into for each frame (pixels, pictures blended in, the tools above), shown in the window by a part that shows a picture as it changes, at the window's frame rate. What it shows is a picture at every moment, so a frame of it goes straight to export (`film.add(buffer.picture())`).
- **How they meet.** A video's frame, a canvas read back, a frame buffer and a loaded picture are all pictures; the tools take and give pictures; export takes pictures. The frame buffer and the canvas show pictures; nothing converts between them.

## To settle when building

- The crates: the camera crate, an MP4 writer and an Opus encoder, each asked for first. The crates for playing are chosen and built.
- An example app that uses it: a small video diary, on a stack no example uses for this yet.
