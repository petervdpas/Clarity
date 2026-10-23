# Sound

How a window plays sound files, tones and melodies with the sound part, on every stack.

## The sound part

A sound is heard, not seen: it stands on a window without a place in its grid, and the designer keeps it in a tray under the window. Drag **Sound** from the Media group of the palette onto the window, then pick it in the tray to set it in the inspector.

```clarity
page Table: window
    sound name rattle source "roll.wav" volume 80
    sound name fanfare
```

- **Source**: a sound file of the resources folder, `.wav`, `.mp3` or `.flac` (the formats every stack plays). Add one under Resources with **+ New resource**, Sound; its view plays it.
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

The web stacks play with the browser engine's audio, C# with SoundFlow. The `dice` example rattles when the dice are thrown and plays a fanfare when every die shows six.
