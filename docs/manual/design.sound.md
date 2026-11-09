# Sound

How a window plays sound files, tones and melodies with the sound part, on every stack.

## The sound part

A sound is heard, not seen: it stands on a window without a place in its grid, and the designer keeps it in a tray under the window. Drag **Sound** from the Media group of the palette onto the window, then pick it in the tray to set it in the inspector.

```clarity
page Table: window
    sound name rattle source "roll.wav" volume 80
    sound name fanfare
```

- **Source**: a sound file of the resources folder, `.wav`, `.mp3`, `.flac` or `.ogg` (the formats every stack plays). Add one under Resources with **+ New resource**, Sound; its view plays it.
- **Volume**: 0 to 100, 100 unless set.
- **Repeat**: plays the source over and over until code stops it.

## Telling it what to play

Code tells a sound what to play, in handlers and the code behind:

| Code | What it does |
|---|---|
| `rattle.play()` | plays its source from the start |
| `rattle.stop()` | stops everything it plays |
| `fanfare.tone(440, 0.5)` | a tone of 440 hertz for half a second |
| `fanfare.melody("C4 E4 G4 C5*2", 120)` | the notes at 120 beats a minute |
| `rattle.volume(50)` | how loud it plays from now on |

A note is a letter `A` to `G`, then `#` or `b`, then an octave `0` to `8` (4 unless given): `C4`, `F#3`, `Bb`. A rest is `-`. A note lasts a beat; `*2` makes it two beats, `/2` half a beat. A melody written in the code is checked, so a word that is not a note is an error.

## Songs

A sound plays a song from disk too, such as a file of a music folder the person picked, and code follows it as it plays:

| Code | What it does |
|---|---|
| `music.play(path)` | plays the song at `path` (mp3, flac, ogg, wav), read as it plays |
| `music.pause()` and `music.resume()` | holds the song where it is, and plays on |
| `music.seek(90)` | moves the song to 90 seconds from its start |
| `music.playing`, `music.paused` | whether a song plays now, or is held |
| `music.position`, `music.length` | whole seconds played, and the song's length in whole seconds |
| `music.file` | the path of the song that plays |
| `music.problem` | why the last song did not play |
| `music.title`, `music.artist`, `music.album`, `music.track` | what the playing song's file says of it; empty when it does not say |

Pick the sound in the tray and wire **On end** (a song played to its end, to play the next one) and **On fail** (a song could not play; `problem` says why) in the inspector. A timer that reads `music.position` every half second moves a slider along as the song plays, and the slider's `on change` seeks.

A source that is no WAV (`.mp3`, `.flac`, `.ogg`) is a song too. On the desktop songs play through Clarity.Audio, a library beside Clarity.UI that reads the file as it plays; Clarity adds it to a program whose sound parts play songs, on every stack, and leaves it out of one that plays only WAV files, tones and melodies. A song that repeats starts again at its end. In a browser the page plays the songs of the resources itself; a file of the computer's disk cannot play there.

The `mp3player` example lists the songs of a music folder the person picks, plays them one after the other with pause, seek and volume, and shows what each song's file says of it.
