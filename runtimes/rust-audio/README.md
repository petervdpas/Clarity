# clarity-audio

Clarity.Audio: the player behind the `sound` part of the programs Clarity generates. It plays
mp3, flac, wav and ogg (Vorbis) files, read from disk as they play, with pause, resume, seek and
a volume, and says the position, the length and when a song has played to its end.

The decoders are [Symphonia](https://github.com/pdeljanov/Symphonia)'s, through
[rodio](https://github.com/RustAudio/rodio): pure Rust, with no system libraries, so the same
library plays every format on Linux, Windows and macOS.

It draws nothing. Clarity.UI's sound part tells it what to play; a program that plays no files
does not carry it. In a browser the page's own audio element plays instead.

## Use

```rust
use clarity_audio::{Player, Speakers};

let speakers = Speakers::open()?;
let mut player = Player::on(speakers.mixer());
player.play_file(std::path::Path::new("song.flac"))?;
player.seek(std::time::Duration::from_secs(30))?;
```

A test plays on a mixer of its own (`clarity_audio::mixer`) and takes its samples as a sound card
would, so it runs without speakers.

## License

MIT.
