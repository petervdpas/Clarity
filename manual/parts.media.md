# Media parts

The parts of the Media group: the sound, which plays sound files, tones, melodies and songs, and the video, which plays a video in its place on the window.

## Playing sound and video

Drag **Sound** or **Video** from the **Media** group of the Parts palette onto the window. A sound is heard, not seen: it has no place in the window's grid, so the designer keeps it in the tray under the window, with the timers; pick it there to set it in the properties. A video stands in its place in the grid like any part.

Code tells both what to play, as it tells a custom part, and reads what they play; what they say is read, not set. The whole of what code tells a sound and reads of it is in [Sound](design.sound.md), and of a video in [Video](design.video.md).

## The parts

### `sound`

Plays the project's sound files, tones and melodies, and songs from disk.

```xml
<Sound name="name" at="col,row" span="1" source="file" volume="n" repeat="true" onend="handler" onfail="handler" />
```

A sound keeps the `at` and `span` the designer gives it, but takes no room in the grid. In Clarity's words it has none: `sound name rattle source "roll.wav" volume 80`.

| Property | Type | Default | What it does |
|---|---|---|---|
| `source` | a `.wav`, `.mp3`, `.flac` or `.ogg` file of the resources folder | none | What `play()` plays (**Source**). Add one under Resources with **+ New resource**, Sound; its resource view plays it. |
| `volume` | 0 to 100 | 100 | How loud it plays (**Volume**). |
| `repeat` | yes or no | no | Plays its source over and over until code stops it (**Repeat**). |

Events: `end` (a song played to its end; not when it was stopped) and `fail` (a song could not play, missing or not a sound; `problem` says why).

```xml
<!-- dice/design/TableView.cxaml -->
<View class="dice.TableView">
  <Sound name="rattle" at="1,1" span="1" source="roll.wav" volume="80" />
  <Sound name="fanfare" at="1,1" span="1" />
</View>
```

```clarity
code TableView
    handler rollClicked
        rattle.play()

    handler diceLanded(total: integer, faces: text)
        if total == count * 6
            fanfare.melody("C5/2 E5/2 G5/2 C6*2", 160)
```

- Code tells it `play()` (its source), `play(path)` (a song from disk), `pause()`, `resume()`, `seek(seconds)`, `stop()`, `tone(hertz, seconds)`, `melody(notes, tempo)` and `volume(level)` (0 to 100).
- It reads `playing` and `paused`; `position` and `length` in whole seconds (`length` is 0 when the file does not say); `file`, the path of the song that plays (empty for none or a resource); `problem`, why the last song did not play (empty when it did); and `title`, `artist`, `album` and `track`, what the song's file says of it (ID3 in mp3, Vorbis comments in flac and ogg; empty, and 0 for the track, when it does not say).
- A note is a letter `A` to `G`, then `#` or `b`, then an octave `0` to `8`, 4 unless given: `C4`, `F#3`, `Bb`. `-` is a rest. Each lasts a beat; `*2` makes it two beats, `/2` half a beat. A melody written in the code is checked; one made while the program runs skips the words that are not notes.
- On the desktop songs play through Clarity.Audio, which Clarity adds to a program that plays them; in a browser the page plays a song of the resources, and a file of the computer's disk cannot play there (`on fail`).

The checker refuses a sound inside another part ("a sound stands on the window, not in another part"), `at`, `span` or `rows` on a sound in Clarity's words, a volume above 100 ("a sound's volume is 0 to 100"), a source that is not a sound every stack plays ("use a .wav, .mp3, .flac or .ogg file") or is not in the resources folder, and in code a word a sound does not hear ("a sound hears play, pause, resume, seek, stop, tone, melody or volume"), the wrong number of values, a word of a melody that is not a note, and setting what it plays ("is read, not set").

### `video`

Plays a video in its place in the grid, as large as its span and rows: an `.mp4` of the resources or a file from disk, shown as large as fits, centred, on black.

```xml
<Video name="name" at="col,row" span="n" rows="n" source="file" volume="n" repeat="true" onend="handler" onfail="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `source` | an `.mp4` file of the resources folder | none | What `play()` plays (**Source**). Add one under Resources with **+ New resource**, Video. |
| `volume` | 0 to 100 | 100 | How loud its sound plays (**Volume**). |
| `repeat` | yes or no | no | Starts again at its end until code stops it (**Repeat**). |

Events: `end` (it played to its end) and `fail` (it could not play; `problem` says why).

```xml
<!-- cinema/design/ScreenView.cxaml -->
<View class="cinema.ScreenView">
  <Video name="film" at="1,1" span="8" rows="6" source="intro.mp4" volume="80" onend="filmEnded" onfail="filmFailed" />
</View>
```

```clarity
code ScreenView
    handler playClicked
        film.play()

    handler filmEnded
        film.seek(0)
```

- Code tells it `play()` (its source), `play(path)`, `pause()`, `resume()`, `seek(seconds)`, `stop()` and `volume(level)`, and reads `playing`, `paused`, `position` and `length` (whole seconds), `file` and `problem`.
- Its pictures come from a codec the project added, the system's own decoder, or Cisco's openh264 carried beside the program; its sound plays through Clarity.Audio, and the pictures follow it. See [Video](design.video.md).

The checker refuses a volume above 100, a source that is not an `.mp4` file or is not in the resources folder, and in code a word a video does not hear ("a video hears play, pause, resume, seek, stop or volume"), the wrong number of values, and setting what it plays ("is read, not set").

## See also

- [Sound](design.sound.md): what code tells a sound and reads of it, and songs.
- [Video](design.video.md): what code tells a video, its decoders and codecs.
- [Time](parts.time.md): a timer that moves a slider along as a song plays.
- [Handlers](code.handlers.md): the code behind that tells them what to play.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Sound | `sound` | nothing | 1 x 1 | no | `source`, `volume`, `repeat` | `end`, `fail` |
| Video | `video` | nothing | 8 x 6 | no | `source`, `volume`, `repeat` | `end`, `fail` |

<!-- /catalog -->
