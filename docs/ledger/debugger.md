# Debugger

Status: not built. Stop the real generated program at a line of Clarity, see its names, step and go on, the same on every stack and system, desktop and browser; later, debug the design.

Progress: 0%

## Code
- Spec: none
- Generators: none (planned: a Debug build writes a mark per statement and a table of places)
- Runtimes: none
- Clarity.UI: none (planned: breakpoints, the stop decision, a DAP server on 127.0.0.1 with a token, the "stopped in the debugger" band)
- IDE: none (planned: Debug beside Run, the gutter, call stack, variables, watches, event log, data view, debug console)
- Engines: Clarity engines run compiled or interpreted inside WebAssembly (`crates/clarity-wasm/src/interpreted.rs`); see [engines](engines.md)
- Docs: none

## Tests
- none (planned: conformance per stack with a small program, its places and a DAP script; headless debug builds of the examples under Xvfb; a test that a normal build has no mark and no port)

## Open
- Step 1, places and marks in the Rust generator (`debug_enter(handler, frame)`, `debug_at(place)`, values as JSON on demand), breakpoints, stepping and variables over DAP, the IDE's gutter and panels: NOT BUILT.
- Step 2, the same marks from the Go, C#, C and C++ generators, held alike by conformance cases: NOT BUILT.
- Step 3, stop on error (`refuse`, a failed save, a runtime error), conditions, hit counts, log points with slots: NOT BUILT.
- Step 4, the event log and the read-only data view: NOT BUILT.
- Step 5, watches as Clarity expressions; stepping through interpreted engines: NOT BUILT.
- Step 6, the browser target (the server's program stops, the browser shows the band) and debugging from VS Code: NOT BUILT.
- Step 7, debugging the design (pick in the app, part state, event breakpoints, design trace, live edits): NOT BUILT. Depends on designer edits by part key, still PARTIAL ([ide-designer](ide-designer.md)).
- To settle when building (needs decision): how much slower a debug build runs (measured on the examples, the games above all); how a frame hands values over without copying on each stack; whether a stopped window may repaint; the DAP crate or a small own one (a new dependency, asked for first).

## Conflicts
- none

## Decisions
- 2026-11-05: debug the real program on its stack, not an imitation; the person debugs Clarity, never the generated Go or C#.
- 2026-11-05: a Debug build marks each statement; a normal build has none and pays nothing.
- 2026-11-05: written once in Clarity.UI's shared library behind its C interface; each generator only writes the marks.
- 2026-11-05: the IDE speaks the Debug Adapter Protocol over a local socket, so VS Code can attach later.
- 2026-11-05: stopping freezes only the app, and a band on its window says why.
- 2026-11-09: debugging the design follows from the change "no translator between design and code", after it. User: "you mean remove the translator between them... that might enable debugging design too?" and "yes, add those to the debugger plan... but that are consequences of the change"
- 2026-11-09: the debugger comes last in the agreed work order, then design debugging. User: "agree"

## Was
- DEBUGGER-PLAN.md (all: decisions, features, debugging the design, pieces, order steps 1-7, to settle)
- memory: debugger-plan.md, roadmap-order.md (item 12)
