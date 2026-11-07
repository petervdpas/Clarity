# Music in Clarity apps

A plan, agreed 2026-11-06 together with Clarity.Audio and Clarity.Video as libraries of their own. All five steps are built (2026-11-07).

## Goal

An app made with Clarity plays the person's own music: mp3, wav, flac and ogg files from a folder they pick or from records, with play, pause, seek, the time played and the length, and the next song when one ends. It does so on every stack (Rust, Go, C#, C, C++), on Linux, Windows and macOS, desktop and browser, without a library of the project's own: Clarity.Audio comes with Clarity. The video player of docs/VIDEO-PLAN.md builds on the same playing, so a video editor made in Clarity shares it.

## Where it stands

- The `sound` part plays only sound files of the project's resources, tones and melodies: `play()`, `stop()`, `volume()`.
- On the desktop every stack plays through Clarity.UI (rodio), which decodes WAV only. An mp3 or flac resource plays nothing there, although the manual names them. The browser decodes them all.
- So an mp3 player needs a library today (the mp3player example uses SoundFlow through one).

## Decisions

- **The sound part grows; no new part.** A sound already plays, has a volume and lives without a place in the grid. Music is the same part told to play a file.
- **Clarity.Audio, a library of its own.** Decoding and playing files is not in Clarity.UI but in Clarity.Audio, so an app that plays no music does not carry decoders. Clarity.UI keeps what it does now (tones, melodies, WAV resources) and hands everything else to Clarity.Audio when the app has it. Clarity.Video (docs/VIDEO-PLAN.md) is its sibling and plays its sound through Clarity.Audio.
- **Pure Rust decoders.** rodio's Symphonia decoders (mp3, flac, wav, ogg vorbis) are Rust crates with no system libraries, so one prebuilt library plays them on every OS (agreed 2026-11-06). AAC (m4a) waits for Clarity.Video, which needs MP4 anyway.
- **Streamed from disk.** A song is read as it plays, not loaded whole, so a long file starts at once and takes little memory.
- **One clock for sound and video.** The playing (open, play, pause, seek, position, length, ended) is Clarity.Audio's player; Clarity.Video's `video` part follows the same clock.

## The libraries

| | Clarity.Audio | Clarity.Video |
|---|---|---|
| Rust crate | `clarity-audio` (`runtimes/rust-audio`) | `clarity-video` (`runtimes/rust-video`) |
| C interface | `clarity_audio.h`, `libclarity_audio` (`runtimes/rust-audio-ffi`), prebuilt per system like Clarity.UI | `clarity_video.h`, `libclarity_video` |
| C# | NuGet `Clarity.Audio` with the native library | NuGet `Clarity.Video` |
| Go, C, C++ | the native library beside `clarity_ui` | the same |
| Browser | nothing: the page's own audio element | nothing: the page's own video element |

- **The seam.** Clarity.UI's sound part plays through its `SoundOutput`. Clarity.Audio gives it a player: in Rust, the `audio` feature of clarity-ui; on the other stacks, `clarity_ui_use_audio(clarity_audio_api())`, a table of C functions handed over at start. Neither library links the other.
- **Only when used.** The generator adds Clarity.Audio to a project whose sound parts play songs (`play(path)`, `pause()`, `resume()`, `seek()`, `on end` or `on fail` wired, a resource that is no WAV; `resources::plays_songs`), as it adds Clarity.UI to a desktop project. Without it, such a call says on the error output that Clarity.Audio is missing, and the window goes on.
- **Released on its own** by tag, like Clarity.UI: `runtimes/rust-audio/vX.Y.Z` for the crate, the native library in the Clarity.UI release workflow beside `clarity_ui`.

## The language

```clarity
page Main: window
    data tracks: list<Track>
    sound name player volume 80 on end: songEnded
    slider name progress at 1,12 span 8 bind shown min 0 max 100

code Main
    handler playClicked
        player.play(chosen.path)            // a path: of a folder setting's files, or written
        player.play("/music/song.flac")

    handler pauseClicked
        if player.paused
            player.resume()
        else
            player.pause()

    handler seekTo(seconds: integer)
        player.seek(seconds)
```

- **Calls:** `play()` (its source, as now), `play(path)`, `pause()`, `resume()`, `seek(seconds)`, `stop()`, `volume(level)`, `tone`, `melody` as now.
- **What code reads:** `player.playing`, `player.paused` (bool), `player.position` and `player.length` (whole seconds, for a slider and a time shown), `player.file` (text, what plays), `player.problem` (text, why the last song did not play). Clarity.UI keeps them in the part's values, which code reads as a custom part's; read, not set.
- **Events:** `on end: handler` when a song has played to its end (not when stopped), `on fail: handler` when a song cannot be played (missing, not a sound); `problem` says why.
- **Tags:** `player.title`, `player.artist`, `player.album` and `player.track` of the song that plays, read once by Clarity.Audio from its file (the C table's `tag`, interface 2), so every stack has them. Tags of many files for a list of songs (a `soundTags(path)` function) want a reader per stack's runtime, or Clarity.Audio called from each; later.
- The checker types the calls and the events; the designer's inspector shows `on end` and `on fail`.

## Steps

1. **The Clarity.Audio crate.** Symphonia decoders in rodio; a player that opens a file or bytes and plays, pauses, resumes, seeks, has a volume, and says its position, length and whether it ended. Tests with small sound files of each kind.
2. **Clarity.UI plays through it.** The `audio` feature in Rust, the C interface and `clarity_ui_use_audio` for the other stacks, the native library in the release workflow and fetched beside Clarity.UI; the generator adds it when used.
3. **Files, pause, seek, position, end in the language.** The calls, the state and the events above, through the reply's hears and the state each runtime keeps (Rust, Go, C#, C, C++ alike, shared cases in `conformance/sound.json`); the browser's player the same.
4. **Tags.** Reading title, artist, album and track from mp3 (ID3), flac and ogg (Vorbis comments), for the song that plays.
5. **The example.** The mp3player example plays with the sound part, its library gone; the manual's sound topic, the agent's instructions and the designer follow.

Then Clarity.Video's player starts from the same clock.

## The agent

The agent made the mp3 player with a library of its own because the sound part could not play files, and wrote the library's adapter wrong. Next to this plan, its instructions say how to take a request:

1. Look at the project and read the manual topics the request touches.
2. Find what Clarity does itself: the parts catalog (parts tool) and the manual first; a library or custom part only for what nothing built in does, and say so.
3. Data first, as a diagram, when the request needs data.
4. Write the windows and code, check after every write, build with the build tool and fix what it reports.
5. Say what is done and what Clarity cannot do yet, never inventing syntax.
