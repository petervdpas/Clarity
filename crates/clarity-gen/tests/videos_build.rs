//! A small video player, built on each desktop stack as the IDE builds it,
//! without a warning: Clarity.Video and Clarity.Audio linked and handed to
//! Clarity.UI (docs/VIDEO-PLAN.md). Each is a whole build, so they are
//! ignored: run one at a time,
//! `cargo test -p clarity-gen --test videos_build -- --ignored --exact <name>`.
//! Go, C and C++ want Clarity.Audio's library beside Clarity.UI's:
//! `cargo build --release -p clarity-ui-ffi -p clarity-audio-ffi -p clarity-video-ffi`.

mod ide_build;

const MODULE: &str = "module player

page Main: window
    title @mainTitle
    data shown: text
    data played: integer
    video name film at 1,1 span 8 rows 6 source \"bar.mp4\" volume 80 on end: filmEnded on fail: filmFailed
    timer name clock every 0.5 on tick: clockTicked
    button @playCaption name playButton at 1,7 span 2 on click: playClicked
    button @pauseCaption name pauseButton at 3,7 span 2 on click: pauseClicked
    button @stopCaption name stopButton at 5,7 span 2 on click: stopClicked
    textbox name shownBox at 1,8 span 6 bind shown readonly

code Main
    handler playClicked
        film.play()

    handler pauseClicked
        if film.paused
            film.resume()
        else
            film.pause()

    handler stopClicked
        film.stop()

    handler clockTicked
        played = film.position
        if film.playing
            shown = film.file

    handler filmEnded
        film.play(\"/videos/holiday.mp4\")

    handler filmFailed
        shown = film.problem
";

fn builds_without_warnings(stack: &str) {
    let name = format!("videos-{}", stack.split(' ').next().unwrap());
    let root = ide_build::repo().join("target/generated").join(&name);
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("player");
    std::fs::create_dir_all(dir.join("resources")).unwrap();
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::copy(ide_build::repo().join("runtimes/rust-video/tests/videos/bar.mp4"), dir.join("resources/bar.mp4")).unwrap();
    std::fs::write(dir.join("player.clarity"), MODULE).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), "translation en\n    mainTitle \"Player\"\n    playCaption \"Play\"\n    pauseCaption \"Pause\"\n    stopCaption \"Stop\"\n").unwrap();
    // A codec library the project adds: the test codec of Clarity.Video, built with the system's C compiler.
    std::fs::create_dir_all(dir.join("codecs")).unwrap();
    let extension = if cfg!(windows) { "dll" } else if cfg!(target_os = "macos") { "dylib" } else { "so" };
    let built = std::process::Command::new("cc").args(["-shared", "-fPIC", "-o"]).arg(dir.join(format!("codecs/libgrey.{extension}"))).arg(ide_build::repo().join("runtimes/rust-video/tests/codec/grey.c")).status();
    let codec = if built.is_ok_and(|s| s.success()) { "    codec grey from \"codecs/libgrey\"\n" } else { "" };
    let file = root.join("player.clarity");
    std::fs::write(&file, format!("project Player\n    title \"Player\"\n    stack {stack}\n    output \"out\"\n    language \"en\"\n{codec}")).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    assert!(clarity_spec::resources::plays_videos(&project));
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a video player on rust desktop; run with --ignored, one at a time"]
fn a_rust_video_player_builds_without_warnings() {
    builds_without_warnings("rust desktop");
}

#[test]
#[ignore = "builds a video player on csharp desktop; run with --ignored, one at a time"]
fn a_csharp_video_player_builds_without_warnings() {
    builds_without_warnings("csharp desktop");
}

#[test]
#[ignore = "builds a video player on go desktop; run with --ignored, one at a time"]
fn a_go_video_player_builds_without_warnings() {
    builds_without_warnings("go desktop");
}

#[test]
#[ignore = "builds a video player on c desktop; run with --ignored, one at a time"]
fn a_c_video_player_builds_without_warnings() {
    builds_without_warnings("c desktop");
}

#[test]
#[ignore = "builds a video player on cpp desktop; run with --ignored, one at a time"]
fn a_cpp_video_player_builds_without_warnings() {
    builds_without_warnings("cpp desktop");
}
