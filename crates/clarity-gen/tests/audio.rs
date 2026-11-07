//! Songs on the desktop (docs/AUDIO-PLAN.md): a project whose sound parts
//! play more than WAV carries Clarity.Audio and hands it to Clarity.UI as
//! it starts, on every stack; one that plays only WAV carries nothing more.

use clarity_spec::resources::{self, ResourceFile};

const PLAYER: &str = "module player

page Main: window
    sound name music source \"SONG\"
    button \"Play\" name playButton at 1,1 span 2 on click: playClicked

code Main
    handler playClicked
        music.play()
";

fn settings(stack: &str) -> clarity_project::Settings {
    clarity_project::parse(&format!("project Player\n    stack {stack}\n")).unwrap()
}

fn generated(stack: &str, song: &str) -> Vec<clarity_gen::OutFile> {
    let mut project = clarity_spec::load_sources([("player.clarity", PLAYER.replace("SONG", song).as_str())]);
    resources::attach(&mut project, vec![ResourceFile { name: song.to_string(), bytes: vec![1, 2, 3] }]);
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    clarity_gen::generate(&project, &settings(stack)).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

fn all_code(files: &[clarity_gen::OutFile]) -> String {
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

#[test]
fn rust_turns_on_clarity_uis_audio_feature() {
    let manifest = file(&generated("rust desktop", "song.mp3"), "Cargo.toml").to_string();
    assert!(manifest.contains("clarity-ui = { version = \""), "{manifest}");
    assert!(manifest.contains("features = [\"audio\"] }"), "{manifest}");
    assert!(!file(&generated("rust desktop", "chime.wav"), "Cargo.toml").contains("audio"));
}

#[test]
fn csharp_references_clarity_audio_and_uses_it_before_the_first_window() {
    let files = generated("csharp desktop", "song.flac");
    assert!(file(&files, "Player.csproj").contains("<PackageReference Include=\"Clarity.Audio\""));
    assert!(file(&files, "Player.csproj").contains("<ProjectReference Include=\"$(ClarityRuntime)/Clarity.Audio/Clarity.Audio.csproj\" />"));
    let program = file(&files, "Program.cs");
    let used = program.find("Clarity.Audio.ClarityAudio.Use();").expect("Program uses Clarity.Audio");
    assert!(used < program.find("DesktopWindow.Run(").unwrap());
    assert!(!all_code(&generated("csharp desktop", "chime.wav")).contains("Clarity.Audio"));
}

#[test]
fn a_csharp_web_program_plays_songs_in_the_browser() {
    let code = all_code(&generated("csharp web", "song.mp3"));
    assert!(!code.contains("Clarity.Audio"), "{code}");
}

#[test]
fn go_links_clarity_audio_and_hands_it_over_as_it_starts() {
    let files = generated("go desktop", "song.ogg");
    let audio = file(&files, "audio.go");
    assert!(audio.contains("#cgo LDFLAGS: -lclarity_audio -lclarity_ui"));
    assert!(audio.contains("C.clarity_ui_use_audio(C.clarity_audio_api())"));
    assert!(!generated("go desktop", "chime.wav").iter().any(|f| f.path == "audio.go"));
}

#[test]
fn c_and_cpp_link_clarity_audio_and_hand_it_over_in_main() {
    for (stack, main) in [("c desktop", "src/main.c"), ("cpp desktop", "src/main.cpp")] {
        let files = generated(stack, "song.mp3");
        assert!(file(&files, "CMakeLists.txt").contains("clarity_use_audio(player)"), "{stack}");
        let text = file(&files, main);
        assert!(text.contains("#include \"clarity_audio.h\""), "{stack}");
        assert!(text.contains("clarity_ui_use_audio(clarity_audio_api())"), "{stack}");
        let chime = generated(stack, "chime.wav");
        assert!(!file(&chime, "CMakeLists.txt").contains("clarity_use_audio"), "{stack}");
        assert!(!file(&chime, main).contains("clarity_audio"), "{stack}");
    }
}

const READS: &str = "module player

page Main: window
    data played: integer
    data shown: text
    sound name music on end: songEnded on fail: songFailed
    button \"Play\" name playButton at 1,1 span 2 on click: playClicked

code Main
    handler playClicked
        music.play(\"/music/song.flac\")
        music.seek(played)
        played = music.position + music.length
        if music.playing or music.paused
            shown = music.file + music.title + music.artist + music.album
            played = music.track

    handler songEnded
        music.pause()

    handler songFailed
        shown = music.problem
";

/// What code reads of a sound comes from the values Clarity.UI keeps for it, on every stack.
#[test]
fn code_reads_what_a_sound_plays_on_every_stack() {
    for stack in ["rust desktop", "csharp desktop", "go desktop", "c desktop", "cpp desktop"] {
        let project = clarity_spec::load_sources([("player.clarity", READS)]);
        assert!(!project.has_errors(), "{:?}", project.diagnostics);
        let code = all_code(&clarity_gen::generate(&project, &settings(stack)).unwrap_or_else(|e| panic!("{stack}: {e:#?}")));
        for read in ["\"position\"", "\"length\"", "\"playing\"", "\"paused\"", "\"file\"", "\"problem\"", "\"title\"", "\"artist\"", "\"album\"", "\"track\""] {
            assert!(code.contains(read), "{stack} reads no {read}");
        }
        assert!(code.contains("\"seek\""), "{stack} tells no seek");
    }
}

const FILM: &str = "module player

page Main: window
    data at: integer
    data shown: text
    video name film at 1,1 span 8 rows 6 on end: filmEnded on fail: filmFailed
    button \"Play\" name playButton at 1,7 span 2 on click: playClicked

code Main
    handler playClicked
        film.play(\"/videos/holiday.mp4\")
        film.seek(at)
        at = film.position + film.length
        if film.playing or film.paused
            shown = film.file

    handler filmEnded
        film.pause()

    handler filmFailed
        shown = film.problem
";

/// What code reads of a video comes from the values Clarity.UI keeps for it, on every stack.
#[test]
fn code_reads_what_a_video_plays_on_every_stack() {
    for stack in ["rust desktop", "csharp desktop", "go desktop", "c desktop", "cpp desktop"] {
        let project = clarity_spec::load_sources([("player.clarity", FILM)]);
        assert!(!project.has_errors(), "{:?}", project.diagnostics);
        let code = all_code(&clarity_gen::generate(&project, &settings(stack)).unwrap_or_else(|e| panic!("{stack}: {e:#?}")));
        for read in ["\"position\"", "\"length\"", "\"playing\"", "\"paused\"", "\"file\"", "\"problem\""] {
            assert!(code.contains(read), "{stack} reads no {read}");
        }
        assert!(code.contains("<Video") || code.contains("Video"), "{stack} carries no video part");
    }
}

#[test]
fn rust_turns_on_clarity_uis_video_feature() {
    let project = clarity_spec::load_sources([("player.clarity", FILM)]);
    let files = clarity_gen::generate(&project, &settings("rust desktop")).unwrap();
    let manifest = file(&files, "Cargo.toml");
    assert!(manifest.contains("features = [\"video\"] }"), "{manifest}");
}

#[test]
fn every_desktop_stack_carries_clarity_video_and_audio_for_a_video() {
    let project = clarity_spec::load_sources([("player.clarity", FILM)]);
    let made = |stack: &str| clarity_gen::generate(&project, &settings(stack)).unwrap();
    let cs = made("csharp desktop");
    assert!(file(&cs, "Player.csproj").contains("<PackageReference Include=\"Clarity.Video\"") && file(&cs, "Player.csproj").contains("<PackageReference Include=\"Clarity.Audio\""));
    let program = file(&cs, "Program.cs");
    assert!(program.contains("Clarity.Audio.ClarityAudio.Use();") && program.contains("Clarity.Video.ClarityVideo.Use();"));
    let go = made("go desktop");
    assert!(file(&go, "video.go").contains("C.clarity_ui_use_video(C.clarity_video_api())") && go.iter().any(|f| f.path == "audio.go"));
    for (stack, main) in [("c desktop", "src/main.c"), ("cpp desktop", "src/main.cpp")] {
        let files = made(stack);
        let cmake = file(&files, "CMakeLists.txt");
        assert!(cmake.contains("clarity_use_video(player)") && cmake.contains("clarity_use_audio(player)"), "{stack}");
        assert!(file(&files, main).contains("clarity_ui_use_video(clarity_video_api())"), "{stack}");
    }
}

/// A codec library the project adds is carried in the program's `codecs`
/// folder and added to Clarity.Video as the program starts, on every stack;
/// one that is not there is an error.
#[test]
fn a_codec_library_the_project_adds_is_carried_and_added() {
    let folder = std::env::temp_dir().join(format!("clarity-codec-gen-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let extension = if cfg!(windows) { "dll" } else if cfg!(target_os = "macos") { "dylib" } else { "so" };
    let file = format!("libgrey.{extension}");
    std::fs::write(folder.join(&file), b"a codec").unwrap();
    let project = clarity_spec::load_sources([("player.clarity", FILM)]);
    for stack in ["rust desktop", "csharp desktop", "go desktop", "c desktop", "cpp desktop"] {
        let text = format!("project Player\n    stack {stack}\n    codec grey from \"libgrey\"\n");
        let settings = clarity_project::build_settings(&folder, &clarity_project::parse(&text).unwrap());
        let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{stack}: {e:?}"));
        let carried = files.iter().find(|f| f.path == format!("codecs/{file}")).unwrap_or_else(|| panic!("{stack} carries no codec"));
        assert_eq!(Some(&b"a codec"[..]), carried.bytes.as_deref(), "{stack}");
        let code = all_code(&files);
        let added = match stack {
            "rust desktop" => format!("clarity_ui::clarity_video::library::add(\"{file}\")"),
            "csharp desktop" => format!("Clarity.Video.ClarityVideo.AddCodec(\"{file}\");"),
            "go desktop" => format!("C.add_codec(C.CString(\"{file}\"), {})", file.len()),
            _ => format!("clarity_video_api()->add_codec(\"{file}\", {})", file.len()),
        };
        assert!(code.contains(&added), "{stack}: {added}");
    }
    let missing = clarity_project::build_settings(&folder, &clarity_project::parse("project Player\n    stack go desktop\n    codec vp9 from \"libvp9\"\n").unwrap());
    let errors = clarity_gen::generate(&project, &missing).unwrap_err();
    assert!(errors.iter().any(|e| e.contains("codec vp9: there is no library at")), "{errors:?}");
    let _ = std::fs::remove_dir_all(&folder);
}
