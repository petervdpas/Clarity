# Video and camera

Status: partial. The `video` part plays on Clarity.Video beside Clarity.UI on every stack; camera, recording, frame buffer and export are not built.

Progress: 30%

## Code
- Spec: `crates/clarity-spec/src/video.rs`; `video` kind in `widgets.rs:1347` (binds nothing); `crates/clarity-spec/src/resources.rs` (`plays_videos`, `carries_audio`; video resources `.mp4`); project line `codec x from "path"` (`crates/clarity-project/src/file.rs:580`)
- Generators: each stack's mod.rs adds Clarity.Video (`parts::VIDEO_KIND`); codec libraries `crates/clarity-gen/src/lib.rs:1347` (`codec_libraries`), :1437 (`with_codecs`, carried in `codecs/`); openh264 placement `crates/clarity-gen/src/clarity_ui.rs:142-195`
- Library: `runtimes/rust-video` (crate `clarity-video`: movie.rs MP4 reading, frames.rs threaded decoding, library.rs codec libraries, openh264.rs, `system/` GStreamer, Media Foundation, AVFoundation); codec C table `runtimes/rust-video/include/clarity_codec.h`; C interface `runtimes/rust-video-ffi` (`clarity_ui_use_video(clarity_video_api())`)
- Runtimes: R clarity-ui feature; Go, C, C++ `libclarity_video` from the Clarity.UI folder; C# NuGet `Clarity.Video` (`AddCodec`)
- Clarity.UI: `runtimes/rust-ui/src/parts/video.rs` (sound through Clarity.Audio as the clock); browser: the page's video element, frames to the worker (`runtimes/rust-ui/src/browser/page.rs:16-22`)
- IDE: `app/src-tauri/src/build.rs` puts openh264 in the Clarity.UI folder when it lacks it (background, checked by hash)
- Release: `.github/scripts/fetch_openh264.py` puts Cisco's binary in the archives, the installer staging and the NuGet
- Docs: manual `design.video.md`, `parts.media.md` (`video`)

## Tests
- `crates/clarity-spec/tests/video.rs`; `crates/clarity-gen/tests/videos_build.rs` (ignored; passes on all five); `runtimes/rust-video` tests with `tests/videos/bar.mp4`
- No conformance file
- Example: `examples/videoplayer` (go desktop)

## Open
- `video` field kept in a file store, played by a bound video part: NOT BUILT; see Design "The video field".
- Camera part, preview and stills (step 2): NOT BUILT; see Design "The camera part".
- Recording with sound (step 3): NOT BUILT; see Design "Recording".
- Camera texts per system, the recording sign: NOT BUILT; see Design "Every system".
- A fake camera and fake codec for tests; conformance cases: NOT BUILT; see Design "Testing".
- The frame buffer (step 5); picture functions in engines; a video frame as a picture: NOT BUILT; see Design "The frame buffer" and "Pictures in engines".
- MP4 export (step 6): NOT BUILT; see Design "Export".
- Encoders and decoders of FFmpeg and the system for recording (step 7): NOT BUILT; see Design "Other codecs".
- The browser camera (step 8): NOT BUILT; see Design "The browser".
- The camera in the IDE: NOT BUILT; see Design "The IDE".
- The crates for capture and recording, the video diary example: NOT BUILT; see Design "To settle when building".
- Windows (Media Foundation) and macOS (AVFoundation) decoders: UNTRIED (compiled, never run).
- A video watched playing in a running window: UNTRIED.

## Design

Working order (Decisions 2026-11-07): step 1 and step 4 are built; next steps 2 and 3, then 5 to 8.

### The video field
- A `video` field type: a recorded or chosen video, always kept in a file store, never in the database.

```clarity
entity Clip
    title: text(80)
    video: video, store clips
    still: image
```

- A `video` part binds a video field and plays it: `<Video name="player" ... bind="clip.video" onend="playEnded" />`. Today the part binds nothing (`crates/clarity-spec/src/widgets.rs:1365`).
- The checker holds a `video` field to a store.

### The camera part
- A built-in `camera` part of Clarity.UI (Decisions 2026-11-05): the live picture of a camera, drawn as every part is.
- It binds a video field it records into: `<Camera name="cam" ... bind="clip.video" audio="true" onrecorded="clipRecorded" ondenied="noCamera" onmissing="noCamera" />`.
- Options: `device`, `audio` (record the microphone too), `resolution` (such as 1280 x 720).
- Calls: `cam.record()` and `cam.stop()` start and stop recording into the bound field; `cam.snapshot()` gives a still of what it shows, a picture for an `image` field.
- Events: `on recorded` (recording ended, the field holds it), `on denied` (the person or the system refused the camera), `on missing` (no camera, or no codec, with the reason).
- Step 2 is the preview and stills only, no encoder: already useful for a profile photo or scanning a page.
- When it fails: a missing camera or codec runs `on missing`, a refusal `on denied`; preview and stills work without a codec. Nothing crashes.
- Clarity's own words name its buttons, the recording sign and "no camera", in every language.
- The checker holds a camera to a video field with a store, and a format to what its codec writes.

