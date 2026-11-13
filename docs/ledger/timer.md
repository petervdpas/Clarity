# Timer

Status: partial. A `timer` part with no place in the window that runs `on tick` every `every n` seconds while its window is on the screen and it is enabled.

Progress: 75%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs:1362` (`WidgetKind::Timer`, palette Time, `Event::Tick`; placeless kinds `off_grid` :1492)
- Generators: written into the window markup as any part (`<Timer>`), left out of the grid (`crates/clarity-gen/src/web_windows.rs`)
- Runtimes: each tick runs the handler as an event of the window (`dispatch_event`); `clock.enabled` is part state in every runtime
- Clarity.UI: `runtimes/rust-ui/src/parts/timer.rs` (ticks are the screen's wake-ups; starts as its window joins the screen, stops as it leaves; `every` kept between 0.1 s and a day)
- IDE: the designer's tray under the window, with the sounds (`app/src/lib/PageDesigner.svelte:622-624`)
- Docs: manual `parts.time.md` (`timer`, Live graphs), `design.sound.md`

## Tests
- No conformance file (`conformance/timers.json` does not exist).
- spec `live_charts.rs`; gen `timers_and_sounds.rs`, `live_charts.rs`
- rust-ui `tests/windows/windows_ticks.rs`

## Open
- `once`, a settable `every`, `start()` and `stop()`, `conformance/timers.json`: PARTIAL. Code starts and stops a timer through `enabled` only (DESIGN.md:2490).

## Conflicts
- none

## Decisions
- 2026-10-25: a tick that comes while a handler runs is left out, not saved up; time a long handler took is not made up.
- 2026-10-31: the clock is the screen's; what a tick does is the runtime's handler.

## Was
- DESIGN 1.7 (timer row of the table), 5.26 (the timer)
- Roadmap: TI1
