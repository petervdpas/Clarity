# Video and camera

Status: partial. The `video` part plays on Clarity.Video beside Clarity.UI on every stack; camera, recording, frame buffer and export are not built.

## Code
- Spec: `crates/clarity-spec/src/video.rs`; `video` kind in `widgets.rs:1332` (binds nothing); `crates/clarity-spec/src/resources.rs` (`plays_videos`, `carries_audio`; video resources `.mp4`); project line `codec x from "path"` (`crates/clarity-project/src/file.rs:550`)
- Generators: each stack's mod.rs adds Clarity.Video (`parts::VIDEO_KIND`); codec libraries `crates/clarity-gen/src/lib.rs:1308` (`codec_libraries`), :1398 (`with_codecs`, carried in `codecs/`); openh264 placement `crates/clarity-gen/src/clarity_ui.rs:142-195`
- Library: `runtimes/rust-video` (crate `clarity-video`: movie.rs MP4 reading, frames.rs threaded decoding, library.rs codec libraries, openh264.rs, `system/` GStreamer, Media Foundation, AVFoundation); codec C table `runtimes/rust-video/include/clarity_codec.h`; C interface `runtimes/rust-video-ffi` (`clarity_ui_use_video(clarity_video_api())`)
- Runtimes: R clarity-ui feature; Go, C, C++ `libclarity_video` from the Clarity.UI folder; C# NuGet `Clarity.Video` (`AddCodec`)
- Clarity.UI: `runtimes/rust-ui/src/parts/video.rs` (sound through Clarity.Audio as the clock); browser: the page's video element, frames to the worker (`runtimes/rust-ui/src/browser/page.rs:16-22`)
- IDE: `app/src-tauri/src/build.rs` puts openh264 in the Clarity.UI folder when it lacks it (background, checked by hash)
- Release: `.github/scripts/fetch_openh264.py` puts Cisco's binary in the archives, the installer staging and the NuGet
- Docs: manual `design.video.md`; LANGUAGE.md "Video" (:1581)

## Tests
- `crates/clarity-spec/tests/video.rs`; `crates/clarity-gen/tests/videos_build.rs` (ignored; passes on all five); `runtimes/rust-video` tests with `tests/videos/bar.mp4`
- No conformance file
- Example: `examples/videoplayer` (go desktop)

## Open
- `video` field kept in a file store, played by a bound video part: NOT BUILT (VIDEO-PLAN.md:26-27, 40).
- Project line `video openh264 | ffmpeg | system`; the `ffmpeg` and `system` codecs: NOT BUILT (VIDEO-PLAN.md:22, 39, step 7).
- Camera part (preview, stills, `on denied`, `on missing`), recording (encoder, MP4 writer, Opus), the browser camera, macOS camera text in Info.plist, a fake camera for tests: NOT BUILT (VIDEO-PLAN.md:41-84, steps 2, 3, 8).
- Frame buffer part, MP4 export, the video diary example: NOT BUILT (VIDEO-PLAN.md:81-108, steps 5, 6).
- Picture functions in engines; a video frame as a picture: NOT BUILT (VIDEO-PLAN.md:99).
- Windows (Media Foundation) and macOS (AVFoundation) decoders: UNTRIED (compiled, never run).
- A video watched playing in a running window: UNTRIED (VIDEO-PLAN.md status line at HEAD).

## Conflicts
- Manual `design.video.md:42` describes the Windows and macOS decoders with no caveat; they have not run.
- VIDEO-PLAN.md:30 writes a page block, which the loader refuses.
- The highlighter's keywords lack `codec` (`crates/clarity-spec/src/highlight.rs:48-90`).

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
- VIDEO-PLAN.md steps 1-3, 5-8, "Decisions", "The language", "Clarity.Video", "Every system", "Testing", "The IDE", "Export", "Pictures" (frame buffer), "To settle when building"
- VIDEO-PLAN.md step 4: see [picture-functions](picture-functions.md) and [canvas-toolbox](canvas-toolbox.md)
- memory: video-camera-plan.md
