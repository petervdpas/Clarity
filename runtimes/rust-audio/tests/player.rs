//! The player on a mixer of its own, without speakers: the test takes the
//! samples a sound card would, so time passes as it would when playing.

use std::num::NonZero;
use std::path::PathBuf;
use std::time::Duration;

use clarity_audio::{mixer, MixerSource, Player, Problem};

const RATE: u32 = 22050;

fn sound(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sounds").join(name)
}

fn output() -> (clarity_audio::Mixer, MixerSource) {
    mixer(NonZero::new(2).unwrap(), NonZero::new(RATE).unwrap())
}

/// Takes `seconds` of samples off the mixer, as the speakers would.
fn listen(speakers: &mut MixerSource, seconds: f32) {
    for _ in 0..(seconds * RATE as f32 * 2.0) as usize {
        speakers.next();
    }
}

#[test]
fn every_format_plays_and_knows_its_length() {
    for name in ["tone.mp3", "tone.flac", "tone.ogg", "tone.wav"] {
        let (mixer, mut speakers) = output();
        let mut player = Player::on(mixer);
        player.play_file(&sound(name)).unwrap_or_else(|why| panic!("{name}: {why}"));
        assert!(player.playing(), "{name}");
        assert_eq!(Some(sound(name).to_string_lossy().into_owned()), player.file());
        let length = player.length().unwrap_or_else(|| panic!("{name} has no length"));
        assert!(length > Duration::from_millis(1900) && length < Duration::from_millis(2200), "{name}: {length:?}");
        listen(&mut speakers, 0.5);
        assert!(player.position() >= Duration::from_millis(400), "{name}: {:?}", player.position());
        assert!(!player.ended(), "{name}");
    }
}

#[test]
fn a_song_that_played_to_its_end_has_ended() {
    let (mixer, mut speakers) = output();
    let mut player = Player::on(mixer);
    player.play_file(&sound("tone.flac")).unwrap();
    listen(&mut speakers, 2.5);
    assert!(player.ended());
    assert!(!player.playing());
}

#[test]
fn a_stopped_song_has_not_ended() {
    let (mixer, mut speakers) = output();
    let mut player = Player::on(mixer);
    player.play_file(&sound("tone.ogg")).unwrap();
    listen(&mut speakers, 0.2);
    player.stop();
    listen(&mut speakers, 0.2);
    assert!(!player.ended());
    assert!(!player.playing());
    assert_eq!(None, player.file());
}

#[test]
fn a_paused_song_stands_still_and_resumes() {
    let (mixer, mut speakers) = output();
    let mut player = Player::on(mixer);
    player.play_file(&sound("tone.wav")).unwrap();
    listen(&mut speakers, 0.3);
    player.pause();
    assert!(player.paused());
    assert!(!player.playing());
    // The audio thread sees the pause within a few milliseconds; from then on the song stands still.
    listen(&mut speakers, 0.02);
    let at = player.position();
    listen(&mut speakers, 0.5);
    assert_eq!(at, player.position());
    player.resume();
    assert!(player.playing());
    listen(&mut speakers, 0.3);
    assert!(player.position() > at);
}

#[test]
fn seeking_moves_the_position() {
    for name in ["tone.mp3", "tone.flac", "tone.ogg", "tone.wav"] {
        let (mixer, mut speakers) = output();
        let mut player = Player::on(mixer);
        player.play_file(&sound(name)).unwrap();
        listen(&mut speakers, 0.1);
        player.seek(Duration::from_millis(1500)).unwrap_or_else(|why| panic!("{name}: {why}"));
        listen(&mut speakers, 0.05);
        let at = player.position();
        assert!(at >= Duration::from_millis(1400) && at < Duration::from_millis(1800), "{name}: {at:?}");
    }
}

#[test]
fn a_paused_song_stays_paused_where_it_was_moved_to() {
    let (mixer, mut speakers) = output();
    let mut player = Player::on(mixer);
    player.play_file(&sound("tone.flac")).unwrap();
    player.pause();
    player.seek(Duration::from_secs(1)).unwrap();
    assert!(player.paused());
    listen(&mut speakers, 0.5);
    assert_eq!(Duration::from_secs(1), player.position());
    // Past the end is the end.
    player.seek(Duration::from_secs(60)).unwrap();
    assert!(player.ended());
    assert_eq!(player.length().unwrap(), player.position());
}

#[test]
fn nothing_playing_cannot_seek() {
    let (mixer, _speakers) = output();
    assert!(matches!(Player::on(mixer).seek(Duration::from_secs(1)), Err(Problem::CannotSeek(_))));
}

#[test]
fn bytes_play_as_a_file_does() {
    let (mixer, mut speakers) = output();
    let mut player = Player::on(mixer);
    player.play_bytes(std::fs::read(sound("tone.mp3")).unwrap(), Some("mp3")).unwrap();
    assert!(player.length().is_some());
    assert_eq!(None, player.file());
    listen(&mut speakers, 2.5);
    assert!(player.ended());
}

#[test]
fn the_volume_is_kept_between_songs() {
    let (mixer, _speakers) = output();
    let mut player = Player::on(mixer);
    player.set_volume(0.25);
    player.play_file(&sound("tone.wav")).unwrap();
    assert_eq!(0.25, player.volume());
    player.set_volume(3.0);
    assert_eq!(1.0, player.volume());
}

#[test]
fn what_cannot_play_says_why_and_plays_nothing() {
    let (mixer, _speakers) = output();
    let mut player = Player::on(mixer);
    let missing = sound("gone.mp3");
    assert_eq!(Err(Problem::Missing(missing.to_string_lossy().into_owned())), player.play_file(&missing));
    assert!(matches!(player.play_file(&sound("not-a-sound.mp3")), Err(Problem::NotASound(_))));
    assert!(!player.playing());
    assert!(!player.ended());
    assert_eq!("not a sound: text", Problem::NotASound("text".into()).to_string());
}

#[test]
fn a_new_song_takes_the_place_of_the_one_playing() {
    let (mixer, mut speakers) = output();
    let mut player = Player::on(mixer);
    player.play_file(&sound("tone.wav")).unwrap();
    listen(&mut speakers, 1.0);
    player.play_file(&sound("tone.flac")).unwrap();
    assert!(player.position() < Duration::from_millis(100));
    assert_eq!(Some(sound("tone.flac").to_string_lossy().into_owned()), player.file());
}

#[test]
fn a_songs_tags_are_read_from_its_file() {
    for name in ["tagged.mp3", "tagged.flac", "tagged.ogg"] {
        let tags = clarity_audio::tags(&sound(name));
        assert_eq!("Morning Tone", tags.title, "{name}");
        assert_eq!("Clarity Band", tags.artist, "{name}");
        assert_eq!("Test Album", tags.album, "{name}");
        assert_eq!(7, tags.track, "{name}");
    }
    // A song without tags, or no song at all, has empty ones.
    assert_eq!(clarity_audio::Tags::default(), clarity_audio::tags(&sound("tone.wav")));
    assert_eq!(clarity_audio::Tags::default(), clarity_audio::tags(&sound("gone.mp3")));
}

#[test]
fn the_player_knows_the_tags_of_what_plays() {
    let (mixer, _speakers) = output();
    let mut player = Player::on(mixer);
    player.play_file(&sound("tagged.flac")).unwrap();
    assert_eq!("Morning Tone", player.tags().title);
    player.play_bytes(std::fs::read(sound("tagged.mp3")).unwrap(), Some("mp3")).unwrap();
    assert_eq!("Clarity Band", player.tags().artist);
    player.stop();
    assert_eq!(clarity_audio::Tags::default(), *player.tags());
}
