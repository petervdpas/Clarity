//! A small player whose sound part plays an mp3, built on each desktop stack
//! as the IDE builds it, without a warning: Clarity.Audio linked and handed
//! to Clarity.UI (docs/AUDIO-PLAN.md). Each is a whole build, so they are
//! ignored: run one at a time,
//! `cargo test -p clarity-gen --test songs_build -- --ignored --exact <name>`.
//! Go, C and C++ want Clarity.Audio's library beside Clarity.UI's:
//! `cargo build --release -p clarity-ui-ffi -p clarity-audio-ffi`.

mod ide_build;

const MODULE: &str = "module player

page Main: window
    title @mainTitle
    data shown: text
    data played: integer
    data length: integer
    sound name music source \"tone.mp3\" volume 80 on end: songEnded on fail: songFailed
    timer name clock every 0.5 on tick: clockTicked
    button @playCaption name playButton at 1,1 span 2 on click: playClicked
    button @pauseCaption name pauseButton at 3,1 span 2 on click: pauseClicked
    button @stopCaption name stopButton at 5,1 span 2 on click: stopClicked
    slider name progress at 1,2 span 6 bind played min 0 max 100 on change: progressChanged
    textbox name shownBox at 1,3 span 6 bind shown readonly

code Main
    handler playClicked
        music.play(\"/music/song.flac\")

    handler pauseClicked
        if music.paused
            music.resume()
        else
            music.pause()

    handler stopClicked
        music.stop()

    handler progressChanged
        music.seek(played)

    handler clockTicked
        played = music.position
        length = music.length
        if music.playing
            shown = music.file

    handler songEnded
        music.play()

    handler songFailed
        shown = music.problem
";

fn builds_without_warnings(stack: &str) {
    let name = format!("songs-{}", stack.split(' ').next().unwrap());
    let root = ide_build::repo().join("target/generated").join(&name);
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("player");
    std::fs::create_dir_all(dir.join("resources")).unwrap();
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::copy(ide_build::repo().join("runtimes/rust-audio/tests/sounds/tone.mp3"), dir.join("resources/tone.mp3")).unwrap();
    std::fs::write(dir.join("player.clarity"), MODULE).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), "translation en\n    mainTitle \"Player\"\n    playCaption \"Play\"\n    pauseCaption \"Pause\"\n    stopCaption \"Stop\"\n").unwrap();
    let file = root.join("player.clarity");
    std::fs::write(&file, format!("project Player\n    title \"Player\"\n    stack {stack}\n    output \"out\"\n    language \"en\"\n")).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    assert!(clarity_spec::resources::plays_songs(&project));
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a song player on rust desktop; run with --ignored, one at a time"]
fn a_rust_song_player_builds_without_warnings() {
    builds_without_warnings("rust desktop");
}

#[test]
#[ignore = "builds a song player on csharp desktop; run with --ignored, one at a time"]
fn a_csharp_song_player_builds_without_warnings() {
    builds_without_warnings("csharp desktop");
}

#[test]
#[ignore = "builds a song player on go desktop; run with --ignored, one at a time"]
fn a_go_song_player_builds_without_warnings() {
    builds_without_warnings("go desktop");
}

#[test]
#[ignore = "builds a song player on c desktop; run with --ignored, one at a time"]
fn a_c_song_player_builds_without_warnings() {
    builds_without_warnings("c desktop");
}

#[test]
#[ignore = "builds a song player on cpp desktop; run with --ignored, one at a time"]
fn a_cpp_song_player_builds_without_warnings() {
    builds_without_warnings("cpp desktop");
}
