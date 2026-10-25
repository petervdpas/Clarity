//! Live graphs (DESIGN 5.26): a `chart line` (or `bar`) of a list of
//! numbers draws them as they are, the last `points` of them; a `trend`
//! part keeps the last `points` values of one number, taken every so many
//! seconds; a `timer` runs a handler every so many seconds while its window
//! is open. Code grows a list of values with `+`.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{Event, WidgetKind, widget_line};
use clarity_spec::{Severity, load_sources};

const WATCH: &str = "module watch

page Monitor: window
    data cpu: decimal
    data readings: list<decimal>
    timer name clock every 1 on tick: sampled
    trend name cpuTrend at 1,1 span 6 rows 4 bind cpu points 60 max 100
    chart line name history at 1,5 span 6 rows 4 bind readings points 30 max 100
    button \"Stop\" name stop at 7,1 span 2 on click: stopClicked

code Monitor
    handler sampled
        cpu = cpu + 1
        readings = readings + [cpu]
    handler stopClicked
        clock.enabled = false
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("watch.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(WATCH.contains(old), "{old:?}");
    WATCH.replacen(old, new, 1)
}

#[test]
fn a_monitor_checks() {
    assert_eq!(errors(WATCH), Vec::<String>::new());
}

#[test]
fn the_parts_read_and_write_back() {
    let (file, diagnostics) = parse(WATCH, "watch.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let widgets = &file.pages[0].widgets;
    let clock = &widgets[0].props;
    assert_eq!(clock.kind, WidgetKind::Timer);
    assert_eq!(clock.every.as_deref(), Some("1"));
    assert!(clock.handlers.iter().any(|h| h.event == Event::Tick));
    let trend = &widgets[1].props;
    assert_eq!((trend.kind, trend.points, trend.max.as_deref()), (WidgetKind::Trend, Some(60), Some("100")));
    assert_eq!(widgets[2].props.points, Some(30));
    assert_eq!(widget_line(clock), "timer name clock every 1 on tick: sampled");
    assert_eq!(widget_line(trend), "trend name cpuTrend at 1,1 span 6 rows 4 bind cpu points 60 max 100");
    assert_eq!(widget_line(&widgets[2].props), "chart line name history at 1,5 span 6 rows 4 bind readings points 30 max 100");
}

#[test]
fn a_timer_has_no_place_and_runs_every_so_many_seconds() {
    let placed = with("timer name clock every 1", "timer name clock at 1,1 every 1");
    assert!(errors(&placed).iter().any(|e| e.contains("a timer has no place in the window")), "{:?}", errors(&placed));
    assert_eq!(errors(&with("timer name clock every 1 on tick", "timer name clock on tick")), ["timer clock runs every so many seconds: write every 1, or every 0.5"]);
    assert!(errors(&with("every 1 on tick", "every 0 on tick")).iter().any(|e| e.contains("a timer runs every 0.1 to 86400 seconds")), "{:?}", errors(&with("every 1 on tick", "every 0 on tick")));
}

#[test]
fn a_trend_follows_one_number() {
    let text = with("    data cpu: decimal\n", "    data cpu: decimal\n    data label: text\n").replace("bind cpu points 60", "bind label points 60");
    assert!(errors(&text).iter().any(|e| e.contains("'label' is text; a trend shows")), "{:?}", errors(&text));
    assert!(errors(&with("points 60", "points 1")).iter().any(|e| e.contains("a trend or chart shows 2 to 1000 points")));
}

#[test]
fn a_chart_of_numbers_draws_them_as_they_are() {
    let texts = with("    data readings: list<decimal>\n", "    data readings: list<text>\n").replace("        readings = readings + [cpu]\n", "");
    assert!(errors(&texts).iter().any(|e| e.contains("a chart of a list of values draws numbers")), "{:?}", errors(&texts));
    let grouped = with("bind readings points 30", "bind readings by x count points 30");
    assert!(errors(&grouped).iter().any(|e| e.contains("a chart of numbers draws them as they are")), "{:?}", errors(&grouped));
    let pie = with("chart line name history", "chart pie name history");
    assert!(errors(&pie).iter().any(|e| e.contains("a pie shares out groups; a list of numbers is drawn as a bar or a line")), "{:?}", errors(&pie));
}

#[test]
fn a_list_of_values_grows_with_plus() {
    let wrong = with("readings = readings + [cpu]", "readings = readings + cpu");
    assert!(errors(&wrong).iter().any(|e| e.contains("a list grows by a list of its items, as in readings + [value]")), "{:?}", errors(&wrong));
    let mixed = with("readings = readings + [cpu]", "readings = readings + [\"x\"]");
    assert!(!errors(&mixed).is_empty());
}
