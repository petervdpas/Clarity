# Time parts

The part of the Time group: the timer, which runs a handler every so many seconds while its window is open, and how it makes live graphs with a trend or a chart.

## Keeping time

Drag **Timer** from the **Time** group of the Parts palette onto the window. A timer keeps time and is not seen: it has no place in the window's grid, so the designer keeps it in the tray under the window, with the sounds. Pick it there to set **Every (s)** and wire its **On tick** handler.

## Live graphs

A window shows numbers as they change with a timer, which runs a handler every so many seconds, and a `trend` or a chart of a list of numbers, which draw them (see [Data parts](parts.data.md)):

```xml
<!-- monitor/design/MonitorView.cxaml -->
<View class="monitor.MonitorView">
  <Data name="cpu" type="decimal" />
  <Data name="readings" type="list&lt;decimal&gt;" />
  <Timer name="clock" at="1,1" span="1" every="1" ontick="sampled" />
  <Trend name="cpuTrend" at="1,1" span="6" rows="4" bind="cpu" max="100" points="60" />
  <Chart name="history" at="1,5" span="6" rows="4" bind="readings" chart="line" max="100" points="30" none="measure" />
</View>
```

```clarity
code MonitorView
    handler sampled
        cpu = load()
        readings = readings + [cpu]
```

- A trend follows one number: every `every` seconds (1 without it) it takes the value its number has, and keeps the last `points` (60 without it). It does not wait for the timer: what code last set is what it takes.
- A list of numbers grows with `+`: `readings + [cpu]` is a new list with the item after the others; both sides are lists of the same kind of item. A chart of the list draws its last `points` numbers.

A timer that reads a sound's `position` every half second moves a slider along as a song plays; see [Sound](design.sound.md).

## The parts

### `timer`

Runs its `on tick` handler every so many seconds while its window is open.

```xml
<Timer name="name" at="col,row" span="1" every="seconds" ontick="handler" />
```

A timer keeps the `at` and `span` the designer gives it, but takes no room in the grid. In Clarity's words it has none: `timer name clock every 1 on tick: sampled`.

| Property | Type | Default | What it does |
|---|---|---|---|
| `every` | 0.1 to 86400 seconds | none: it is required | Seconds between its ticks (**Every (s)**). |
| `enabled` | yes or no, set by code | yes | `clock.enabled = false` stops it until code sets it back to true. |

Event: `tick`.

```xml
<Timer name="clock" at="1,1" span="1" every="0.5" ontick="clockTicked" />
```

```clarity
code PlayerView
    handler clockTicked
        played = music.position
```

A tick that comes while a handler still runs is left out, not saved up.

The checker refuses a timer without `every` ("write every 1, or every 0.5"), an `every` outside 0.1 to 86400 seconds ("a timer runs every 0.1 to 86400 seconds"), a timer inside another part ("a timer stands on the window, not in another part"), and `at`, `span` or `rows` on a timer in Clarity's words.

## See also

- [Data parts](parts.data.md): the trend and the chart, which draw what a timer samples.
- [Media parts](parts.media.md): the sound, whose song a timer follows.
- [Handlers](code.handlers.md): the code behind that a tick runs.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Timer | `timer` | nothing | 1 x 1 | no | `every` | `tick` |

<!-- /catalog -->
