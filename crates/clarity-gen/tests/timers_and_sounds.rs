//! Timers and sounds in a window's markup (DESIGN 5.22, 5.26, 7.9): they have
//! no place on the screen, but a screen that reads only the markup (Clarity.UI)
//! runs a timer's ticks and plays a sound from there, so each is written
//! as any part of the window, and the grid leaves it out.

use clarity_spec::keyed::load_keyed;

const MONITOR: &str = "module monitor

page Monitor: window
    data readings: list<integer>
    button \"Sample\" name sampleButton at 1,1 span 2 on click: sampled
    sound name ding source \"ding.wav\" volume 80 repeat
    timer name clock every 0.5 on tick: sampled

code Monitor
    handler sampled
        ding.play()
";

fn markup() -> String {
    let project = load_keyed([("monitor.clarity", MONITOR)], &[], "en");
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    let page = project.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == "Monitor").unwrap();
    clarity_gen::web_windows::window_markup(page)
}

#[test]
fn a_window_s_timers_and_sounds_are_in_its_markup() {
    let markup = markup();
    for line in [
        "  <Sound name=\"ding\" at=\"1,1\" span=\"1\" source=\"ding.wav\" volume=\"80\" repeat=\"true\" />",
        "  <Timer name=\"clock\" at=\"1,1\" span=\"1\" every=\"0.5\" ontick=\"sampled\" />",
    ] {
        assert!(markup.contains(line), "{line}\nin\n{markup}");
    }
}

/// The C# program carries the markup with them, as Clarity.UI reads it.
#[test]
fn the_csharp_window_carries_its_timer_and_sound() {
    let project = load_keyed([("monitor.clarity", MONITOR)], &[], "en");
    let settings = clarity_project::parse("project Monitor\n    stack csharp desktop\n    datasource main sqlite \"data/monitor.db\"\n").unwrap();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let window = &files.iter().find(|f| f.path == "Windows/Monitor.cxaml").expect("the window's markup").text;
    assert!(window.contains("<Timer name=\"clock\""), "{window}");
    assert!(window.contains("<Sound name=\"ding\""), "{window}");
}
