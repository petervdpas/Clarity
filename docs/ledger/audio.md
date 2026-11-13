# Audio

Status: built. The `sound` part (resources, tones, melodies, songs from files) on Clarity.Audio beside Clarity.UI, every stack, desktop and browser.

Progress: 90%

## Code
- Spec: `crates/clarity-spec/src/sound.rs` (calls, melodies :73); `crates/clarity-spec/src/resources.rs` (`plays_songs`; sounds wav, mp3, flac, ogg)
- Generators: each stack's mod.rs adds Clarity.Audio when `resources::plays_songs`; reads kept as custom-part values (`crates/clarity-gen/src/parts.rs`); `crates/clarity-gen/src/clarity_ui.rs` (`audio_file`)
- Library: `runtimes/rust-audio` (crate `clarity-audio` 0.2.0; rodio with Symphonia: mp3, flac, wav, ogg; play, pause, resume, seek, tags); C table `runtimes/rust-audio-ffi` (`libclarity_audio`, `clarity_audio.h`, interface 2 adds `tag`)
- Runtimes: R clarity-ui `audio` feature; Go, C, C++ `clarity_ui_use_audio(clarity_audio_api())` with `libclarity_audio` from the Clarity.UI folder; C# NuGet `Clarity.Audio` (`ClarityAudio.Use()`)
- Clarity.UI: `runtimes/rust-ui/src/parts/sound.rs`, `src/melodies.rs:16`; browser plays songs through the page's audio elements (`runtimes/rust-ui/src/browser/page.rs:16-22`)
- IDE: Media palette, tray under the window, inspector `on end` and `on fail`
- Docs: manual `design.sound.md`; LANGUAGE.md "Sound" (:1491)

## Tests
- `conformance/melody.json`: run by Clarity.UI (`runtimes/rust-ui/tests/parts/core_parts.rs`), spec (`crates/clarity-spec/tests/sound.rs`), C# (`MelodyTests.cs`); Go, C, C++ use Clarity.UI's melodies
- `crates/clarity-gen/tests/sound.rs`, `audio.rs`, `timers_and_sounds.rs`, `songs_build.rs` (ignored; passes on all five)
- Example: `examples/mp3player` (csharp desktop)

## Open
- `conformance/sound.json` for play, pause and seek: NOT BUILT (AUDIO-PLAN.md:63).
- `soundTags(path)` (tags of many files): NOT BUILT, deferred (AUDIO-PLAN.md:70).
- AAC (m4a) playback: NOT BUILT (AUDIO-PLAN.md:19).
- Played by a person on every stack and OS: UNTRIED (DESIGN SD1 "Not yet heard on every stack by a person").

## Conflicts
- Melodies are written three times: `crates/clarity-spec/src/sound.rs:73`, `runtimes/rust-ui/src/melodies.rs:16`, `runtimes/csharp/Clarity.Runtime/Melodies.cs:20` (equal today; the C# copy has no callers outside tests).
- AUDIO-PLAN.md:40 writes a page block, which the loader refuses.

## Decisions
- 2026-10-24: a built-in `sound` part with no place in the grid, kept in the designer's tray.
- 2026-11-06: the sound part grows, no new part; decoding lives in Clarity.Audio, not Clarity.UI. User: "go ahead with both"
- 2026-11-06: pure Rust Symphonia decoders via rodio, one prebuilt library for every OS.
- 2026-11-06: Clarity.Audio is added only to a program whose sound parts play songs; neither library links the other.
- 2026-11-06: one clock for sound and video; Clarity.Video plays its sound through Clarity.Audio.

## Was
- DESIGN 5.22, roadmap SD1
- AUDIO-PLAN.md steps 1-5, "Decisions", "The libraries", "The language" ("The agent" section: see [agent](agent.md))
- memory: audio-plan.md
