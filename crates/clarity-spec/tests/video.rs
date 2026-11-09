//! The video part (docs/VIDEO-PLAN.md): a part of a window's grid that plays
//! a video, a file of the resources or one from disk, told by code as a
//! sound is told, with what plays read back and `on end` and `on fail`.

use clarity_spec::resources::{self, ResourceFile};
use clarity_spec::widgets::{WidgetKind, widget_line};
use clarity_spec::{Project, Severity, load_converted};

const SCREEN: &str = "module m

page Screen: window
    data shown: text
    data at: integer
    video name film at 1,1 span 8 rows 6 source \"intro.mp4\" volume 80 on end: filmEnded on fail: filmFailed
    button \"Play\" name playButton at 1,7 span 2 on click: playClicked

code Screen
    handler playClicked
        film.play()
        film.play(\"/videos/holiday.mp4\")
        if film.paused
            film.resume()
        else
            film.pause()
        film.seek(12.5)
        film.volume(50)
        at = film.position + film.length
        shown = film.file

    handler filmEnded
        film.stop()

    handler filmFailed
        shown = film.problem
";

fn with_files(source: &str, names: &[&str]) -> Project {
    let mut project = load_converted([("m.clarity", source)]);
    resources::attach(&mut project, names.iter().map(|n| ResourceFile { name: n.to_string(), bytes: vec![0, 0, 0, 24] }).collect());
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn code_plays_pauses_seeks_and_reads_a_video() {
    let project = with_files(SCREEN, &["intro.mp4"]);
    assert_eq!(errors(&project), Vec::<String>::new());
    let film = &project.pages().nth(0).unwrap().widgets[0].props;
    assert_eq!(film.kind, WidgetKind::Video);
    assert_eq!(film.source.as_deref(), Some("intro.mp4"));
    assert_eq!(widget_line(film), "video name film at 1,1 span 8 rows 6 source \"intro.mp4\" volume 80 on end: filmEnded on fail: filmFailed");
}

#[test]
fn what_code_tells_and_reads_of_a_video_is_checked() {
    let e = errors(&with_files(&SCREEN.replace("film.stop()", "film.melody(\"C4\", 60)"), &["intro.mp4"]));
    assert!(e.iter().any(|m| m.contains("a video hears play, pause, resume, seek, stop or volume, not 'melody'")), "{e:?}");
    let e = errors(&with_files(&SCREEN.replace("shown = film.file", "film.position = 3"), &["intro.mp4"]));
    assert!(e.iter().any(|m| m.contains("film.position is read, not set")), "{e:?}");
}

#[test]
fn a_video_source_is_a_video_of_the_resources() {
    let e = errors(&with_files(SCREEN, &[]));
    assert!(e.iter().any(|m| m.contains("video 'intro.mp4' is not in the resources folder")), "{e:?}");
    let e = errors(&with_files(&SCREEN.replace("intro.mp4", "intro.avi"), &["intro.avi"]));
    assert!(e.iter().any(|m| m.contains("'intro.avi' is not a video every stack can play; use an .mp4 file")), "{e:?}");
    assert_eq!(resources::kind_of("clip.MP4"), resources::ResourceKind::Video);
}

#[test]
fn a_project_with_a_video_plays_videos() {
    assert!(resources::plays_videos(&with_files(SCREEN, &["intro.mp4"])));
    assert!(!resources::plays_videos(&with_files("module m\n\npage P: window\n    sound name s\n", &[])));
}