### Recording
- Capture per system in Clarity.Video: V4L2 or PipeWire on Linux, AVFoundation on macOS, Media Foundation on Windows, through one Rust crate. Frames are drawn into the part as pictures.
- Encoding with openh264's encoder (the binary every app with video carries) into MP4, written apart from the codec.
- Sound recorded through `cpal` (Clarity.Audio uses it) and encoded as Opus in the MP4 (Decisions 2026-11-07: not AAC).
- A codec added with `codec x from "path"` may give an encoder too; the codec C table (`runtimes/rust-video/include/clarity_codec.h`) gets an encoder beside its decoder.

### Every system
- macOS wants a text saying why the app uses the camera and microphone; the generator writes it into the app's Info.plist.
- Windows asks through its privacy settings; Linux needs access to `/dev/video` or the PipeWire portal.
- A recording sign shows while recording, and screen readers hear the part's state.

### Testing
- A fake camera showing a moving test pattern and a fake codec, so the camera part is tested headless and under Xvfb with no webcam, test first.
- Conformance cases every stack runs: recording states, file names in the store, the events and what they give.

### The frame buffer
- A picture a program and its engines draw into each frame (pixels, pictures blended in, the picture functions), shown in the window by a part that shows a changing picture at the window's frame rate.
- A frame of it goes straight to export: `film.add(buffer.picture())`.
- Which part shows it, and its name, is an open question.

### Pictures in engines
- Engines get the picture functions too, so heavy per-pixel work runs in an engine (see [engines](engines.md)).
- A video's frame can be had as a picture, the same RGBA picture playing gives.
- A video frame, `board.picture()` (see [canvas-toolbox](canvas-toolbox.md) Design), a frame buffer and a loaded picture are all pictures; nothing converts between them.

### Export
- A program makes a video of its own: code or an engine gives Clarity.Video one picture at a time at a frame rate, with sound when it has some, and gets an H.264 MP4 file. All five stacks, through the same C table as playing.
- On the encoder and MP4 writer of recording, sound in Opus (or AAC from a system encoder).
- The language, to settle when building: an export opened on a path with a size and a frame rate (`let film = export("out.mp4", 1080, 1080, 30)`), `film.add(picture)`, `film.sound(samples)` or a sound file, `film.finish()`. The file goes where a file store or a path says. A long export runs as a long call with the progress bar and Stop, as engines do.
- Pictures come from the frame buffer, the painting canvas read back, the picture functions, a video's own frames, an engine, a library or a custom part.

### Other codecs
- Recording and export through FFmpeg (also WebM, VP9, AV1) and the system's own encoders (Media Foundation, AVFoundation, GStreamer).
- Decoding through the system is built; which codec plays is decided as in Decisions 2026-11-07. How a project asks for FFmpeg or a system encoder (a `codec ... from` line or something else) is an open question.

### The browser
- getUserMedia for the camera and MediaRecorder for recording, through the browser host. The browser needs none of Clarity.Video.

### The IDE
- The camera part in the Media palette, a still placeholder frame in the designer, the inspector's options (device, audio, resolution), and the manual (`design.video.md`, `parts.media.md`).

### To settle when building
- The crates: a camera crate (such as `nokhwa`), an MP4 writer and an Opus encoder, each asked for first.
- An example: a small video diary, on a stack no video example uses yet.

## Conflicts
- Manual `design.video.md:42` describes the Windows and macOS decoders with no caveat; they have not run.
- The highlighter's keywords lack `codec` (`crates/clarity-spec/src/highlight.rs:49-91`).

## Decisions
- 2026-11-05: camera and video are built-in Clarity.UI parts, not a custom canvas or custom part.
- 2026-11-06: capture, codecs and container live in Clarity.Video, a library of its own; sound through Clarity.Audio.
- 2026-11-07: which decoder plays: a codec the project added, else the system decoder, else openh264 (user's choice, replacing "openh264 by default").
- 2026-11-07: every app with video carries Cisco's openh264 beside its program; nothing is downloaded while an app runs; the IDE keeps it current when online, never while building (user's choice).
- 2026-11-07: codecs people add: `codec vp9 from "codecs/libvp9"` with a C table, or `clarity_video::register` in Rust (user's wish).
- 2026-11-07: crates approved: openh264 0.9 (Cisco 2.6.0 binary), mp4 0.14, bzip2, symphonia aac and isomp4.
- 2026-11-07: working order: step 1, then step 4 (image tools, painting canvas), then steps 2 and 3, then the rest (user's decision).
- 2026-11-07: sound in recordings is Opus, not AAC (patents).
- An app carrying openh264 means its maker holds the H.264 licence (Via LA); the manual says so.

## Was
- VIDEO-PLAN.md "Decisions", "Clarity.Video" (codec interface, codecs people add: built)
- VIDEO-PLAN.md "The language": the `video` field, the camera part and its calls and events now in Design; the `video openh264|ffmpeg|system` project line and `on ended` are outdated (the code has `codec x from` and `on end`)
- VIDEO-PLAN.md steps 1 (field: Design "The video field"), 2 (Design "The camera part"), 3 (Design "Recording"), 5 (Design "The frame buffer"), 6 (Design "Export"), 7 (Design "Other codecs"), 8 (Design "The browser")
- VIDEO-PLAN.md "Every system", "Testing", "The IDE", "Export", "Pictures" (frame buffer, engines, a video frame), "To settle when building": now Design sections of the same names
- VIDEO-PLAN.md step 4: see [picture-functions](picture-functions.md) and [canvas-toolbox](canvas-toolbox.md)
- memory: video-camera-plan.md
