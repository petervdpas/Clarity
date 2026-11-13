# Charts and trends

Status: built. `chart` draws a list the window holds as bars, a line or a pie (grouped by a field, or a list of numbers as they are); `trend` follows one number over time.

Progress: 95%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (`Chart`, `Trend`), `model.rs` (`ChartKind`, `ChartMeasure`), `check.rs` (`check_number_chart`: a pie or `by` refused on a list of numbers)
- Generators: through window markup (`crates/clarity-gen/src/web_windows.rs`)
- Runtimes: groups decided in the runtime and sent in the part's view (DESIGN 1.7): R `runtimes/rust/src/charts.rs`, Go `clarity/charts.go`, C `clarity/charts.c`, C++ `clarity/charts.cpp`, C# `Charts.cs`
- Clarity.UI: `runtimes/rust-ui/src/parts/data/chart.rs`, `data/trend.rs` (own clock, `every`, `points`), `data/groups.rs`
- IDE: `app/src/lib/PageProperties.svelte`, `PartTile.svelte` (the designer shows a still sample of a trend)
- Docs: manual `parts.data.md` (`chart`, `trend`), `parts.time.md` (Live graphs)

## Tests
- `conformance/charts.json`: run by R, Go, C, C++, rust-ui; skipped by C# (`Charts.cs` names it in a comment, no test loads it)
- spec `charts.rs`, `live_charts.rs`; gen `charts.rs`, `live_charts.rs` (a monitor window on every stack)
- rust-ui `tests/data/data_chart.rs`, `data_trend.rs`

## Open
- Chart grouping in one place: PARTIAL. Still in `runtimes/rust/src/charts.rs` and `runtimes/rust-ui/src/parts/data/groups.rs` (DESIGN.md:2000), each checked against the same cases.

## Conflicts
- Chart groups decided twice in Rust: `runtimes/rust/src/charts.rs` and `runtimes/rust-ui/src/parts/data/groups.rs`.
- C# `Charts.cs` has no callers outside tests.

## Decisions
- 2026-10-23: `chart bar|line|pie ... by field count|sum f|average f`, grouped over the rows the window holds (the user's choice).
- 2026-10-25: both forms for live numbers: a chart of a list of numbers (`points n`, `max n`) and a `trend` part of its own (the user chose both).
- 2026-10-31: a chart's groups and a trend's points are the runtime's; every reply carries them.

## Was
- DESIGN 1.7 (chart row of the table), 5.26 (chart of numbers, trend)
- Roadmap: U1c (chart)
